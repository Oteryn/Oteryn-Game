//! Screenshot destination edits use the same scoped Apply/OK/Cancel owner as other options.
use super::*;
use bevy::{
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};

#[derive(Component)]
pub(super) struct Folder(String);

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    let label = commands.spawn(source::text(
        "F12 saves a BMP of the window. Folder: absolute path, or empty for the default Oteryn/screenshots folder. Use folder, then Apply or OK.",
        source::font(skins.font.clone(), false),
    )).id();
    let field = commands
        .spawn((
            Folder(draft.screenshots.effective().directory),
            TextInput,
            EditableText::new(draft.screenshots.effective().directory),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Name::new("screenshots.folder"),
            Node {
                width: percent(100.),
                height: px(24.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 100.),
            Name::new("screenshots.use-folder"),
        ))
        .observe(use_folder)
        .id();
    let open = section::action(
        commands,
        parent,
        skins,
        "Open screenshots folder",
        170.,
        "screenshots.open-folder",
        false,
    );
    commands.entity(open).observe(open_folder);
    let caption = commands
        .spawn(source::button_caption("Use folder", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands
        .entity(parent)
        .add_children(&[label, field, button]);
}
#[derive(Resource, Default)]
pub(super) struct FolderLaunch(Option<(u64, bevy_tasks::Task<std::io::Result<()>>)>);

pub(super) fn project(
    mut owner: NonSendMut<HostOwner>,
    mut launch: ResMut<FolderLaunch>,
    mut fields: Query<(&mut Folder, &mut EditableText)>,
) {
    if let Some((generation, task)) = &mut launch.0
        && let Some(result) = bevy_tasks::futures::check_ready(task)
    {
        if *generation == owner.generation()
            && let Err(error) = result
        {
            owner.settings.feedback = format!("Could not open screenshots folder: {error}");
        }
        launch.0 = None;
    }
    let Some(draft) = owner.preferences_draft() else {
        return;
    };
    let directory = draft.screenshots.effective().directory;
    for (mut held, mut field) in &mut fields {
        if held.0 != directory {
            field.editor_mut().set_text(&directory);
            held.0.clone_from(&directory);
        }
    }
}

#[cfg(test)]
#[derive(Resource)]
struct FolderOpener(fn(&std::path::Path) -> std::io::Result<()>);

fn open_folder(
    event: On<Activate>,
    owner: NonSend<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
    mut launch: ResMut<FolderLaunch>,
    #[cfg(test)] opener: Option<Res<FolderOpener>>,
) {
    if launch.0.is_some()
        || !section::editable(&owner, &state, &guard, event.entity, Page::Screenshots)
    {
        return;
    }
    let directory = owner.settings.preferences.screenshots.effective().directory;
    #[cfg(test)]
    let opener = opener.map(|opener| opener.0);
    let task = bevy_tasks::IoTaskPool::get().spawn(async move {
        let folder =
            crate::play::screenshot::screenshot_directory(std::path::Path::new(&directory))?;
        crate::play::screenshot::ensure_screenshot_directory(&folder)?;
        #[cfg(test)]
        if let Some(open) = opener {
            return open(&folder);
        }
        let mut command = crate::bevy_shell::diagnostics::folder_command(&folder);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::bevy_shell::diagnostics::launch_folder(command).await
    });
    launch.0 = Some((owner.generation(), task));
}

#[cfg(all(test, unix))]
#[test]
fn folder_launcher_releases_io_worker_while_browser_is_open()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{os::unix::net::UnixStream, process::Command, time::Duration};
    let (reader, writer) = UnixStream::pair()?;
    let mut command = Command::new("cat");
    command
        .stdin(std::os::fd::OwnedFd::from(reader))
        .stdout(std::process::Stdio::null());
    let pool = bevy_tasks::TaskPoolBuilder::new().num_threads(1).build();
    let (started, signal) = std::sync::mpsc::channel();
    let browser = pool.spawn(async move {
        let _ = started.send(());
        crate::bevy_shell::diagnostics::launch_folder(command).await
    });
    signal.recv_timeout(Duration::from_secs(5))?;
    let (done, signal) = std::sync::mpsc::channel();
    pool.spawn(async move {
        let _ = done.send(());
    })
    .detach();
    let progress = signal.recv_timeout(Duration::from_secs(2));
    drop(writer);
    bevy_tasks::block_on(browser)?;
    progress?;
    Ok(())
}
fn use_folder(
    event: On<Activate>,
    fields: Query<&EditableText, With<Folder>>,
    mut owner: NonSendMut<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || state.page != Page::Screenshots
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(field) = fields.single() else {
        return;
    };
    owner.edit_preferences(|draft| {
        draft.screenshots.directory = field.value().to_string();
        draft.screenshots.folder = field.value().to_string();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_action_uses_applied_capture_directory() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = tempfile::tempdir()?;
        let folder = fixture.path().join("captures with spaces");
        let mut app = super::super::navigation_tests::options_page(Page::Screenshots)?;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .screenshots
            .directory = folder.to_string_lossy().into_owned();
        app.insert_resource(FolderOpener(|directory| {
            std::fs::write(directory.join("opened"), [])
        }));
        let button = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, name)| {
                (name.as_str() == "screenshots.open-folder").then_some(entity)
            })
            .ok_or("folder button")?;
        app.world_mut().trigger(Activate { entity: button });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !folder.join("opened").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(folder.join("opened").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(folder.metadata()?.permissions().mode() & 0o077, 0);
        }
        Ok(())
    }
    #[test]
    fn destination_uses_draft_cancel_and_persisted_apply() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path().join("gap60-settings");
        let path = root.join("preferences.json");
        let folder = root.join("captures").to_string_lossy().into_owned();
        let mut app = super::super::tests::app(Page::Screenshots, Some(path.clone()))?;
        app.world_mut()
            .spawn((Folder(String::new()), EditableText::new(&folder)));
        let button = app.world_mut().spawn_empty().observe(use_folder).id();
        app.world_mut().trigger(Activate { entity: button });
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(
            owner
                .preferences_draft()
                .ok_or("draft")?
                .screenshots
                .directory,
            folder
        );
        assert!(owner.settings.preferences.screenshots.directory.is_empty());
        app.world_mut().non_send_mut::<HostOwner>().cancel();
        app.world_mut().non_send_mut::<HostOwner>().open_graphics();
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .ok_or("draft")?
                .screenshots
                .directory
                .is_empty()
        );
        app.world_mut().trigger(Activate { entity: button });
        app.world_mut().non_send_mut::<HostOwner>().apply(true);
        assert_eq!(
            crate::play::SettingsState::new(Some(path))
                .preferences
                .screenshots
                .directory,
            folder
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "screenshots/automatic_tests.rs"]
mod automatic_tests;

