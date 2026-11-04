//! WB2/P02: client bytes -> real router -> durable boss slots, fee and replay.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, capabilities::SelectedCapabilities};
use oteryn_protocol_oteryn::bosstiary_slots::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    request: BossSlotRequest,
) -> TestResult<BossSlotReply> {
    let payload = request.encode().ok_or("slot request")?;
    let id = admitted.continuity.next_command_id;
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: COMMAND_TYPE_BOSSTIARY_SLOTS,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(bytes.len())?.to_be_bytes())
        .await?;
    client.write_all(&bytes).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(a)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(a, _) => {
            *admitted = a
        }
        _ => return Err("connection".into()),
    }
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    while cursor < output.len() {
        let n = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let env = crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + n])
            .map_err(debug)?;
        cursor += 4 + n;
        if env.message_type() == crate::foundation::MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(env.payload()).map_err(debug)?;
            assert_eq!(result.command_id, id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            return BossSlotReply::decode(result.payload).ok_or("slot reply".into());
        }
    }
    Err("no slot result".into())
}
async fn ready(reply: &BossSlotReply) -> TestResult {
    // Wait for the server's actual deadline, rather than guessing a UI delay.
    loop {
        let now = u64::try_from(crate::gameplay_transport::npc_trade_command::unix_ms_now())?;
        if now >= reply.ready_at_ms {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(reply.ready_at_ms - now)).await;
    }
}
#[test]
fn bosstiary_slots_real_dispatch_persistence_unlock_remove_fee_and_boosted_kills_postgres()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "boss slots panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "wb2_boss_slots", true, false).await?;
        eprintln!("WB2_P02_POSTGRES_STARTED");
        prepare_current_session(&h).await?;
        super::charm_progression::seed(&h, &[]).await?;
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
        let (pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            pin,
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
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[117])
                .ok_or("boss capability")?;
        let bosses = crate::gameplay_transport::charm_progression::canonical_bosses()
            .ok_or("canonical bosses")?;
        let boss = bosses
            .iter()
            .find(|b| b.key == "oteryn:creature.ferumbras")
            .ok_or("ferumbras")?;
        let locked = exchange(&owner, &mut admitted, BossSlotRequest::Query).await?;
        assert_eq!(locked.unlocked, 0);
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                BossSlotRequest::Equip {
                    slot: 1,
                    boss: boss.key.clone()
                }
            )
            .await?
            .status,
            1
        );
        // Earn prowess through the production kill drain and its durable owner.
        let mut facts = {
            let mut rt = owner.runtime.lock().await;
            super::bosstiary::death(&mut rt, fence)?
        };
        facts.boosted_boss = true;
        super::bosstiary::settle(&owner, facts, boss).await?;
        super::bosstiary::settle(&owner, facts, boss).await?;
        let count:i64=sqlx::query_scalar("SELECT kill_count FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2").bind(id(41).as_slice()).bind(&boss.key).fetch_one(&h.pool).await?;
        assert_eq!(count, 3, "boosted credit and exact occurrence replay");
        let equipped = exchange(
            &owner,
            &mut admitted,
            BossSlotRequest::Equip {
                slot: 1,
                boss: boss.key.clone(),
            },
        )
        .await?;
        assert_eq!(equipped.status, 0);
        assert_eq!(equipped.bosses[0], boss.key);
        let reload = exchange(&owner, &mut admitted, BossSlotRequest::Query).await?;
        assert_eq!(reload.bosses, equipped.bosses);
        let bonuses = h
            .root
            .boss_slot_loot_bonuses(
                &authority,
                &h.node,
                fence.runtime_scope,
                fence.scope_ownership_generation,
                boss.clone(),
                vec![id(41)],
                u64::try_from(crate::gameplay_transport::npc_trade_command::unix_ms_now())?,
                false,
            )
            .await
            .map_err(debug)?;
        assert!(bonuses[0].1 >= 25);
        let boosted_bonus = h
            .root
            .boss_slot_loot_bonuses(
                &authority,
                &h.node,
                fence.runtime_scope,
                fence.scope_ownership_generation,
                boss.clone(),
                vec![id(41)],
                u64::try_from(crate::gameplay_transport::npc_trade_command::unix_ms_now())?,
                true,
            )
            .await
            .map_err(debug)?;
        // Crystal config.lua.dist:243 and src/io/io_bosstiary.cpp:225-232: the daily
        // boosted boss multiplier is 250%, while an equipped personal slot uses its level bonus.
        assert_eq!(boosted_bonus[0].1, 250);
        assert!(boosted_bonus[0].1 > bonuses[0].1);
        let at_death = h
            .root
            .boss_slot_loot_bonuses(
                &authority,
                &h.node,
                fence.runtime_scope,
                fence.scope_ownership_generation,
                boss.clone(),
                vec![id(41)],
                0,
                false,
            )
            .await
            .map_err(debug)?;
        assert_eq!(at_death[0].1, 0, "equip after death cannot add loot");
        // Wait out each recorded deadline; first remove is free, second costs 100000.
        ready(&equipped).await?;
        let removed = exchange(&owner, &mut admitted, BossSlotRequest::Remove { slot: 1 }).await?;
        assert_eq!(removed.status, 0);
        assert!(removed.bosses[0].is_empty());
        let outside_slot = h
            .root
            .boss_slot_loot_bonuses(
                &authority,
                &h.node,
                fence.runtime_scope,
                fence.scope_ownership_generation,
                boss.clone(),
                vec![id(41)],
                u64::try_from(crate::gameplay_transport::npc_trade_command::unix_ms_now())?,
                false,
            )
            .await
            .map_err(debug)?;
        assert_eq!(outside_slot[0].1, 0);
        assert!(bonuses[0].1 > outside_slot[0].1);
        assert_eq!(removed.remove_fee, 100000);
        ready(&removed).await?;
        let equipped = exchange(
            &owner,
            &mut admitted,
            BossSlotRequest::Equip {
                slot: 1,
                boss: boss.key.clone(),
            },
        )
        .await?;
        ready(&equipped).await?;
        let paid = exchange(&owner, &mut admitted, BossSlotRequest::Remove { slot: 1 }).await?;
        assert_eq!(paid.status, 0);
        assert_eq!(paid.remove_fee, 400000);
        let fees:(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(fee_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=16").fetch_one(&h.pool).await?;
        assert_eq!(fees, (1, 100000));
        let operations: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_boss_slot_operations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            operations, 4,
            "rejected equip and queries have no operations"
        );
        let mut stale = admitted;
        stale.item_fence = Some(
            crate::durability::item_transfer::CurrentCharacterItemFence {
                connection_generation: crate::foundation::ConnectionGeneration::new(999)
                    .map_err(debug)?,
                ..fence.item_fence()
            },
        );
        assert!(matches!(
            owner
                .boss_cooldowns_wire(stale, 999, COMMAND_TYPE_BOSSTIARY_SLOTS, &[0])
                .await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));
        // Fixture-only earned progress establishes the second slot's point threshold.
        // Production writes below still run under the restricted runtime role.
        let mut seeded = 0;
        let mut other = None;
        // Archfoe/nemesis mastery earns 100 points; bane mastery earns only 50.
        // Crystal src/io/io_bosstiary.hpp:52-55: seed exactly 1500 earned points.
        for candidate in bosses
            .iter()
            .filter(|b| b.key != boss.key && matches!(b.category.as_str(), "archfoe" | "nemesis"))
            .take(15)
        {
            let kills = crate::domain::bosstiary::kill_thresholds(&candidate.category)
                .ok_or("category")?[2];
            let points = crate::domain::bosstiary::progress(&candidate.category, kills)
                .ok_or("progress")?
                .points;
            sqlx::query("INSERT INTO game_character_bosstiary_kill_receipts(character_id,occurrence_id,boss_key,category) SELECT encode($1,'hex')::uuid,game_character_uuid_v7(),$2,$3 FROM generate_series(1::bigint,$4::bigint)")
                .bind(id(41).as_slice()).bind(&candidate.key).bind(&candidate.category).bind(i64::try_from(kills)?).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count,boss_points) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
                .bind(id(41).as_slice()).bind(&candidate.key).bind(i64::try_from(kills)?).bind(i64::from(points)).execute(&h.pool).await?;
            seeded += u32::from(points);
            other = Some(candidate.key.clone());
        }
        assert!(seeded >= 1500);
        let unlocked = exchange(&owner, &mut admitted, BossSlotRequest::Query).await?;
        assert_eq!(unlocked.unlocked, 3);
        ready(&paid).await?;
        let second = exchange(
            &owner,
            &mut admitted,
            BossSlotRequest::Equip {
                slot: 2,
                boss: other.ok_or("second boss")?,
            },
        )
        .await?;
        assert_eq!(second.status, 0);
        assert!(!second.bosses[1].is_empty());
        let persisted = exchange(&owner, &mut admitted, BossSlotRequest::Query).await?;
        assert_eq!(persisted.bosses, second.bosses);
        eprintln!("WB2_P02_POSTGRES_COMPLETED");
        Ok(())
    })
}
