mod assets;
mod network;
use assets::Assets;
use network::{LoginState, Network};
use shared::{current_timestamp, Inputs, NpcModel, ServerRequest, UserProfile, WALK_SPEED};
use std::collections::HashMap;

fn main() {
    dotenv::dotenv().unwrap_or_default();
    SingleWindowApplication::run_with(|| CosmicCantina::new());
}

use kludgine::prelude::*;
mod config;

enum GameState {
    MainMenu(MainMenuState),
    Diner(DinerState),
}

#[derive(Default)]
struct MainMenuState {
    x_offset: f32,
    pan_left: bool,
}

#[derive(Default)]
struct DinerState {
    x_offset: f32,
    players: HashMap<i64, Player>,
    last_update: Option<f64>,
}

struct Player {
    profile: UserProfile,
    sprite: Sprite,
}

struct CosmicCantina {
    state: GameState,
    assets: Assets,
    you: Sprite,
}

const BACKDROP_SIZE: Size = Size::new(662f32, 214f32);

impl CosmicCantina {
    async fn new() -> Self {
        let assets = Assets::load().await.unwrap();
        Self {
            state: GameState::Diner(DinerState::default()),
            you: assets.npcs[&NpcModel::GreenGuy].new_instance().await,
            assets,
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
        self.you.set_current_tag(Some("idle")).await?;
        let background_scale = self.background_scale(scene).await;
        match &mut self.state {
            GameState::MainMenu(main_menu) => {
                // if let Some(elapsed) = scene.elapsed() {
                //     let max_x_offset = BACKDROP_SIZE.width - scene.size().width / background_scale;
                //     let pan_direction = if main_menu.pan_left { -1.0 } else { 1.0 };
                //     let delta_to_center = (max_x_offset / 2.0 - main_menu.x_offset).abs();
                //     let percent_from_center = delta_to_center / (max_x_offset / 2.0);
                //     let speed = (0.2 + (1.0 - percent_from_center) * 0.8) * 16.0;
                //     main_menu.x_offset =
                //         main_menu.x_offset + speed * pan_direction * elapsed.as_secs_f32();
                //     if main_menu.x_offset <= 0.0 {
                //         main_menu.x_offset = 0.0;
                //         main_menu.pan_left = false;
                //     } else if main_menu.x_offset >= max_x_offset {
                //         main_menu.x_offset = max_x_offset;
                //         main_menu.pan_left = true;
                //     }
                // }
            }
            GameState::Diner(diner) => {
                let horizontal_movement = if scene.key_pressed(VirtualKeyCode::Left)
                    || scene.key_pressed(VirtualKeyCode::A)
                {
                    -1.0
                } else if scene.key_pressed(VirtualKeyCode::Right)
                    || scene.key_pressed(VirtualKeyCode::D)
                {
                    1.0
                } else {
                    0.0
                };

                diner.x_offset += horizontal_movement
                    * WALK_SPEED
                    * scene.elapsed().unwrap_or_default().as_secs_f32();

                diner.x_offset = diner.x_offset.max(0.0);

                let now = current_timestamp();
                if diner.last_update.map(|t| t + 0.1 <= now).unwrap_or(true) {
                    Network::request(ServerRequest::Update {
                        new_inputs: Some(Inputs {
                            horizontal_movement,
                            interact: false,
                        }),
                        x_offset: diner.x_offset,
                        timestamp: now,
                    })
                    .await;
                    diner.last_update = Some(now);
                }

                if let Some((world_timestamp, profiles)) = Network::last_world_update().await {
                    println!("Got world update with {} players", profiles.len());
                    let elapsed = (now - world_timestamp) as f32;
                    for profile in profiles {
                        let player = diner
                            .players
                            .entry(profile.id)
                            .and_modify(|player| {
                                player.profile = profile.clone();
                            })
                            .or_insert(Player {
                                profile: profile,
                                sprite: self.assets.npcs[&NpcModel::OrangeGuy].new_instance().await,
                            });

                        player.sprite.set_current_tag(Some("idle")).await?;
                        // Update the position based on the time
                        player.profile.x_offset +=
                            elapsed * WALK_SPEED * player.profile.horizontal_input;
                    }
                } else {
                    for player in diner.players.values_mut() {
                        player.profile.x_offset +=
                            scene.elapsed().unwrap_or_default().as_secs_f32()
                                * WALK_SPEED
                                * player.profile.horizontal_input;
                    }
                }
            }
        }

        Ok(())
    }

    async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        match &self.state {
            GameState::MainMenu(main_menu) => self.render_main_menu(main_menu, scene).await,
            GameState::Diner(diner) => self.render_diner(diner, scene).await,
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
        self.render_inside_scene(None, main_menu.x_offset, scene)
            .await?;
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

    async fn render_diner<'a>(
        &self,
        diner: &DinerState,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<()> {
        self.render_inside_scene(diner.players.values(), diner.x_offset, scene)
            .await?;
        self.render_network_status(scene).await
    }

    async fn render_network_status<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let text = match Network::login_state().await {
            LoginState::Authenticated { profile } => Text::span(
                format!(
                    "Logged in as @{} - {:.2}ms",
                    profile.username,
                    Network::ping().await * 1_000.0
                ),
                &Style {
                    font_family: Some("Press Start 2P".to_owned()),
                    font_size: Some(8.0),
                    color: Some(Color::new(0.5, 1.0, 0.5, 1.0)),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            LoginState::Connected { .. } => Text::span(
                format!("Connected - {:.2}ms", Network::ping().await * 1_000.0),
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

    async fn render_inside_scene<'a, 'b, I: IntoIterator<Item = &'b Player>>(
        &self,
        players: I,
        x_offset: f32,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<()> {
        let backdrop_scale = self.background_scale(scene).await;
        let mut zoomed = scene.set_zoom(backdrop_scale);
        let screen_left_offset = x_offset - zoomed.size().width / 2.0;
        let sprite_offset = Point::new(x_offset - screen_left_offset - 63.0 / 2.0, 180.0 - 48.0);

        let frame = self.assets.backdrop_1.get_frame(zoomed.elapsed()).await?;
        frame
            .render_at(&mut zoomed, Point::new(-screen_left_offset, 0.0))
            .await;

        for player in players {
            let render = match Network::login_state().await {
                LoginState::Authenticated { profile } => true, //profile.id != player.profile.id,
                _ => true,
            };

            if render {
                let frame = player.sprite.get_frame(zoomed.elapsed()).await?;
                frame
                    .render_at(
                        &mut zoomed,
                        Point::new(player.profile.x_offset - sprite_offset.x, sprite_offset.y),
                    )
                    .await;
            }
        }

        let frame = self.you.get_frame(zoomed.elapsed()).await?;
        frame.render_at(&mut zoomed, sprite_offset).await;
        Ok(())
    }

    async fn background_scale<'a>(&self, scene: &mut SceneTarget<'a>) -> f32 {
        scene.size().height / BACKDROP_SIZE.height
    }
}
