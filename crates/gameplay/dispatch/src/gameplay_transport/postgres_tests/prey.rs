//! PREY-LIVE-1: offered command bytes, durable draws/receipts and hunting expiry.
use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::prey as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn prey_postgres_real_dispatch_draw_select_reroll_expiry_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "prey scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "prey_live", true, false).await?;
        eprintln!("PREY_POSTGRES_STARTED");
        // Deliberate reapplication must retain constraints, roles and the private draw root.
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/0337_prey_owner.sql"
        ))
        .execute(&h.pool)
        .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let formula = owner
            .active_generation
            .and_then(|a| a.native_gameplay())
            .and_then(|n| n.training_formula())
            .ok_or("formula")?;
        let before = crate::durability::character_build::DurableBuildState::default();
        let choice = crate::gameplay_transport::vocation_choice::vocation_choice_request(
            crate::durability::character_build::BuildOccurrence::from_bytes(id(94))
                .map_err(debug)?,
            before,
            "monk",
            formula,
            None,
        )
        .map_err(debug)?;
        owner
            .revision_sequencer
            .acquire(current.character_id)
            .await
            .commit_build(
                owner.root,
                owner.character,
                owner.holder,
                current,
                choice,
                formula,
                None,
            )
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        assert!(owner.prey_available());
        assert!(owner.daily_reward_available());
        let base = crate::gameplay_transport::daily_reward::offered(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            true,
        );
        let offered = crate::gameplay_transport::prey::offered(&base, true);
        assert!(offered.iter().any(|c| c.id == 140));
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[1, 6, 32, 140]).ok_or("prey capability")?;
        sqlx::query("UPDATE game_prey_decision_root SET secret=decode($1,'hex')")
            .bind("a9f248eab2b6a601f753a75c7741a6c8be1e428c3923b13995e3f3ef64b0ac2a")
            .execute(&h.pool)
            .await?;
        assert!(owner.admit_prey(admitted).await);
        let initial = admitted;
        let query = wire::encode_request(&wire::Request::Query).map_err(debug)?;
        let values = exchange(&owner, &mut admitted, &[(1, &query)]).await?;
        assert!(
            admitted.continuity.selected_capabilities.contains(32),
            "negotiated daily capability survives Prey query"
        );
        assert_eq!(values[0].0, CommandStatus::Accepted);
        let state = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(state.slots.len(), 3);
        assert_eq!(state.slots[0].state, 1);
        assert_eq!(state.slots[0].creatures.len(), 9);
        assert_eq!(state.slots[1].state, 0);
        assert_eq!(state.slots[2].state, 0);
        let locked = wire::encode_request(&wire::Request::ListReroll { slot: 3 }).map_err(debug)?;
        let native = owner
            .active_generation
            .and_then(|a| a.native_gameplay())
            .ok_or("native")?;
        let index = state.slots[0]
            .creatures
            .iter()
            .position(|key| {
                native
                    .reward_table()
                    .row(key)
                    .is_ok_and(|r| r.loot_table.entries.len() <= 7)
            })
            .ok_or("supported drawn creature")?;
        let select = wire::encode_request(&wire::Request::Select {
            slot: 1,
            index: (index + 1) as u8,
        })
        .map_err(debug)?;
        let values = exchange(&owner, &mut admitted, &[(2, &locked), (3, &select)]).await?;
        assert_eq!(values[0].0, CommandStatus::Rejected);
        assert_eq!(values[1].0, CommandStatus::Accepted);
        let active_bytes = values[1].1.clone();
        let selected = wire::decode_reply(&active_bytes).map_err(debug)?;
        assert_eq!(selected.slots[0].selected, state.slots[0].creatures[index]);
        assert_eq!(selected.slots[0].hunting_seconds, 7200);
        assert!((1..=4).contains(&selected.slots[0].bonus));
        assert!((2..=10).contains(&selected.slots[0].grade));
        assert_eq!(
            exchange(&owner, &mut admitted, &[(4, &select)]).await?[0].0,
            CommandStatus::Rejected
        );
        let reroll = wire::encode_request(&wire::Request::ListReroll { slot: 1 }).map_err(debug)?;
        sqlx::query("UPDATE game_prey_decision_root SET secret=decode($1,'hex')")
            .bind("a9f248eab2b6a601f753a75c7741a6c8be1e428c3923b13995e3f3ef64b0ac2a")
            .execute(&h.pool)
            .await?;
        let values = exchange(&owner, &mut admitted, &[(5, &reroll)]).await?;
        let rerolled = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(rerolled.slots[0].state, 1);
        assert_eq!(rerolled.slots[0].bonus, selected.slots[0].bonus);
        assert_eq!(rerolled.slots[0].grade, selected.slots[0].grade);
        assert_eq!(rerolled.slots[0].hunting_seconds, 0);
        assert_eq!(rerolled.slots[0].creatures.len(), 9);
        assert!(rerolled.slots[0].free_reroll_at > state.slots[0].free_reroll_at);
        // No coins or bank: paid reroll is refused and preserves its free deadline.
        assert_eq!(
            exchange(&owner, &mut admitted, &[(6, &reroll)]).await?[0].0,
            CommandStatus::Rejected
        );
        let index = rerolled.slots[0]
            .creatures
            .iter()
            .position(|key| {
                native
                    .reward_table()
                    .row(key)
                    .is_ok_and(|r| r.loot_table.entries.len() <= 7)
            })
            .ok_or("supported reroll creature")?;
        let select_again = wire::encode_request(&wire::Request::Select {
            slot: 1,
            index: (index + 1) as u8,
        })
        .map_err(debug)?;
        let values = exchange(&owner, &mut admitted, &[(7, &select_again)]).await?;
        assert_eq!(values[0].0, CommandStatus::Accepted);
        // Real hunting clock: idle does not spend; two activity samples spend one minute.
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states.load_stamina(&runtime, deps.actor, admitted.game_session_id, 2520);
            states.hunting_activity(&runtime, deps.actor, admitted.game_session_id, 0);
            states.hunting_activity(&runtime, deps.actor, admitted.game_session_id, 60_000_000);
        }
        assert!(owner.flush_prey(admitted.game_session_id).await);
        let left: i32 =
            sqlx::query_scalar("SELECT hunting_s_left FROM game_character_prey_slots WHERE slot=1")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(left, 7140);
        // Drive the same server hunting tick to expiry; no wall/offline elapsed time is used.
        for minute in 2..=120 {
            let runtime = owner.runtime.lock().await;
            owner.states.spell_states.lock().await.hunting_activity(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                minute * 60_000_000,
            );
        }
        assert!(owner.flush_prey(admitted.game_session_id).await);
        let values = exchange(&owner, &mut admitted, &[(8, &query)]).await?;
        let expired = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(expired.slots[0].state, 1);
        assert_eq!(expired.slots[0].bonus, 0);
        assert!(expired.slots[0].selected.is_empty());
        assert_eq!(expired.slots[0].creatures.len(), 9);
        // Seed ordinary carried coins using the existing Item fixture convention. Only setup
        // uses the admin replication role; the command runs with every production trigger on.
        {
            let index = native.item_index().ok_or("items")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (seed, key, quantity) in [
                (210, "oteryn:item.tibia.i2854", 1),
                (212, "oteryn:item.tibia.i3043", 2),
            ] {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        key,
                        index.revision(key).ok_or("revision")?,
                        quantity,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(214).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(id(42).as_slice()).bind(id(215).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let values = exchange(&owner, &mut admitted, &[(9, &reroll)]).await?;
        assert_eq!(values[0].0, CommandStatus::Accepted);
        let paid = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(
            paid.slots[0].free_reroll_at,
            rerolled.slots[0].free_reroll_at
        );
        let fee: i64 = sqlx::query_scalar(
            "SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=12",
        )
        .fetch_one(&h.pool)
        .await?;
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(fee, level * 200);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(212).as_slice()).fetch_one(&h.pool).await?,1);
        // Obtain cards through actual Daily Reward commands; all ledger constraints stay on.
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&coordinator);
        let shrine = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement: b"oteryn:placement/daily-reward-shrine".to_vec(),
                expected_revision: 0,
            },
        )
        .map_err(debug)?;
        for day in 1..=24 {
            super::daily_reward::epoch(&h, day).await?;
            let lane = (day - 1) % 7 + 1;
            let pick = match lane {
                1 | 2 => super::daily_reward::pick("oteryn:item.tibia.i266", 5)?,
                4 => super::daily_reward::pick("oteryn:item.tibia.i266", 10)?,
                6 => super::daily_reward::pick("oteryn:item.tibia.i28540", 1)?,
                _ => vec![],
            };
            let (end, bytes) =
                super::daily_reward::serve(&owner, &mut admitted, &[(2, &shrine), (32, &pick)])
                    .await?;
            let results = super::daily_reward::results(&bytes)?;
            assert_eq!(
                results.last().and_then(|r| r.2.get(1)),
                Some(&1),
                "grant day {day}: {end}; {results:?}"
            );
        }
        assert_eq!(
            sqlx::query_scalar::<_, i16>("SELECT balance FROM game_account_prey_wildcards")
                .fetch_one(&h.pool)
                .await?,
            12
        );
        // Oversized source tables are explicit content refusals, never skipped bonus rolls.
        let unsupported = wire::encode_request(&wire::Request::SelectAny {
            slot: 1,
            creature: "oteryn:creature.dragon".into(),
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let values = exchange(&owner, &mut admitted, &[(command, &unsupported)]).await?;
        let refused = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(refused.status, wire::STATUS_CONTENT_UNAVAILABLE);
        assert_eq!(refused.wildcard_balance, 12);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_prey_wildcard_entries WHERE kind='SPEND'"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        );
        let any = wire::encode_request(&wire::Request::SelectAny {
            slot: 1,
            creature: "oteryn:creature.rat".into(),
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let values = exchange(&owner, &mut admitted, &[(command, &any)]).await?;
        assert_eq!(values[0].0, CommandStatus::Accepted);
        let chosen = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(chosen.slots[0].selected, "oteryn:creature.rat");
        assert_eq!(chosen.wildcard_balance, 7);
        // Each expiry checkpoints the actual native hunting signal, then debits atomically.
        for (option, cost) in [(2, 5), (1, 1)] {
            let bytes = wire::encode_request(&wire::Request::SetOption { slot: 1, option })
                .map_err(debug)?;
            let command = admitted.continuity.next_command_id;
            assert_eq!(
                exchange(&owner, &mut admitted, &[(command, &bytes)]).await?[0].0,
                CommandStatus::Accepted
            );
            owner.states.spell_states.lock().await.prey_hunting_tick(
                deps.actor,
                admitted.game_session_id,
                7200,
            );
            let before: i16 = sqlx::query_scalar("SELECT balance FROM game_account_prey_wildcards")
                .fetch_one(&h.pool)
                .await?;
            assert!(owner.flush_prey(admitted.game_session_id).await);
            assert_eq!(
                sqlx::query_scalar::<_, i16>("SELECT balance FROM game_account_prey_wildcards")
                    .fetch_one(&h.pool)
                    .await?,
                before - cost
            );
            let command = admitted.continuity.next_command_id;
            let values = exchange(&owner, &mut admitted, &[(command, &query)]).await?;
            let renewed = wire::decode_reply(&values[0].1).map_err(debug)?;
            assert_eq!(renewed.slots[0].state, 2);
            assert_eq!(renewed.slots[0].hunting_seconds, 7200);
            if option == 2 {
                assert_eq!(renewed.slots[0].bonus, chosen.slots[0].bonus);
                assert_eq!(renewed.slots[0].grade, chosen.slots[0].grade);
            }
        }
        let reroll_bonus =
            wire::encode_request(&wire::Request::BonusReroll { slot: 1 }).map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, &[(command, &reroll_bonus)]).await?[0].0,
            CommandStatus::Accepted
        );
        assert_eq!(
            sqlx::query_scalar::<_, i16>("SELECT balance FROM game_account_prey_wildcards")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, &[(command, &reroll_bonus)]).await?[0].0,
            CommandStatus::Rejected
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_prey_wildcard_entries WHERE kind='SPEND'"
            )
            .fetch_one(&h.pool)
            .await?,
            4
        );
        // The retained AUTO_REROLL now has no cards: expiry clears it and draws a list.
        owner.states.spell_states.lock().await.prey_hunting_tick(
            deps.actor,
            admitted.game_session_id,
            7200,
        );
        assert!(owner.flush_prey(admitted.game_session_id).await);
        let command = admitted.continuity.next_command_id;
        let values = exchange(&owner, &mut admitted, &[(command, &query)]).await?;
        let unfunded = wire::decode_reply(&values[0].1).map_err(debug)?;
        assert_eq!(unfunded.slots[0].state, 1);
        assert_eq!(unfunded.slots[0].option, 0);
        // Replay after later writes returns the original frozen result, without another write.
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_prey_receipts")
            .fetch_one(&h.pool)
            .await?;
        drop(owner);
        let mut restarted = deps.owner(&h, &authority);
        restarted.active_generation = generation.active();
        deps.initialize(&restarted).await;
        let mut replay = initial;
        let values = exchange(
            &restarted,
            &mut replay,
            &[(1, &query), (2, &locked), (3, &select)],
        )
        .await?;
        assert_eq!(values[2], (CommandStatus::Accepted, active_bytes));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_receipts")
                .fetch_one(&h.pool)
                .await?,
            count
        );
        let mut stale = initial;
        stale
            .item_fence
            .as_mut()
            .ok_or("stale fence")?
            .character_lease_generation += 1;
        assert!(matches!(
            restarted.prey_command(stale, 999, &reroll).await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));
        assert!(matches!(
            restarted.prey_command(initial, 3, &reroll).await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_receipts")
                .fetch_one(&h.pool)
                .await?,
            count
        );
        let mut legacy_owner = deps.owner(&h, &authority);
        legacy_owner.active_generation = generation.active();
        let mut unsupported = initial;
        unsupported.continuity.selected_capabilities =
            SelectedCapabilities::select(legacy_owner.offered_capabilities(), &[6])
                .ok_or("base")?;
        assert_eq!(
            exchange(&legacy_owner, &mut unsupported, &[(1, &query)]).await?[0].0,
            CommandStatus::Rejected
        );
        drop(legacy_owner);
        restarted.premium_coordinator = Some(&coordinator);
        // A real permanent Account+World fact unlocks slot 3; Premium alone only opens slot 2.
        sqlx::query("INSERT INTO game_account_prey_unlocks(account_id,world_id,unlock_kind,target,source,claiming_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'PREY_SLOT_PERMANENT',3,'STORE_CLAIM',encode($3,'hex')::uuid)").bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
        assert!(restarted.admit_prey(replay).await);
        let command = replay.continuity.next_command_id;
        let bytes = exchange(&restarted, &mut replay, &[(command, &query)]).await?;
        let unlocked = wire::decode_reply(&bytes[0].1).map_err(debug)?;
        assert_eq!(
            unlocked.slots.iter().map(|s| s.state).collect::<Vec<_>>(),
            [1, 1, 1]
        );
        assert!(unlocked.slots.iter().all(|s| s.creatures.len() == 9));
        let keys: std::collections::BTreeSet<_> = unlocked
            .slots
            .iter()
            .flat_map(|s| s.creatures.iter())
            .collect();
        assert_eq!(keys.len(), 27);
        restarted.premium_coordinator = None;
        let command = replay.continuity.next_command_id;
        let bytes = exchange(&restarted, &mut replay, &[(command, &query)]).await?;
        let free = wire::decode_reply(&bytes[0].1).map_err(debug)?;
        assert_eq!(free.slots[1].state, 0);
        assert_eq!(free.slots[2].state, 1);
        drop(restarted);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u64, &[u8])],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let mut input = Vec::new();
    for (command_id, payload) in commands {
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: *command_id,
                command_type: wire::COMMAND_TYPE_PREY,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend_from_slice(&u32::try_from(frame.len())?.to_be_bytes());
        input.extend_from_slice(&frame);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = tokio::time::timeout(
        Duration::from_secs(30),
        serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS),
    )
    .await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = current,
        other => return Err(format!("prey connection ended {other:?}").into()),
    }
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
        cursor += 4 + length;
    }
    assert_eq!(
        results.len(),
        commands.len(),
        "Prey command results: {end:?}"
    );
    Ok(results)
}
