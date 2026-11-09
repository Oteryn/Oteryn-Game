//! M30a: the framed ordinary-attack caller must deliver its own native hit credit.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use std::{sync::Arc, time::Duration};

const PHASE: &str = "oteryn:quest-progress/engenc/child/phase";
const SPEAR: &str = "oteryn:quest-progress/engenc/child/spear";

#[test]
fn engenc_postgres_framed_ordinary_spear_hit_credits_child_training() -> TestResult {
    party_xp::run(Case::EngencHits)
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    assert!(owner.proficiency_catalogue.is_none());
    let mut bindings = Bindings::parse(include_bytes!(
        "../../../../../../content/quests/authored-bindings/engenc-child-of-destiny.json"
    ))?;
    bindings.verify_embedded_recipe_evidence()?;
    // Start this isolated caller qualification at the authored spear lesson.
    bindings.quests[0]
        .tracks
        .iter_mut()
        .find(|track| track.key == PHASE)
        .ok_or("training phase")?
        .initial = 2;
    owner.quest_catalogue = Some(Arc::new(
        bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?,
    ));
    owner.refresh_quest_session(session, true).await;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("active native content")?;
    assert!(
        native
            .item_policy("oteryn:item.tibia.i17110", "definition-r1")
            .ok_or("admitted training spear")?
            .record()
            .authoring
            .ranged
            .is_some()
    );
    {
        let sessions = owner.states.quest_sessions.lock().map_err(debug)?;
        let copy = sessions
            .get(&session)
            .and_then(|entry| entry.copy.as_ref())
            .ok_or("current training copy")?;
        let catalogue = owner
            .quest_catalogue
            .as_deref()
            .ok_or("training catalogue")?;
        assert!(
            oteryn_gameplay_quests::quest::authored_bindings::hits::objectives(catalogue)
                .any(|(_, objective)| objective.key.ends_with("engenc-child-s3")
                    && objective.active(catalogue, copy.tracks()))
        );
    }
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?, 0);
    // Provision custody only; source selection and damage use ordinary combat owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i17110','definition-r1',10,1,encode($1,'hex')::uuid)")
        .bind(id(116).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(id(42).as_slice()).bind(id(116).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let target = {
        let runtime = owner.runtime.lock().await;
        let row = *runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("training skeleton")?;
        assert_eq!(
            runtime
                .companion_snapshot(row.actor)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            "oteryn:creature.party_skeleton"
        );
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: row.actor.placement_identity(),
            generation: row.generation,
        }
    };
    super::equipment_mods::framed_target(owner, actor, session, target).await?;
    let mut credited = false;
    for _ in 0..64 {
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        credited = sqlx::query_scalar::<_, i64>(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)"
        ).bind(character.as_bytes().as_slice()).bind(SPEAR).fetch_one(&h.pool).await? == 1;
        if credited {
            break;
        }
        owner.clock_origin -= Duration::from_secs(3);
    }
    assert!(
        credited,
        "ordinary spear damage did not reach native quest settlement"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2"
    ).bind(character.as_bytes().as_slice()).bind(PHASE).fetch_one(&h.pool).await?, 3);
    owner.drain_auto_attacks().await;
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='native_hit'"
    ).bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?, 1);
    eprintln!("M30A_ORDINARY_SPEAR_POSTGRES_EXECUTED");
    Ok(())
}
