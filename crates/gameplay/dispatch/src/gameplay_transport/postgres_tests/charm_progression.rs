//! PARITY-PROGRESSION-QUERIES-1: admitted command 84 bytes -> native charm port -> durable
//! receipts, assignment and gold fee -> reply bytes and reload (capability 67).
#![allow(clippy::expect_used)]
use super::*;
use crate::domain::bestiary::BestiaryRace;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
    CharmRemoveDisposition,
};
use oteryn_protocol_oteryn::CommandStatus;
use std::num::NonZeroU32;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A UUIDv7-shaped fixture id.
fn occurrence(n: u8) -> [u8; 16] {
    let mut bytes = id(n);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    bytes
}

/// One command on a fresh connection that continues this GameSession's command stream.
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: &CharmProgressionAction,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command_id = admitted.continuity.next_command_id;
    let payload = wire::encode_charm_progression_request(action).map_err(debug)?;
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: wire::COMMAND_TYPE_CHARM_PROGRESSION,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&[u32::try_from(bytes.len())?.to_be_bytes().as_slice(), &bytes].concat())
        .await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
    | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(value.command_id, command_id);
            return Ok((value.status, value.payload.to_vec()));
        }
    }
    Err(format!("no result for command {command_id}: {end:?}").into())
}

async fn reply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: CharmProgressionAction,
) -> TestResult<CharmProgressionReply> {
    let (status, bytes) = exchange(owner, admitted, &action).await?;
    assert_eq!(status, CommandStatus::Accepted, "{action:?}");
    wire::decode_charm_progression_reply(&bytes).map_err(debug)
}

async fn seed(h: &Harness, races: &[&BestiaryRace]) -> TestResult {
    // Explicit fixture seed, never a product writer. Runtime assertions below use the
    // normal restricted runtime role, including fee audit and revision constraints.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&mut *tx)
        .await?;
    let uuid = |n| crate::durability::item_fee_burn::test_support::uuid(n);
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({}, {}, {},1,sha256('seed'),{},{},{},{},1,1000,0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,1000,0,1000,{}); INSERT INTO game_account_bank_balances VALUES({},{},1000,{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "{}{} INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES({},{},{},{}); INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},1,{});",
        crate::durability::item_fee_burn::test_support::item(
            70,
            "oteryn:item.tibia.i2854",
            "definition-r1",
            1
        ),
        crate::durability::item_fee_burn::test_support::item(
            72,
            "oteryn:item.tibia.i3031",
            "definition-r1",
            5
        ),
        uuid(41),
        uuid(70),
        uuid(42),
        uuid(74),
        uuid(41),
        uuid(70),
        uuid(72),
        uuid(42),
        uuid(75)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    // Completed Bestiary entries: the fixture stands in for their kill receipts.
    for (n, race) in (90..).zip(races) {
        sqlx::query(
            "INSERT INTO game_character_bestiary_progress VALUES \
               (encode($1,'hex')::uuid, $2, $3, 2, encode($4,'hex')::uuid)",
        )
        .bind(id(41).as_slice())
        .bind(race.key())
        .bind(i64::from(race.final_kill_threshold()))
        .bind(occurrence(n).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}


#[test]
fn charm_progression_real_dispatch_unlocks_assigns_and_removes_for_the_gold_fee() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "charm progression scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "charm_progression", true, false).await?;
        eprintln!("POSTGRES_STARTED PARITY-PROGRESSION-QUERIES-1 charm_progression_real_dispatch");
        prepare_current_session(&harness).await?;
        // Three completed 100-point entries: 300 Charm Points.
        let races = crate::gameplay_transport::charm_progression::canonical_races()
            .ok_or("canonical Bestiary")?;
        let completed: Vec<&BestiaryRace> = races
            .iter()
            .filter(|race| race.final_kill_threshold() == 5000)
            .take(3)
            .collect();
        seed(&harness, &completed).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = generation.active();
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            dependencies.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = dependencies.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        assert!(owner.charm_progression_available());
        // Unnegotiated: the command never reaches the charm port.
        let (status, _) = exchange(&owner, &mut admitted, &CharmProgressionAction::Query).await?;
        assert_eq!(status, CommandStatus::Rejected);
        let offer = crate::gameplay_transport::charm_progression::offered(
            PRODUCTION_OFFERED_CAPABILITIES,
            owner.charm_progression_available(),
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &offer,
            &[1, wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1],
        )
        .ok_or("selection")?;

        let query = reply(&owner, &mut admitted, CharmProgressionAction::Query).await?;
        assert_eq!(query.outcome, CharmProgressionOutcome::Query);
        assert_eq!(query.charms.charm_points_available, 300);
        assert!(query.charms.charms.iter().all(|c| c.unlocked_stage == 0));
        // The wire index is the position in race-key order.
        let mut keys: Vec<&str> = races.iter().map(BestiaryRace::key).collect();
        keys.sort_unstable();
        let race = |key: &str| {
            keys.binary_search(&key)
                .ok()
                .and_then(|at| NonZeroU32::new(u32::try_from(at + 1).ok()?))
                .ok_or("race index")
        };
        let first = race(completed[0].key())?;
        assert!(query.bestiary.iter().any(|p| p.race == first));
        // The 240-point major charm (dodge).
        let charm = query
            .charms
            .charms
            .iter()
            .find(|c| c.next_stage_cost == 240)
            .ok_or("dodge")?
            .charm;

        let removed = reply(&owner, &mut admitted, CharmProgressionAction::Remove { charm }).await?;
        assert_eq!(
            (removed.outcome, removed.fee_gold),
            (CharmProgressionOutcome::Remove(CharmRemoveDisposition::NotAssigned), 0)
        );
        let unlocked = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Unlock {
                charm,
                expected_stage: 0,
            },
        )
        .await?;
        assert_eq!(
            unlocked.outcome,
            CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
        );
        assert_eq!(unlocked.charms.charm_points_available, 60);
        let assigned = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Assign { charm, race: first },
        )
        .await?;
        assert_eq!(
            assigned.outcome,
            CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        );
        let state = |view: &CharmProgressionReply| {
            view.charms
                .charms
                .iter()
                .find(|c| c.charm == charm)
                .map(|c| c.assigned_race)
        };
        assert_eq!(state(&assigned), Some(Some(first)));

        let level: i64 =
            sqlx::query_scalar("SELECT level FROM game_character_progression_state")
                .fetch_one(&harness.pool)
                .await?;
        let removed = reply(&owner, &mut admitted, CharmProgressionAction::Remove { charm }).await?;
        assert_eq!(
            (removed.outcome, removed.fee_gold),
            (
                CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed),
                u64::try_from(level)? * 100
            )
        );
        assert_eq!(state(&removed), Some(None), "the race is free again");
        let durable: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_charm_assignments), \
                    (SELECT count(*) FROM game_character_charm_receipts WHERE command_kind = 3), \
                    (SELECT sum(fee_gold_units)::bigint FROM game_item_fee_burns \
                      WHERE cause_kind = 1)",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(durable, (0, 1, level * 100));
        // A second remove has nothing to free and pays nothing.
        let again = reply(&owner, &mut admitted, CharmProgressionAction::Remove { charm }).await?;
        assert_eq!(
            again.outcome,
            CharmProgressionOutcome::Remove(CharmRemoveDisposition::NotAssigned)
        );
        // The charm can be assigned again after a remove (0195 assignment cycles).
        let reassigned = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Assign {
                charm,
                race: race(completed[1].key())?,
            },
        )
        .await?;
        assert_eq!(
            reassigned.outcome,
            CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        );
        Ok(())
    })
}
