//! Command 133 through the real admitted connection, fenced PostgreSQL read and wire result.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::hazard::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
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
                    command_type: COMMAND_TYPE_HAZARD,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                assert!(r.payload.len() <= 2);
                break (r.status, r.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("cooldown command {id} ended {ended:?}: {error}").into())
}

#[test]
fn hazard_postgres_real_dispatch_selection_reload_fences_and_capability() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("hazard-dispatch", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "hazard_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
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
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_HAZARD_V1])
                .ok_or("hazard capability not offered")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, &[]).await?,
            (CommandStatus::Accepted, vec![1, 1])
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 2, &[2]).await?,
            (CommandStatus::Rejected, vec![])
        );
        // Administrative seed represents earned unlocks; no client packet grants them.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(fixture::id(41).as_slice()).fetch_one(&mut *seed).await?;
        sqlx::query("INSERT INTO game_character_hazard_receipts(character_id,command_session_id,connection_generation,command_id,selected_requested,result,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,encode($1,'hex')::uuid,1,900,1,decode('0c01','hex'),character_revision,character_revision+1,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($2,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(fixture::id(41).as_slice()).execute(&mut *seed).await?;
        for query in [
            "UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
            "UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
        ] {
            sqlx::query(query)
                .bind(fixture::id(41).as_slice())
                .execute(&mut *seed)
                .await?;
        }
        sqlx::query("UPDATE game_character_hazard SET unlocked=12 WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(41).as_slice()).execute(&mut *seed).await?;
        seed.commit().await?;
        assert_eq!(
            exchange(&owner, &mut admitted, 3, &[12]).await?,
            (CommandStatus::Accepted, vec![12, 12])
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 3, &[12]).await?,
            (CommandStatus::DuplicateReplay, vec![12, 12])
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 4, &[0]).await?,
            (CommandStatus::Rejected, vec![])
        );
        // New owner state reads the durable value, rather than inheriting the command cache.
        let reopened = deps.owner(&h, &authority);
        deps.initialize(&reopened).await;
        assert!(reopened.admit_hazard(admitted).await);
        assert_eq!(
            reopened
                .runtime
                .lock()
                .await
                .hazard_level(deps.actor, admitted.game_session_id),
            Some(12)
        );
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .character_hazard(&authority, &h.node, stale, Some((6, 1)))
                .await
                .is_err()
        );
        assert_eq!(
            h.root
                .character_hazard(&authority, &h.node, fence, None)
                .await
                .map_err(debug)?,
            Some((12, 12, vec![12, 12]))
        );
        admitted.continuity.selected_capabilities = unselected;
        assert_eq!(
            exchange(&owner, &mut admitted, 5, &[]).await?,
            (CommandStatus::Rejected, vec![])
        );
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "3"); // fixture unlock and the one real selection each advance once.
        let replay = h
            .root
            .character_hazard(&authority, &h.node, fence, Some((3, 12)))
            .await
            .map_err(debug)?;
        assert_eq!(replay, Some((12, 12, vec![12, 12])));
        assert!(
            h.root
                .character_hazard(&authority, &h.node, fence, Some((3, 1)))
                .await
                .is_err()
        );
        // A same-state selection has a durable receipt but no semantic revision successor.
        assert_eq!(
            h.root
                .character_hazard(&authority, &h.node, fence, Some((6, 12)))
                .await
                .map_err(debug)?,
            Some((12, 12, vec![12, 12]))
        );
        let bad = sqlx::query(
            "UPDATE game_character_hazard SET selected=1 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fixture::id(41).as_slice())
        .execute(&h.pool)
        .await;
        assert!(
            bad.is_err(),
            "a state change without a receipt must roll back"
        );
        // Migration is safe to execute again and grants exactly the required runtime access.
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/0332_character_hazard.sql"
        ))
        .execute(&h.pool)
        .await?;
        let access: (bool,bool,bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_character_hazard','SELECT'),has_table_privilege('oteryn_game_runtime','game_character_hazard','UPDATE'),has_table_privilege('oteryn_game_runtime','game_character_hazard','DELETE')").fetch_one(&h.pool).await?;
        assert_eq!(access, (true, true, false));
        eprintln!(
            "HAZARD PostgreSQL: dispatched selection, replay, reload, capability and stale lease verified"
        );
        drop(reopened);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
