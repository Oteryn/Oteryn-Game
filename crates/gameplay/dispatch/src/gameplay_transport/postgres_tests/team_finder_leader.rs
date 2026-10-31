//! S14a: commands 390/391 through the real admitted connection into the durable listing.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::team_finder::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult<CommandStatus> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let status = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                assert!(r.payload.is_empty());
                break r.status;
            }
        };
        client.shutdown().await?;
        Ok(status)
    };
    let (ended, result): (_, TestResult<CommandStatus>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("team finder command {id} ended {ended:?}: {error}").into())
}

#[test]
fn team_finder_leader_listing_create_edit_remove_survives_relog_through_the_wire() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("team-finder-leader", scenario)?;
    eprintln!(
        "S14A_PG_CASE_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

type Row = (i32, i32, i16, i32, i32, bool, i64, i16, i32, Option<i32>);

async fn listing(h: &Harness) -> TestResult<Option<Row>> {
    Ok(sqlx::query_as("SELECT min_level,max_level,vocation_mask,team_slots,free_slots,party_members_join,client_timestamp,team_type,target_id,hunt_area FROM game_team_finder_leader_listings WHERE leader_character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(41).as_slice()).fetch_optional(&h.pool).await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "team_finder_leader", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, _pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let unselected = admitted.continuity.selected_capabilities;
        let hunt = LeaderListing {
            min_level: 8,
            max_level: 120,
            vocation_mask: 15,
            slots: 5,
            free_slots: 4,
            party_members_join: true,
            timestamp: 1_700_000_000,
            target: TeamTarget::Hunt { kind: 2, area: 9 },
        };
        let upsert = COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT;
        let remove = COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE;
        // Without the selected capability the route refuses and writes nothing.
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                1,
                upsert,
                &encode_upsert(hunt).map_err(debug)?
            )
            .await?,
            CommandStatus::Rejected
        );
        assert_eq!(listing(&h).await?, None);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_TEAM_FINDER_LEADER_V1],
        )
        .filter(|selected| selected.contains(CAPABILITY_TEAM_FINDER_LEADER_V1))
        .ok_or("team finder capability not offered by bundle-world nodes")?;
        // A malformed listing (free slots above team slots) is refused before the owner.
        let mut bad = encode_upsert(hunt).map_err(debug)?;
        let at = bad.iter().position(|b| *b == 5 << 3).ok_or("free slots")?;
        bad[at + 1] = 6;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, upsert, &bad).await?,
            CommandStatus::Rejected
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 3, remove, &[1]).await?,
            CommandStatus::Rejected
        );
        assert_eq!(listing(&h).await?, None);
        // Create.
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                4,
                upsert,
                &encode_upsert(hunt).map_err(debug)?
            )
            .await?,
            CommandStatus::Accepted
        );
        assert_eq!(
            listing(&h).await?,
            Some((8, 120, 15, 5, 4, true, 1_700_000_000, 2, 2, Some(9)))
        );
        // Edit replaces the whole listing, including the target kind.
        let boss = LeaderListing {
            free_slots: 0,
            party_members_join: false,
            target: TeamTarget::Boss(1234),
            ..hunt
        };
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                5,
                upsert,
                &encode_upsert(boss).map_err(debug)?
            )
            .await?,
            CommandStatus::Accepted
        );
        let edited = Some((8, 120, 15, 5, 0, false, 1_700_000_000, 1, 1234, None));
        assert_eq!(listing(&h).await?, edited);
        // A replayed command id returns its retained result and writes nothing again.
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                5,
                upsert,
                &encode_upsert(boss).map_err(debug)?
            )
            .await?,
            CommandStatus::DuplicateReplay
        );
        assert_eq!(listing(&h).await?, edited);
        // Relog: a new owner over the same character still sees the durable listing.
        drop(owner);
        let mut reopened = deps.owner(&h, &authority);
        reopened.active_generation = Some(active);
        deps.initialize(&reopened).await;
        assert_eq!(listing(&h).await?, edited);
        // The new connection starts its command ids again; the receipts are per command id.
        let mut relogged = deps.admitted;
        relogged.first_entry = FirstEntryOutcome::Positioned;
        relogged.item_fence = admitted.item_fence;
        relogged.continuity.selected_capabilities = admitted.continuity.selected_capabilities;
        let mut admitted = relogged;
        assert_eq!(
            exchange(&reopened, &mut admitted, 1, remove, &[]).await?,
            CommandStatus::Accepted
        );
        assert_eq!(listing(&h).await?, None);
        // Removing again succeeds, as Crystal's reset does.
        assert_eq!(
            exchange(&reopened, &mut admitted, 2, remove, &[]).await?,
            CommandStatus::Accepted
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_team_finder_leader_receipts WHERE accepted",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 4);
        admitted.continuity.selected_capabilities = unselected;
        assert_eq!(
            exchange(&reopened, &mut admitted, 3, remove, &[]).await?,
            CommandStatus::Rejected
        );
        // Migration is safe to execute again and grants exactly the runtime access it needs.
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/0578_team_finder_leader_listing.sql"
        ))
        .execute(&h.pool)
        .await?;
        let access: (bool, bool, bool, bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_team_finder_leader_listings','DELETE'),has_table_privilege('oteryn_game_runtime','game_team_finder_leader_receipts','INSERT'),has_table_privilege('oteryn_game_runtime','game_team_finder_leader_receipts','UPDATE'),has_table_privilege('oteryn_game_control','game_team_finder_leader_listings','SELECT'),has_table_privilege('oteryn_game_control','game_team_finder_leader_listings','INSERT'),has_table_privilege('public','game_team_finder_leader_listings','SELECT')").fetch_one(&h.pool).await?;
        assert_eq!(access, (true, true, false, true, false, false));
        drop(reopened);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
