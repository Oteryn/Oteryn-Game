//! The real entry-created Aspect must finish its copied phase callback before
//! its ordinary native death can publish the single plain Ground corpse.
use super::*;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_simulation_determinism::SemanticTimeMicros;

const ASPECT: &str = "oteryn:creature.aspect_of_power";
const CORPSE: &str = "oteryn:item.tibia.i33949";

pub(super) async fn actual_entry_corpse(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actual Megalomania Player")?;
    let fence = admitted.item_fence.ok_or("actual Megalomania Item fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual Megalomania native content")?;
    assert_eq!(
        native
            .reward_table()
            .encounter_facts(ASPECT)
            .and_then(|facts| facts.experience),
        Some(0),
        "the actual Aspect source gives no XP"
    );
    let before: i64 = sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let now = owner.owner_now();
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(65006).map_err(debug)?,
    );
    super::super::srx3_rotten_callbacks::publish_native_fixture_player_source(
        owner,
        fence,
        actor,
        command,
        now.get(),
    )
    .await?;
    let (child, position, receipt) = {
        let mut runtime = owner.runtime.lock().await;
        let children: Vec<_> = runtime
            .visible_entities()
            .creatures
            .iter()
            .filter(|entry| runtime.matches_live_creature_identity(entry.actor, ASPECT.as_bytes()))
            .map(|entry| entry.actor)
            .collect();
        assert_eq!(children.len(), 1, "the actual entry creates one Aspect");
        let child = children[0];
        let current = runtime.companion_snapshot(child).map_err(debug)?;
        let definition = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Creature,
            key: current.state.policy.definition_key.clone(),
            revision: current.state.policy.definition_revision.clone(),
        };
        let callbacks = native
            .native_death_events(&definition)
            .ok_or("actual Aspect copied death events")?;
        assert_eq!(
            callbacks
                .events()
                .iter()
                .map(|event| event.name())
                .collect::<Vec<_>>(),
            ["SoulWarAspectOfPowerDeath"]
        );
        assert!(
            current.state.master.is_none() && runtime.native_summon_master(child).is_none(),
            "the real unmastered child must execute its full copied phase callback"
        );
        let position = runtime
            .read_actor_position(child)
            .map_err(debug)?
            .position();
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command,
            occurrence: SpellOccurrenceBinding {
                id: "srx3-megalomania-initial-aspect".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srx3-megalomania-initial-aspect".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: child,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(child).map_err(debug)?,
                    magnitude: current.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(
            receipt
                .effects
                .iter()
                .any(|effect| effect.health.is_some_and(|health| health.health_after == 0)),
            "the actual entry-created Aspect has a canonical lethal receipt"
        );
        let notice = runtime
            .next_source_creature_death()
            .map_err(debug)?
            .ok_or("actual Aspect lethal must retain its copied callback")?;
        assert_eq!(notice.actor(), child);
        assert_eq!(notice.enqueued_at_us(), Some(now.get()));
        let attack = owner.states.attack.lock().await;
        crate::gameplay_transport::kill_reward::record_batch_kills(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        (child, position, receipt)
    };
    drain_copied_death(owner, child, now).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(after, before, "actual Aspect death grants zero XP");

    let ground = owner
        .root
        .read_item_loop_ground_for_scope(fence.runtime_scope)
        .await?;
    let bodies: Vec<_> = ground
        .iter()
        .filter(|item| item.item.definition.production_key == CORPSE)
        .collect();
    assert_eq!(bodies.len(), 1, "one physical source33949 Ground body");
    let body = bodies[0];
    assert!(body.source_plain_corpse && body.field_collision.is_none());
    assert!(
        owner.runtime.lock().await.visible_corpses().is_empty(),
        "a plain Aspect body is not an openable corpse container"
    );
    let item = body.item.item_instance_id;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT game_item_plain_corpse_source(encode($1,'hex')::uuid)"
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?,
        "the exact physical Ground item retains its native plain-corpse provenance"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_item_mint_receipts m USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.lifecycle=1 AND i.quantity=1 AND m.loot_purpose_key='CORPSE_MATERIALIZATION'",
    ).bind(item.as_slice()).bind(CORPSE).fetch_one(&h.pool).await?, 1);
    let map = owner.bundle_map().ok_or("actual Megalomania Bundle map")?;
    let overlay = crate::gameplay_transport::loop_move::loop_ground(
        owner,
        map.overlay.base(),
        native.item_index().map(|index| index.as_ref()),
    )
    .await
    .ok_or("actual Aspect corpse Ground overlay")?;
    let (at, _) = overlay
        .ground_entry(&item)
        .ok_or("the same physical Aspect body must appear on its native map tile")?;
    assert_eq!(
        at,
        crate::map::overlay::TilePos {
            x: u16::try_from(position.x)?,
            y: u16::try_from(position.y)?,
            floor: i8::try_from(position.floor)?
                .checked_neg()
                .ok_or("native Aspect corpse floor")?,
        }
    );
    let receipts: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT event_id::text,transaction_id::text,item_instance_id::text FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' ORDER BY event_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(receipts.len(), 1, "one canonical corpse MINT receipt");

    // Replay the original committed combat receipt, never a new lethal result.
    {
        let mut runtime = owner.runtime.lock().await;
        let attack = owner.states.attack.lock().await;
        crate::gameplay_transport::kill_reward::record_batch_kills(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
    }
    drain_copied_death(owner, child, now).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    assert_eq!(sqlx::query_as::<_, (String, String, String)>(
        "SELECT event_id::text,transaction_id::text,item_instance_id::text FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' ORDER BY event_id",
    ).fetch_all(&h.pool).await?, receipts, "same death cannot mint a second body");
    let after = owner
        .root
        .read_item_loop_ground_for_scope(fence.runtime_scope)
        .await?;
    let ids: Vec<_> = after
        .iter()
        .filter(|body| body.item.definition.production_key == CORPSE)
        .map(|body| body.item.item_instance_id)
        .collect();
    assert_eq!(
        ids,
        vec![item],
        "replay retains the original Ground custody"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, before,
        "replay preserves the zero-XP outcome");
    Ok(())
}

async fn drain_copied_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    child: crate::foundation::ExactActorRef,
    now: SemanticTimeMicros,
) -> TestResult {
    // Real entry may retain an older native pass. Drain that bounded prefix at
    // the same death clock, without firing the new five-second Aspect respawn.
    const MAX_TURNS: usize = 256;
    for turn in 0..=MAX_TURNS {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let head = runtime.next_source_creature_death().map_err(debug)?;
        if let Some(head) = &head {
            assert_eq!(head.actor(), child, "only the actual initial Aspect died");
        }
        if head.is_none() && states.source_death_corpse.is_none() {
            assert!(states.source_death_action.is_none());
            return Ok(());
        }
        if turn == MAX_TURNS {
            break;
        }
        drop(states);
        drop(runtime);
        owner.drain_monster_melee_at(now).await;
    }
    Err("actual Aspect copied death did not complete within 256 owner turns".into())
}
