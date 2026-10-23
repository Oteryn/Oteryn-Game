//! DAILY-1: admitted session -> literal wire -> atomic Inbox/claim -> result/reload.
use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::daily_reward as wire;

fn frame(command_id: u64, command_type: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    Ok([u32::try_from(bytes.len())?.to_be_bytes().as_slice(), &bytes].concat())
}
fn pick(key: &str, quantity: u8) -> TestResult<Vec<u8>> {
    let mut bytes = vec![
        10,
        u8::try_from(key.len() + 4)?,
        10,
        u8::try_from(key.len())?,
    ];
    bytes.extend(key.as_bytes());
    bytes.extend([16, quantity]);
    Ok(bytes)
}
fn results(output: &[u8]) -> TestResult<Vec<(u64, crate::foundation::CommandStatus, Vec<u8>)>> {
    let mut cursor = 0;
    let mut values = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            values.push((value.command_id, value.status, value.payload.to_vec()));
        }
    }
    Ok(values)
}
async fn epoch(h: &Harness, value: u64) -> TestResult {
    // World owner fixture: not a daily runtime clock or a client-supplied epoch.
    if value == 1 {
        sqlx::query("INSERT INTO game_world_reset_epochs VALUES(encode($1,'hex')::uuid,0,$2,'daily-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,'2026-10-01 08:00:00Z','2026-10')")
            .bind(id(42).as_slice()).bind([8_u8;32].as_slice()).execute(&h.pool).await?;
        let epoch:String=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_history WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(epoch, "1");
        return Ok(());
    }
    sqlx::query("INSERT INTO game_world_reset_history SELECT encode($1,'hex')::uuid,$2::text::numeric,((DATE '2026-10-01'+($2::text::integer-1))-DATE '1970-01-01')::bigint,extract(epoch FROM ((DATE '2026-10-01'+($2::text::integer-1))::timestamp + interval '10 hours') AT TIME ZONE 'Europe/Berlin')::bigint*1000,extract(epoch FROM ((DATE '2026-10-01'+($2::text::integer-1))::timestamp + interval '10 hours') AT TIME ZONE 'Europe/Berlin')::bigint*1000")
        .bind(id(42).as_slice()).bind(value.to_string()).execute(&h.pool).await?;
    Ok(())
}

