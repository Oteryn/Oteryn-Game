//! Account character selection; credentials never enter the view model.
use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
use oteryn_platform_contracts::{DirectoryPvpType, DirectorySnapshot};

mod training;
pub use training::{TrainingQueue, TrainingSelection};

#[cfg(windows)]
mod native;

#[derive(Debug, Clone)]
pub struct CharacterRow {
    pub character: OwnerCharacter,
    pub world_name: String,
    pub channels: Vec<ChannelChoice>,
    pub available: bool,
}

/// One Channel of the character's World as the Platform directory lists it (client-directory-v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelChoice {
    pub id: String,
    pub name: String,
    pub pvp_type: DirectoryPvpType,
    pub region: String,
}

impl ChannelChoice {
    /// The PvP type column of the Channel list (GAME-CHANNEL-01).
    #[must_use]
    pub fn pvp_label(&self) -> &'static str {
        match self.pvp_type {
            DirectoryPvpType::OptionalPvp => "Optional PvP",
            DirectoryPvpType::OpenPvp => "Open PvP",
        }
    }

    /// `name · PvP type · region`, the label of the selection-screen Channel button.
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} · {} · {}", self.name, self.pvp_label(), self.region)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionAction {
    Play {
        character_id: String,
        channel_id: Option<String>,
    },
    Train {
        character_id: String,
        channel_id: Option<String>,
    },
    TrainMany {
        characters: Vec<TrainingSelection>,
    },
    Create,
    AccountServices,
    Refresh,
    Cancel,
}

pub type CharacterSelector = std::sync::Arc<
    dyn Fn(&[CharacterRow], &str, oteryn_foundation::CancellationToken) -> SelectionAction
        + Send
        + Sync,
>;

pub fn join(characters: Vec<OwnerCharacter>, directory: &DirectorySnapshot) -> Vec<CharacterRow> {
    characters
        .into_iter()
        .map(|character| {
            let world = directory
                .worlds
                .iter()
                .find(|world| world.world_ref.as_str() == character.world_id);
            CharacterRow {
                available: character.availability == CharacterAvailability::Available
                    && world.is_some(),
                world_name: world.map_or_else(
                    || tr!("World unavailable").into(),
                    |world| world.display_name.as_str().to_owned(),
                ),
                channels: world.map_or_else(Vec::new, |world| {
                    world
                        .channels
                        .iter()
                        .map(|channel| ChannelChoice {
                            id: channel.channel_ref.as_str().to_owned(),
                            name: channel.display_name.as_str().to_owned(),
                            pvp_type: channel.pvp_type,
                            region: channel.region.as_str().to_owned(),
                        })
                        .collect()
                }),
                character,
            }
        })
        .collect()
}

pub fn select(
    rows: &[CharacterRow],
    preselect: &str,
    cancellation: oteryn_foundation::CancellationToken,
) -> SelectionAction {
    #[cfg(windows)]
    {
        native::select(rows, preselect, cancellation)
    }
    #[cfg(not(windows))]
    {
        let _ = (rows, preselect, cancellation);
        SelectionAction::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worlds_join_by_id_and_unavailable_rows_cannot_play() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = oteryn_platform_client::parse_directory(br#"{"epoch":1,"worlds":[{"id":"world","name":"Alpha","channels":[{"id":"channel","name":"First","pvp_type":"OPTIONAL_PVP","region":"EU"}],"characters":[]}]}"#)?;
        let character = OwnerCharacter {
            character_id: "owner".into(),
            world_id: "world".into(),
            name: "Ari".into(),
            availability: CharacterAvailability::Available,
        };
        let mut unavailable = character.clone();
        unavailable.availability = CharacterAvailability::Unavailable;
        let mut missing = character.clone();
        missing.world_id = "absent".into();
        let rows = join(vec![character, unavailable, missing], &directory);
        assert_eq!(rows[0].world_name, "Alpha");
        assert_eq!(
            rows[0].channels,
            [ChannelChoice {
                id: "channel".into(),
                name: "First".into(),
                pvp_type: DirectoryPvpType::OptionalPvp,
                region: "EU".into(),
            }]
        );
        assert_eq!(rows[0].channels[0].label(), "First · Optional PvP · EU");
        assert!(rows[0].available);
        assert!(!rows[1].available);
        assert!(!rows[2].available);
        assert!(rows[2].channels.is_empty());
        Ok(())
    }
}
