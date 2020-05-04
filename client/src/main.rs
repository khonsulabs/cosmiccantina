mod network;
use network::{LoginState, Network};
use shared::ServerRequest;

fn main() {
    dotenv::dotenv().unwrap_or_default();
    SingleWindowApplication::run_with(|| CosmicCantina::new());
}

use kludgine::prelude::*;

enum GameState {
    MainMenu(MainMenuState),
    Outside,
}

#[derive(Default)]
struct MainMenuState {
    x_offset: f32,
    pan_left: bool,
}

struct CosmicCantina {
    state: GameState,
    assets: Assets,
}

struct Assets {
    logo: SourceSprite,
    backdrop_1: SourceSprite,
    press_start_2p: Font,
}

impl Assets {
    async fn load() -> Self {
        Self {
            logo: SourceSprite::entire_texture(
                include_texture!("../../assets/whitevaultstudios/title_with_subtitle.png").unwrap(),
            )
            .await,
            backdrop_1: SourceSprite::entire_texture(
                include_texture!("../../assets/whitevaultstudios/BackDrop_01.png").unwrap(),
            )
            .await,
            press_start_2p: include_font!(
                "../../assets/fonts/PressStart2P/PressStart2P-Regular.ttf"
            ),
        }
    }
}

impl CosmicCantina {
    async fn new() -> Self {
        Self {
            state: GameState::MainMenu(MainMenuState::default()),
            assets: Assets::load().await,
        }
    }
}

impl WindowCreator<CosmicCantina> for CosmicCantina {
    fn window_title() -> String {
        "Cosmic Cantina".to_owned()
    }
}

#[async_trait]
impl Window for CosmicCantina {
    async fn initialize(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        scene.register_font(&self.assets.press_start_2p).await;
        Network::spawn().await;
        Ok(())
    }

    async fn update<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let background_scale = self.background_scale(scene).await;
        match &mut self.state {
            GameState::MainMenu(main_menu) => {
                if let Some(elapsed) = scene.elapsed() {
                    let max_x_offset = self.assets.backdrop_1.size().await.width as f32
                        - scene.size().width / background_scale;
                    let pan_direction = if main_menu.pan_left { -1.0 } else { 1.0 };
                    let delta_to_center = (max_x_offset / 2.0 - main_menu.x_offset).abs();
                    let percent_from_center = delta_to_center / (max_x_offset / 2.0);
                    let speed = (0.2 + (1.0 - percent_from_center) * 0.8) * 16.0;
                    main_menu.x_offset =
                        main_menu.x_offset + speed * pan_direction * elapsed.as_secs_f32();
                    if main_menu.x_offset <= 0.0 {
                        main_menu.x_offset = 0.0;
                        main_menu.pan_left = false;
                    } else if main_menu.x_offset >= max_x_offset {
                        main_menu.x_offset = max_x_offset;
                        main_menu.pan_left = true;
                    }
                }
            }
            GameState::Outside => {}
        }
        Ok(())
    }

    async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        match &self.state {
            GameState::MainMenu(main_menu) => self.render_main_menu(main_menu, scene).await,
            GameState::Outside => self.render_outside(scene).await,
        }
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseButton { .. } => {
                Network::request(ServerRequest::AuthenticationUrl).await;
            }
            _ => {}
        }
        Ok(())
    }
}

impl CosmicCantina {
    async fn render_main_menu<'a>(
        &self,
        main_menu: &MainMenuState,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<()> {
        self.render_inside_scene(main_menu.x_offset, scene).await?;
        let logo_size = self.assets.logo.size().await;
        self.assets
            .logo
            .render_at(
                scene,
                Point::new(
                    (scene.size().width - logo_size.width as f32) / 2.0,
                    (scene.size().height - logo_size.height as f32) / 2.0,
                ),
            )
            .await;
        self.render_network_status(scene).await
    }

    async fn render_outside<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        Ok(())
    }

    async fn render_network_status<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let text = match Network::login_state().await {
            LoginState::Authenticated { profile } => Text::span(
                format!("Logged in as @{}", profile.username),
                &Style {
                    font_family: Some("Press Start 2P".to_owned()),
                    font_size: Some(8.0),
                    color: Some(Color::new(0.5, 1.0, 0.5, 1.0)),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            LoginState::Connected { .. } => Text::span(
                format!("Connected"),
                &Style {
                    font_family: Some("Press Start 2P".to_owned()),
                    font_size: Some(8.0),
                    color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            LoginState::LoggedOut => Text::span(
                "Connecting...",
                &Style {
                    font_family: Some("Press Start 2P".to_owned()),
                    font_size: Some(8.0),
                    color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            LoginState::Error { message } => Text::span(
                format!("Error connecting. {}", message.unwrap_or_default()),
                &Style {
                    font_family: Some("Press Start 2P".to_owned()),
                    font_size: Some(8.0),
                    color: Some(Color::new(1.0, 0.5, 0.5, 1.0)),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
        };
        text.render_at(scene, Point::new(5.0, 5.0), TextWrap::NoWrap)
            .await
    }

    async fn render_inside_scene<'a>(
        &self,
        x_offset: f32,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<()> {
        let backdrop_size = self.assets.backdrop_1.size().await;
        let backdrop_scale = self.background_scale(scene).await;
        let mut zoomed = scene.set_camera(backdrop_scale, Point::new(0.0, 0.0));
        let zoomed_size = zoomed.size();
        self.assets
            .backdrop_1
            .render_at(
                &mut zoomed,
                Point::new(
                    zoomed_size.width as f32 / -2.0 - x_offset,
                    backdrop_size.height as f32 / -2.0,
                ),
            )
            .await;
        Ok(())
    }

    async fn background_scale<'a>(&self, scene: &mut SceneTarget<'a>) -> f32 {
        let backdrop_size = self.assets.backdrop_1.size().await;
        scene.size().height / backdrop_size.height as f32
    }
}
