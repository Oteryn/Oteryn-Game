use super::*;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};

pub(super) fn run_scenario() -> TestResult {
    run(async |admin| {
        reset();
        let mut h = Harness::create_with_admission_seed(admin, "vip_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        eprintln!("VIP-PG installed 0163, real admitted wire scenario");
        let base = super::super::fixture::fence(1)?.item_fence();
        characters::historical_character(
            &h.pool,
            90,
            CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?,
            characters::sid(90, 2),
            id(42),
            "Alice",
        )
        .await?;
        let target = characters::historical_session(
            &h.pool,
            90,
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: 1,
                ..base
            },
        )
        .await?;
        let deps = Dependencies::new()?;
        let mut target_deps = Dependencies::new()?;
        let actor = {
            let mut runtime = target_deps.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(target.game_session_id)
                .map_err(debug)?;
            runtime.commit_fresh_session(reservation).map_err(debug)?
        };
        target_deps.actor = actor;
        target_deps.admitted.runtime_actor = Some(actor);
        target_deps.admitted.game_session_id = target.game_session_id;
        target_deps
            .admitted
            .controller
            .as_mut()
            .ok_or("controller")?
            .account_id = characters::sid(90, 2);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (consumer, clock, _, _) = purchase_fixture::current(id(40));
        // Empty durable activation latch exercises the actual NotActivated physical transaction.
        let coordinator =
            SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&coordinator);
        let target_owner = target_deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        target_deps.initialize(&target_owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        target_owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(target_deps.actor)
            .map_err(debug)?;
        let mut admitted = selected(deps.admitted)?;
        admitted.item_fence = Some(
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: 2,
                ..base
            },
        );
        let mut target_admitted = selected(target_deps.admitted)?;
        target_admitted.item_fence = Some(target);
        let add_command = admitted.continuity.next_command_id;
        let (mut left_server, mut left) = tokio::io::duplex(128 * 1024);
        let (mut right_server, mut right) = tokio::io::duplex(128 * 1024);
        let left_serving = serve_admitted(&mut left_server, admitted, &owner, IDLE_LIVENESS);
        let right_serving = serve_admitted(
            &mut right_server,
            target_admitted,
            &target_owner,
            IDLE_LIVENESS,
        );
        let driving = async {
            login(&mut left)
                .await
                .map_err(|e| format!("VIP owner login: {e}"))?;
            login(&mut right)
                .await
                .map_err(|e| format!("VIP target login: {e}"))?;
            let (mut a, mut b) = (
                admitted.continuity.next_command_id,
                target_admitted.continuity.next_command_id,
            );
            let add = ok(&mut left, &mut a, &named(1, "Alice")).await?;
            // Retained ingress outcomes replay without interpreting replacement bytes.
            for name in ["Alice", "Other Name"] {
                let (status, bytes) = reply(&mut left, add_command, &named(1, name)).await?;
                assert_eq!(status, CommandStatus::DuplicateReplay);
                assert_eq!(bytes, add);
            }
            let entry = number(&add)?;
            assert_eq!(entry, 1);
            let invitation = view(&ok(&mut right, &mut b, &[8, 12]).await?)?;
            assert_eq!(invitation["consents"][0]["name"], "Fixture Hero");
            let handle = invitation["consents"][0]["handle"]
                .as_u64()
                .ok_or("handle")?;
            assert_eq!(
                view(&ok(&mut left, &mut a, &[8, 12]).await?)?["entries"][0]["confirmed"],
                false
            );
            ok(&mut right, &mut b, &numbered(7, handle)).await?;
            assert_eq!(
                view(&ok(&mut left, &mut a, &[8, 12]).await?)?["entries"][0]["confirmed"],
                true
            );
            ok(&mut right, &mut b, &[8, 11, 16, 2]).await?;
            assert_eq!(
                view(&ok(&mut right, &mut b, &[8, 12]).await?)?["shown"],
                false
            );
            let group = number(&ok(&mut left, &mut a, &named(4, "Raids")).await?)?;
            assert_eq!(group, 4);
            let mut rename = numbered(5, group);
            rename.extend(&named(4, "Allies")[2..]);
            ok(&mut left, &mut a, &rename).await?;
            ok(
                &mut left,
                &mut a,
                &edit(entry, "  trusted friend  ", 10, true, &[2, 4]),
            )
            .await?;
            let snapshot = view(&ok(&mut left, &mut a, &[8, 12]).await?)?;
            assert_eq!(snapshot["entries"][0]["description"], "trusted friend");
            assert_eq!(snapshot["entries"][0]["groups"], json!([2, 4]));
            assert_eq!(snapshot["groups"][0]["name"], "Allies");
            let serialized = snapshot.to_string();
            for private in [
                "account_id",
                "character_id",
                "invitation_id",
                "channel",
                "online",
            ] {
                assert!(!serialized.contains(private));
            }
            // Revoke, re-invite, then reject the fresh invitation. Old handles must stay stale.
            ok(&mut right, &mut b, &numbered(9, handle)).await?;
            ok(&mut left, &mut a, &named(1, "Alice")).await?;
            let blocked_pending = view(&ok(&mut right, &mut b, &[8, 12]).await?)?;
            let blocked_handle = blocked_pending["consents"][0]["handle"]
                .as_u64()
                .ok_or("blocked handle")?;
            // The installed hook is the same one reached by PARTY-1's existing block writer.
            sqlx::query("INSERT INTO game_character_social_blocks(character_id,blocked_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(target.character_id.as_bytes().as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
            assert!(
                view(&ok(&mut right, &mut b, &[8, 12]).await?)?["consents"]
                    .as_array()
                    .ok_or("blocked consents")?
                    .is_empty()
            );
            assert_eq!(
                command(&mut right, &mut b, &numbered(7, blocked_handle))
                    .await?
                    .0,
                7
            );
            ok(&mut left, &mut a, &named(1, "Alice")).await?;
            sqlx::query("DELETE FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid")
                .bind(target.character_id.as_bytes().as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
            ok(&mut left, &mut a, &named(1, "Alice")).await?;
            let pending = view(&ok(&mut right, &mut b, &[8, 12]).await?)?;
            let next = pending["consents"][0]["handle"]
                .as_u64()
                .ok_or("new handle")?;
            assert_ne!(handle, next);
            assert_eq!(
                command(&mut right, &mut b, &numbered(7, handle)).await?.0,
                7
            );
            ok(&mut right, &mut b, &numbered(8, next)).await?;
            ok(&mut left, &mut a, &named(1, "Alice")).await?;
            ok(&mut left, &mut a, &numbered(10, entry)).await?;
            assert!(
                view(&ok(&mut right, &mut b, &[8, 12]).await?)?["consents"]
                    .as_array()
                    .ok_or("consents")?
                    .is_empty()
            );
            // Invalid metadata is refused atomically; unknown groups do not partially replace it.
            assert_eq!(
                command(&mut left, &mut a, &edit(entry, "bad", 0, false, &[8]))
                    .await?
                    .0,
                7
            );
            assert_eq!(
                command(&mut left, &mut a, &edit(entry, "bad", 11, false, &[]))
                    .await?
                    .0,
                10
            );
            ok(&mut left, &mut a, &numbered(6, group)).await?;
            let snapshot = view(&ok(&mut left, &mut a, &[8, 12]).await?)?;
            assert_eq!(snapshot["entries"][0]["groups"], json!([2]));
            // The query cursor is applied by the real codec and SQL owner.
            let after = view(&ok(&mut left, &mut a, &numbered(12, entry)).await?)?;
            assert!(
                after["entries"]
                    .as_array()
                    .ok_or("paged entries")?
                    .is_empty()
            );
            assert_eq!(after["next_entry"], 0);
            assert_eq!(
                command(&mut left, &mut a, &numbered(12, u64::MAX)).await?.0,
                10
            );
            // Fresh refusals consume the durable bucket; exhausted outcomes are immutable.
            let mut exhausted = None;
            for _ in 0..21 {
                let sequence = a;
                let (code, bytes) = command(&mut left, &mut a, &named(1, "Unknown Friend")).await?;
                assert!(code == 2 || code == 8);
                if code == 8 {
                    exhausted = Some((sequence, bytes));
                    break;
                }
            }
            let (exhausted_id, exhausted_bytes) = exhausted.ok_or("VIP bucket did not exhaust")?;
            assert!((1..=3).contains(&wait(&exhausted_bytes)?));
            let replayed = owner
                .vip_command(admitted, exhausted_id, 48, &named(1, "Unknown Friend"))
                .await
                .ok_or("rate replay")?;
            assert_eq!(replayed, exhausted_bytes);
            // Test-only elapsed bucket DATA; do not wait on the shared machine's clock.
            sqlx::query("UPDATE game_vip_lists SET rate_at=rate_at-60 WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).execute(&h.pool).await?;
            // Reload keeps the final metadata entry; removal and a new entry number follow restart.
            let receipt_count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_vip_command_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert!(receipt_count >= 15);
            let rows: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_roots WHERE character_revision<>1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(rows, 0);
            left.shutdown().await?;
            right.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>((a, b, add, entry))
        };
        let (_, outcome) = tokio::time::timeout(
            Duration::from_secs(60),
            fixture::join_two(fixture::join_two(left_serving, right_serving), driving),
        )
        .await?;
        let (next, _, add, entry) = outcome?;
        let second =
            DurabilityRoot::connect_test_runtime(h.pool.connect_options().to_url_lossy().as_str())?;
        assert!(second.maintain_ready_once().await?);
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut second_owner = deps.owner(&h, &second_authority);
        second_owner.root = &second;
        let request = named(1, "Unknown Friend");
        let (one, two) = fixture::join_two(
            owner.vip_command(admitted, 500, 48, &request),
            second_owner.vip_command(admitted, 500, 48, &request),
        )
        .await;
        assert_eq!(
            one.ok_or("first concurrent result")?,
            two.ok_or("second concurrent result")?
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_vip_command_receipts WHERE command_id=500",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 1);
        drop(second_owner);
        drop(second_authority);
        super::negatives::fences(&owner, admitted, add_command, &h.pool).await?;
        let wrong_node = crate::durability::runtime_scope_assignment::NodeIncarnationProof::new(
            h.node.fact(),
            crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([0; 32]),
        );
        let mut wrong_owner = deps.owner(&h, &authority);
        wrong_owner.holder = &wrong_node;
        assert_eq!(
            wrong_owner
                .vip_command(admitted, 700, 48, &named(1, "Alice"))
                .await
                .ok_or("node refusal")?
                .get(1)
                .copied()
                .ok_or("node result code")?,
            10
        );
        // Historical receipt replay before live session checks; changed binding is rejected.
        let ended = "UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid";
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(ended)
            .bind(id(50).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        assert_eq!(
            owner
                .vip_command(admitted, add_command, 48, &named(1, "Alice"))
                .await
                .ok_or("replay")?,
            add
        );
        let conflict = owner
            .vip_command(admitted, add_command, 48, &named(1, "Other Name"))
            .await
            .ok_or("binding conflict")?;
        assert_eq!(code(&conflict)?, 10);
        let fresh = owner
            .vip_command(admitted, next, 48, &named(1, "Alice"))
            .await
            .ok_or("stale refusal")?;
        assert_eq!(code(&fresh)?, 10);
        // Restore session DATA, without reconstructing any recovery or node proof.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(50).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        drop(owner);
        drop(target_owner);
        drop(authority);
        let url = h.pool.connect_options().to_url_lossy();
        h.root = DurabilityRoot::connect_test_runtime(url.as_str())?;
        assert!(h.root.maintain_ready_once().await?);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        // A new owner has a fresh SessionCommands directory. Use initial continuity to
        // exercise the durable journal, not same-GameSession recovery after truth loss.
        let (mut server, mut client) = tokio::io::duplex(128 * 1024);
        let serving = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
        let driving = async {
            login(&mut client).await?;
            let mut a = admitted.continuity.next_command_id;
            assert_eq!(a, add_command);
            assert_eq!(
                command(&mut client, &mut a, &named(1, "Other Name")).await?.0,
                10
            );
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (_, outcome) =
            tokio::time::timeout(Duration::from_secs(30), fixture::join_two(serving, driving))
                .await?;
        outcome?;
        drop(owner);
        // Discard the conflict's retained rejection before testing reload independently.
        let owner = deps.owner(&h, &authority);
        let resumed = admitted;
        let (mut server, mut client) = tokio::io::duplex(128 * 1024);
        let serving = serve_admitted(&mut server, resumed, &owner, IDLE_LIVENESS);
        let driving = async {
            login(&mut client).await?;
            let mut a = resumed.continuity.next_command_id;
            let snapshot = view(&ok(&mut client, &mut a, &[8, 12]).await?)?;
            assert_eq!(snapshot["entries"][0]["description"], "trusted friend");
            assert_eq!(snapshot["entries"][0]["icon"], 10);
            assert_eq!(snapshot["entries"][0]["notify"], true);
            ok(&mut client, &mut a, &numbered(2, entry)).await?;
            let added = ok(&mut client, &mut a, &named(1, "Alice")).await?;
            assert!(number(&added)? > entry);
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(a)
        };
        let (ended, outcome) =
            tokio::time::timeout(Duration::from_secs(30), fixture::join_two(serving, driving))
                .await?;
        let next = outcome?;
        let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(
            mut old,
        ) = ended else {
            return Err(format!("VIP reload connection ended {ended:?}").into());
        };
        assert_eq!(old.continuity.next_command_id, next);
        assert!(old.continuity.server_sequence > resumed.continuity.server_sequence);
        // Capability-negative wire: command gate returns the common empty rejection.
        old.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[])
                .ok_or("old selection")?;
        let (mut server, mut client) = tokio::io::duplex(128 * 1024);
        let serving = serve_admitted(&mut server, old, &owner, IDLE_LIVENESS);
        let driving = async {
            login(&mut client).await?;
            let frame = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: old.continuity.next_command_id,
                    command_type: 50,
                    payload: &[8, 12],
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let env = decode_wire_envelope(&frame).map_err(debug)?;
                if env.message_type() == MessageType::CommandResult {
                    let r = decode_command_result(env.payload()).map_err(debug)?;
                    assert_eq!(r.status, CommandStatus::Rejected);
                    assert!(r.payload.is_empty());
                    break;
                }
            }
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (_, outcome) =
            tokio::time::timeout(Duration::from_secs(30), fixture::join_two(serving, driving))
                .await?;
        outcome?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
