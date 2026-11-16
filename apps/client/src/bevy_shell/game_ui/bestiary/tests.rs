use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::{
    charm_progression::{self as progression, catalogue as wire},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn command(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("feature command required".into());
    };
    Ok(command)
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == kind)
        .map(|(entity, _)| entity)
        .ok_or("control missing")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn labels(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn enabled(app: &mut App, kind: Kind) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(&Control, Has<bevy::ui::InteractionDisabled>)>()
        .iter(app.world())
        .find(|(control, _)| control.kind == kind)
        .map(|(_, disabled)| !disabled)
        .ok_or_else(|| "control missing".into())
}
/// The real charm catalogue, served in two pages: names, categories and effect descriptions.
fn charm_page(after: u16) -> wire::Page {
    let card = |race: u16, name: &str, category: &str, detail: &str| wire::Card {
        race,
        name: name.into(),
        category: category.into(),
        outfit_look_type: 0,
        object_look_type: None,
        detail: detail.into(),
        echo_warden_completed: None,
    };
    if after == 0 {
        wire::Page {
            next: Some(1),
            categories: vec!["Major".into()],
            cards: vec![card(
                1,
                "Dodge",
                "Major",
                "Avoid a qualifying creature attack.",
            )],
        }
    } else {
        wire::Page {
            next: None,
            categories: vec!["Minor".into()],
            cards: vec![card(2, "Gut", "Minor", "More creature products.")],
        }
    }
}
#[test]
fn bestiary_catalogue_selection_details_and_close_use_retained_owner() -> Result {
    let mut owner = HostOwner::new(None)?;
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_bestiary_initialize();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .charm_progression_open();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    assert!(labels(&mut app).contains("Bestiary"));
    let query = command(&mut receiver).map_err(|error| format!("query: {error}"))?;
    assert_eq!((query.capability, query.kind), (67, 84));
    // End the progress query, then let the Bevy system request the catalogue.
    let publish_progress = |query: &FeatureCommand, kills| -> Result {
        outbox.bevy_bestiary_result(
            query,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: progression::encode_charm_progression_reply(
                    &progression::CharmProgressionReply {
                        outcome: progression::CharmProgressionOutcome::Query,
                        fee_gold: 0,
                        bestiary: vec![progression::bestiary::BestiaryRaceProgress {
                            race: std::num::NonZeroU32::new(9).ok_or("race")?,
                            kill_count: kills,
                            kill_thresholds: [10, 20, 30],
                        }],
                        charms: Default::default(),
                    },
                )
                .map_err(|e| format!("{e:?}"))?,
            },
        );
        Ok(())
    };
    publish_progress(&query, 12)?;
    app.update();
    let catalogue = command(&mut receiver).map_err(|error| format!("catalogue: {error}"))?;
    assert_eq!((catalogue.capability, catalogue.kind), (67, 84));
    let mut page = wire::Page {
        next: Some(9),
        categories: vec!["Amphibic".into()],
        cards: vec![
            wire::Card {
                race: 7,
                name: "Toad".into(),
                category: "Amphibic".into(),
                outfit_look_type: 226,
                object_look_type: None,
                detail: String::new(),
                echo_warden_completed: None,
            },
            wire::Card {
                race: 9,
                name: "Frog".into(),
                category: "Amphibic".into(),
                outfit_look_type: 226,
                object_look_type: None,
                detail: String::new(),
                echo_warden_completed: None,
            },
        ],
    };
    let publish = |command: &FeatureCommand, page: &wire::Page| -> Result {
        outbox.bevy_bestiary_result(
            command,
            &GatedCommandResult {
                command_id: 2,
                server_sequence: 2,
                status: oteryn_session::CommandStatus::Accepted,
                payload: wire::encode_page(page).map_err(|e| format!("{e:?}"))?,
            },
        );
        Ok(())
    };
    publish(&catalogue, &page)?;
    app.update();
    // Opening also loads the charm catalogue, page after page.
    for after in [0, 1] {
        let charms = command(&mut receiver).map_err(|error| format!("charms: {error}"))?;
        assert_eq!(
            wire::decode_charm_request(&charms.payload).map_err(|e| format!("{e:?}"))?,
            after
        );
        publish(&charms, &charm_page(after))?;
        app.update();
    }
    assert!(receiver.try_recv().is_err(), "every charm page is loaded");
    assert!(labels(&mut app).contains("Toad [Amphibic]"));
    assert!(labels(&mut app).contains("Progress not reported"));
    click(&mut app, Kind::Select(9))?;
    let detail = command(&mut receiver).map_err(|error| format!("detail: {error}"))?;
    assert_eq!(
        wire::decode_request(&detail.payload)
            .map_err(|e| format!("{e:?}"))?
            .race,
        9
    );
    page.cards.remove(0);
    page.cards[0].detail = "Health: 20\r\nLoot:\r\nGold coin".into();
    page.next = Some(1);
    publish(&detail, &page)?;
    app.update();
    assert!(labels(&mut app).contains("Health: 20"));
    assert!(labels(&mut app).contains("Stage 1: 10/10 complete"));
    assert!(labels(&mut app).contains("Stage 2: 12/20 in progress"));
    assert!(!labels(&mut app).contains("Gold coin"));
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .charm_progression_panel()
            .catalogue
            .next_loot
            .is_none()
    );
    click(&mut app, Kind::Catalogue(3))?;
    assert!(
        receiver.try_recv().is_err(),
        "undiscovered loot stays locked"
    );
    assert!(!labels(&mut app).contains("Gold coin"));
    app.update();
    // Loot pagination becomes available only after confirmed completed discovery.
    click(&mut app, Kind::Progress)?;
    let progress = command(&mut receiver).map_err(|error| format!("progress: {error}"))?;
    publish_progress(&progress, 30)?;
    app.update();
    click(&mut app, Kind::Select(9))?;
    let completed = command(&mut receiver).map_err(|error| format!("completed: {error}"))?;
    publish(&completed, &page)?;
    app.update();
    assert!(labels(&mut app).contains("Gold coin"));
    click(&mut app, Kind::Catalogue(3))?;
    let loot = command(&mut receiver).map_err(|error| format!("loot: {error}"))?;
    let request = wire::decode_request(&loot.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.race, request.after), (9, 1));
    page.next = None;
    publish(&loot, &page)?;
    app.update();
    click(&mut app, Kind::Catalogue(5))?;
    let category = command(&mut receiver).map_err(|error| format!("category: {error}"))?;
    assert_eq!(
        wire::decode_request(&category.payload)
            .map_err(|e| format!("{e:?}"))?
            .category,
        1
    );
    app.update();
    let focused = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == Kind::Close)
        .map(|(entity, _)| entity)
        .ok_or("close")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, bevy::input_focus::FocusCause::Navigated);
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<InputFocus>().get().is_none());
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .next()
            .is_none()
    );
    Ok(())
}
fn charm_reply(
    outcome: progression::CharmProgressionOutcome,
    fee_gold: u64,
    first: (u8, Option<u32>, u32),
    second_race: Option<u32>,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    use progression::charm::{CharmKind, CharmState, CharmView};
    let id = |value: u32| std::num::NonZeroU32::new(value).ok_or("nonzero");
    let (stage, race, cost) = first;
    Ok(
        progression::encode_charm_progression_reply(&progression::CharmProgressionReply {
            outcome,
            fee_gold,
            bestiary: vec![progression::bestiary::BestiaryRaceProgress {
                race: id(9)?,
                kill_count: 30,
                kill_thresholds: [10, 20, 30],
            }],
            charms: CharmView {
                charms: vec![
                    CharmState {
                        charm: id(1)?,
                        kind: CharmKind::Major,
                        unlocked_stage: stage,
                        assigned_race: race.map(id).transpose()?,
                        next_stage_cost: cost,
                        effect_active: true,
                    },
                    CharmState {
                        charm: id(2)?,
                        kind: CharmKind::Minor,
                        unlocked_stage: 1,
                        assigned_race: second_race.map(id).transpose()?,
                        next_stage_cost: 50,
                        effect_active: false,
                    },
                ],
                charm_points_available: 500,
                minor_charm_echoes_available: 60,
                assignment_slot_limit: Some(id(2)?),
            },
        })
        .map_err(|e| format!("{e:?}"))?,
    )
}
#[test]
fn bestiary_charms_unlock_assign_remove_and_reset_through_the_progression_owner() -> Result {
    use progression::{
        CharmProgressionAction as Action, CharmProgressionOutcome as Outcome,
        CharmRemoveDisposition as Removed,
        charm::{CharmAssignDisposition, CharmUnlockDisposition},
    };
    let mut owner = HostOwner::new(None)?;
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_bestiary_initialize();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .charm_progression_open();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let mut sequence = 0;
    let mut publish = |command: &FeatureCommand, payload: Vec<u8>| {
        sequence += 1;
        outbox.bevy_bestiary_result(
            command,
            &GatedCommandResult {
                command_id: sequence,
                server_sequence: sequence,
                status: oteryn_session::CommandStatus::Accepted,
                payload,
            },
        );
    };
    let query = command(&mut receiver).map_err(|error| format!("query: {error}"))?;
    publish(
        &query,
        charm_reply(Outcome::Query, 0, (0, None, 100), Some(7))?,
    );
    app.update();
    let catalogue = command(&mut receiver).map_err(|error| format!("catalogue: {error}"))?;
    let page = wire::Page {
        next: None,
        categories: vec!["Amphibic".into()],
        cards: vec![wire::Card {
            race: 9,
            name: "Frog".into(),
            category: "Amphibic".into(),
            outfit_look_type: 226,
            object_look_type: None,
            detail: String::new(),
            echo_warden_completed: None,
        }],
    };
    publish(
        &catalogue,
        wire::encode_page(&page).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    // The real opening path loads every charm catalogue page before any charm is acted on.
    for after in [0, 1] {
        let charms = command(&mut receiver).map_err(|error| format!("charms: {error}"))?;
        assert_eq!(
            wire::decode_charm_request(&charms.payload).map_err(|e| format!("{e:?}"))?,
            after
        );
        publish(
            &charms,
            wire::encode_page(&charm_page(after)).map_err(|e| format!("{e:?}"))?,
        );
        app.update();
    }
    assert!(receiver.try_recv().is_err(), "every charm page is loaded");
    assert!(labels(&mut app).contains("Charms — points: 500 | minor echoes: 60"));
    assert!(labels(&mut app).contains("Dodge (Major) — stage 0/3, next cost 100, unassigned"));
    assert!(labels(&mut app).contains("Gut (Minor) — stage 1/3, next cost 50, race 7"));
    assert!(!labels(&mut app).contains("Charm 1"));
    click(&mut app, Kind::Charm(2))?;
    assert!(receiver.try_recv().is_err(), "choosing a charm is local");
    app.update();
    assert!(labels(&mut app).contains("Gut — Minor"));
    assert!(labels(&mut app).contains("More creature products."));
    assert!(!labels(&mut app).contains("Avoid a qualifying creature attack."));
    click(&mut app, Kind::Charm(1))?;
    app.update();
    assert!(labels(&mut app).contains("> Dodge (Major)"));
    assert!(labels(&mut app).contains("Dodge — Major"));
    assert!(labels(&mut app).contains("Avoid a qualifying creature attack."));
    click(&mut app, Kind::CharmAction(Intent::Unlock))?;
    let unlock = command(&mut receiver).map_err(|error| format!("unlock: {error}"))?;
    assert_eq!((unlock.capability, unlock.kind), (67, 84));
    assert_eq!(
        progression::decode_charm_progression_request(&unlock.payload)
            .map_err(|e| format!("{e:?}"))?,
        Action::Unlock {
            charm: std::num::NonZeroU32::new(1).ok_or("charm")?,
            expected_stage: 0,
        }
    );
    publish(
        &unlock,
        charm_reply(
            Outcome::Unlock(CharmUnlockDisposition::Unlocked),
            0,
            (1, None, 200),
            Some(7),
        )?,
    );
    app.update();
    assert!(labels(&mut app).contains("Completed: Unlock(Unlocked)"));
    assert!(labels(&mut app).contains("> Dodge (Major) — stage 1/3, next cost 200"));
    click(&mut app, Kind::Select(9))?;
    let detail = command(&mut receiver).map_err(|error| format!("detail: {error}"))?;
    publish(
        &detail,
        wire::encode_page(&page).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::CharmAction(Intent::Assign))?;
    let assign = command(&mut receiver).map_err(|error| format!("assign: {error}"))?;
    assert_eq!(
        progression::decode_charm_progression_request(&assign.payload)
            .map_err(|e| format!("{e:?}"))?,
        Action::Assign {
            charm: std::num::NonZeroU32::new(1).ok_or("charm")?,
            race: std::num::NonZeroU32::new(9).ok_or("race")?,
        }
    );
    publish(
        &assign,
        charm_reply(
            Outcome::Assign(CharmAssignDisposition::Assigned),
            0,
            (1, Some(9), 200),
            Some(7),
        )?,
    );
    app.update();
    assert!(labels(&mut app).contains("Completed: Assign(Assigned)"));
    assert!(labels(&mut app).contains("stage 1/3, next cost 200, Frog"));
    click(&mut app, Kind::CharmAction(Intent::Remove))?;
    assert!(
        receiver.try_recv().is_err(),
        "the gold fee waits for confirmation"
    );
    app.update();
    assert!(labels(&mut app).contains(
        "Remove Dodge from its creature? The gold fee depends on your level, which is not reported yet."
    ));
    assert!(!enabled(&mut app, Kind::Confirm(Intent::Remove))?);
    click(&mut app, Kind::Confirm(Intent::Remove))?;
    assert!(receiver.try_recv().is_err(), "no fee shown, nothing sent");
    // The character level arrives through the skill display owner: level 30, two slots.
    outbox.bevy_hud_skills_fixture()?;
    app.update();
    assert!(labels(&mut app).contains("Remove Dodge from its creature for 3000 gold?"));
    assert!(enabled(&mut app, Kind::Confirm(Intent::Remove))?);
    click(&mut app, Kind::Confirm(Intent::Remove))?;
    let remove = command(&mut receiver).map_err(|error| format!("remove: {error}"))?;
    assert_eq!(
        progression::decode_charm_progression_request(&remove.payload)
            .map_err(|e| format!("{e:?}"))?,
        Action::Remove {
            charm: std::num::NonZeroU32::new(1).ok_or("charm")?,
        }
    );
    publish(
        &remove,
        charm_reply(
            Outcome::Remove(Removed::Removed),
            3000,
            (1, None, 200),
            None,
        )?,
    );
    app.update();
    assert!(labels(&mut app).contains("Completed: Remove(Removed); fee: 3000 gold"));
    assert!(!labels(&mut app).contains("for 3000 gold?"));
    // No charm is assigned, but unlocked stages remain to be refunded.
    assert!(enabled(&mut app, Kind::CharmAction(Intent::ResetAll))?);
    click(&mut app, Kind::CharmAction(Intent::ResetAll))?;
    assert!(
        receiver.try_recv().is_err(),
        "the reset fee waits for confirmation"
    );
    app.update();
    assert!(labels(&mut app).contains("Reset all charms for 100000 gold?"));
    click(&mut app, Kind::Cancel)?;
    assert!(receiver.try_recv().is_err(), "cancel sends nothing");
    app.update();
    assert!(!labels(&mut app).contains("Reset all charms for 100000 gold?"));
    click(&mut app, Kind::CharmAction(Intent::ResetAll))?;
    app.update();
    click(&mut app, Kind::Confirm(Intent::ResetAll))?;
    let reset = command(&mut receiver).map_err(|error| format!("reset: {error}"))?;
    assert_eq!(
        progression::decode_charm_progression_request(&reset.payload)
            .map_err(|e| format!("{e:?}"))?,
        Action::ResetAll
    );
    publish(
        &reset,
        charm_reply(
            Outcome::ResetAll(Removed::Removed),
            100_000,
            (1, None, 200),
            None,
        )?,
    );
    app.update();
    assert!(labels(&mut app).contains("Completed: ResetAll(Removed); fee: 100000 gold"));
    assert!(
        labels(&mut app)
            .contains("Gut (Minor) — stage 1/3, next cost 50, unassigned, not yet active")
    );
    Ok(())
}
