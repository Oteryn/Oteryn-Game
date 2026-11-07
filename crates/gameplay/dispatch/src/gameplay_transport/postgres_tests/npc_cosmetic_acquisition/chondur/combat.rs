//! Actual1200HP/780XP Stampor kills and source-material TAKE, no reward table override.
use super::*;
use crate::foundation::MovementLocalPosition as Pos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
pub(super) async fn initialize(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut table: oteryn_world_bundle::spawn::Table,
    home: Pos,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native source")?;
    for source in &mut table.sources {
        source.points.retain(|point| {
            i32::from(point.x) == home.x
                && i32::from(point.y) == home.y
                && i16::from(source.floor.unsigned_abs()) == home.floor
                && table
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|c| c.key == "oteryn:creature.stampor")
        });
    }
    table.sources.retain(|source| !source.points.is_empty());
    assert_eq!(
        table.sources.iter().map(|s| s.points.len()).sum::<usize>(),
        1,
        "one original declared point, never a relocated synthetic creature"
    );
    let mut runtime = owner.runtime.lock().await;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let facts = crate::map::boot::table_activation_spawns(&table, |key| {
        let policy = runtime.companion_policy(key).ok()?;
        assert_eq!(policy.maximum_health, 1200);
        // Crystal201 stampor.lua61 isBlockable=false: no nearby-player exemption is invented.
        assert!(!policy.blocked_by_nearby_players);
        Some((1200, policy.blocked_by_nearby_players))
    })
    .ok_or("original source point activation")?;
    runtime
        .begin_activation_spawn_sources(facts)
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        1
    );
    Ok(())
}
pub(super) async fn respawn(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    crate::gameplay_transport::monster_ai_cycle::drive_owner_spawns(
        &mut runtime,
        owner.owner_now().get(),
    );
    Ok(())
}
pub(super) async fn kill(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    home: Pos,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let actor = admitted.runtime_actor.ok_or("actual source player")?;
    let session = admitted.game_session_id;
    let target = {
        let runtime = owner.runtime.lock().await;
        runtime
            .spawn_point_creatures()
            .into_iter()
            .find(|(_, p, _)| *p == home)
            .and_then(|(_, _, a)| a)
            .ok_or("original source actor alive")?
    };
    let target_pose = owner
        .runtime
        .lock()
        .await
        .read_actor_position(target)
        .map_err(debug)?
        .position();
    let mut standing = None;
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let p = Pos {
            x: target_pose.x + dx,
            y: target_pose.y + dy,
            floor: target_pose.floor,
        };
        let from = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        if walk::reachable(owner, from, p) {
            standing = Some(p);
            break;
        }
    }
    walk::walk(owner, admitted, standing.ok_or("real source attack cell")?).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    {
        let runtime = owner.runtime.lock().await;
        let monster = runtime.companion_snapshot(target).map_err(debug)?;
        assert_eq!((monster.health, monster.maximum_health), (1200, 1200));
        let visible = runtime.visible_entities();
        let v = visible
            .creatures
            .iter()
            .find(|c| c.actor == target)
            .ok_or("actual native target")?;
        let reference = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: v.actor.placement_identity(),
            generation: v.generation,
        };
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms.gate_attack_target(),
                actor,
                session,
                owner.owner_now(),
                Some(reference),
                1
            ),
            AttackIntentDisposition::Ok
        );
    }
    for swing in 0..64 {
        // Same clearly identified combat-only arrangement as the Natural7 probes.
        // Restore it BEFORE durable kill/XP settlement; no progression row/history is seeded.
        let original_level = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let s = states
                .get_mut(&runtime, actor, session)
                .ok_or("actual combatant")?;
            let old = s.facts.level;
            s.facts.level = 1_000_000;
            old
        };
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .get_mut(&runtime, actor, session)
                .ok_or("actual combatant after swing")?
                .facts
                .level = original_level;
        }
        if owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session)
        {
            break;
        }
        assert!(
            swing < 63,
            "actual1200HP target dies through real owner attacks"
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(3))
            .ok_or("swing clock")?;
    }
    let outcome = owner.drain_kill_rewards(session).await;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after,
        before + 1,
        "real source native death must settle once: {outcome:?}"
    );
    owner.refresh_level_up_vitals(actor, session).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current earned fence")?;
    admitted.item_fence = Some(fence.item_fence());
    owner.refresh_quest_session(session, false).await;
    Ok(())
}

/// Real Native vocation choice at the historical eligible level; never a build-row seed.
pub(super) async fn choose_class(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("earned-level class actor")?;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual class Native")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("earned-level current class fence")?;
    let formula = native
        .training_formula()
        .ok_or("actual class training formula")?;
    let build = h
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    h.root
        .commit_character_build(
            owner.character,
            &h.node,
            fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(91))
                    .map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                after: crate::durability::character_build::convert_vocation(
                    formula, &build, "knight",
                )
                .map_err(debug)?,
                before: build,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("actual earned Knight facts".into());
    };
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    admitted.game_session_id,
                    facts,
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
    }
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("after class current fence")?
            .item_fence(),
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn take_materials(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::item_view::ItemKey;
    let actor = admitted.runtime_actor.ok_or("real source player")?;
    let corpses = owner.runtime.lock().await.visible_corpses();
    for corpse in corpses {
        if corpse.item.production_key != "oteryn:item.tibia.i12325" {
            continue;
        }
        let contents = owner
            .root
            .read_corpse_contents(owner.character, corpse.item.item_instance_id)
            .await?
            .ok_or("actual Stampor contents")?;
        for entry in contents {
            let natural:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_mint_receipts r USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key IN('oteryn:item.tibia.i12312','oteryn:item.tibia.i12313','oteryn:item.tibia.i12314') AND r.loot_table_production_key='oteryn:loot.creature.stampor')")
    .bind(entry.item.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
            if !natural {
                continue;
            }
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(debug)?
                .ok_or("current TAKE fence")?
                .item_fence();
            let result = owner
                .take_corpse_entry(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: admitted.continuity.next_command_id,
                        item_fence: Some(fence),
                    },
                    ItemKey::Entity(corpse.identity),
                    ItemKey::Instance(entry.item.item_instance_id),
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(
                    result,
                    crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
                ),
                "real natural source material pickup"
            );
            admitted.continuity.next_command_id += 1;
            admitted.item_fence = Some(
                owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(debug)?
                    .ok_or("after TAKE")?
                    .item_fence(),
            );
        }
    }
    Ok(())
}
