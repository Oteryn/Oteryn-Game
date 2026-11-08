//! Canonical completion joins the existing fenced NPC hand-in transaction.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{
    QuestTransitionOutcome,
    npc_program::{NpcProgramCommit, NpcProgramRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
    world_callbacks::{Offering, Position},
};
use std::sync::Arc;

#[test]
fn engnpc_m04_completion_is_atomic_canonical_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engnpc-m04", || {
        super::quest_world_callbacks::scenario(
            "engnpc-m04",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!("ENGNPC_M04_PG_EXECUTED duration={:?}", started.elapsed());
    Ok(())
}

fn fixture(
    mode: &str,
) -> TestResult<(
    NpcRuleCatalog,
    Arc<QuestStateCatalogue>,
    String,
    String,
    String,
)> {
    let quest = format!("oteryn:quest.engnpc_m04_{mode}");
    let track = format!("oteryn:quest-progress/engnpc/m04/{mode}/stage");
    let npc = format!("oteryn:npc.engnpc_m04_{mode}");
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13";
    let mut document = serde_json::json!({"schema":1,"quest":quest,"tracks":[{
        "key":track,"quest":quest,"initial":0,"min":0,"max":2,"sources":[source]}],"rules":[{
        "key":format!("oteryn:quest-npc/m04/{mode}/start"),"quest":quest,"npc":npc,"trigger":"say",
        "keywords":["start"],"topics":[0],"guards":[{"kind":"storage","track":track,"op":"eq","value":0}],
        "effects":[{"kind":"set","track":track,"value":1}],"reply":["Bring the token."],"sources":[source]
    },{
        "key":format!("oteryn:quest-npc/m04/{mode}/finish"),"quest":quest,"npc":npc,"trigger":"say",
        "keywords":["yes"],"topics":[1],"guards":[{"kind":"storage","track":track,"op":"eq","value":1},
        {"kind":"item","item":"oteryn:item.tibia.i3031","count":1,"op":"ge"}],
        "effects":[{"kind":"take_item","item":"oteryn:item.tibia.i3031","count":1},
        {"kind":"give_item","item":"oteryn:item.tibia.i3035","count":1},
        {"kind":"set","track":track,"value":if mode=="zero" {1}else{2}}],
        "completes":true,"reply":["Complete."],"sources":[source]
    }]});
    if mode == "zero" {
        document["rules"][1]["effects"]
            .as_array_mut()
            .ok_or("fixture effects")?
            .pop();
    }
    let source = NpcRuleCatalog::from_json_documents(&[&document.to_string()]).map_err(debug)?;
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
            max: 2,
        }],
        vec![QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: quest.clone(),
            effects: vec![QuestEffect {
                track: track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?;
    Ok((source, Arc::new(catalogue), quest, track, npc))
}

async fn invoke(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: NpcProgramRequest,
    catalogue: Arc<QuestStateCatalogue>,
    source: &NpcRuleCatalog,
) -> Result<NpcProgramCommit, CharacterProgressionError> {
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or(CharacterProgressionError::AuthorityRejected)?;
    let _slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or(CharacterProgressionError::InvalidInput)?;
    owner
        .root
        .commit_fixture_npc_program(
            authority,
            owner.holder,
            &permit,
            fence,
            request,
            catalogue,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            source,
        )
        .await
}

type Snapshot = (i64, Option<i64>, i64, i64, i64, i64, i64, i64);
async fn snapshot(
    h: &Harness,
    character: &[u8; 16],
    quest: &str,
    track: &str,
) -> TestResult<Snapshot> {
    Ok(sqlx::query_as("SELECT (SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT completed_character_revision::bigint FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2),coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$3),0),(SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3031'),(SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3035'),(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2),(SELECT count(*) FROM game_reward_claim_mint_receipts WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_audit_outbox)")
        .bind(character.as_slice()).bind(quest).bind(track).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    for (index, mode) in ["normal", "zero"].into_iter().enumerate() {
        let (source, catalogue, quest, track, npc) = fixture(mode)?;
        let request = |offset, finish| -> TestResult<NpcProgramRequest> {
            Ok(NpcProgramRequest {
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9400 + index as u64 * 10 + offset).map_err(debug)?,
                ),
                npc: npc.clone(),
                trigger: NpcRuleTrigger::Say,
                message: if finish { "yes" } else { "start" }.into(),
                topic: if finish { 1 } else { 0 },
                transition_key: format!(
                    "oteryn:quest-npc/m04/{mode}/{}",
                    if finish { "finish" } else { "start" }
                ),
            })
        };
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let initial = invoke(
            owner,
            authority,
            fence,
            request(0, false)?,
            catalogue.clone(),
            &source,
        )
        .await
        .map_err(debug)?;
        assert!(
            matches!(initial.transition,QuestTransitionOutcome::Committed(ref r) if !r.completes)
        );
        let character = *fence.character_id.as_bytes();
        assert_eq!(snapshot(h, &character, &quest, &track).await?.1, None);
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "item fence")?
            .ok_or("item fence")?;
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        let token = Offering {
            position: Position {
                x: u16::try_from(at.x)?,
                y: u16::try_from(at.y)?,
                floor: -i8::try_from(at.floor)?,
            },
            item: "oteryn:item.tibia.i3031".into(),
            count: 2,
        };
        super::quest_world_callbacks::input_item(
            owner,
            current.item_fence(),
            &token,
            700 + index as u64,
            true,
        )
        .await?;
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "ready fence")?
            .ok_or("ready fence")?;
        let before = snapshot(h, &character, &quest, &track).await?;
        let final_request = request(1, true)?;
        let mut wrong_topic = final_request.clone();
        wrong_topic.topic = 0;
        assert!(matches!(
            invoke(
                owner,
                authority,
                current,
                wrong_topic,
                catalogue.clone(),
                &source
            )
            .await
            .map_err(debug)?
            .transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(h, &character, &quest, &track).await?, before);
        if mode == "zero" {
            sqlx::raw_sql("CREATE FUNCTION engnpc_m04_corrupt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.completes := false; RETURN NEW; END $$; CREATE TRIGGER engnpc_m04_corrupt BEFORE INSERT ON game_quest_npc_program_receipts FOR EACH ROW EXECUTE FUNCTION engnpc_m04_corrupt()").execute(&h.pool).await?;
            assert!(matches!(
                invoke(
                    owner,
                    authority,
                    current,
                    final_request.clone(),
                    catalogue.clone(),
                    &source
                )
                .await,
                Err(CharacterProgressionError::Unavailable(
                    crate::durability::DurabilityError::Database(
                        crate::durability::SqlxFailureClass::Database(None)
                    )
                ))
            ));
            assert_eq!(snapshot(h, &character, &quest, &track).await?, before);
            sqlx::raw_sql("DROP TRIGGER engnpc_m04_corrupt ON game_quest_npc_program_receipts; DROP FUNCTION engnpc_m04_corrupt()").execute(&h.pool).await?;
        }
        let result = invoke(
            owner,
            authority,
            current,
            final_request.clone(),
            catalogue.clone(),
            &source,
        )
        .await
        .map_err(debug)?;
        let QuestTransitionOutcome::Committed(receipt) = result.transition else {
            return Err(format!("terminal receipt: {result:?}").into());
        };
        assert!(
            receipt.completes,
            "final NPC hand-in must complete the canonical quest"
        );
        let after = snapshot(h, &character, &quest, &track).await?;
        assert_eq!(
            (
                after.0, after.1, after.2, after.3, after.4, after.5, after.6
            ),
            (
                before.0 + 1,
                Some(before.0 + 1),
                if mode == "zero" { 1 } else { 2 },
                before.3 - 1,
                before.4 + 1,
                before.5 + 1,
                before.6 + 1
            )
        );
        assert!(after.7 > before.7);
        if mode == "zero" {
            assert!(receipt.changes.is_empty());
        }
        let replay = invoke(
            owner,
            authority,
            current,
            final_request.clone(),
            catalogue.clone(),
            &source,
        )
        .await
        .map_err(debug)?;
        assert!(
            matches!(replay.transition,QuestTransitionOutcome::AlreadyCommitted(ref r) if r.completes)
        );
        assert_eq!(snapshot(h, &character, &quest, &track).await?, after);
        let fresh = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fresh fence")?
            .ok_or("fresh fence")?;
        let mut repeated = final_request;
        repeated.command = CommandRef::new(
            admitted.game_session_id,
            CommandId::new(9402 + index as u64 * 10).map_err(debug)?,
        );
        assert!(matches!(
            invoke(owner, authority, fresh, repeated, catalogue, &source)
                .await
                .map_err(debug)?
                .transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(h, &character, &quest, &track).await?, after);
    }
    Ok(())
}
