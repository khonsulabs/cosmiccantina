use super::NpcModel;
use kludgine::prelude::*;
use std::collections::HashMap;

pub struct Assets {
    pub logo: SourceSprite,
    pub backdrop_1: Sprite,
    pub press_start_2p: Font,
    pub npcs: HashMap<NpcModel, Sprite>,
}

impl Assets {
    pub async fn load() -> KludgineResult<Self> {
        let mut npcs = HashMap::new();
        npcs.insert(
            NpcModel::GreenGuy,
            Sprite::merged(vec![
                (
                    "idle",
                    include_aseprite_sprite!("../../assets/whitevaultstudios/NPC_01_Wait").await?,
                ),
                (
                    "sitting",
                    include_aseprite_sprite!("../../assets/whitevaultstudios/NPC_01_sitting")
                        .await?,
                ),
            ])
            .await,
        );
        npcs.insert(
            NpcModel::OrangeGuy,
            Sprite::merged(vec![
                (
                    "idle",
                    include_aseprite_sprite!("../../assets/whitevaultstudios/NPC_03_Wait").await?,
                ),
                (
                    "sitting",
                    include_aseprite_sprite!("../../assets/whitevaultstudios/NPC_03_sitting")
                        .await?,
                ),
            ])
            .await,
        );
        Ok(Self {
            logo: SourceSprite::entire_texture(include_texture!(
                "../../assets/whitevaultstudios/title_with_subtitle.png"
            )?)
            .await,
            backdrop_1: include_aseprite_sprite!(
                "../../assets/whitevaultstudios/Bistro_Full_Backdrop"
            )
            .await?,
            press_start_2p: include_font!(
                "../../assets/fonts/PressStart2P/PressStart2P-Regular.ttf"
            ),
            npcs,
        })
    }
}
