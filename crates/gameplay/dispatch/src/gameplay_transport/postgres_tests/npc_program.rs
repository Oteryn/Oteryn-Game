//! Production NPC command, atomic acceptance, bound replay and session fence qualification.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
    world_callbacks::Position,
};

const ACCEPT: &str = "oteryn:quest-npc/the_order_of_the_stag/kesar_the_younger_day/writ-offer";
const KESAR: &str = "oteryn:npc.kesar_the_younger_day";
const DEMERON: &str = "oteryn:npc.second_in_command_demeron";
const STAG: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";

#[test]
fn catalogue_npc_source_acceptance_is_atomic_and_replay_bound() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("NPC_PROGRAM_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_source_acceptance_is_atomic_and_replay_bound {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let source = NpcRuleCatalog::load_embedded().map_err(debug)?;
    // The merged Paradox and citizen Source owners add exactly two programs,
    // 31 rules and four NPCs. Keep the prior cohort's exact census as well.
    let inherited = source
        .programs()
        .iter()
        .filter(|p| {
            p.document.quest != "oteryn:quest.the_paradox_tower_quest"
                && p.document.program_key.as_deref()
                    != Some("oteryn:quest-npc-program.citizen-source-sagas")
        })
        .collect::<Vec<_>>();
    assert_eq!(source.programs().len(), 25);
    assert_eq!(inherited.len(), 23);
    assert_eq!(
        inherited
            .iter()
            .map(|p| p.document.rules.len())
            .sum::<usize>(),
        1212
    );
    let unique = inherited
        .iter()
        .flat_map(|p| p.document.rules.iter().map(|r| r.npc.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 97);
    for (quest, program_key, count, npcs) in [
        (
            "oteryn:quest.the_paradox_tower_quest",
            None,
            28,
            vec!["oteryn:npc.riddler"],
        ),
        (
            "oteryn:quest.outfit_and_addon_quests",
            Some("oteryn:quest-npc-program.citizen-source-sagas"),
            3,
            vec!["oteryn:npc.amber", "oteryn:npc.hanna", "oteryn:npc.lubo"],
        ),
    ] {
        let added = source
            .programs()
            .iter()
            .filter(|p| p.document.quest == quest && p.document.program_key.as_deref() == program_key)
            .collect::<Vec<_>>();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].document.rules.len(), count);
        assert_eq!(
            added[0]
                .document
                .rules
                .iter()
                .map(|rule| rule.npc.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            npcs
        );
    }
    let [kesar, demeron, tereban] =
        super::classic_quests::install(owner, admitted, [KESAR, DEMERON, "oteryn:npc.tereban"])
            .await?;
    super::classic_quests::talk(owner, admitted, kesar, 2400, "hi").await?;
    // An unfinished Lion prerequisite cannot create a Stag receipt or source stage.
    super::classic_quests::talk(owner, admitted, kesar, 2401, "help").await?;
    let mut fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let empty = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert!(!empty.tracks().contains_key(STAG));
    // Establish the completed prior Lion mission using its existing native owner.
    // This fixture does not add a route around any missing Stag item hand-in.
    for (id, key) in [(2410, "npc_3"), (2411, "creature_event_1")] {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome=slot.commit_quest_transition(owner.root,authority,owner.holder,fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key:format!("oteryn:quest-transition/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission/{key}"),
                cause:crate::durability::quest_state::QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(id).map_err(debug)?)),
            },owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone()).await.map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    // Both the foreign Lion >=5 and Stag <1 predicates are rechecked by the durable owner.
    let accepted = super::classic_quests::talk(owner, admitted, kesar, 2402, "help").await?;
    assert!(!accepted.is_empty());
    fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, STAG).await?,
        1
    );
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(2402).map_err(debug)?,
        ),
        npc: KESAR.into(),
        trigger: NpcRuleTrigger::Say,
        message: "help".into(),
        topic: 0,
        transition_key: ACCEPT.into(),
    };
    let replay = owner
        .root
        .reconcile_npc_program(authority, fence.character_id, request.clone())
        .await
        .map_err(debug)?
        .ok_or("receipt")?;
    assert_eq!(replay.reply, accepted);
    let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else {
        return Err("replayed receipt".into());
    };
    assert_eq!(receipt.changes.len(), 1);
    assert_eq!(receipt.changes[0].before, -1);
    assert_eq!(
        receipt.committed_character_revision.get(),
        receipt.original_character_revision.get() + 1
    );
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let mut changed = request.clone();
    changed.message = "help different".into();
    assert!(matches!(owner.root.reconcile_npc_program(authority,fence.character_id,changed).await,Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)));
    let mut stale = fence;
    stale.character_lease_generation += 1;
    let mut fresh = request;
    fresh.command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(2403).map_err(debug)?,
    );
    let item_permit = owner.spell_lane_permit().await.ok_or("NPC item lane")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    assert!(matches!(
        slot.commit_npc_program(
            owner.root,
            authority,
            owner.holder,
            &item_permit,
            stale,
            fresh,
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(
                owner
                    .active_generation
                    .and_then(|g| g.native_gameplay())
                    .ok_or("native")?
            ),
        )
        .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    drop(slot);
    drop(item_permit);
    assert_eq!(
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    // Demeron's distinct <4 guard reads the same authoritative stage. These
    // source topic changes do not invent a revision or consume a proof item.
    super::classic_quests::talk(owner, admitted, demeron, 2404, "hi").await?;
    assert!(
        !super::classic_quests::talk(owner, admitted, demeron, 2405, "mission")
            .await?
            .is_empty()
    );
    super::classic_quests::talk(owner, admitted, demeron, 2406, "tell").await?;
    for id in 2420..2423 {
        super::classic_quests::talk(owner, admitted, demeron, id, "yes").await?;
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(ACCEPT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    // The source absent sentinel remains readable after lowering to the native
    // boolean/counter domains. Acceptance is still one eleven-track receipt.
    for (command, message) in [(2430, "hi"), (2431, "mission"), (2432, "yes")] {
        super::classic_quests::talk(owner, admitted, tereban, command, message).await?;
    }
    let father: i32 = sqlx::query_scalar(
        "SELECT cardinality(track_keys) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind("oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(father, 11);
    for (suffix, expected) in [("quest_log", 1), ("status", 1), ("progress", 0)] {
        let key = format!("oteryn:quest-progress/quest/u8_6/afathers_burden/{suffix}");
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, &key).await?,
            expected
        );
    }
    Ok(())
}

#[test]
fn catalogue_npc_delayed_end_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-end", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-end",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_delayed_end_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_end(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let [emperor] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.emperor_rehal"]).await?;
    super::classic_quests::talk(owner, admitted, emperor, 2800, "hi").await?;
    let session = admitted.game_session_id;
    assert!(owner.states.chat.engaged(session).is_some());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let update = {
            let runtime = owner.runtime.lock().await;
            owner.states.npc_conversations.follow(
                &owner.states.chat,
                &runtime,
                session,
                Some(emperor),
            )
        };
        if let Some(update) = update
            && update.conversation.is_some_and(|c| c.open.is_none())
        {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("authored delayed close did not reach domain 7".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(owner.states.chat.engaged(session).is_none());
    Ok(())
}

#[test]
fn catalogue_npc_calendar_month_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-calendar", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-calendar",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_calendar_month_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_calendar(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [chef] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.jean_pierre"]).await?;
    let month: i32 = sqlx::query_scalar(
        "SELECT extract(month FROM statement_timestamp() AT TIME ZONE 'UTC')::integer",
    )
    .fetch_one(&h.pool)
    .await?;
    let lines = super::classic_quests::talk(owner, admitted, chef, 2900, "hi").await?;
    if month == 8 {
        assert!(
            !lines.is_empty(),
            "August opens the source cooking conversation"
        );
    } else {
        assert!(
            lines.is_empty(),
            "the source rejects the greeting outside August"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while owner
            .states
            .chat
            .engaged(admitted.game_session_id)
            .is_some()
        {
            {
                let runtime = owner.runtime.lock().await;
                let _ = owner.states.npc_conversations.follow(
                    &owner.states.chat,
                    &runtime,
                    admitted.game_session_id,
                    Some(chef),
                );
            }
            if std::time::Instant::now() >= deadline {
                return Err("outside-August dismissal did not close".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    Ok(())
}

#[test]
fn catalogue_npc_general_xp_handin_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-xp", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-xp",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_general_xp_handin_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_xp(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    const TRACK: &str =
        "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/harlan";
    const NPC: &str = "oteryn:npc.lieutenant_harlan";
    const RULE: &str = "oteryn:quest-npc/the_order_of_the_stag/lieutenant_harlan/supplies-deliver";
    let initial: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    let [harlan] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    for (command, text) in [
        (3000, "hi"),
        (3001, "help"),
        (3002, "yes"),
        (3003, "no"),
        (3004, "help"),
        (3005, "yes"),
    ] {
        super::classic_quests::talk(owner, admitted, harlan, command, text).await?;
    }
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    let premature: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(RULE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(premature, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for index in 0..20_u64 {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: "oteryn:item.tibia.i52636".into(),
                count: 1,
            },
            600 + index,
            true,
        )
        .await?;
    }
    super::classic_quests::talk(owner, admitted, harlan, 3006, "help").await?;
    super::classic_quests::talk(owner, admitted, harlan, 3007, "yes").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        3
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let total: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(total, initial + 10_000);
    let consumed: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1 AND before_item->>'definition_production_key'='oteryn:item.tibia.i52636'").bind(RULE).fetch_one(&h.pool).await?;
    assert_eq!(consumed, 20);
    let replay = owner
        .root
        .reconcile_npc_program(
            authority,
            fence.character_id,
            NpcProgramRequest {
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(3007).map_err(debug)?,
                ),
                npc: NPC.into(),
                trigger: NpcRuleTrigger::Say,
                message: "yes".into(),
                topic: 3,
                transition_key: RULE.into(),
            },
        )
        .await
        .map_err(debug)?
        .ok_or("XP receipt")?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let after_replay: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after_replay, total);
    Ok(())
}
