//! Canonical NPC completion on the real authority owner, without a World boot fixture.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::sync::Arc;

struct Content;
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(&self, _: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        None
    }
}

#[test]
fn m04_npc_completion_is_canonical_atomic_fenced_and_replay_bound() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engnpc_m04_completion", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGNPC_M04_AUTHORITY_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M04 authority PG thread failed")?
        .map_err(Into::into)
}

fn programme(
    zero: bool,
) -> fixture::TestResult<(NpcRuleCatalog, Arc<QuestStateCatalogue>, String, String)> {
    let quest = format!(
        "oteryn:quest.m04_authority_{}",
        if zero { "zero" } else { "normal" }
    );
    let track = format!(
        "oteryn:quest-progress/m04_authority/{}/stage",
        if zero { "zero" } else { "normal" }
    );
    let rule = format!(
        "oteryn:quest-npc/m04_authority/{}/finish",
        if zero { "zero" } else { "normal" }
    );
    let reference = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13";
    let effects = if zero {
        serde_json::json!([])
    } else {
        serde_json::json!([
            {"kind":"set","track":track,"value":1}, {"kind":"achievement","name":"Bastard"}
        ])
    };
    let text = serde_json::json!({"schema":1,"quest":quest,
        "tracks":[{"key":track,"quest":quest,"initial":0,"min":0,"max":1,"sources":[reference]}],
        "rules":[{"key":rule,"quest":quest,"npc":"oteryn:npc.m04_fixture","trigger":"say",
            "keywords":["yes"],"topics":[2],"guards":[{"kind":"storage","track":track,"op":"eq","value":0}],
            "effects":effects,"reply":["Complete."],"completes":true,"sources":[reference]}]}).to_string();
    let source = NpcRuleCatalog::from_json_documents(&[&text]).map_err(fixture::debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: track.clone(),
            quest: quest.clone(),
            initial: 0,
            min: 0,
            max: 1,
        }],
        vec![QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: quest.clone(),
            effects: vec![QuestEffect {
                track,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(fixture::debug)?;
    Ok((source, Arc::new(catalogue), quest, rule))
}

type Snapshot = (String, i64, i64, i64, i64, i64, i64, Option<String>);
async fn snapshot(h: &fixture::Harness, quest: &str) -> fixture::TestResult<Snapshot> {
    Ok(sqlx::query_as("SELECT (SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_npc_program_receipts),(SELECT count(*) FROM game_character_quest_states),(SELECT count(*) FROM game_character_quest_tracks),(SELECT count(*) FROM game_account_achievement_grant_requests),(SELECT count(*) FROM game_account_achievements),(SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2)")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(quest).fetch_one(&h.pool).await?)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice())
        .bind([1_u8;32].as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fixture::fence(1)?.runtime_scope)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    for (index, zero) in [false, true].into_iter().enumerate() {
        let (source, catalogue, quest, rule) = programme(zero)?;
        let revision: u64 = h.root_revision().await?.parse()?;
        let fence = fixture::fence(revision)?;
        let request = |id| -> fixture::TestResult<NpcProgramRequest> {
            Ok(NpcProgramRequest {
                command: CommandRef::new(session, CommandId::new(id).map_err(fixture::debug)?),
                npc: "oteryn:npc.m04_fixture".into(),
                trigger: NpcRuleTrigger::Say,
                message: "yes".into(),
                topic: 2,
                transition_key: rule.clone(),
            })
        };
        let invoke = |fence, request| {
            h.root.commit_fixture_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                request,
                catalogue.clone(),
                &Content,
                &source,
            )
        };
        let before = snapshot(h, &quest).await?;
        let command = 9840 + index as u64 * 10;
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(invoke(stale, request(command)?).await.is_err());
        assert_eq!(snapshot(h, &quest).await?, before);
        let mut wrong_topic = request(command + 1)?;
        wrong_topic.topic = 1;
        assert!(matches!(
            invoke(fence, wrong_topic)
                .await
                .map_err(fixture::debug)?
                .transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(h, &quest).await?, before);
        // Fault injection must run before the existing BEFORE receipt guard.
        sqlx::raw_sql("CREATE FUNCTION engnpc_m04_corrupt_response() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN NEW.completes:=false;RETURN NEW;END$$;CREATE TRIGGER engnpc_m04_corrupt_response BEFORE INSERT ON game_quest_npc_program_receipts FOR EACH ROW EXECUTE FUNCTION engnpc_m04_corrupt_response()")
            .execute(&h.pool).await?;
        let rejected = invoke(fence, request(command + 2)?).await;
        assert!(matches!(rejected, Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(crate::durability::DurabilityError::Database(crate::durability::SqlxFailureClass::Database(None))))), "{rejected:?}");
        assert_eq!(snapshot(h, &quest).await?, before);
        sqlx::raw_sql("DROP TRIGGER engnpc_m04_corrupt_response ON game_quest_npc_program_receipts;DROP FUNCTION engnpc_m04_corrupt_response()")
            .execute(&h.pool).await?;
        if !zero {
            sqlx::raw_sql("CREATE FUNCTION m04_reject_achievement() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'M04 forced late grant failure';END$$;CREATE TRIGGER m04_reject_achievement BEFORE INSERT ON game_account_achievement_grant_requests FOR EACH ROW EXECUTE FUNCTION m04_reject_achievement()")
                .execute(&h.pool).await?;
            assert!(invoke(fence, request(command + 2)?).await.is_err());
            assert_eq!(snapshot(h, &quest).await?, before);
            sqlx::raw_sql("DROP TRIGGER m04_reject_achievement ON game_account_achievement_grant_requests;DROP FUNCTION m04_reject_achievement()")
                .execute(&h.pool).await?;
        }
        let committed = invoke(fence, request(command + 2)?)
            .await
            .map_err(fixture::debug)?;
        let QuestTransitionOutcome::Committed(receipt) = committed.transition else {
            return Err(format!("{committed:?}").into());
        };
        assert!(receipt.completes);
        assert_eq!(receipt.changes.len(), usize::from(!zero));
        let after = snapshot(h, &quest).await?;
        assert_eq!(
            after,
            (
                (revision + 1).to_string(),
                before.1 + 1,
                before.2 + 1,
                before.3 + 1,
                before.4 + i64::from(!zero),
                before.5 + i64::from(!zero),
                before.6 + i64::from(!zero),
                Some((revision + 1).to_string())
            )
        );
        let same: bool = sqlx::query_scalar("SELECT s.xmin=q.xmin AND q.xmin=n.xmin FROM game_character_quest_states s JOIN game_character_quest_receipts q USING(character_id,quest_key) JOIN game_quest_npc_program_receipts n ON n.character_id=q.character_id AND n.game_session_id=q.cause_id::uuid AND n.command_id=q.cause_ordinal WHERE s.quest_key=$1")
            .bind(&quest).fetch_one(&h.pool).await?;
        assert!(same);
        let replay = invoke(fence, request(command + 2)?)
            .await
            .map_err(fixture::debug)?;
        assert!(
            matches!(replay.transition, QuestTransitionOutcome::AlreadyCommitted(ref r) if r.completes)
        );
        assert_eq!(snapshot(h, &quest).await?, after);
        let mut changed = request(command + 2)?;
        changed.message.push_str(" please");
        assert!(invoke(fence, changed).await.is_err());
        assert_eq!(snapshot(h, &quest).await?, after);
        assert!(matches!(
            invoke(fixture::fence(revision + 1)?, request(command + 3)?)
                .await
                .map_err(fixture::debug)?
                .transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(h, &quest).await?, after);
    }
    // This is a final-boundary fixture: a fenced fixture transition establishes
    // prior mission progress; the actual shipped source programme owns completion.
    for (offset, npc, quest, previous_mission) in [
        (0, "malor", "the_djinn_war_efreet_faction", 3),
        (10, "gabel", "the_djinn_war_marid_faction", 2),
    ] {
        let quest = format!("oteryn:quest.{quest}");
        let source = NpcRuleCatalog::runtime().map_err(fixture::debug)?;
        let program = source.program(&quest).ok_or("missing shipped finale")?;
        let digest = program
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let tracks = program
            .document
            .tracks
            .iter()
            .map(|t| QuestTrack {
                key: t.key.clone(),
                quest: quest.clone(),
                initial: t.initial,
                min: t.min,
                max: t.max,
            })
            .collect::<Vec<_>>();
        let mission2 = tracks
            .iter()
            .find(|t| t.key.ends_with("/mission02"))
            .ok_or("mission02")?
            .key
            .clone();
        let mission3 = tracks
            .iter()
            .find(|t| t.key.ends_with("/mission03"))
            .ok_or("mission03")?
            .key
            .clone();
        let seed = format!("oteryn:quest-transition/engnpc-final-boundary/{npc}");
        let catalogue = Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                tracks.clone(),
                vec![
                    QuestTransition {
                        key: format!("oteryn:quest-npc-pin/{digest}"),
                        quest: quest.clone(),
                        effects: vec![QuestEffect {
                            track: mission2.clone(),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Computed,
                        }],
                        completes: false,
                        experience: None,
                        world_change: None,
                    },
                    QuestTransition {
                        key: seed.clone(),
                        quest: quest.clone(),
                        effects: vec![
                            QuestEffect {
                                track: mission2,
                                from: QuestComparison::Eq(-1),
                                effect: QuestEffectKind::Set(previous_mission),
                            },
                            QuestEffect {
                                track: mission3.clone(),
                                from: QuestComparison::Eq(-1),
                                effect: QuestEffectKind::Set(2),
                            },
                        ],
                        completes: false,
                        experience: None,
                        world_change: None,
                    },
                ],
            )
            .map_err(fixture::debug)?,
        );
        let revision: u64 = h.root_revision().await?.parse()?;
        let seeded = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(revision)?,
                crate::durability::quest_state::QuestTransitionRequest {
                    transition_key: seed,
                    cause: crate::durability::quest_state::QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(9900 + offset).map_err(fixture::debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(fixture::debug)?;
        assert!(matches!(seeded,QuestTransitionOutcome::Committed(ref r) if !r.completes));
        let finish = program
            .document
            .rules
            .iter()
            .find(|r| r.completes)
            .ok_or("final rule")?;
        let request = NpcProgramRequest {
            command: CommandRef::new(
                session,
                CommandId::new(9901 + offset).map_err(fixture::debug)?,
            ),
            npc: format!("oteryn:npc.{npc}"),
            trigger: NpcRuleTrigger::Say,
            message: "yes".into(),
            topic: 2,
            transition_key: finish.key.clone(),
        };
        let fence = fixture::fence(revision + 1)?;
        let before = snapshot(h, &quest).await?;
        let mut refused = request.clone();
        refused.topic = 1;
        assert!(matches!(
            h.root
                .commit_character_npc_program(
                    &authority,
                    &h.node,
                    &permit,
                    fence,
                    refused,
                    catalogue.clone(),
                    &Content
                )
                .await
                .map_err(fixture::debug)?
                .transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(h, &quest).await?, before);
        let result = h
            .root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                request.clone(),
                catalogue.clone(),
                &Content,
            )
            .await
            .map_err(fixture::debug)?;
        assert!(
            matches!(result.transition,QuestTransitionOutcome::Committed(ref r) if r.completes && r.changes.len()==2)
        );
        let after = snapshot(h, &quest).await?;
        assert_eq!(after.0, (revision + 2).to_string());
        assert_eq!(after.7, Some((revision + 2).to_string()));
        assert_eq!(after.5, before.5 + 1);
        assert_eq!(after.6, before.6 + 1);
        let replay = h
            .root
            .commit_character_npc_program(
                &authority, &h.node, &permit, fence, request, catalogue, &Content,
            )
            .await
            .map_err(fixture::debug)?;
        assert!(
            matches!(replay.transition,QuestTransitionOutcome::AlreadyCommitted(ref r) if r.completes)
        );
        assert_eq!(snapshot(h, &quest).await?, after);
    }
    Ok(())
}
