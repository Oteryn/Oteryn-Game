use super::*;

fn member() -> GrizzlyState {
    [
        (quest::JOIN_STORAGE, 1),
        (quest::QUEST_LOG_STORAGE, 0),
        (quest::POINTS_STORAGE, 0),
        (quest::BOSS_POINTS_STORAGE, 0),
    ]
    .into_iter()
    .map(|(key, value)| (quest::storage_track(key), value))
    .collect()
}

fn player() -> GrizzlyActor {
    GrizzlyActor {
        level: 20,
        premium: true,
        level_after_reward: None,
    }
}

#[test]
fn grizzly_task_choice_requires_confirmation_and_cancel_resets_topic() {
    let state = member();
    let mut topic = Topic::None;
    assert!(
        dialogue(&state, player(), &mut topic, "Crocodiles")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::Start(1));
    assert_eq!(
        dialogue(&state, player(), &mut topic, "YES").actions,
        vec![GrizzlyAction::StartTask(1)]
    );
    assert_eq!(topic, Topic::None);
    assert!(
        dialogue(&state, player(), &mut topic, "yes")
            .actions
            .is_empty()
    );
    let _ = dialogue(&state, player(), &mut topic, "crocodiles");
    let _ = dialogue(&state, player(), &mut topic, "no");
    assert!(
        dialogue(&state, player(), &mut topic, "yes")
            .actions
            .is_empty()
    );
}

#[test]
fn grizzly_cancel_requires_named_active_task_and_confirmation() {
    let mut state = member();
    state.insert(quest::storage_track(quest::QUEST_STORAGE_BASE + 1), 1);
    let mut topic = Topic::None;
    assert!(
        dialogue(&state, player(), &mut topic, "cancel")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::CancelMenu);
    assert!(
        dialogue(&state, player(), &mut topic, "crocodiles")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::Cancel(1));
    assert_eq!(
        dialogue(&state, player(), &mut topic, "yes").actions,
        vec![GrizzlyAction::CancelTask(1)]
    );
}

#[test]
fn grizzly_active_outgrown_task_status_clears_the_previous_start_confirmation() {
    let mut state = member();
    state.insert(quest::storage_track(quest::QUEST_STORAGE_BASE + 1), 1);
    state.insert(quest::storage_track(quest::KILL_COUNTER_BASE + 1), 37);
    let actor = GrizzlyActor {
        level: 50,
        ..player()
    };
    let mut topic = Topic::None;
    assert!(
        dialogue(&state, actor, &mut topic, "ice golems")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::Start(12));
    let status = dialogue(&state, actor, &mut topic, "crocodile");
    assert!(status.actions.is_empty());
    assert!(status.line.contains("37/300"));
    assert_eq!(topic, Topic::None);
    assert!(
        dialogue(&state, actor, &mut topic, "yes")
            .actions
            .is_empty(),
        "the status request cannot confirm the previous ice-golem choice"
    );
}

#[test]
fn grizzly_eligible_special_request_clears_an_ordinary_task_confirmation() {
    let mut state = member();
    state.insert(quest::storage_track(quest::POINTS_STORAGE), 70);
    let actor = GrizzlyActor {
        level: 80,
        ..player()
    };
    let mut topic = Topic::None;
    assert!(
        dialogue(&state, actor, &mut topic, "underwater quara")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::Start(23));
    assert_eq!(
        dialogue(&state, actor, &mut topic, "special").actions,
        vec![GrizzlyAction::StartSpecialTask]
    );
    assert_eq!(topic, Topic::None);
    assert!(
        dialogue(&state, actor, &mut topic, "yes")
            .actions
            .is_empty()
    );
}

#[test]
fn grizzly_one_task_utterance_freezes_all_ready_tasks() {
    let mut state = member();
    for id in [1u8, 2] {
        state.insert(
            quest::storage_track(quest::QUEST_STORAGE_BASE + u32::from(id)),
            1,
        );
        state.insert(
            quest::storage_track(quest::KILL_COUNTER_BASE + u32::from(id)),
            1_000,
        );
    }
    let reply = dialogue(&state, player(), &mut Topic::None, "tasks");
    assert_eq!(
        reply.actions,
        vec![
            GrizzlyAction::CompleteTask(1),
            GrizzlyAction::CompleteTask(2)
        ]
    );
    state.insert(quest::storage_track(quest::POINTS_STORAGE), 10);
    assert!(
        dialogue(&state, player(), &mut Topic::None, "tasks")
            .actions
            .is_empty()
    );
    assert_eq!(
        dialogue(&state, player(), &mut Topic::None, "promotion").actions,
        vec![GrizzlyAction::Promote]
    );
}

