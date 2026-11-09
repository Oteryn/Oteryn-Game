//! Account character selection; credentials never enter the view model.
use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
use oteryn_platform_contracts::DirectorySnapshot;

#[cfg(windows)]
mod native;

#[derive(Debug, Clone)]
pub struct CharacterRow {
    pub character: OwnerCharacter,
    pub world_name: String,
    pub channels: Vec<(String, String)>,
    pub available: bool,
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
                        .map(|channel| {
                            (
                                channel.channel_ref.as_str().to_owned(),
                                channel.display_name.as_str().to_owned(),
                            )
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
        let directory = oteryn_platform_client::parse_directory(br#"{"epoch":1,"worlds":[{"id":"world","name":"Alpha","channels":[{"id":"channel","name":"First"}],"characters":[]}]}"#)?;
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
        assert_eq!(rows[0].channels, [("channel".into(), "First".into())]);
        assert!(rows[0].available);
        assert!(!rows[1].available);
        assert!(!rows[2].available);
        assert!(rows[2].channels.is_empty());
        Ok(())
    }
}
