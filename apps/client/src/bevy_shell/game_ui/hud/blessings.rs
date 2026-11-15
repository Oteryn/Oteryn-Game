//! Compact blessing ownership over the retained 0199 self-only query.
use super::*;
use bevy::{picking::hover::Hovered, window::PrimaryWindow};
use std::time::Duration;

// Storage keys and names from rulesets/character/blessings/catalogue.json.
const BLESSINGS: [(&str, &str, &str); 8] = [
    ("the_spiritual_shielding", "SS", "The Spiritual Shielding"),
    ("the_embrace_of_tibia", "ET", "The Embrace of Tibia"),
    ("the_fire_of_the_suns", "FS", "The Fire of the Suns"),
    ("the_spark_of_the_phoenix", "SP", "The Spark of the Phoenix"),
    ("the_wisdom_of_solitude", "WS", "The Wisdom of Solitude"),
    ("blood_of_the_mountain", "BM", "Blood of the Mountain"),
    ("heart_of_the_mountain", "HM", "Heart of the Mountain"),
    ("twist_of_fate", "TF", "Twist of Fate"),
];
#[derive(Component)]
struct Panel(Entity);
#[derive(Component)]
struct Tooltip;
#[derive(Default, Resource)]
struct Hover(bool);
#[derive(Default)]
struct State {
    generation: Option<u64>,
    snapshot: Option<Vec<String>>,
    last_refresh: Option<Duration>,
    hover_since: Option<Duration>,
}