#[cfg(test)]
#[path = "screenshots/loot_tests.rs"]
mod loot_test;

#[cfg(test)]
mod bestiary_tests {
    use super::*;
    use bevy::ui_widgets::ValueChange;
    use oteryn_client_runtime::{
        charm_progression as progression, raw_feature::GatedCommandResult,
    };
    #[test]
    fn confirmed_bestiary_stage_changes_trigger_only_the_selected_capture()
    -> Result<(), Box<dyn std::error::Error>> {
        for (id, expected) in [
            ("screenshots.bestiary-unlock", &[1, 2][..]),
            ("screenshots.bestiary-complete", &[3][..]),
            ("screenshots.bestiary-unlock", &[2][..]),
        ] {
            let mut app = super::super::tests::app(Page::Screenshots, None)?;
            let checkbox = app
                .world_mut()
                .spawn(Control::Check(id))
                .observe(super::super::toggle)
                .id();
            app.world_mut().trigger(ValueChange {
                source: checkbox,
                value: true,
                is_final: true,
            });
            let (link, mut receiver, outbox) = crate::play::play_channel();
            outbox.bevy_bestiary_initialize();
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.apply(true);
            let start = std::time::Instant::now();
            for (i, kills) in [0, 10, 20, 30, 30].into_iter().enumerate() {
                let now = start + std::time::Duration::from_secs(4 * i as u64);
                if expected == [2] && i == 1 {
                    owner.settings.preferences.screenshots.automatic_disabled = true;
                    owner.settings.prepare_screenshots(now, Some(&link));
                    owner.settings.preferences.screenshots.automatic_disabled = false;
                }
                owner.settings.prepare_screenshots(now, Some(&link));
                let crate::combat_input::PlayCommand::Feature(query) = receiver.try_recv()? else {
                    return Err("missing bestiary request".into());
                };
                link.charm_progression_edit(0, "17".into());
                assert_eq!(
                    link.charm_progression_panel().fields[0],
                    "17",
                    "background capture must not discard a player's edit"
                );
                outbox.bevy_bestiary_result(
                    &query,
                    &GatedCommandResult {
                        command_id: i as u64 + 1,
                        server_sequence: i as u64 + 1,
                        status: oteryn_session::CommandStatus::Accepted,
                        payload: progression::encode_charm_progression_reply(
                            &progression::CharmProgressionReply {
                                outcome: progression::CharmProgressionOutcome::Query,
                                fee_gold: 0,
                                bestiary: if kills == 0 {
                                    vec![]
                                } else {
                                    vec![progression::bestiary::BestiaryRaceProgress {
                                        race: 9_u32.try_into()?,
                                        kill_count: kills,
                                        kill_thresholds: [10, 20, 30],
                                    }]
                                },
                                charms: Default::default(),
                            },
                        )
                        .map_err(|e| format!("{e:?}"))?,
                    },
                );
                assert_eq!(
                    matches!(
                        owner.settings.prepare_screenshots(now, Some(&link)),
                        crate::play::screenshot::CaptureAction::Save
                    ),
                    expected.contains(&i),
                    "{id} at {kills} kills"
                );
                owner.settings.cancel_screenshot();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod reward_tests {
    use super::*;
    use bevy::ui_widgets::ValueChange;
    use oteryn_client_runtime::{
        boss_encounter,
        boss_rewards::{BossRewardsReply, BossRewardsResponse, boss_rewards},
        raw_feature::GatedCommandResult,
    };
    use oteryn_session::CommandStatus;
    #[test]
    fn boss_defeat_and_reward_treasure_require_their_own_accepted_receipts()
    -> Result<(), Box<dyn std::error::Error>> {
        for (id, boss) in [
            ("screenshots.boss", true),
            ("screenshots.reward_chest", false),
        ] {
            let mut app = super::super::tests::app(Page::Screenshots, None)?;
            let checkbox = app
                .world_mut()
                .spawn(Control::Check(id))
                .observe(super::super::toggle)
                .id();
            app.world_mut().trigger(ValueChange {
                source: checkbox,
                value: true,
                is_final: true,
            });
            let (link, mut commands, outbox) = crate::play::play_channel();
            outbox.bevy_bosstiary_initialize();
            let now = std::time::Instant::now();
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.apply(true);
            owner.settings.prepare_screenshots(now, Some(&link));
            use oteryn_client_runtime::cyclopedia_tracker::{
                TrackerEntry, TrackerSnapshot, encode_domain_snapshot,
            };
            let mut seen = false;
            for kills in [0, 1, 1] {
                let payload = encode_domain_snapshot(
                    &TrackerSnapshot {
                        boss: false,
                        entries: vec![],
                    },
                    &TrackerSnapshot {
                        boss: true,
                        entries: vec![TrackerEntry {
                            creature_key: "oteryn:boss.test".into(),
                            kill_count: kills,
                            thresholds: [5, 25, 250],
                            completed: false,
                        }],
                    },
                )
                .ok_or("snapshot")?;
                outbox.bevy_cyclopedia_tracker_push(payload);
                let save = matches!(
                    owner.settings.prepare_screenshots(now, Some(&link)),
                    crate::play::screenshot::CaptureAction::Save
                );
                assert_eq!(save, boss && kills == 1 && !seen);
                seen |= kills == 1;
                owner.settings.cancel_screenshot();
            }
            if boss {
                // Tracker pushes can arrive while the first full Bosstiary scan is in flight.
                let crate::combat_input::PlayCommand::Feature(query) = commands.try_recv()? else {
                    return Err("initial Bosstiary scan".into());
                };
                assert_eq!(query.kind, boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS);
                use oteryn_protocol_oteryn::boss_encounter::{
                    BosstiaryProgressEntry, encode_bosstiary_list,
                };
                outbox.bevy_bosstiary_result(&GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: CommandStatus::Accepted,
                    payload: encode_bosstiary_list(
                        0,
                        &[],
                        false,
                        &[BosstiaryProgressEntry {
                            boss_key: "oteryn:boss.test".into(),
                            kill_count: 1,
                        }],
                        0,
                        false,
                    )
                    .map_err(|e| format!("{e:?}"))?,
                });
                assert!(!matches!(
                    owner.settings.prepare_screenshots(now, Some(&link)),
                    crate::play::screenshot::CaptureAction::Save
                ));
            }
            link.boss_encounter_edit(0, "oteryn:boss.test".into());
            link.boss_encounter_request(oteryn_client_ui::boss_encounter::Intent::Defeated);
            assert!(
                matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.kind == boss_encounter::COMMAND_TYPE_BOSS_DEFEATED)
            );
            let mut handoff = [1; 16];
            handoff[6] = 0x70;
            handoff[8] = 0x80;
            outbox.bevy_bosstiary_result(&GatedCommandResult {
                command_id: 2,
                server_sequence: 2,
                status: CommandStatus::Accepted,
                payload: oteryn_protocol_oteryn::boss_encounter::encode_boss_activity_reply(
                    &boss_encounter::BossActivityReply::Accepted {
                        handoff,
                        deadline_unix_ms: None,
                    },
                )
                .map_err(|e| format!("{e:?}"))?,
            });
            assert!(!matches!(
                owner.settings.prepare_screenshots(now, Some(&link)),
                crate::play::screenshot::CaptureAction::Save
            ));
            owner.settings.cancel_screenshot();
            for status in [CommandStatus::DuplicateReplay, CommandStatus::Accepted] {
                outbox.bevy_boss_rewards_result(&BossRewardsResponse {
                    status,
                    reply: Some(BossRewardsReply::Collect(boss_rewards::CollectReply {
                        status: boss_rewards::BossRewardStatus::Ok,
                        moved: 2,
                        remaining: 0,
                    })),
                });
                assert_eq!(
                    matches!(
                        owner.settings.prepare_screenshots(now, Some(&link)),
                        crate::play::screenshot::CaptureAction::Save
                    ),
                    !boss && status == CommandStatus::Accepted
                );
                owner.settings.cancel_screenshot();
            }
        }
        Ok(())
    }
}
