mod network;
use network::Network;

fn main() {
    SingleWindowApplication::run(block_on(CosmicCantina::new()));
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

    async fn initialize(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        Network::spawn().await;
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
