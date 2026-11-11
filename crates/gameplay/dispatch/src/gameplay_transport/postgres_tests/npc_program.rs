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
            &owner.premium_coordinator,
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

const FATHER: &str = "oteryn:quest.a_father_s_burden_quest";
const FATHER_ACCEPT: &str =
    "oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept";

#[test]
fn catalogue_npc_father_fresh_acceptance_is_one_eleven_track_transaction() -> TestResult {
    father_case(false)
}

#[test]
fn catalogue_npc_father_legacy_hash_preserves_defaults_and_restart_receipts() -> TestResult {
    father_case(true)
}

fn father_case(legacy: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let name = if legacy {
        "sidec-catalogue-npc-legacy"
    } else {
        "sidec-catalogue-npc-father"
    };
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED {name} {:?}", started.elapsed());
    Ok(())
}

pub(super) async fn exercise_father(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    legacy: bool,
    handins: bool,
) -> TestResult {
    use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let mut fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    let old = oteryn_gameplay_quests::quest::loader::parse_quest_state(
        include_str!("../../../../../../content/quests/missions/quest-state.json"),
        catalogue.content_revision(),
    )
    .map_err(debug)?;
    if legacy {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seeded = slot
            .commit_quest_transition(
                owner.root,
                authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_6/afathers_burden/quest_log/npc_1".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(2500).map_err(debug)?,
                    )),
                },
                std::sync::Arc::new(old.catalogue().clone()),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
    }
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    if legacy {
        let state = before.states().get(FATHER).ok_or("legacy state")?;
        assert_eq!(
            Some(state.definition_hash),
            old.catalogue().definition_hash(FATHER)
        );
        assert!(
            oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                &catalogue,
                FATHER,
                &state.definition_hash
            )
            .is_some()
        );
    }
    let [tereban] = super::classic_quests::install(owner, admitted, ["oteryn:npc.tereban"]).await?;
    super::classic_quests::talk(owner, admitted, tereban, 2501, "hi").await?;
    let offer = super::classic_quests::talk(owner, admitted, tereban, 2502, "mission").await?;
    assert!(offer.iter().any(|line| line.contains("four sons")));
    // Force failure after the receipt and earlier tracks were attempted. The real
    // dispatch must expose no partial tracks, pin, response receipt or revision.
    sqlx::query("CREATE SEQUENCE fixture_father_failure_seen")
        .execute(&h.pool)
        .await?;
    sqlx::query(
        "GRANT USAGE, SELECT ON SEQUENCE fixture_father_failure_seen TO oteryn_game_runtime",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("CREATE FUNCTION questpc_npc_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.track_key = 'oteryn:quest-progress/quest/u8_6/afathers_burden/wood' THEN PERFORM nextval('fixture_father_failure_seen'); RAISE EXCEPTION 'questpc injected final-track failure'; END IF; RETURN NEW; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER questpc_npc_fail BEFORE INSERT OR UPDATE ON game_character_quest_tracks FOR EACH ROW EXECUTE FUNCTION questpc_npc_fail()")
        .execute(&h.pool).await?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 2503,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: tereban,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(result.disposition, NpcIntentDisposition::Rejected);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_father_failure_seen")
            .fetch_one(&h.pool)
            .await?,
        "the rollback must reach the final material-track write"
    );
    assert_eq!(
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let absent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(FATHER_ACCEPT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(absent, 0);
    sqlx::query("DROP TRIGGER questpc_npc_fail ON game_character_quest_tracks")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION questpc_npc_fail()")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP SEQUENCE fixture_father_failure_seen")
        .execute(&h.pool)
        .await?;
    // Resolve a possibly unknown outcome before recreating a conversation.
    let reconciled = owner
        .resume_npc_bank(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 2503,
                item_fence: Some(fence.item_fence()),
            },
        )
        .await
        .ok_or("retained Father acceptance")?;
    assert!(!reconciled.unknown);
    assert_eq!(reconciled.disposition, NpcIntentDisposition::Rejected);
    super::classic_quests::talk(owner, admitted, tereban, 2504, "hi").await?;
    super::classic_quests::talk(owner, admitted, tereban, 2505, "mission").await?;
    let reply = super::classic_quests::talk(owner, admitted, tereban, 2506, "yes").await?;
    assert!(reply.iter().any(|line| line.contains("relieved")));
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(2506).map_err(debug)?,
        ),
        npc: "oteryn:npc.tereban".into(),
        trigger: NpcRuleTrigger::Say,
        message: "yes".into(),
        topic: 1,
        transition_key: FATHER_ACCEPT.into(),
    };
    // Clear the session copy and reload it from persistence, then recover the
    // historical command response without executing a second acceptance.
    owner.admit_quest_session(admitted).await;
    let replay = owner
        .root
        .reconcile_npc_program(authority, fence.character_id, request.clone())
        .await
        .map_err(debug)?
        .ok_or("father receipt")?;
    assert_eq!(reply, replay.reply);
    let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else {
        return Err("father replay".into());
    };
    assert_eq!(receipt.changes.len(), 11);
    assert_eq!(
        receipt.committed_character_revision.get(),
        receipt.original_character_revision.get() + 1
    );
    for change in &receipt.changes {
        assert_eq!(
            change.before,
            if legacy {
                if change.track.ends_with("/quest_log") {
                    1
                } else {
                    0
                }
            } else {
                -1
            }
        );
        assert_eq!(
            change.after,
            if change.track.ends_with("/progress") {
                0
            } else {
                1
            }
        );
    }
    if legacy {
        assert_eq!(
            Some(receipt.definition_hash),
            old.catalogue().definition_hash(FATHER)
        );
    } else {
        assert_eq!(
            Some(receipt.definition_hash),
            catalogue.definition_hash(FATHER)
        );
    }
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        after
            .states()
            .get(FATHER)
            .ok_or("father state")?
            .definition_hash,
        receipt.definition_hash
    );
    assert_eq!(
        after
            .tracks()
            .iter()
            .filter(|(key, _)| key.starts_with("oteryn:quest-progress/quest/u8_6/afathers_burden/"))
            .count(),
        11
    );
    let mut changed = request;
    changed.message = "not yes".into();
    assert!(matches!(owner.root.reconcile_npc_program(authority, fence.character_id, changed).await,
        Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(FATHER_ACCEPT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let native = slot
        .commit_quest_transition(
            owner.root,
            authority,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/quest/u8_6/afathers_burden/status/npc_1"
                    .into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2507).map_err(debug)?,
                )),
            },
            if legacy {
                catalogue
            } else {
                std::sync::Arc::new(old.catalogue().clone())
            },
        )
        .await
        .map_err(debug)?;
    if legacy {
        // An existing native transition also keeps the exact historical pin.
        let QuestTransitionOutcome::Committed(native) = native else {
            return Err("legacy native writer compatibility".into());
        };
        assert_eq!(
            Some(native.definition_hash),
            old.catalogue().definition_hash(FATHER)
        );
        assert_eq!(native.changes[0].before, 1);
        assert_eq!(native.changes[0].after, 2);
        let values = owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?
            .tracks()
            .clone();
        fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        // Reverse compatibility for the historical pin: the original artifact
        // can still execute an original native transition after the new owner.
        let reverse = slot
            .commit_quest_transition(
                owner.root,
                authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_6/afathers_burden/quest_log/npc_1".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(2508).map_err(debug)?,
                    )),
                },
                std::sync::Arc::new(old.catalogue().clone()),
            )
            .await
            .map_err(debug)?;
        let QuestTransitionOutcome::Committed(reverse) = reverse else {
            return Err("legacy original artifact compatibility".into());
        };
        assert_eq!(
            Some(reverse.definition_hash),
            old.catalogue().definition_hash(FATHER)
        );
        assert_eq!(
            owner
                .root
                .read_character_quest_state(authority, fence.character_id)
                .await
                .map_err(debug)?
                .tracks(),
            &values
        );
    } else {
        // The old artifact cannot reinterpret a new pin: incompatible rollback
        // refuses before a write rather than silently replacing the definition.
        assert!(matches!(
            native,
            QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::RevisionMismatch
            )
        ));
    }
    drop(slot);
    if handins {
        super::fathers_burden::exercise(owner, admitted, h, authority, tereban).await?;
    }
    Ok(())
}

