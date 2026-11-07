//! Three actual deaths must settle A1, B1, A2 even when B's timer drains first.
use super::*;
use std::time::Duration;

async fn hit(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: (ExactActorRef, GameSessionId),
    target: ExactActorRef,
    command: u64,
    lethal: bool,
) -> TestResult {
    let (actor, session) = player;
    let initial = if lethal { 1 } else { 1_000 };
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(target, initial)
            .map_err(debug)?;
        let row = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| row.actor == target)
            .ok_or("living ordered-death target")?;
        let mut attack = owner.states.attack.lock().await;
        assert!(attack.set_modes(
            &runtime,
            actor,
            session,
            oteryn_protocol_oteryn::attack::FightModes {
                fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
            }
        ));
        assert_eq!(
            attack.set_target(
                &runtime,
                owner.rooms.gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: target.placement_identity(),
                    generation: row.generation,
                }),
                command
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    for pass in 0..256 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        let runtime = owner.runtime.lock().await;
        let remaining = runtime.live_creature_health(target).unwrap_or(0);
        if (lethal && remaining == 0) || (!lethal && remaining < initial.unsigned_abs()) {
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms.gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                None,
                command + 1,
            );
            return Ok(());
        }
        assert!(pass + 1 < 256, "the real ordered-death swing must land");
    }
    Err("ordered-death swing did not land".into())
}

pub(in super::super) async fn qualify_ordered_deaths(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let policy = native
        .item_policy_for_source_id(51978)
        .ok_or("production bounty talisman")?;
    let definition = &policy.record().production_definition;
    // Previously equipped, correctly admitted talismans and near-complete boards
    // are fixture input. All attacks, qualification, completion and retries are real.
    for (index, &(actor, session)) in players.iter().enumerate() {
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("current fence")?;
        let item = crate::bestiary_postgres_harness::id(118 + u8::try_from(index)?);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid)")
            .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice())
            .bind(&definition.production_key).bind(&definition.revision_ref).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,10,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice())
            .bind(item.as_slice()).execute(&mut *tx).await?;
        for key in task_board::TALISMAN_PATHS {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,355,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(key).bind(task_board::QUEST)
                .execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE game_character_task_boards SET body=jsonb_set(body,'{bounty,active,current}','24'::jsonb) WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get_mut(&runtime, actor, session)
            .ok_or("player spell state")?
            .facts
            .level = 100;
    }
    let rats: Vec<_> = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .creatures
            .into_iter()
            .filter(|row| {
                runtime.companion_snapshot(row.actor).is_ok_and(|snapshot| {
                    snapshot.state.policy.definition_key == "oteryn:creature.rat"
                })
            })
            .map(|row| row.actor)
            .collect()
    };
    assert_eq!(rats.len(), 3);
    hit(owner, players[1], rats[1], 4_000_000, false).await?;
    hit(owner, players[0], rats[0], 4_000_010, true).await?;
    hit(owner, players[1], rats[1], 4_000_020, true).await?;
    hit(owner, players[0], rats[2], 4_000_030, true).await?;
    let entries = {
        let attack = owner.states.attack.lock().await;
        let mut held = Vec::new();
        for session in [players[0].1, players[1].1, players[0].1] {
            match attack.kills().take(session, 2) {
                kill_reward::TakeOutcome::Taken(taken) => held.push(taken),
                _ => return Err("three interleaved real deaths".into()),
            }
        }
        let entries = held
            .iter()
            .map(|taken| taken.entry().cloned().ok_or("pending entry"))
            .collect::<Result<Vec<_>, _>>()?;
        for taken in held {
            taken.requeue();
        }
        entries
    };
    assert!(
        entries
            .iter()
            .all(|entry| entry.captured_talisman().is_none())
    );

    // A failure at the oldest Bestiary write must also stop the later principal.
    sqlx::query("CREATE FUNCTION task_board_order_retry() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'TASK_BOARD_ORDER_RETRY'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER task_board_order_retry BEFORE INSERT ON game_character_bestiary_kill_receipts FOR EACH ROW EXECUTE FUNCTION task_board_order_retry()")
        .execute(&h.pool).await?;
    let attempted = owner.drain_kill_rewards(players[1].1).await;
    sqlx::query("DROP TRIGGER task_board_order_retry ON game_character_bestiary_kill_receipts")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION task_board_order_retry()")
        .execute(&h.pool)
        .await?;
    assert_eq!(attempted, kill_reward::DrainOutcome::Deferred);
    assert!(entries[0].captured_talisman().is_some());
    assert!(
        entries[1].captured_talisman().is_none(),
        "B1 cannot pass retrying A1"
    );
    assert!(
        entries[2].captured_talisman().is_none(),
        "A2 cannot pass retrying A1"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_task_board_receipts WHERE cause_kind='creature_death'"
        )
        .fetch_one(&h.pool)
        .await?,
        0
    );
    owner.root.maintain_ready_once().await?;
    assert_eq!(
        owner.drain_kill_rewards(players[1].1).await,
        kill_reward::DrainOutcome::Drained
    );
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 0);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(
            entry
                .task_board_recipients
                .get()
                .ok_or("frozen recipients")?
                .len(),
            2,
            "both real shared members must receive every kill"
        );
        let bonuses = entry
            .captured_talisman()
            .ok_or("frozen talisman")?
            .bonuses("oteryn:creature.rat");
        assert_eq!(bonuses[3], if index == 0 { 10_000 } else { 0 });
        assert_eq!(
            bonuses[2],
            if index == 0 { 5_000 } else { 0 },
            "only the first completing death retains final-kill loot"
        );
        let increment: i16 = sqlx::query_scalar("SELECT kill_count_increment FROM game_character_bestiary_kill_receipts WHERE bestiary_occurrence_id=encode($1,'hex')::uuid")
            .bind(entry.facts.occurrence.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(increment, if index == 0 { 2 } else { 1 });
    }
    let before = ledger(h).await?;
    assert_eq!(before.1.len(), 6);
    for (_, body) in &before.0 {
        let board: TaskBoardData = serde_json::from_value(body.clone())?;
        assert_eq!(board.bounty.state, BountyStatus::Completed);
        assert_eq!(board.bounty.active.ok_or("completed bounty")?.current, 25);
        assert_eq!(board.weekly.ok_or("weekly task")?.any_kills, 3);
    }
    // Replay in a different insertion order cannot change any accepted result.
    for entry in entries.into_iter().rev() {
        assert_eq!(
            owner.states.attack.lock().await.kills().append(entry),
            kill_reward::AppendOutcome::Queued
        );
    }
    owner.revision_sequencer = Default::default();
    assert_eq!(
        owner.drain_kill_rewards(players[1].1).await,
        kill_reward::DrainOutcome::Drained
    );
    assert_eq!(ledger(h).await?, before);
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: interleaved_A1_B1_A2_retry_order_and_single_final_bonus"
    );
    Ok(())
}
