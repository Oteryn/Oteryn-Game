//! Real command bytes -> production connection/composed owner -> runtime-role PostgreSQL -> result.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::house as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut n = value.len();
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(value);
}
fn intent(action: u8, payload: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    bytes(&mut b, action, payload);
    let mut target = Vec::new();
    bytes(&mut target, 1, &id(42));
    bytes(&mut target, 2, HOUSE.as_bytes());
    bytes(&mut b, 8, &target);
    b
}
fn acl(subowners: bool, revision: u64, members: &[[u8; 16]]) -> Vec<u8> {
    let mut b = Vec::new();
    scalar(&mut b, 1, if subowners { 2 } else { 1 });
    scalar(&mut b, 2, revision);
    for member in members {
        bytes(&mut b, 3, member);
    }
    intent(4, &b)
}
fn query() -> Vec<u8> {
    let mut b = Vec::new();
    bytes(&mut b, 1, &id(42));
    bytes(&mut b, 2, b"oteryn:content.area.city.liberty_bay");
    b
}
/// Sends the session's next fresh CommandId and keeps the ended continuity.
async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    kind: u32,
    body: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    resend(owner, admitted, command, kind, body).await
}
/// SESSION-INGRESS-1: each connection resumes this GameSession's command stream, so the
/// ended high-water mark and server sequence carry into the next connection.
async fn resend(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    kind: u32,
    body: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let frame = encode_client_command(
        before.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: kind,
            payload: body,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&(frame.len() as u32).to_be_bytes()).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut c = 0;
    while c < output.len() {
        let n = u32::from_be_bytes(output[c..c + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&output[c + 4..c + 4 + n]).map_err(debug)?;
        c += 4 + n;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command);
            assert_eq!(
                admitted.continuity.next_command_id,
                before.continuity.next_command_id
                    + u64::from(command == before.continuity.next_command_id)
            );
            assert!(admitted.continuity.server_sequence > before.continuity.server_sequence);
            return Ok((result.status, result.payload.to_vec()));
        }
    }
    Err(format!("House result {command}/{kind} missing: {end:?}").into())
}
/// Reserves fresh IDs with reads up to `command` on a new session directory.
async fn reach(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
) -> TestResult {
    while admitted.continuity.next_command_id < command {
        assert_eq!(
            send(owner, admitted, 57, &query()).await?.0,
            CommandStatus::Accepted
        );
    }
    assert_eq!(admitted.continuity.next_command_id, command);
    Ok(())
}
/// Even a rejected command advances ingress: keep its continuity, restore the selection.
fn resume(admitted: &mut AdmittedSession, ended: AdmittedSession) {
    let selected = admitted.continuity.selected_capabilities;
    admitted.continuity = ended.continuity;
    admitted.continuity.selected_capabilities = selected;
}
#[test]
fn house_wire_postgres_real_dispatch_acl_notice_replay_reload_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "house wire PG thread panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "house_wire_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let world = super::house_wire_bundle::house_bundle(
            deps.admitted.world_id,
            deps.admitted.channel_id,
        )?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let digest = owner
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        sqlx::query("WITH property AS (INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,acquisition_receipt_ref,paid_until) SELECT encode($1,'hex')::uuid,$2,'private_house',3,100,$4,character_id,account_id,game_character_uuid_v7(),statement_timestamp()+interval '30 days' FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid RETURNING *), slot AS (INSERT INTO game_account_housing_slots(account_id,world_id,house_key) SELECT owner_account_id,world_id,house_key FROM property) INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,owner_character_id,ownership_generation,acquisition_receipt_ref FROM property")
            .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;

        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 49])
                .ok_or("House selected")?;
        assert!(admitted.continuity.selected_capabilities.contains(49));
        let body = acl(false, 1, &[id(41)]);
        let ok = wire::encode_house_result(wire::HouseDisposition::Ok, 2);
        // deps.owner builds a fresh SessionCommands directory each time; later synthetic
        // connections start there from this continuity to reach the durable House receipts.
        let initial = admitted;
        assert_eq!(initial.continuity.next_command_id, 1);
        assert_eq!(
            send(&owner, &mut admitted, 56, &body).await?,
            (CommandStatus::Accepted, ok.clone())
        );
        // A reconnect before acknowledgement replays the retained result.
        assert_eq!(
            resend(&owner, &mut admitted, 1, 56, &body).await?,
            (CommandStatus::DuplicateReplay, ok.clone())
        );
        // FND-02 §13.4: replacement bytes under a reserved ID are not a new House intent.
        assert_eq!(
            resend(&owner, &mut admitted, 1, 56, &acl(false, 1, &[])).await?,
            (CommandStatus::DuplicateReplay, ok.clone())
        );
        let row:(String,i64)=sqlx::query_as("SELECT acl_revision::text,(SELECT count(*) FROM game_house_acl_entries WHERE house_key=$1 AND list_key='guest') FROM game_house_properties WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(row, ("2".into(), 1));
        assert_eq!(
            send(&owner, &mut admitted, 56, &body).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Conflict, 0)
        );
        assert_eq!(
            send(&owner, &mut admitted, 56, &acl(false, 2, &[])).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Ok, 3)
        );
        assert_eq!(
            send(&owner, &mut admitted, 56, &acl(true, 3, &[id(41)]))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Ok, 4)
        );
        // Unknown Character and cross-World inputs refuse before any durable effect.
        assert_eq!(
            send(&owner, &mut admitted, 56, &acl(false, 4, &[id(99)]))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        let mut foreign = body.clone();
        let at = foreign
            .windows(16)
            .position(|w| w == id(42))
            .ok_or("World field")?;
        foreign[at + 15] ^= 1;
        assert_eq!(
            send(&owner, &mut admitted, 56, &foreign).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        // Two days from the durable clock, plus replay and a range refusal.
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let due = u64::try_from(now)? + 2 * 86_400_000;
        let mut date = Vec::new();
        scalar(&mut date, 1, due);
        let notice = intent(2, &date);
        let notice_command = admitted.continuity.next_command_id;
        assert_eq!(notice_command, 7);
        assert_eq!(
            send(&owner, &mut admitted, 56, &notice).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_result(wire::HouseDisposition::Ok, 1)
            )
        );
        assert_eq!(
            resend(&owner, &mut admitted, notice_command, 56, &notice).await?,
            (
                CommandStatus::DuplicateReplay,
                wire::encode_house_result(wire::HouseDisposition::Ok, 1)
            )
        );
        let persisted: Option<i64> = sqlx::query_scalar(
            "SELECT due_at_unix_ms FROM game_house_move_out_notices WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(persisted, Some(i64::try_from(due)?));
        // Here ingress has no retained result, so changed input under command 1 reaches the
        // native House receipt conflict check.
        let fresh = deps.owner(&h, &authority);
        let mut conflict = initial;
        assert_eq!(
            send(&fresh, &mut conflict, 56, &acl(false, 1, &[])).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Conflict, 0)
        );
        drop(fresh);
        // A new composed runtime owner reads the durable notice (no cached management state).
        let mut restarted = deps.owner(&h, &authority);
        restarted.movement_cells = &cells;
        let mut admitted = initial;
        reach(&restarted, &mut admitted, 8).await?;
        let page = send(&restarted, &mut admitted, 57, &query()).await?.1;
        assert!(page.windows(HOUSE.len()).any(|w| w == HOUSE.as_bytes()));
        let cancel_command = admitted.continuity.next_command_id;
        assert_eq!(cancel_command, 9);
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(3, &[])).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Ok, 2)
        );
        let persisted: Option<i64> = sqlx::query_scalar(
            "SELECT due_at_unix_ms FROM game_house_move_out_notices WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(persisted, None);
        let mut old = admitted;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            send(&restarted, &mut old, 56, &body).await?,
            (CommandStatus::Rejected, vec![])
        );
        resume(&mut admitted, old);
        let mut stale = admitted;
        let generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        stale.item_fence =
            stale.item_fence.map(
                |f| crate::durability::item_transfer::CurrentCharacterItemFence {
                    connection_generation: generation,
                    ..f
                },
            );
        // Direct authority call: an ID the wire stream never reaches in this scenario.
        let stale_result = owner
            .house_intent(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 1_000,
                    item_fence: stale.item_fence,
                },
                wire::decode_house_intent(&acl(false, 4, &[])).map_err(debug)?,
            )
            .await;
        assert_eq!(
            stale_result,
            Some(wire::encode_house_result(
                wire::HouseDisposition::Rejected,
                0
            ))
        );
        let mut position = Vec::new();
        scalar(&mut position, 1, 6);
        scalar(&mut position, 2, 0);
        scalar(&mut position, 3, 13);
        let mut door = Vec::new();
        bytes(&mut door, 1, &position);
        scalar(&mut door, 2, 4);
        bytes(&mut door, 3, &id(41));
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(5, &door))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Ok, 5)
        );
        let members:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_acl_entries WHERE house_key=$1 AND list_key='door:6:0:-7'").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(members, 1);
        let mut old = admitted;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            send(&restarted, &mut old, 57, &query()).await?,
            (CommandStatus::Rejected, vec![])
        );
        resume(&mut admitted, old);
        let mut invalid_date = Vec::new();
        scalar(&mut invalid_date, 1, 0);
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(2, &invalid_date))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        let mut wrong_position = Vec::new();
        scalar(&mut wrong_position, 1, 7);
        scalar(&mut wrong_position, 2, 0);
        scalar(&mut wrong_position, 3, 13);
        let mut wrong_door = Vec::new();
        bytes(&mut wrong_door, 1, &wrong_position);
        scalar(&mut wrong_door, 2, 0);
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(5, &wrong_door))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        let revision: String = sqlx::query_scalar(
            "SELECT acl_revision::text FROM game_house_properties WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(revision, "5");
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(3, &[])).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        // A later global revision never changes an already committed command's result:
        // a new directory replays the durable receipts for commands 1 and 9.
        let replayed = deps.owner(&h, &authority);
        let mut replay = initial;
        assert_eq!(
            send(&replayed, &mut replay, 56, &body).await?,
            (CommandStatus::Accepted, ok)
        );
        reach(&replayed, &mut replay, cancel_command).await?;
        assert_eq!(
            send(&replayed, &mut replay, 56, &intent(3, &[])).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Ok, 2)
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_management_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 6);
        drop(replayed);
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
#[path = "house_wire/bid.rs"]
mod bid;