#[test]
fn grizzly_nonpremium_cannot_confirm_hidden_task_and_unjoined_must_join() {
    let state = member();
    let mut topic = Topic::None;
    let free = GrizzlyActor {
        premium: false,
        ..player()
    };
    let reply = dialogue(&state, free, &mut topic, "crocodiles");
    assert!(reply.actions.is_empty());
    assert_eq!(topic, Topic::None);
    assert!(reply.line.contains("Premium"));
    assert!(
        dialogue(&GrizzlyState::new(), player(), &mut topic, "tasks")
            .actions
            .is_empty()
    );
    assert_eq!(
        dialogue(&GrizzlyState::new(), player(), &mut topic, "join").actions,
        vec![GrizzlyAction::Join]
    );
}

#[test]
fn grizzly_direct_creature_name_refuses_an_unjoined_player_without_auto_join() {
    let state = GrizzlyState::new();
    let mut topic = Topic::None;
    let reply = dialogue(&state, player(), &mut topic, "crocodiles");
    assert!(
        reply.actions.is_empty(),
        "Q16: a creature name neither joins nor starts a task"
    );
    assert!(reply.line.contains("join"));
    assert_eq!(topic, Topic::None);
    assert!(!quest::joined(&state));
    assert_eq!(
        quest::plan_action(&state, player(), &GrizzlyAction::StartTask(1)),
        Err(quest::GrizzlyRefusal::NotJoined)
    );
}

#[test]
fn grizzly_retained_boss_menu_cannot_spend_after_the_last_point()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = member();
    state.insert(quest::storage_track(quest::BOSS_POINTS_STORAGE), 1);
    let mut topic = Topic::None;
    assert!(
        dialogue(&state, player(), &mut topic, "bosses")
            .actions
            .is_empty()
    );
    assert_eq!(topic, Topic::BossMenu);
    let first = dialogue(&state, player(), &mut topic, "snapper");
    assert_eq!(first.actions, vec![GrizzlyAction::SelectBoss(1)]);
    let plan = quest::plan_action(&state, player(), &first.actions[0])
        .map_err(|error| format!("first boss selection: {error:?}"))?;
    for change in plan.changes {
        state.insert(change.track, change.after);
    }
    assert_eq!(quest::storage_value(&state, quest::BOSS_POINTS_STORAGE), 0);
    assert_eq!(topic, Topic::BossMenu, "the source retains the menu topic");
    let second = dialogue(&state, player(), &mut topic, "hide");
    assert_eq!(second.actions, vec![GrizzlyAction::SelectBoss(2)]);
    assert_eq!(
        quest::plan_action(&state, player(), &second.actions[0]),
        Err(quest::GrizzlyRefusal::BossPoints)
    );
    assert_eq!(
        quest::storage_value(&state, quest::BOSS_POINTS_STORAGE),
        0,
        "Q17: the retained menu cannot create a negative balance"
    );
    assert_eq!(
        quest::storage_value(&state, quest::boss(2).ok_or("Hide fixture")?.storage),
        -1
    );
    Ok(())
}

#[test]
fn grizzly_failed_boss_is_rearmed_before_completed_boss_is_reported()
-> Result<(), Box<dyn std::error::Error>> {
    let data =
        quest::catalogue().map_err(|error| format!("Grizzly fixture catalogue: {error:?}"))?;
    let mut state = member();
    let [first, second, ..] = data.bosses.as_slice() else {
        return Err("two boss fixtures required".into());
    };
    for (boss, kills) in [(first, 0), (second, 1)] {
        state.insert(quest::storage_track(boss.storage), 2);
        state.insert(quest::storage_track(boss.kill_storage), kills);
    }
    assert_eq!(
        dialogue(&state, player(), &mut Topic::None, "report").actions,
        vec![
            GrizzlyAction::ReportBoss(first.id),
            GrizzlyAction::ReportBoss(second.id)
        ]
    );
    Ok(())
}
