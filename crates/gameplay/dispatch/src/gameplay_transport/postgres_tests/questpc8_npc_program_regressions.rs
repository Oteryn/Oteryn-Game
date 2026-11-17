//! Production NPC command, atomic acceptance, bound replay and session fence qualification.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    npc_rules::NpcRuleTrigger,
    world_callbacks::Position,
};

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
        "catalogue-npc-legacy"
    } else {
        "catalogue-npc-father"
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
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-scoped", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-scoped",
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
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-hero", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-hero",
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
    // The composed plain-item owner now admits Xelvar's complete source branch.
    // Both the four crystals and its single track must commit exactly once.
    let intent = || NpcTalkIntent {
        npc_actor: xelvar,
        text: "adventures".into(),
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 2611,
        item_fence: Some(fence.item_fence()),
    };
    let outcome = owner
        .npc_talk(admitted.runtime_actor.ok_or("actor")?, command, intent())
        .await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.unknown);
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let mut expected = before.tracks().clone();
    expected.insert(
        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line".into(),
        1,
    );
    assert_eq!(after.tracks(), &expected);
    let replay = owner
        .npc_talk(admitted.runtime_actor.ok_or("actor")?, command, intent())
        .await;
    assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
    assert!(!replay.unknown);
    assert_eq!(
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?,
        after
    );
    let crystals: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i16167'")
        .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        crystals, 4,
        "the complete grant is replayed without minting twice"
    );
    let receipts: Vec<(String, i32)> = sqlx::query_as("SELECT quest_key,cardinality(track_keys) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-npc/%' ORDER BY quest_key LIMIT 5")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        receipts,
        vec![
            ("oteryn:quest.bigfoot_s_burden_quest".into(), 1),
            ("oteryn:quest.hero_of_rathleton_quest".into(), 1),
            ("oteryn:quest.hot_cuisine_quest".into(), 3),
            ("oteryn:quest.in_service_of_yalahar_quest".into(), 1),
        ]
    );
    Ok(())
}