#[test]
fn catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("sidec-catalogue-npc-scoped", || {
        super::quest_world_callbacks::scenario(
            "sidec-catalogue-npc-scoped",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn hero_of_rathleton_pipe_greeting_commits_access_once_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("sidec-catalogue-npc-hero", || {
        super::quest_world_callbacks::scenario(
            "sidec-catalogue-npc-hero",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED hero_of_rathleton_pipe_greeting_commits_access_once_and_replays {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_scoped(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    hero_only: bool,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    // Static NPC placement is a once-per-Channel operation. Keep every actor
    // alive while the witness visits the existing source conversations.
    let [pipe, palimuth, jean, eustacio, xelvar] = super::classic_quests::install(
        owner,
        admitted,
        [
            "oteryn:npc.strange_pipe",
            "oteryn:npc.palimuth",
            "oteryn:npc.jean_pierre",
            "oteryn:npc.eustacio",
            "oteryn:npc.xelvar",
        ],
    )
    .await?;
    let greeting = super::classic_quests::talk(owner, admitted, pipe, 2600, "hi").await?;
    let source_catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = source_catalogue
        .service_model_for_project()
        .map_err(debug)?;
    let source_greeting = model
        .npcs()
        .iter()
        .find(|(reference, _)| reference.key == "oteryn:npc.strange_pipe")
        .and_then(|(_, npc)| npc.bank_dialogue.as_ref())
        .and_then(|dialogue| dialogue.greet.first());
    assert_eq!(source_greeting, None, "pipe inherits NpcHandler greeting");
    assert_eq!(
        greeting,
        vec![oteryn_gameplay_npc::quest_greetings::render_player_name(
            "Greetings, |PLAYERNAME|.",
            "Quest witness"
        )]
    );
    // Reopen the ordinary conversation, then replay the original source callback
    // occurrence: its empty reply must not suppress the original handler greeting.
    super::classic_quests::talk(owner, admitted, pipe, 2613, "bye").await?;
    assert_eq!(
        super::classic_quests::talk(owner, admitted, pipe, 2600, "hi").await?,
        greeting
    );
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/hero_rathleton/access_door"
        )
        .await?,
        1
    );
    if hero_only {
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind("oteryn:quest-npc/hero_of_rathleton_quest/strange_pipe/greet-access")
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 1, "repeated greeting occurrence writes once");
        return Ok(());
    }
    super::classic_quests::talk(owner, admitted, palimuth, 2601, "hi").await?;
    super::classic_quests::talk(owner, admitted, palimuth, 2602, "mission").await?;
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline"
        )
        .await?,
        3
    );
    for (id, word) in [
        (2603, "hi"),
        (2604, "cook"),
        (2605, "recipe"),
        (2606, "apprentice"),
        (2607, "yes"),
    ] {
        super::classic_quests::talk(owner, admitted, jean, id, word).await?;
    }
    for suffix in ["quest_start", "current_dish", "quest_log"] {
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/{suffix}")
            )
            .await?,
            1
        );
    }
    let greeting = super::classic_quests::talk(owner, admitted, eustacio, 2608, "hi").await?;
    assert!(greeting.iter().any(|line| line.contains("ongoing raids")));
    let name = super::classic_quests::talk(owner, admitted, eustacio, 2609, "name").await?;
    assert_eq!(name, vec!["I am Eustacio. At your service."]);
    super::classic_quests::talk(owner, admitted, xelvar, 2610, "hi").await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    // Xelvar's source sets a track and grants four crystal items. The shared
    // item owner now supports that branch; withhold its admitted generation
    // here to retain the negative storage-only composition case.
    let generation = owner.active_generation.take();
    let refused = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 2611,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: xelvar,
                text: "adventures".into(),
            },
        )
        .await;
    owner.active_generation = generation;
    assert_eq!(refused.disposition, NpcIntentDisposition::Rejected);
    assert!(!refused.unknown);
    assert_eq!(
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let receipts: Vec<(String, i32)> = sqlx::query_as("SELECT quest_key,cardinality(track_keys) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-npc/%' ORDER BY quest_key LIMIT 4")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        receipts,
        vec![
            ("oteryn:quest.hero_of_rathleton_quest".into(), 1),
            ("oteryn:quest.hot_cuisine_quest".into(), 3),
            ("oteryn:quest.in_service_of_yalahar_quest".into(), 1),
        ]
    );
    Ok(())
}
