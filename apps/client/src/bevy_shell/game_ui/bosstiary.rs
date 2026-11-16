//! Server-confirmed Bosstiary catalogue, progress and hazard level, independent of slots and trackers.
use super::super::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::boss_encounter::{BossCategory, Intent, catalogue_cards, slot_text};

#[derive(Component)]
struct Window;
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    kind: Kind,
}
/// One Bosstiary control: a server request, or a retained selection the next request uses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Boss(bool),
    Slot,
    SlotBoss(bool),
}

#[derive(Resource, Default)]
struct Snapshot(Option<(u64, oteryn_client_ui::boss_encounter::BossEncounterPanel)>);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Snapshot>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    roots: Query<Entity, With<Window>>,
    mut snapshot: ResMut<Snapshot>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| {
            owner.phase == Phase::Playing && owner.settings.preferences.panel_visible("bosstiary")
        })
        .map(|play| (owner.generation(), play.link.boss_encounter_panel()));
    if snapshot.0 == panel {
        return;
    }
    snapshot.0 = panel.clone();
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((_, panel)) = panel else {
        return;
    };
    let root = commands
        .spawn((
            Window,
            bevy::ui_widgets::ScrollArea,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                left: px(220.),
                top: px(90.),
                width: px(650.),
                max_height: percent(75.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(12.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.09, 0.12)),
        ))
        .id();
    let mut text = String::from("Bosstiary\n");
    if let Some(page) = &panel.catalogue {
        text.push_str(&format!("Boss points: {}\n", page.points));
    }
    if let Some((unlocked, selected)) = panel.hazard {
        text.push_str(&format!("Hazard level: {selected} (unlocked {unlocked})\n"));
    }
    for entry in catalogue_cards(&panel) {
        text.push_str(&format!(
            "{} — {} — {} kills\n",
            entry.name,
            entry.category_name(),
            entry.kills
        ));
        for (i, stage) in ["Prowess", "Expertise", "Mastery"].iter().enumerate() {
            text.push_str(&format!(
                "  {stage}: {}/{} ({} points)\n",
                entry.kills.min(entry.thresholds[i]),
                entry.thresholds[i],
                entry.points[i]
            ));
        }
    }
    if !panel.fields[0].is_empty() {
        let name = panel
            .boss_names
            .get(&panel.fields[0])
            .map_or(panel.fields[0].as_str(), String::as_str);
        text.push_str(&format!("Selected boss: {name}\n"));
    }
    text.push_str(&slot_text(&panel));
    if !panel.summary.is_empty() {
        text.push_str(&panel.summary);
        text.push('\n');
    }
    if let Some(feedback) = &panel.feedback {
        text.push_str(feedback);
    }
    let label = commands
        .spawn((
            Text::new(text),
            TextFont {
                font: skins.font.clone().into(),
                font_size: 15.0.into(),
                ..default()
            },
        ))
        .id();

    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    for (label, kind) in [
        ("Refresh", Kind::Request(Intent::CatalogueRefresh)),
        ("Next page", Kind::Request(Intent::CatalogueNext)),
        ("Hazard", Kind::Request(Intent::HazardQuery)),
        ("Lower hazard", Kind::Request(Intent::HazardLower)),
        ("Raise hazard", Kind::Request(Intent::HazardRaise)),
        ("Previous boss", Kind::Boss(true)),
        ("Next boss", Kind::Boss(false)),
        ("Enter boss room", Kind::Request(Intent::Enter)),
        ("Leave: defeated", Kind::Request(Intent::Defeated)),
        ("Leave: time out", Kind::Request(Intent::Timeout)),
        ("Boss slots", Kind::Request(Intent::SlotsDetailed)),
        ("Next slot", Kind::Slot),
        ("Previous slot boss", Kind::SlotBoss(true)),
        ("Next slot boss", Kind::SlotBoss(false)),
        ("Equip slot", Kind::Request(Intent::SlotEquipSelected)),
        ("Remove slot", Kind::Request(Intent::SlotRemoveSelected)),
    ] {
        let button = commands
            .spawn((
                oteryn_client_ui_bevy::button(skins.buttons.clone(), 120.),
                Action {
                    generation: owner.generation(),
                    kind,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn((
                Text::new(label),
                TextFont {
                    font: skins.font.clone().into(),
                    font_size: 15.0.into(),
                    ..default()
                },
            ))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(row).add_child(button);
    }
    commands.entity(root).add_child(row);
    commands.entity(root).add_child(label);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Action>,
    owner: NonSend<HostOwner>,
    guard: super::modal::ModalGuard,
) {
    let Ok(action) = controls.get(event.entity) else {
        return;
    };
    if guard.allows(event.entity)
        && owner.phase == Phase::Playing
        && owner.generation() == action.generation
        && !owner.modal_active()
        && let Some(play) = &owner.play
    {
        match action.kind {
            Kind::Request(
                intent @ (Intent::Catalogue | Intent::CatalogueNext | Intent::CatalogueRefresh),
            ) => {
                play.link
                    .boss_encounter_browse("", BossCategory::All, false);
                play.link.boss_encounter_request(intent);
            }
            Kind::Request(intent) => play.link.boss_encounter_request(intent),
            Kind::Boss(previous) => {
                play.link
                    .boss_encounter_browse("", BossCategory::All, previous)
            }
            Kind::Slot => play.link.boss_slot_select_slot(),
            Kind::SlotBoss(previous) => play.link.boss_slot_select_boss(previous),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bosstiary_activation_is_fenced_by_foreground_map_modal()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_bosstiary_initialize();
        let generation = owner.generation();
        owner.phase = Phase::Playing;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        });
        let mut app = App::new();
        app.insert_non_send(owner);
        let entity = app
            .world_mut()
            .spawn((
                Action {
                    generation,
                    kind: Kind::Request(Intent::Catalogue),
                },
                Node::default(),
            ))
            .observe(activate)
            .id();
        let modal = app
            .world_mut()
            .spawn((super::super::GameModal, Node::default(), GlobalZIndex(250)))
            .id();
        app.world_mut().trigger(Activate { entity });
        assert!(commands.try_recv().is_err());
        app.world_mut().despawn(modal);
        app.world_mut().trigger(Activate { entity });
        assert!(commands.try_recv().is_ok());
        Ok(())
    }
    #[test]
    fn bosstiary_toolbar_requests_catalogue_and_projects_window()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_bosstiary_initialize();
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = Phase::Playing;
        crate::bevy_shell::ui::toolbar::open_toolbar(
            &mut owner,
            crate::bevy_shell::ui::Control::Panel("bosstiary"),
        );
        assert!(
            matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(c) if c.kind == 133 && c.payload.starts_with(b"BCT1"))
        );
        let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
            next: None,
            points: 10,
            entries: vec![oteryn_client_runtime::boss_encounter::catalogue::Entry {
                key: "test:boss".into(),
                name: "Confirmed boss".into(),
                category: 1,
                kills: 5,
                thresholds: [5, 10, 20],
                points: [10, 30, 60],
                look: 1,
                object: false,
            }],
        };
        outbox.bevy_bosstiary_result(&oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: page.encode().ok_or("catalogue")?,
        });
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            oteryn_client_ui_bevy::GlobalUiPlugin,
        ))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Window>>()
                .iter(app.world())
                .count(),
            1
        );
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert!(texts.iter().any(|text| text.contains("Confirmed boss")
            && text.contains("Prowess: 5/5")
            && text.contains("Mastery: 5/20")));
        Ok(())
    }
    #[test]
    fn bosstiary_hazard_buttons_query_and_raise_the_server_hazard_level()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::combat_input::PlayCommand;
        use oteryn_client_runtime::raw_feature::GatedCommandResult;
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_bosstiary_initialize();
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = Phase::Playing;
        crate::bevy_shell::ui::toolbar::open_toolbar(
            &mut owner,
            crate::bevy_shell::ui::Control::Panel("bosstiary"),
        );
        assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
        let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
            next: None,
            points: 0,
            entries: vec![],
        };
        outbox.bevy_bosstiary_result(&GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: page.encode().ok_or("catalogue")?,
        });
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            oteryn_client_ui_bevy::GlobalUiPlugin,
        ))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        app.update();
        let press = |app: &mut App, intent: Intent| -> Result<(), Box<dyn std::error::Error>> {
            let entity = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(entity, action)| {
                    (action.kind == Kind::Request(intent)).then_some(entity)
                })
                .ok_or("hazard button missing")?;
            app.world_mut().trigger(Activate { entity });
            Ok(())
        };
        press(&mut app, Intent::HazardQuery)?;
        let PlayCommand::Feature(query) = commands.try_recv()? else {
            return Err("hazard query missing".into());
        };
        assert_eq!((query.capability, query.kind), (117, 169));
        assert!(query.payload.is_empty());
        outbox.bevy_bosstiary_result(&GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: vec![5, 3],
        });
        app.update();
        let shown = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Hazard level: 3 (unlocked 5)"));
        assert!(shown, "the server hazard level is not shown");
        press(&mut app, Intent::HazardRaise)?;
        let PlayCommand::Feature(raise) = commands.try_recv()? else {
            return Err("hazard raise missing".into());
        };
        assert_eq!((raise.kind, raise.payload), (169, vec![4]));
        Ok(())
    }
    fn click(app: &mut App, kind: Kind) -> Result<(), Box<dyn std::error::Error>> {
        app.update();
        let entity = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find(|(_, action)| action.kind == kind)
            .map(|(entity, _)| entity)
            .ok_or("bosstiary control")?;
        app.world_mut().trigger(Activate { entity });
        Ok(())
    }
    fn window_text(app: &mut App) -> String {
        app.update();
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn accepted(payload: Vec<u8>) -> oteryn_client_runtime::raw_feature::GatedCommandResult {
        oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload,
        }
    }
    /// A UUIDv7-shaped handoff, as the boss activity reply requires.
    const HANDOFF: [u8; 16] = [7, 7, 7, 7, 7, 7, 0x77, 7, 0x87, 7, 7, 7, 7, 7, 7, 7];
    #[test]
    fn bosstiary_window_pulls_boss_lever_and_equips_boss_slot()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::combat_input::PlayCommand;
        use oteryn_client_runtime::boss_encounter::{
            BossActivityReply,
            bosstiary_slots::{BossSlotReply, BossSlotRequest, SlotDetails},
            encode_boss_activity_reply, encode_boss_key,
        };
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_bosstiary_initialize();
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = Phase::Playing;
        crate::bevy_shell::ui::toolbar::open_toolbar(
            &mut owner,
            crate::bevy_shell::ui::Control::Panel("bosstiary"),
        );
        assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
        let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
            next: None,
            points: 10,
            entries: vec![oteryn_client_runtime::boss_encounter::catalogue::Entry {
                key: "oteryn:creature.test_boss".into(),
                name: "Confirmed boss".into(),
                category: 1,
                kills: 5,
                thresholds: [5, 10, 20],
                points: [10, 30, 60],
                look: 1,
                object: false,
            }],
        };
        outbox.bevy_bosstiary_result(&accepted(page.encode().ok_or("catalogue")?));
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            oteryn_client_ui_bevy::GlobalUiPlugin,
        ))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        assert!(window_text(&mut app).contains("Selected boss: Confirmed boss"));

        click(&mut app, Kind::Request(Intent::Enter))?;
        let PlayCommand::Feature(lever) = commands.try_recv()? else {
            return Err("boss lever command".into());
        };
        assert_eq!(
            (lever.capability, lever.kind, lever.payload),
            (
                117,
                178,
                encode_boss_key("oteryn:creature.test_boss").map_err(|e| format!("{e:?}"))?
            )
        );
        outbox.bevy_bosstiary_result(&accepted(
            encode_boss_activity_reply(&BossActivityReply::Accepted {
                handoff: HANDOFF,
                deadline_unix_ms: Some(90_000),
            })
            .map_err(|e| format!("{e:?}"))?,
        ));
        assert!(window_text(&mut app).contains("Boss entry accepted. Deadline: 90000"));
        for (kind, intent) in [(179, Intent::Defeated), (180, Intent::Timeout)] {
            click(&mut app, Kind::Request(intent))?;
            assert!(
                matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == kind && c.payload == encode_boss_key("oteryn:creature.test_boss").map_err(|e| format!("{e:?}"))?)
            );
            outbox.bevy_bosstiary_result(&accepted(
                encode_boss_activity_reply(&BossActivityReply::Accepted {
                    handoff: HANDOFF,
                    deadline_unix_ms: None,
                })
                .map_err(|e| format!("{e:?}"))?,
            ));
            assert!(window_text(&mut app).contains("Boss return accepted"));
        }

        let slots = |bosses: [&str; 2]| BossSlotReply {
            details: Some(SlotDetails {
                base_bonus: 4,
                bonuses: [4, 0],
                boosted: String::new(),
                now: 10,
            }),
            status: 0,
            unlocked: 1,
            bonus: 4,
            points: 10,
            remove_fee: 0,
            ready_at_ms: 0,
            bosses: bosses.map(str::to_owned),
        };
        click(&mut app, Kind::Request(Intent::SlotsDetailed))?;
        assert!(
            matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 491 && Some(c.payload.clone()) == BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)).encode())
        );
        outbox.bevy_bosstiary_result(&accepted(slots(["", ""]).encode().ok_or("slots")?));
        assert!(window_text(&mut app).contains("Slot 1: Empty"));
        click(&mut app, Kind::Slot)?;
        click(&mut app, Kind::SlotBoss(false))?;
        click(&mut app, Kind::Request(Intent::SlotEquipSelected))?;
        let PlayCommand::Feature(equip) = commands.try_recv()? else {
            return Err("boss slot command".into());
        };
        assert_eq!(equip.kind, 491);
        assert_eq!(
            Some(equip.payload),
            BossSlotRequest::Detailed(Box::new(BossSlotRequest::Equip {
                slot: 1,
                boss: "oteryn:creature.test_boss".into(),
            }))
            .encode()
        );
        outbox.bevy_bosstiary_result(&accepted(
            slots(["oteryn:creature.test_boss", ""])
                .encode()
                .ok_or("slots")?,
        ));
        assert!(window_text(&mut app).contains("> Slot 1: Confirmed boss | +4% equipment loot"));
        Ok(())
    }
}
