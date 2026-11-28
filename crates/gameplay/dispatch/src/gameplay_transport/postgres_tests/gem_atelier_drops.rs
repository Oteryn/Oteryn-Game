use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::boss_encounter::reward_chest::scoped_death_id;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    WorldId,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[test]
fn gem_atelier_fiendish_corpse_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("gem-atelier-fiendish", move || run(admin, false))
}

#[test]
fn gem_atelier_archfoe_personal_chest_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("gem-atelier-archfoe", move || run(admin, true))
}

fn run(admin: String, archfoe: bool) -> TestResult {
    crate::bestiary_postgres_harness::runtime()?.block_on(async move {
        let tag = if archfoe {
            "gem_archfoe"
        } else {
            "gem_fiendish"
        };
        let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
        let result = scenario(&h, archfoe).await;
        let cleanup = h.cleanup().await;
        result.and(cleanup)
    })
}

async fn scenario(h: &Harness, archfoe: bool) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let frame_digest = pin.frame_binding().digest();
    let deps = Dependencies::with_content(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    deps.initialize(&owner).await;
    owner.movement_cells = deps.room.movement_cells().into();
    let session = deps.admitted.game_session_id;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?
        .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
    let attacker = CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice())
        .bind(deps.admitted.channel_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice())
        .bind(deps.admitted.channel_id.as_bytes().as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(frame_digest.as_slice())
        .execute(&h.pool)
        .await?;

    let gems: Vec<_> = [44_602..=44_613, 49_371..=49_373]
        .into_iter()
        .flatten()
        .map(|id| format!("oteryn:item.tibia.i{id}"))
        .collect();
    let rewards = native.reward_table();
    let creature = if archfoe {
        native
            .policies()
            .policies()
            .iter()
            .find(|policy| {
                policy.reward_boss == Some(true)
                    && rewards.row(&policy.definition_key).is_ok_and(|row| {
                        row.boss
                            .as_ref()
                            .is_some_and(|boss| boss.category == "archfoe")
                            && row.corpse_item.is_some()
                            && !row
                                .loot_table
                                .entries
                                .iter()
                                .any(|entry| gems.contains(&entry.item.production_key))
                    })
            })
            .ok_or("native co-op Archfoe without authored gems")?
            .definition_key
            .as_str()
    } else {
        "oteryn:creature.rat"
    };
    {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(attacker, fence.character_lease_generation).map_err(debug)?,
            )
            .map_err(debug)?;
    }
    let committed_gems = if archfoe {
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_boss_reward_items
         WHERE definition_production_key = ANY($1)"
    } else {
        "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_corpse_container_entries e
           JOIN game_item_instances i USING (item_instance_id)
           JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
            AND m.transaction_id=e.placed_transaction_id
            AND m.destination_parent_item_instance_id=e.parent_item_instance_id
         WHERE i.definition_production_key = ANY($1)"
    };
    let (batch, death) = {
        let mut runtime = owner.runtime.lock().await;
        let pin = runtime.content_pin().clone();
        let mut selected = None;
        // Select a winning first Regular (Archfoe) or Greater (Fiendish) draw.
        // This uses real actor identities and can also be run on the base code.
        for source in 100..356 {
            let spawn = runtime
                .realize_native_qualification_spawn_for_source(
                    source,
                    pin.clone(),
                    creature,
                    MovementLocalPosition {
                        x: 3,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            let death = death_id(spawn.actor);
            if winning_draw(pin.server_artifact_digest(), attacker, death, archfoe)? {
                selected = Some((spawn.actor, death));
                break;
            }
            runtime.remove_test_actor(spawn.actor).map_err(debug)?;
        }
        let (actor, death) =
            selected.ok_or("content-pinned fixture must have a winning gem draw")?;
        if !archfoe {
            runtime
                .drive_spawn_respawns(10_000_000, |_, _| true)
                .map_err(debug)?;
        }
        let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
        if !archfoe {
            assert_eq!(
                snapshot.forge,
                Some(crate::domain::forge_creature::ForgeCreature::Fiendish)
            );
        }
        let now = owner.owner_now();
        let batch = OwnerCombatBatch {
            caster: deps.actor,
            attacker,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(session, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "gem-atelier-drop".into(),
                revisions: ["r1"; 5].map(str::to_owned),
            },
            binding: b"gem-atelier-drop".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target: actor,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(actor).map_err(debug)?,
                    magnitude: snapshot.maximum_health,
                },
            }],
            deferred: None,
        };
        (batch, death)
    };
    let mut first = None;
    for replay in [false, true] {
        {
            let mut runtime = owner.runtime.lock().await;
            let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
            let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
            assert_eq!(receipt.applied, !replay);
            let attack = owner.states.attack.try_lock().map_err(debug)?;
            record_batch_kills(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: attack.kills(),
                    rewards,
                    charms: None,
                },
                &receipt,
                batch.now_ms,
            );
        }
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        let total = sqlx::query_scalar::<_, i64>(committed_gems)
            .bind(&gems)
            .fetch_one(&h.pool)
            .await?;
        assert!(
            total > 0,
            "eligible death with a winning draw must settle a dynamic Gem Atelier drop"
        );
        if let Some(before) = first {
            assert_eq!(
                total, before,
                "lethal replay must keep the same durable gem quantity"
            );
        }
        first = Some(total);
    }
    if archfoe {
        let page = h
            .root
            .read_boss_reward_items(&authority, &h.node, fence, death, 0, native.source_digest())
            .await
            .map_err(debug)?
            .ok_or("current personal chest page")?;
        assert!(
            page.items
                .iter()
                .any(|item| gems.contains(&item.definition.production_key))
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_corpse_container_entries e
                   JOIN game_item_instances i USING (item_instance_id)
                 WHERE i.definition_production_key = ANY($1)"
            )
            .bind(&gems)
            .fetch_one(&h.pool)
            .await?,
            0,
            "co-op gems exist only in the personal reward chest"
        );
    } else {
        let corpse = owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .into_iter()
            .next()
            .ok_or("Fiendish corpse projection")?;
        let contents = h
            .root
            .read_corpse_contents(&authority, corpse.item.item_instance_id)
            .await
            .map_err(debug)?
            .ok_or("durable corpse contents")?;
        assert!(
            contents
                .iter()
                .any(|entry| gems.contains(&entry.item.definition.production_key))
        );
    }
    eprintln!("GEM_ATELIER_DROP_POSTGRES_ASSERTIONS_COMPLETED archfoe={archfoe}");
    Ok(())
}

fn death_id(actor: ExactActorRef) -> [u8; 16] {
    scoped_death_id(
        actor.runtime_scope(),
        actor.scope_generation().get(),
        actor.actor_local_id(),
        actor.actor_local_generation(),
    )
}

fn winning_draw(
    digest: [u8; 32],
    attacker: CharacterId,
    death: [u8; 16],
    archfoe: bool,
) -> TestResult<bool> {
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(digest)
            .chain_update(attacker.as_bytes())
            .finalize()
            .into(),
    );
    let roll = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(death),
        "GEM_ATELIER_DROP_CHANCE",
        if archfoe { 2 } else { 4 },
    )
    .map_err(debug)?;
    Ok(crate::domain::uniform_draw::uniform_draw(roll, 1, 100_000) <= 9_000)
}