/// Mount the panel and its projection together from the HUD's single spawn call.
pub(super) fn spawn(app: &mut App) {
    app.init_resource::<Hover>()
        .init_resource::<UiScale>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (hover, project).chain().after(super::project));
}
fn setup(mut commands: Commands, regions: Res<super::super::GameRegions>, skins: Res<Skins>) {
    let icons = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    let root = commands
        .spawn((
            Name::new("Blessings status"),
            Panel(icons),
            Hovered::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                padding: UiRect::all(px(4.)),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let title = commands
        .spawn(source::text(
            "Blessings",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(root).add_children(&[title, icons]);
    commands.entity(regions.equipment).add_child(root);
    commands
        .spawn((
            Name::new("Blessings tooltip"),
            Tooltip,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::all(px(6.)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(25, 25, 25)),
            GlobalZIndex(120),
        ))
        .insert(TextLayout::default());
}
fn hover(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    panels: Query<(Entity, &Hovered), With<Panel>>,
    mut hover: ResMut<Hover>,
) {
    hover.0 = !owner.modal_active()
        && panels
            .iter()
            .any(|(entity, hovered)| hovered.get() && guard.allows(entity));
}
// The independent queries keep panel nodes and the floating tooltip disjoint.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    time: Res<Time<Real>>,
    hover: Res<Hover>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<(&Panel, &mut Node), Without<Tooltip>>,
    mut tooltips: Query<(&mut Node, &mut Text), (With<Tooltip>, Without<Panel>)>,
    mut state: Local<State>,
) {
    let Ok((panel, mut root)) = panels.single_mut() else {
        return;
    };
    let Ok((mut tooltip, mut text)) = tooltips.single_mut() else {
        return;
    };
    tooltip.display = Display::None;
    let Some((play, character)) = owner.play.as_ref().and_then(|play| {
        let character = play.link.cyclopedia_character_panel();
        character.selected.then_some((play, character))
    }) else {
        root.display = Display::None;
        if state.generation.is_some() {
            commands.entity(panel.0).despawn_children();
        }
        text.0.clear();
        *state = State::default();
        return;
    };
    root.display = Display::Flex;
    let now = time.elapsed();
    let new_session = state.generation != Some(owner.generation());
    if new_session {
        *state = State {
            generation: Some(owner.generation()),
            ..default()
        };
    }
    // Ownership has no pushed wire delta. Reuse the existing query, bounded to one
    // refresh per five seconds; request_blessings also serializes character queries.
    if state
        .last_refresh
        .is_none_or(|last| now.saturating_sub(last) >= Duration::from_secs(5))
        && !character.pending
        && !character.blessings.pending
    {
        play.link.blessings_request();
        state.last_refresh = Some(now);
    }
    let blessings = character.blessings;
    let snapshot = blessings.snapshot.as_ref().map(|s| &s.blessings);
    if new_session || state.snapshot.as_ref() != snapshot {
        commands.entity(panel.0).despawn_children();
        if let Some(keys) = snapshot {
            for (key, short, _) in BLESSINGS {
                let key = format!("oteryn:blessing.{key}");
                icon(
                    &mut commands,
                    &skins,
                    panel.0,
                    &key,
                    short,
                    keys.contains(&key),
                );
            }
            // Do not drop a blessing introduced by a newer server/model.
            for key in keys {
                if !BLESSINGS
                    .iter()
                    .any(|(known, _, _)| key.strip_prefix("oteryn:blessing.") == Some(*known))
                {
                    icon(&mut commands, &skins, panel.0, key, &label(key), true);
                }
            }
        }
        state.snapshot = snapshot.cloned();
    }
    let next = snapshot.map_or_else(
        || {
            blessings
                .feedback
                .unwrap_or("Waiting for blessing state from the server.")
                .to_owned()
        },
        |keys| {
            if keys.is_empty() {
                "You are not protected by any blessing.".into()
            } else {
                format!(
                    "Active blessings:\n{}",
                    keys.iter()
                        .map(|key| label(key))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        },
    );
    if text.0 != next {
        text.0 = next;
    }
    if !hover.0 || !owner.settings.preferences.tooltips_enabled {
        state.hover_since = None;
        return;
    }
    let since = *state.hover_since.get_or_insert(now);
    if now.saturating_sub(since)
        < Duration::from_millis(u64::from(owner.settings.preferences.tooltip_delay_ms))
    {
        return;
    }
    if let Ok(window) = windows.single()
        && window.focused
        && let Some(pointer) = window.cursor_position()
    {
        let size = Vec2::new(window.width(), window.height()) / scale.0;
        let width = (size.x - 8.).clamp(1., 280.);
        let height = (text.0.lines().count() as f32 * 13. + 12.).min(size.y);
        let position = (pointer / scale.0 + Vec2::new(12., 20.))
            .min((size - Vec2::new(width, height)).max(Vec2::ZERO));
        tooltip.left = px(position.x);
        tooltip.top = px(position.y);
        tooltip.width = px(width);
        tooltip.max_height = px(size.y);
        tooltip.overflow = Overflow::clip();
        tooltip.display = Display::Flex;
    }
}
fn icon(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: &str,
    short: &str,
    active: bool,
) {
    let entity = commands
        .spawn((
            Name::new(key.to_owned()),
            source::text(
                format!("{} {short}", if active { "+" } else { "-" }),
                source::font(skins.font.clone(), false),
            ),
        ))
        .insert(TextColor(if active {
            Color::srgb_u8(120, 220, 120)
        } else {
            source::DISABLED_TEXT_COLOR
        }))
        .id();
    commands.entity(parent).add_child(entity);
}
fn label(key: &str) -> String {
    let short = key.strip_prefix("oteryn:blessing.").unwrap_or(key);
    BLESSINGS
        .iter()
        .find(|(known, _, _)| *known == short)
        .map_or_else(
            || short.replace('_', " "),
            |(_, _, name)| (*name).to_owned(),
        )
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{
        combat_input::{CombatCommand, CombatReport},
        play::Stepper,
    };
    use oteryn_client_runtime::{
        cyclopedia_character::{StoreSummary, TabView},
        raw_feature::FeatureCommand,
    };
    use oteryn_session::{
        GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
    };
    use std::time::Duration;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    struct Feed(tokio::sync::mpsc::UnboundedReceiver<Vec<String>>);
    impl Stepper for Feed {
        fn feature_selected(&self, capability: u32) -> bool {
            capability == 199
        }
        async fn feature(
            &mut self,
            command: &FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            if command.capability != 199 || command.kind != 388 || command.payload != [6, 0, 0, 20]
            {
                return Err(SessionError::CapabilityNotSelected {
                    capability: command.capability,
                });
            }
            let blessings = self.0.recv().await.ok_or(SessionError::SessionUnusable)?;
            let payload = TabView::StoreSummary(StoreSummary {
                blessings,
                permanent_prey_slots: 0,
                prey_wildcards: 0,
                hirelings: 0,
            })
            .encode()
            .map_err(|_| SessionError::SessionUnusable)?;
            Ok(GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload,
            })
        }
        fn drain(&mut self) -> Vec<SessionEvent> {
            Vec::new()
        }
        async fn serve(&mut self, _duration: Duration) -> Result<(), SessionError> {
            tokio::task::yield_now().await;
            Ok(())
        }
        async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
            Ok(None)
        }
    }
    fn run(check: impl FnOnce(&mut App) -> TestResult + Send + 'static) -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                let (mut app, commands, outbox) =
                    super::super::tests::app().map_err(|e| e.to_string())?;
                runtime.block_on(async {
                    let (send, receive) = tokio::sync::mpsc::unbounded_channel();
                    let task =
                        tokio::spawn(crate::play::run_session(Feed(receive), commands, outbox));
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while !app
                            .world()
                            .non_send::<HostOwner>()
                            .play
                            .as_ref()
                            .is_some_and(|p| p.link.cyclopedia_character_panel().selected)
                        {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    for keys in [
                        vec!["oteryn:blessing.the_spiritual_shielding".to_string()],
                        Vec::new(),
                    ] {
                        send.send(keys.clone())?;
                        app.world()
                            .non_send::<HostOwner>()
                            .play
                            .as_ref()
                            .ok_or("play")?
                            .link
                            .blessings_request();
                        tokio::time::timeout(Duration::from_secs(5), async {
                            while !app
                                .world()
                                .non_send::<HostOwner>()
                                .play
                                .as_ref()
                                .is_some_and(|p| {
                                    p.link
                                        .blessings_panel()
                                        .snapshot
                                        .as_ref()
                                        .is_some_and(|s| s.blessings == keys)
                                })
                            {
                                tokio::task::yield_now().await;
                            }
                        })
                        .await?;
                        app.update();
                        check_state(&mut app, !keys.is_empty())?;
                    }
                    check(&mut app)?;
                    task.abort();
                    Ok(())
                })
            })?
            .join()
            .map_err(|_| "blessings test thread failed")?
    }
    fn check_state(app: &mut App, active: bool) -> TestResult {
        let world = app.world_mut();
        let mut query = world.query::<(&Name, &Text, &TextColor)>();
        let (_, text, color) = query
            .iter(world)
            .find(|(name, _, _)| name.as_str() == "oteryn:blessing.the_spiritual_shielding")
            .ok_or("blessing icon missing")?;
        assert_eq!(text.0, if active { "+ SS" } else { "- SS" });
        assert_eq!(
            color.0,
            if active {
                Color::srgb_u8(120, 220, 120)
            } else {
                source::DISABLED_TEXT_COLOR
            }
        );
        let (_, inactive, _) = query
            .iter(world)
            .find(|(name, _, _)| name.as_str() == "oteryn:blessing.twist_of_fate")
            .ok_or("inactive blessing icon missing")?;
        assert_eq!(inactive.0, "- TF");
        let (_, tooltip, _) = query
            .iter(world)
            .find(|(name, _, _)| name.as_str() == "Blessings tooltip")
            .ok_or("tooltip missing")?;
        assert_eq!(
            tooltip.0,
            if active {
                "Active blessings:\nThe Spiritual Shielding"
            } else {
                "You are not protected by any blessing."
            }
        );
        Ok(())
    }
    #[test]
    fn blessing_icons_state() -> TestResult {
        run(|_| Ok(()))
    }
    #[test]
    fn blessing_tooltip_none() -> TestResult {
        run(|app| {
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .settings
                .preferences
                .tooltip_delay_ms = 0;
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .settings
                .preferences
                .tooltips_enabled = true;
            let mut window = Window::default();
            window.set_cursor_position(Some(Vec2::new(200., 100.)));
            app.world_mut().spawn((window, bevy::window::PrimaryWindow));
            let panel = app
                .world_mut()
                .query::<(Entity, &Name)>()
                .iter(app.world())
                .find_map(|(entity, name)| (name.as_str() == "Blessings status").then_some(entity))
                .ok_or("blessings panel")?;
            app.world_mut()
                .entity_mut(panel)
                .insert(bevy::picking::hover::Hovered(true));
            app.update();
            let visible = app
                .world_mut()
                .query::<(&Name, &Node)>()
                .iter(app.world())
                .any(|(name, node)| {
                    name.as_str() == "Blessings tooltip" && node.display == Display::Flex
                });
            assert!(visible, "hover must show the actual tooltip");
            assert!(
                app.world_mut()
                    .query::<(&Name, &Text)>()
                    .iter(app.world())
                    .any(|(name, text)| name.as_str() == "Blessings tooltip"
                        && text.0 == "You are not protected by any blessing.")
            );
            Ok(())
        })
    }
}
