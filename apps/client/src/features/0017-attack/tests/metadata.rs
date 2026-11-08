use crate::play::feature_0017_attack::view::GuildFilter;
use crate::play::{PreferenceStore, ReferencePreferences, play_channel};
use oteryn_client_runtime::cui3::battle_metadata::{PlayerMetadata, decode_query, encode_reply};
use oteryn_session::{
    ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
};
#[test]
fn battle_profession_and_guild_filters_query_real_metadata_and_attack_the_filtered_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let actor = |id, health| WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 1,
            y: 2,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: health,
        },
    };
    let knight = actor(1, 70);
    let mage = actor(2, 40);
    let unknown = actor(3, 30);
    let (link, mut commands, outbox) = play_channel();
    outbox.battle_publish(
        Some(oteryn_session::ActorCombatState {
            target: None,
            modes: oteryn_session::FightModes::DEFAULT,
            in_fight: false,
        }),
        Some(7),
        &[knight, mage, unknown],
    );
    outbox.battle_metadata_initialize(true);
    let mut prefs = ReferencePreferences::default();
    prefs.battle_lists[0].vocation = Some(4);
    prefs.battle_lists[0].guild = GuildFilter::SameGuild;
    let mut active = 0;
    link.sync_battle_list_state(&mut prefs.battle_lists, &mut active, 0);
    assert!(link.filtered_battle_rows(0).is_empty());
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("metadata command".into());
    };
    assert_eq!((command.capability, command.command_type), (17, 736));
    assert_eq!(decode_query(&command.payload).ok_or("query")?.len(), 3);
    assert!(link.filtered_battle_rows(0).is_empty());
    assert!(commands.try_recv().is_err());
    outbox.battle_metadata_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: encode_reply(&[
                PlayerMetadata {
                    entity: knight.entity,
                    vocation: Some(4),
                    same_guild: Some(true),
                },
                PlayerMetadata {
                    entity: mage.entity,
                    vocation: Some(1),
                    same_guild: Some(false),
                },
                PlayerMetadata {
                    entity: unknown.entity,
                    vocation: None,
                    same_guild: None,
                },
            ])
            .ok_or("reply")?,
        },
    );
    assert_eq!(link.filtered_battle_rows(0), vec![knight]);
    link.battle_request(oteryn_client_ui::battle::Action::Attack(
        link.filtered_battle_rows(0)[0].entity,
    ));
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Battle(crate::combat_input::CombatCommand::Attack(entity)) if entity==knight.entity)
    );
    let path =
        std::env::temp_dir().join(format!("battle-metadata-prefs-{}.json", std::process::id()));
    let mut store = PreferenceStore::defaults(path.clone());
    store.apply(prefs.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?
            .preferences()
            .battle_lists,
        prefs.battle_lists
    );
    let replacement = WorldSpatialEntity {
        entity: EntityRef {
            generation: 2,
            ..knight.entity
        },
        ..knight
    };
    outbox.battle_publish(link.battle_panel().combat, Some(7), &[replacement]);
    assert!(
        link.filtered_battle_rows(0).is_empty(),
        "metadata must not cross actor generations"
    );
    std::fs::remove_file(path)?;
    Ok(())
}
