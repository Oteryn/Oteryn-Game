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
        motion: None,
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
    assert_eq!((command.capability, command.command_type), (17, 740));
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
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.party_skull.icons = vec![
            oteryn_client_runtime::party_skull::PartyIcon {
                entity: knight.entity,
                shield: 4,
                skull: 0,
            },
            oteryn_client_runtime::party_skull::PartyIcon {
                entity: mage.entity,
                shield: 0,
                skull: 3,
            },
        ];
    }
    for (field, expected) in [
        ("hide_party", vec![mage, unknown]),
        ("hide_non_skull", vec![mage]),
    ] {
        let mut lists = vec![serde_json::from_value(serde_json::json!({(field): true}))?];
        link.sync_battle_list_state(&mut lists, &mut active, 0);
        assert_eq!(link.filtered_battle_rows(0), expected);
    }
    for (hidden, expected) in [(4, vec![mage, unknown]), (1, vec![knight, unknown])] {
        let mut lists = vec![serde_json::from_value(
            serde_json::json!({"hidden_vocations": [hidden]}),
        )?];
        link.sync_battle_list_state(&mut lists, &mut active, 0);
        assert_eq!(link.filtered_battle_rows(0), expected);
    }
    link.sync_battle_list_state(&mut prefs.battle_lists, &mut active, 0);
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("battle-metadata-prefs.json");
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

#[test]
fn confirmed_party_summons_are_excluded_only_from_lists_that_hide_them()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::party::party;
    let (link, mut commands, outbox) = play_channel();
    let owner = EntityRef {
        identity: [7; 16],
        generation: 1,
    };
    let pet = EntityRef {
        identity: [8; 16],
        generation: 1,
    };
    let mut party = party::PartyResult::refused(party::PartyDisposition::Ok);
    let entry = |entity, summon, name: &str| party::PartyListEntity {
        entity,
        name: name.into(),
        vocation: 0,
        summon,
        distance: 3,
        health_percent: Some(100),
        mana_percent: Some(0),
        skull: Some(0),
        portrait: None,
        portrait_colours: [[0; 3]; 4],
    };
    party.members.push(party::PartyEntry {
        handle: 1,
        channel: None,
        name: "Mage".into(),
        leader: true,
        expires_at: 0,
        party_spell_remaining_ms: None,
        list: vec![entry(owner, false, "Mage"), entry(pet, true, "Wolf")],
    });
    let bytes = party::encode_party_result(&party).map_err(|e| format!("{e:?}"))?;
    outbox.party_initialize(true, false);
    let creature = WorldSpatialEntity {
        entity: pet,
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: 1,
            y: 3,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            health_percent: 100,
            appearance_ref: 1,
        },
        display_name: None,
        motion: None,
    };
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(1, 1), 7, &[])?
        .with_entities(vec![creature])?;
    view.tick(&link).map_err(|_| "tick")?;
    let plain = view.scene().sprites().len();
    assert!(!link.party_is_open());
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Party(_)
    ));
    outbox.party_result(&oteryn_client_runtime::party::PartyResponse {
        status: oteryn_session::CommandStatus::Accepted,
        operation: bytes.clone(),
        view: Some(bytes),
    });
    view.tick(&link).map_err(|_| "tick")?;
    assert!(
        view.scene().sprites().len() > plain,
        "known summon draws its overhead badge"
    );
    outbox.battle_publish(None, Some(7), &[creature]);
    for hide in [true, false] {
        let mut lists = vec![serde_json::from_value(
            serde_json::json!({"hide_summons":hide}),
        )?];
        link.sync_battle_lists(&mut lists, 0);
        assert_eq!(link.filtered_battle_rows(0).contains(&creature), !hide);
    }
    Ok(())
}

#[test]
fn battle_editor_independently_hides_each_profession() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::FeatureForm;
    let (link, mut commands, outbox) = play_channel();
    let rows: Vec<_> = (1_u8..=5)
        .map(|family| WorldSpatialEntity {
            entity: EntityRef {
                identity: [family; 16],
                generation: 7,
            },
            kind: EntityKind::Player,
            position: ActorPosition {
                x: i32::from(family),
                y: 1,
                floor: 7,
            },
            display_name: None,
            motion: None,
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                health_percent: 100,
                appearance_ref: 1,
            },
        })
        .collect();
    outbox.battle_publish(None, Some(7), &rows);
    outbox.battle_metadata_initialize(true);
    link.battle_list_editor_open();
    let form = FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(&link) == "Battle lists")
        .ok_or("list editor")?;
    for (index, name) in ["sorcerer", "druid", "paladin", "knight", "monk"]
        .iter()
        .enumerate()
    {
        let mut values = link.feature_form_view(form).fields;
        values[8] = (*name).to_owned();
        link.feature_form_submit(form, 0, values);
        if index == 0 {
            let _ = link.filtered_battle_rows(0);
            let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()?
            else {
                return Err("metadata command".into());
            };
            let facts = decode_query(&command.payload)
                .ok_or("query")?
                .into_iter()
                .map(|entity| PlayerMetadata {
                    entity,
                    vocation: Some(entity.identity[0]),
                    same_guild: None,
                })
                .collect::<Vec<_>>();
            outbox.battle_metadata_result(
                &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: encode_reply(&facts).ok_or("reply")?,
                },
            );
        }
        let expected = rows
            .iter()
            .copied()
            .filter(|r| r.entity != rows[index].entity)
            .collect::<Vec<_>>();
        assert_eq!(link.filtered_battle_rows(0), expected, "{name}");
    }
    Ok(())
}
