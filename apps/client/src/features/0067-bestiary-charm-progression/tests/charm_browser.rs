use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    charm_progression::{self as wire, catalogue as cards},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::charm_progression::Intent;
use std::num::NonZeroU32;
#[test]
fn named_charms_and_creatures_drive_the_real_progression_caller()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    lock(&link.mailbox).properties.charm_progression.selected = true;
    link.charm_catalogue_request(false);
    let PlayCommand::Feature(metadata) = commands.try_recv()? else {
        return Err("metadata command".into());
    };
    assert_eq!((metadata.capability, metadata.kind), (67, 84));
    assert_eq!(
        cards::decode_charm_request(&metadata.payload).map_err(|e| format!("{e:?}"))?,
        0
    );
    link.charm_catalogue_request(false);
    assert!(commands.try_recv().is_err());
    let page = cards::Page { next: None, categories: vec!["Major".into()], cards: vec![cards::Card { outfit_look_type: 0, object_look_type: None,
        race: 1, name: "Dodge".into(), category: "Major".into(), detail: "Avoid a qualifying creature attack.\r\nStage 1: 240 Charm Points; trigger chance 5.00%".into(),
    }] };
    let accepted = |payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    };
    outbox.properties_result(
        &metadata,
        &accepted(cards::encode_page(&page).map_err(|e| format!("{e:?}"))?),
    );
    assert_eq!(
        link.charm_progression_panel()
            .charm_browser
            .cards
            .get(&1)
            .map(|c| c.name.as_str()),
        Some("Dodge")
    );
    // The creature name also comes through the real catalogue adapter.
    link.bestiary_catalogue_request(0);
    let PlayCommand::Feature(creatures) = commands.try_recv()? else {
        return Err("creature command".into());
    };
    let page = cards::Page {
        next: None,
        categories: vec!["Forest dwellers".into()],
        cards: vec![cards::Card { outfit_look_type: 0, object_look_type: None,
            race: 7,
            name: "Grove watcher".into(),
            category: "Forest dwellers".into(),
            detail: String::new(),
        }],
    };
    outbox.properties_result(
        &creatures,
        &accepted(cards::encode_page(&page).map_err(|e| format!("{e:?}"))?),
    );
    link.charm_progression_request(Intent::Query);
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("balance query".into());
    };
    let charm = NonZeroU32::new(1).ok_or("charm")?;
    let race = NonZeroU32::new(7).ok_or("race")?;
    let mut reply = wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Query,
        fee_gold: 0,
        bestiary: vec![wire::bestiary::BestiaryRaceProgress {
            race,
            kill_count: 20,
            kill_thresholds: [5, 10, 20],
        }],
        charms: wire::charm::CharmView {
            charm_points_available: 1000,
            minor_charm_echoes_available: 10,
            assignment_slot_limit: NonZeroU32::new(2),
            charms: vec![wire::charm::CharmState {
                charm,
                kind: wire::charm::CharmKind::Major,
                unlocked_stage: 0,
                next_stage_cost: 240,
                assigned_race: None,
                effect_active: true,
            }],
        },
    };
    outbox.properties_result(
        &query,
        &accepted(wire::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?),
    );
    link.charm_progression_edit(0, "Dodge".into());
    link.charm_progression_edit(1, "Grove watcher".into());
    let panel = link.charm_progression_panel();
    assert_eq!(panel.fields, ["1", "7"]);
    assert_eq!(
        oteryn_client_ui::charm_progression::charm_browser::named_fields(&panel),
        ["Dodge", "Grove watcher"]
    );
    let text = oteryn_client_ui::charm_progression::charms_text(&panel);
    assert!(text.contains("1000 Charm Points"));
    assert!(text.contains("Dodge"));
    assert!(text.contains("trigger chance 5.00%"));
    link.charm_progression_request(Intent::Unlock);
    let PlayCommand::Feature(unlock) = commands.try_recv()? else {
        return Err("unlock".into());
    };
    assert_eq!(
        wire::decode_charm_progression_request(&unlock.payload).map_err(|e| format!("{e:?}"))?,
        wire::CharmProgressionAction::Unlock {
            charm,
            expected_stage: 0
        }
    );
    assert_eq!(
        link.charm_progression_panel()
            .reply
            .as_ref()
            .ok_or("confirmed state")?
            .charms
            .charms[0]
            .unlocked_stage,
        0
    );
    reply.outcome =
        wire::CharmProgressionOutcome::Unlock(wire::charm::CharmUnlockDisposition::Unlocked);
    reply.charms.charms[0].unlocked_stage = 1;
    reply.charms.charms[0].next_stage_cost = 360;
    reply.charms.charm_points_available = 760;
    outbox.properties_result(
        &unlock,
        &accepted(wire::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?),
    );
    link.charm_progression_request(Intent::Assign);
    let PlayCommand::Feature(assign) = commands.try_recv()? else {
        return Err("assign".into());
    };
    assert_eq!(
        wire::decode_charm_progression_request(&assign.payload).map_err(|e| format!("{e:?}"))?,
        wire::CharmProgressionAction::Assign { charm, race }
    );
    assert_eq!(
        link.charm_progression_panel()
            .reply
            .as_ref()
            .ok_or("confirmed assignment")?
            .charms
            .charms[0]
            .assigned_race,
        None
    );
    // A stale metadata reply cannot consume the pending assignment.
    outbox.properties_result(&metadata, &accepted(Vec::new()));
    assert!(link.charm_progression_panel().pending);
    lock(&link.mailbox).cancel_activity_requests();
    let panel = link.charm_progression_panel();
    assert!(!panel.pending);
    assert!(panel.charm_browser.cards.is_empty());
    assert!(panel.catalogue.names.is_empty());
    assert!(panel.reply.is_none());
    assert!(panel.summary.is_empty());
    outbox.properties_result(
        &assign,
        &accepted(wire::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?),
    );
    assert!(link.charm_progression_panel().reply.is_none());
    Ok(())
}

#[test]
fn duplicate_creature_captions_keep_the_selected_server_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.charm_progression;
        panel.selected = true;
        panel.fields = ["1".into(), "9".into()];
        panel
            .catalogue
            .names
            .extend([(7, "Grove watcher".into()), (9, "Grove watcher".into())]);
    }
    link.charm_progression_edit(1, "Grove watcher".into());
    link.charm_progression_request(Intent::Assign);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("assignment".into());
    };
    assert!(
        matches!(wire::decode_charm_progression_request(&command.payload).map_err(|e|format!("{e:?}"))?, wire::CharmProgressionAction::Assign { race, .. } if race.get() == 9)
    );
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.properties.charm_progression.pending = false;
        mailbox.properties.charm_progression.fields[1].clear();
    }
    link.charm_progression_edit(1, "Grove watcher".into());
    link.charm_progression_request(Intent::Assign);
    assert!(
        commands.try_recv().is_err(),
        "an ambiguous caption must not silently choose another creature"
    );
    Ok(())
}
