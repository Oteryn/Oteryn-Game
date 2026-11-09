use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    charm_progression as w,
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::charm_progression::Intent;
use std::num::NonZeroU32;
#[test]
fn t081_named_selector_uses_confirmed_descriptions_and_routes_all_operations()
-> Result<(), Box<dyn std::error::Error>> {
    let id = |n| NonZeroU32::new(n).ok_or("id");
    let (link, mut receiver, outbox) = play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .properties
        .charm_progression
        .selected = true;
    let mut reply = w::CharmProgressionReply {
        outcome: w::CharmProgressionOutcome::Query,
        fee_gold: 0,
        bestiary: vec![
            w::bestiary::BestiaryRaceProgress {
                race: id(1)?,
                kill_count: 3,
                kill_thresholds: [1, 2, 3],
            },
            w::bestiary::BestiaryRaceProgress {
                race: id(2)?,
                kill_count: 1,
                kill_thresholds: [1, 2, 3],
            },
        ],
        charms: w::charm::CharmView {
            charm_points_available: 1000,
            charms: vec![w::charm::CharmState {
                charm: id(1)?,
                kind: w::charm::CharmKind::Major,
                unlocked_stage: 0,
                assigned_race: None,
                next_stage_cost: 240,
                effect_active: true,
            }],
            ..Default::default()
        },
    };
    link.charm_progression_request(Intent::Query);
    let PlayCommand::Feature(query) = receiver.try_recv()? else {
        return Err("query".into());
    };
    let answer = |command: &oteryn_client_runtime::raw_feature::FeatureCommand, payload| {
        outbox.properties_result(
            command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        )
    };
    answer(
        &query,
        w::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?,
    );
    link.charm_catalogue_request(0);
    let PlayCommand::Feature(query) = receiver.try_recv()? else {
        return Err("metadata".into());
    };
    assert_eq!(query.payload, vec![255, 2, 0]);
    link.charm_catalogue_request(0);
    assert!(receiver.try_recv().is_err());
    assert!(!link.charm_selector_select(31, None));
    let metadata = w::catalogue::Page {
        next: None,
        categories: vec!["Major".into()],
        cards: vec![w::catalogue::Card {
            race: 1,
            name: "Fire mark".into(),
            category: "Major".into(),
            detail: "Fire damage on an attack; stage chances 5%, 10%, 11%.".into(),
            ..Default::default()
        }],
    };
    answer(
        &query,
        w::catalogue::encode_page(&metadata).map_err(|e| format!("{e:?}"))?,
    );
    assert_eq!(
        link.charm_progression_panel().charm_choices()[0].1,
        "Fire mark"
    );
    link.charm_race_names_request();
    let PlayCommand::Feature(names) = receiver.try_recv()? else {
        return Err("creature names".into());
    };
    assert_eq!(names.payload, vec![255, 3, 1, 0, 2, 0]);
    let names_page = w::catalogue::Page {
        next: None,
        categories: vec!["Grove".into()],
        cards: vec![
            w::catalogue::Card {
                race: 1,
                name: "Grove watcher".into(),
                category: "Grove".into(),
                ..Default::default()
            },
            w::catalogue::Card {
                race: 2,
                name: "Moss keeper".into(),
                category: "Grove".into(),
                ..Default::default()
            },
        ],
    };
    answer(
        &names,
        w::catalogue::encode_page(&names_page).map_err(|e| format!("{e:?}"))?,
    );
    {
        let mut m=crate::play::mailbox::lock(&link.mailbox);
        let p=&mut m.properties.charm_progression;
        p.catalogue.page=names_page.clone();
        p.race_filter=oteryn_client_ui::charm_progression::RaceFilter::Complete;
        assert_eq!(p.catalogue_rows().iter().map(|c|c.race).collect::<Vec<_>>(),vec![1]);
        p.race_filter=oteryn_client_ui::charm_progression::RaceFilter::InProgress;
        assert_eq!(p.catalogue_rows().iter().map(|c|c.race).collect::<Vec<_>>(),vec![2]);
        assert!(p.catalogue_label(&names_page.cards[0]).contains("3 kills, stage 3/3"));
    }
    assert!(link.charm_selector_select(1, None));
    assert!(
        link.charm_progression_panel().assignment_choices()[0]
            .1
            .contains("Grove watcher")
    );
    assert_eq!(link.charm_progression_panel().assignment_choices().len(), 1);
    assert!(!link.charm_selector_select(1, Some(2)));
    assert!(
        link.charm_progression_panel()
            .selected_charm_text()
            .contains("Fire damage")
    );
    for intent in [
        Intent::Unlock,
        Intent::Assign,
        Intent::Remove,
        Intent::ResetAll,
    ] {
        if intent == Intent::Assign {
            assert!(link.charm_selector_select(1, Some(1)));
            crate::play::mailbox::lock(&link.mailbox)
                .properties
                .charm_progression
                .fields[1] = "2".into();
        }
        link.charm_selector_request(intent);
        let PlayCommand::Feature(command) = receiver.try_recv()? else {
            return Err("operation".into());
        };
        let action =
            w::decode_charm_progression_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        use w::{CharmProgressionAction as A, CharmProgressionOutcome as O};
        reply.outcome = match intent {
            Intent::Unlock => {
                assert_eq!(
                    action,
                    A::Unlock {
                        charm: id(1)?,
                        expected_stage: 0
                    }
                );
                reply.charms.charms[0].unlocked_stage = 1;
                reply.charms.charms[0].next_stage_cost = 360;
                O::Unlock(w::charm::CharmUnlockDisposition::Unlocked)
            }
            Intent::Assign => {
                assert_eq!(
                    action,
                    A::Assign {
                        charm: id(1)?,
                        race: id(1)?
                    }
                );
                reply.charms.charms[0].assigned_race = Some(id(1)?);
                O::Assign(w::charm::CharmAssignDisposition::Assigned)
            }
            Intent::Remove => {
                assert_eq!(action, A::Remove { charm: id(1)? });
                reply.charms.charms[0].assigned_race = None;
                reply.fee_gold = 100;
                O::Remove(w::CharmRemoveDisposition::Removed)
            }
            Intent::ResetAll => {
                assert_eq!(action, A::ResetAll);
                reply.fee_gold = 0;
                O::ResetAll(w::CharmRemoveDisposition::NotAssigned)
            }
            _ => return Err("intent".into()),
        };
        answer(
            &command,
            w::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?,
        );
        assert!(!link.charm_progression_panel().pending);
    }
    outbox.logged_out();
    link.charm_catalogue_request(0);
    assert!(receiver.try_recv().is_err());
    assert!(link.charm_progression_panel().charm_choices().is_empty());
    Ok(())
}
