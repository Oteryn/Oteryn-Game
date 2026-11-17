//! NX-2: real Svargrond/Drome routes, PostgreSQL receipts and native panel reduction.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction};

#[test]
fn nx2_arena_status_postgres_wire_receipt_and_generation_fence() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            let harness = Harness::create_with_admission_seed(admin, "nx2_arena", false, false).await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor, deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2,
                    ).map_err(debug)?,
                ).map_err(debug)?;
            }
            let owner = deps.owner(&harness, &authority);
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[170],
            ).ok_or("arena capability")?;
            let payload = wire::encode_match_request(ArenaAction::Status).map_err(debug)?;
            let command_id = admitted.continuity.next_command_id;
            let (status, bytes) = super::guild_wire::exchange(&owner, &mut admitted,
                command_id, wire::COMMAND_TYPE_ARENA_MATCH, &payload).await?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            let decoded = wire::decode_match_result(&bytes).map_err(debug)?;
            assert_eq!(decoded.disposition, wire::ArenaDisposition::Applied);
            assert_eq!(decoded.state, wire::ArenaState::default());
            assert_eq!(owner.arena_match_command(admitted, command_id, &payload).await,
                Some(bytes), "exact persisted replay");
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(count, 1);
            // Decode real command-436 Drome replies in the native panel reducer.
            // This is the same request/reducer used by the native window, not a
            // synthetic Drome response or a second server implementation.
            let mut panel = oteryn_dev_client::client_ui::arena::ArenaPanel { selected: true, ..Default::default() };
            for action in [wire::DromeAction::Status, wire::DromeAction::Enter] {
                let intent = oteryn_dev_client::client_ui::arena::Intent::Action(ArenaAction::Drome(action));
                let request = oteryn_dev_client::client_ui::arena::request(&mut panel, intent).ok_or("native Drome request")?;
                let id = admitted.continuity.next_command_id;
                let (status, payload) = super::guild_wire::exchange(&owner, &mut admitted,
                    id, request.kind, &request.payload).await?;
                assert!(oteryn_dev_client::client_ui::arena::apply_result(&mut panel,
                    &oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult {
                        command_id: id, status, server_sequence: 1, payload,
                    }));
                let state = panel.drome.ok_or("decoded native Drome state")?;
                assert_eq!((state.level, state.points, state.rank, state.potions), (0, 0, 0, 0));
                assert_eq!(state.disposition, if action == wire::DromeAction::Status {1} else {3});
                let text = oteryn_dev_client::client_ui::arena::panel_text(&panel);
                assert!(text.contains("Tibiadrome"));
                assert!(text.contains("Drome points: 0"));
                if action == wire::DromeAction::Enter { assert!(text.contains("Premium and level 50")); }
                assert!(panel.pending.is_none());
                assert!(panel.result.is_none());
            }
            let mut stale = admitted;
            stale.continuity.connection_generation += 1;
            assert!(owner.arena_match_command(stale, admitted.continuity.next_command_id, &payload).await.is_none());
            let count_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(count_after, count, "stale native request writes no receipt");
            Ok(())
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "NX-2 arena PG thread panicked")??;
    eprintln!(
        "NX2_POSTGRES_EXECUTED native_drome_status_refusal_svargrond_receipt_replay_generation_fence duration={:?}",
        started.elapsed()
    );
    Ok(())
}
