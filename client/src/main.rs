use shared::{ServerRequest, ServerResponse, UserProfile};
use std::time::Duration;
use uuid::Uuid;
use yarws::{Client, Msg};

enum LoginState {
    LoggedOut,
    Connected { installation_id: Uuid },
    Authenticated { profile: UserProfile },
}

fn main() {
    SingleWindowApplication::run(block_on(CosmicCantina::new()));
}

async fn network_loop() {
    loop {
        let mut login_state = LoginState::LoggedOut;
        let socket = match Client::new("ws://localhost:7878/ws").connect().await {
            Ok(socket) => socket,
            Err(err) => {
                println!("Error connecting to socket. {}", err);
                tokio::time::delay_for(Duration::from_millis(100)).await;
                continue;
            }
        };
        let (mut tx, mut rx) = socket.into_channel().await;
        tx.send(Msg::Binary(
            bincode::serialize(&ServerRequest::Authenticate {
                installation_id: None,
            })
            .unwrap(),
        ))
        .await
        .unwrap_or_default();
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Binary(bytes) => match bincode::deserialize::<ServerResponse>(&bytes) {
                    Ok(response) => match response {
                        ServerResponse::Error { message } => {
                            println!("Authentication error {:?}", message);
                        }
                        ServerResponse::AdoptInstallationId { installation_id } => {
                            println!("Received app token {}", installation_id);
                            login_state = LoginState::Connected { installation_id };
                        }
                        ServerResponse::Authenticated { profile } => {
                            println!("Authenticated as {}", profile.username);
                            login_state = LoginState::Authenticated { profile };
                        }
                        ServerResponse::AuthenticateAtUrl { url } => {
                            webbrowser::open(&url).expect("Error launching URL");
                        }
                    },
                    Err(_) => println!("Error deserializing message."),
                },
                _ => {}
            }
        }
    }
}

use futures::executor::block_on;
use kludgine::prelude::*;

struct CosmicCantina {
    ui: UserInterface,
}

impl CosmicCantina {
    async fn new() -> Self {
        Self {
            ui: Self::create_interface().await.unwrap(),
        }
    }

    async fn initialize(&mut self) -> KludgineResult<()> {
        Runtime::spawn(network_loop());
        Ok(())
    }
    async fn create_interface() -> KludgineResult<UserInterface> {
        let grid = Component::new(
            Grid::new(1, 2)
                .with_cell(
                    Point::new(0, 0),
                    Component::new(Interface {
                        message: "Cosmic Cantina",
                    }),
                )?
                .with_cell(
                    Point::new(0, 1),
                    Component::new(
                        Grid::new(2, 1)
                            .with_cell(
                                Point::new(0, 0),
                                Component::new(ViewController::new(
                                    Label::default()
                                        .with_value("programmed by @khonsulabs")
                                        .with_style(Style {
                                            font_size: Some(14.0),
                                            color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
                                            ..Default::default()
                                        })
                                        .with_hover_style(Style {
                                            color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                                            ..Default::default()
                                        })
                                        .with_padding(Surround::uniform(Dimension::Auto))
                                        .build()?,
                                )),
                            )?
                            .with_cell(
                                Point::new(1, 0),
                                Component::new(ViewController::new(
                                    Label::default()
                                        .with_value("visualized by @whitevaultstudios")
                                        .with_style(Style {
                                            font_size: Some(14.0),
                                            color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
                                            ..Default::default()
                                        })
                                        .with_hover_style(Style {
                                            color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                                            ..Default::default()
                                        })
                                        .with_padding(Surround::uniform(Dimension::Auto))
                                        .build()?,
                                )),
                            )?,
                    ),
                )?,
        );
        let ui = UserInterface::new(Style::default());
        ui.set_root(grid).await;
        Ok(ui)
    }
}

impl WindowCreator<CosmicCantina> for CosmicCantina {
    fn window_title() -> String {
        "Cosmic Cantina".to_owned()
    }
}

#[async_trait]
impl Window for CosmicCantina {
    async fn render<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        self.ui.render(scene).await?;

        Ok(())
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        self.ui.process_input(event).await.map(|_| ())
    }
}

#[derive(Debug)]
struct Interface {
    message: &'static str,
}

#[async_trait]
impl Controller for Interface {
    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
        Label::default()
            .with_value(self.message)
            .with_style(Style {
                font_size: Some(60.0),
                color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
                ..Default::default()
            })
            .with_hover_style(Style {
                color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                ..Default::default()
            })
            .with_padding(Surround::uniform(Dimension::Auto))
            .build()
    }
}
