use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::battle::Action;
use oteryn_session::{ActorCombatState, AttackIntentDisposition, FightModes};

#[test]
fn named_battle_lists_filter_sort_save_and_attack_live_entities()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0017_attack::view::{BattleListPreferences, BattleSort};
    use crate::play::{PreferenceStore, ReferencePreferences};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
    };
    let (link, mut commands, outbox) = play_channel();
    let creature = |id, health| WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
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
    let low = creature(1, 20);
    let high = WorldSpatialEntity {
        display_name: Some(
            oteryn_session::EntityDisplayName::new("River wolf")
                .map_err(|e| std::io::Error::other(format!("{e:?}")))?,
        ),
        ..creature(2, 90)
    };
    let player = WorldSpatialEntity {
        kind: EntityKind::Player,
        ..creature(3, 50)
    };
    outbox.battle_publish(
        Some(ActorCombatState {
            target: None,
            modes: FightModes::DEFAULT,
            in_fight: false,
        }),
        Some(7),
        &[high, player, low],
    );
    let mut preferences = ReferencePreferences {
        battle_lists: vec![
            BattleListPreferences {
                name: "Wounded".into(),
                players: false,
                max_health: 50,
                sort: BattleSort::HealthAscending,
                ..Default::default()
            },
            BattleListPreferences {
                name: "Players".into(),
                creatures: false,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(
        link.sync_battle_lists(&mut preferences.battle_lists, 0)
            .is_none()
    );
    let sorted = BattleListPreferences {
        sort: BattleSort::HealthAscending,
        ..Default::default()
    };
    assert_eq!(
        sorted.rows(&link.battle_panel().rows),
        vec![low, player, high]
    );
    let search = BattleListPreferences {
        search: "WOLF".into(),
        ..Default::default()
    };
    assert_eq!(search.rows(&link.battle_panel().rows), vec![high]);
    let rows = preferences.battle_lists[0].rows(&link.battle_panel().rows);
    assert_eq!(rows, vec![low]);
    assert_eq!(
        preferences.battle_lists[1].rows(&link.battle_panel().rows),
        vec![player]
    );
    link.battle_request(Action::Attack(rows[0].entity));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Battle(CombatCommand::Attack(target)) if target == low.entity)
    );
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        link.battle_panel().combat,
        Some(CombatCommand::Attack(low.entity)),
    );
    outbox.battle_publish(link.battle_panel().combat, Some(7), &[high]);
    link.battle_request(Action::Attack(low.entity));
    assert!(commands.try_recv().is_err());
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle_lists.preferences[0].name = "Low health".into();
        mailbox.battle_lists.dirty = true;
        mailbox.battle_lists.edit = crate::play::feature_0017_attack::view::next_edit();
    }
    assert!(
        link.sync_battle_lists(&mut preferences.battle_lists, 0)
            .is_some()
    );
    assert_eq!(preferences.battle_lists[0].name, "Low health");
    assert!(
        link.sync_battle_lists(&mut preferences.battle_lists, 0)
            .is_none()
    );
    let path = std::env::temp_dir().join(format!("clientgap-battle-{}.json", std::process::id()));
    let mut store = PreferenceStore::defaults(path.clone());
    store.apply(preferences.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &preferences
    );
    let original = std::fs::read(&path)?;
    preferences.battle_lists[1].name = "Low health".into();
    assert!(store.apply(preferences).is_err());
    assert_eq!(std::fs::read(&path)?, original);
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.sync_battle_lists(&link);
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle_lists.preferences[0].name = "Earlier edit".into();
        mailbox.battle_lists.edit = crate::play::feature_0017_attack::view::next_edit();
        mailbox.battle_lists.dirty = true;
    }
    settings.reset_defaults()?;
    settings.sync_battle_lists(&link);
    assert_eq!(settings.preferences, ReferencePreferences::default());
    assert_eq!(
        lock(&link.mailbox).battle_lists.preferences,
        settings.preferences.battle_lists
    );
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle_lists.preferences[0].name = "After reset".into();
        mailbox.battle_lists.edit = crate::play::feature_0017_attack::view::next_edit();
        mailbox.battle_lists.dirty = true;
    }
    settings.sync_battle_lists(&link);
    assert_eq!(
        PreferenceStore::load(path.clone())?
            .preferences()
            .battle_lists[0]
            .name,
        "After reset"
    );
    std::fs::remove_file(path.with_extension("backup.json"))?;
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn world_click_report_does_not_unlock_a_pending_panel_command() {
    let (link, _commands, outbox) = play_channel();
    let state = ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    };
    lock(&link.mailbox).battle.combat = Some(state);
    link.battle_request(Action::Stop);
    assert!(link.battle_panel().pending);
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        Some(state),
        None,
    );
    assert!(link.battle_panel().pending);
    // The panel's matching stop response releases the controls.
    outbox.battle_result(
        CombatReport::Stop(AttackIntentDisposition::Ok),
        Some(state),
        Some(CombatCommand::Stop),
    );
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().feedback, Some("Attack stopped"));
}

#[test]
fn feedback_matches_both_panel_origin_and_command_kind() {
    let (link, _commands, outbox) = play_channel();
    let command = CombatCommand::Attack(oteryn_session::EntityRef {
        identity: [9; 16],
        generation: 1,
    });
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle.pending = true;
        mailbox.battle.pending_command = Some(command);
    }
    // An earlier world click can answer with the same report kind as a panel attack.
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        None,
        None,
    );
    assert!(link.battle_panel().pending);
    // Even a panel-origin report must answer the pending command kind.
    outbox.battle_result(
        CombatReport::Modes(AttackIntentDisposition::Ok),
        None,
        Some(command),
    );
    assert!(link.battle_panel().pending);
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        None,
        Some(command),
    );
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().pending_command, None);
}

#[test]
fn failed_panel_send_releases_pending_controls() {
    let (link, commands, _outbox) = play_channel();
    drop(commands);
    lock(&link.mailbox).battle.combat = Some(ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    });
    link.battle_request(Action::Stop);
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().pending_command, None);
}

#[test]
fn active_named_battle_list_survives_character_switches() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::feature_0017_attack::view::BattleListPreferences;
    use crate::play::{PreferenceStore, SettingsState};
    let root = std::env::temp_dir().join(format!("active-battle-list-{}", std::process::id()));
    let base = root.join("preferences.json");
    let mut settings = SettingsState::new(Some(base.clone()));
    settings.select_character(Some([15; 16]));
    let mut preferences = settings.preferences.clone();
    preferences.battle_lists.push(BattleListPreferences {
        name: "Targets".into(),
        ..Default::default()
    });
    settings.apply(preferences);
    let (link, _, _) = play_channel();
    settings.sync_battle_lists(&link);
    assert!(link.battle_select_list(1));
    settings.sync_battle_lists(&link);
    assert_eq!(
        PreferenceStore::load(PreferenceStore::character_path(&base, [15; 16]))?
            .preferences()
            .active_battle_list,
        1
    );
    settings.select_character(Some([40; 16]));
    settings.sync_battle_lists(&link);
    assert_eq!(link.active_battle_list(), 0);
    settings.select_character(Some([15; 16]));
    settings.sync_battle_lists(&link);
    assert_eq!(link.active_battle_list(), 1);
    assert!(!link.battle_select_list(2));
    assert_eq!(link.active_battle_list(), 1);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
