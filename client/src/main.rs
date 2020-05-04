mod network;
use network::{LoginState, Network};
use shared::ServerRequest;

fn main() {
    SingleWindowApplication::run_with(|| CosmicCantina::new());
}

use kludgine::prelude::*;

enum GameState {
    MainMenu,
    Outside,
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
            state: GameState::MainMenu,
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

    async fn render<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        match &self.state {
            GameState::MainMenu => self.render_main_menu(scene).await,
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
    async fn render_main_menu<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        self.render_inside_scene(scene).await?;
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

    async fn render_outside<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        Ok(())
    }

    async fn render_network_status<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<()> {
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
            LoginState::Error => Text::span(
                "Error connecting. Retrying...",
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

    async fn render_inside_scene<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        self.assets
            .backdrop_1
            .render_at(scene, Point::new(0.0, 0.0))
            .await;
        Ok(())
    }
}