#[test]
fn daily_reward_real_dispatch_delivers_replays_reloads_and_orders_seven_days() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "DAILY e2e panicked")?
        .map_err(Into::into)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let mut harness =
            Harness::create_with_admission_seed(admin, "daily_dispatch", true, false).await?;
        prepare_current_session(&harness).await?;
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!("DAILY-DISPATCH-PG database={database} server_version_num={version}");
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
        assert!(
            owner.daily_reward_available(),
            "the shipped native profile supports DAILY"
        );
        let offer = crate::gameplay_transport::daily_reward::offered(
            PRODUCTION_OFFERED_CAPABILITIES,
            owner.daily_reward_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offer, &[1, wire::CAPABILITY_DAILY_REWARD_V1])
                .ok_or("selection")?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_DAILY_REWARD_V1)
        );
        epoch(&harness, 1).await?;
        let potion = pick("oteryn:item.tibia.i266", 5)?;
        let shrine = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement: b"oteryn:placement/daily-reward-shrine".to_vec(),
                expected_revision: 0,
            },
        )
        .map_err(debug)?;
        let input = [
            frame(1, 33, &[])?,
            frame(2, 32, &potion)?,
            frame(3, 2, &shrine)?,
            frame(4, 32, &potion)?,
            frame(5, 33, &[])?,
            frame(6, 32, &potion)?,
        ]
        .concat();
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let values = results(&output)?;
        assert_eq!(values.len(), 6, "{end}; {values:?}");
        assert_eq!(values[1].2, [8, 4], "no shrine opened; {values:?}");
        assert_eq!(values[3].2.first(), Some(&8), "{values:?}");
        assert_eq!(values[3].2.get(1), Some(&1), "{values:?}");
        assert_eq!(values[3].2.len(), 20);
        let first_id = values[3].2[4..20].to_vec();
        assert_eq!(values[5].2, [8, 2]);
        let receipt: (i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_daily_reward_claims),(SELECT count(*) FROM game_character_daily_reward_lines),(SELECT count(*) FROM game_item_bindings),(SELECT count(*) FROM game_item_character_inbox_locations)").fetch_one(&harness.pool).await?;
        assert_eq!(receipt, (1, 1, 1, 1));
        let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE minted_transaction_id=encode($1,'hex')::uuid").bind(&first_id).fetch_one(&harness.pool).await?;
        assert_eq!(quantity, 5);
        // No selected capability: no DAILY payload and no additional durable row.
        let mut legacy = admitted;
        legacy.continuity.selected_capabilities = SelectedCapabilities::NONE;
        let (_, output) = super::weather_wire::fixture_access::serve_input(
            &owner,
            legacy,
            &frame(1, 32, &potion)?,
        )
        .await?;
        assert!(
            results(&output)?
                .iter()
                .all(|r| r.1 == crate::foundation::CommandStatus::Rejected && r.2.is_empty())
        );
        // Same-session new connection bytes settle a lost acknowledgement.
        admitted.continuity.next_command_id = 4;
        let (_, output) = super::weather_wire::fixture_access::serve_input(
            &owner,
            admitted,
            &frame(4, 32, &potion)?,
        )
        .await?;
        assert_eq!(results(&output)?[0].2, [8, 2]);
        for day in 2..=7 {
            admitted.continuity.next_command_id = day * 10;
            epoch(&harness, day).await?;
            let picks = match day {
                2 => potion.clone(),
                4 => pick("oteryn:item.tibia.i266", 10)?,
                6 => pick("oteryn:item.daily.training.sword", 1)?,
                _ => Vec::new(),
            };
            let input = [
                frame(day * 10, 2, &shrine)?,
                frame(day * 10 + 1, 32, &picks)?,
            ]
            .concat();
            let (end, output) =
                super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
            let values = results(&output)?;
            assert_eq!(
                values.last().and_then(|r| r.2.get(1)),
                Some(&1),
                "day {day}: {end}; {values:?}"
            );
        }
        let lines:Vec<(i16,String,i32,String,Option<i32>)>=sqlx::query_as("SELECT c.lane_day,l.kind,l.quantity,l.status,l.charges FROM game_character_daily_reward_lines l JOIN game_character_daily_reward_claims c ON c.transaction_id=l.claim_id ORDER BY c.reset_epoch,l.line").fetch_all(&harness.pool).await?;
        assert_eq!(lines.len(), 7);
        assert_eq!(
            lines[2],
            (3, "PREY_WILDCARD".into(), 1, "PENDING".into(), None)
        );
        assert_eq!(lines[4].4, Some(100));
        assert_eq!(lines[5].4, Some(50));
        assert_eq!(lines[6], (7, "XP_BOOST".into(), 10, "PENDING".into(), None));
        let state: (i16, i32) =
            sqlx::query_as("SELECT lane_day,streak FROM game_character_daily_rewards")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(state, (1, 7));
        // Rebuild the composed owner and replay an OLD command after later epochs.
        admitted.continuity.next_command_id = 4;
        let (_, output) = super::weather_wire::fixture_access::serve_input(
            &owner,
            admitted,
            &frame(4, 32, &potion)?,
        )
        .await?;
        assert_eq!(results(&output)?[0].2, [8, 2]);
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(count, 7);
        epoch(&harness, 9).await?;
        admitted.continuity.next_command_id = 90;
        let gap = [frame(90, 2, &shrine)?, frame(91, 32, &potion)?].concat();
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &gap).await?;
        assert_eq!(results(&output)?.last().and_then(|r| r.2.get(1)), Some(&1));
        let jokers:(i16,i32,i16)=sqlx::query_as("SELECT j.jokers,s.streak,c.jokers_spent FROM game_account_daily_reward_jokers j JOIN game_character_daily_rewards s USING(world_id) JOIN game_character_daily_reward_claims c ON c.character_id=s.character_id AND c.reset_epoch=s.last_claim_epoch").fetch_one(&harness.pool).await?;
        assert_eq!(jokers, (0, 8, 1));
        // A deferred proof failure AFTER native MINT rolls back every value row.
        epoch(&harness, 10).await?;
        sqlx::raw_sql("CREATE FUNCTION daily_fixture_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected DAILY line failure'; END $$; CREATE CONSTRAINT TRIGGER daily_fixture_fail AFTER INSERT ON game_character_daily_reward_lines DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION daily_fixture_fail()")
            .execute(&harness.pool).await?;
        admitted.continuity.next_command_id = 100;
        let retry = [frame(100, 2, &shrine)?, frame(101, 32, &potion)?].concat();
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &retry).await?;
        assert_ne!(results(&output)?.last().and_then(|r| r.2.get(1)), Some(&1));
        let rollback:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_daily_reward_claims),(SELECT count(*) FROM game_character_daily_reward_lines),(SELECT count(*) FROM game_item_bindings),(SELECT count(*) FROM game_item_character_inbox_locations)").fetch_one(&harness.pool).await?;
        assert_eq!(rollback, (8, 8, 6, 6));
        sqlx::query("DROP TRIGGER daily_fixture_fail ON game_character_daily_reward_lines")
            .execute(&harness.pool)
            .await?;
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &retry).await?;
        let retried = results(&output)?;
        assert_eq!(
            retried.last().and_then(|r| r.2.get(1)),
            Some(&1),
            "retry end={end}; results={retried:?}"
        );

        epoch(&harness, 11).await?;
        admitted.continuity.next_command_id = 110;
        let invalid = [frame(110, 32, &[255])?, frame(111, 32, &potion)?].concat();
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &invalid).await?;
        assert_eq!(
            results(&output)?
                .iter()
                .map(|r| r.2.clone())
                .collect::<Vec<_>>(),
            [vec![8, 3], vec![8, 3]]
        );
        let mut stale = admitted;
        stale.continuity.next_command_id = 112;
        let mut stale_fence = stale.item_fence.ok_or("fence")?;
        stale_fence.connection_generation =
            crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
        stale.item_fence = Some(stale_fence);
        let refused = crate::gameplay_transport::daily_reward::dispatch(
            &owner,
            dependencies.actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: stale.game_session_id,
                command_id: 112,
                item_fence: stale.item_fence,
            },
            32,
            &[],
        )
        .await;
        assert_eq!(refused, Some(vec![8, 5]));
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, stale, &frame(112, 32, &[])?)
                .await?;
        assert!(
            results(&output)?.is_empty(),
            "the connection drops a stale generation before command ingress"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.restart_appearance_root().await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut restarted = dependencies.owner(&harness, &authority);
        restarted.active_generation = generation.active();
        dependencies.initialize(&restarted).await;
        admitted.continuity.next_command_id = 4;
        let (_, output) = super::weather_wire::fixture_access::serve_input(
            &restarted,
            admitted,
            &frame(4, 32, &potion)?,
        )
        .await?;
        assert_eq!(results(&output)?.last().and_then(|r| r.2.get(1)), Some(&2));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(count, 9);
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        eprintln!("DAILY-DISPATCH-PG cleanup database={database}");
        Ok(())
    })
}
