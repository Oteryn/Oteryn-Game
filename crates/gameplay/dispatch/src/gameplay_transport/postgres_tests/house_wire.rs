//! Real command bytes -> production connection/composed owner -> runtime-role PostgreSQL -> result.
#[path = "house_wire/bid.rs"]
mod bid;
#[path = "house_wire/runtime_gate.rs"]
mod runtime_gate;

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
    query_for("oteryn:content.area.city.liberty_bay")
}
fn query_for(town: &str) -> Vec<u8> {
    let mut b = Vec::new();
    bytes(&mut b, 1, &id(42));
    bytes(&mut b, 2, town.as_bytes());
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
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
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
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_house_wire_scenario(false)
}

#[test]
fn house_wire_postgres_ordered_acl_patterns() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_house_wire_scenario(true)
}

fn run_house_wire_scenario(patterns: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(patterns).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "house wire PG thread panicked")??;
    Ok(())
}
fn scenario(patterns: bool) -> TestResult {
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
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        if patterns {
            owner
                .runtime
                .lock()
                .await
                .initialize_pinned_test_position(
                    deps.actor,
                    crate::foundation::MovementLocalPosition {
                        x: 5,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        } else {
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
        }
        let digest = owner
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        sqlx::query("WITH property AS (INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,acquisition_receipt_ref,paid_until) SELECT encode($1,'hex')::uuid,$2,'private_house',3,100,$4,character_id,account_id,game_character_uuid_v7(),statement_timestamp()+interval '30 days' FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid RETURNING *), slot AS (INSERT INTO game_account_housing_slots(account_id,world_id,house_key) SELECT owner_account_id,world_id,house_key FROM property) INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,owner_character_id,ownership_generation,acquisition_receipt_ref FROM property")
            .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
        const GUILDHALL: &str = "oteryn:content.house.bamboo_fortress";
        let mut classification = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,$2,'guildhall',1,100,$3)")
            .bind(id(42).as_slice()).bind(GUILDHALL).bind(digest.as_slice()).execute(&mut *classification).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,NULL,1,encode($3,'hex')::uuid)")
            .bind(id(42).as_slice()).bind(GUILDHALL).bind(id(88).as_slice()).execute(&mut *classification).await?;
        classification.commit().await?;
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
        let owned: bool = sqlx::query_scalar(
            "SELECT p.state=3 AND p.content_digest=$3 AND
                    game_house_effective_owner(p.world_id,p.house_key)=encode($4,'hex')::uuid
             FROM game_house_properties p WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2",
        )
        .bind(id(42).as_slice())
        .bind(HOUSE)
        .bind(digest.as_slice())
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert!(
            owned,
            "the fixture must reproduce S7's settled, served owner"
        );
        super::journal_inventory::runtime_role(&h.root).await?;
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
            send(&owner, &mut admitted, 56, &acl(false, 2, &[]))
                .await?
                .1,
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
            send(&fresh, &mut conflict, 56, &acl(false, 1, &[]))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Conflict, 0)
        );
        drop(fresh);
        // A new composed runtime owner reads the durable notice (no cached management state).
        let mut restarted = deps.owner(&h, &authority);
        restarted.movement_cells = (&cells).into();
        let mut admitted = initial;
        reach(&restarted, &mut admitted, 8).await?;
        let page = send(&restarted, &mut admitted, 57, &query()).await?.1;
        assert!(page.windows(HOUSE.len()).any(|w| w == HOUSE.as_bytes()));
        let cancel_command = admitted.continuity.next_command_id;
        assert_eq!(cancel_command, 9);
        assert_eq!(
            send(&restarted, &mut admitted, 56, &intent(3, &[]))
                .await?
                .1,
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
        scalar(&mut position, 3, 14);
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
        for (selector, members) in [
            (wire::HouseAclSelector::Guest, vec![]),
            (wire::HouseAclSelector::Subowner, vec![id(41)]),
            (
                wire::HouseAclSelector::Door {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
                vec![id(41)],
            ),
        ] {
            let read = wire::HouseAclRead {
                house: HOUSE.into(),
                selector,
            };
            let payload = wire::encode_house_acl_read_query(
                id(42),
                "oteryn:content.area.city.liberty_bay",
                &read,
            )
            .map_err(debug)?;
            let (status, response) = send(&restarted, &mut admitted, 57, &payload).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                wire::decode_house_acl_snapshot(&response).map_err(debug)?,
                wire::HouseAclSnapshot {
                    revision: 5,
                    members
                }
            );
            let wrong_world = wire::encode_house_acl_read_query(
                id(99),
                "oteryn:content.area.city.liberty_bay",
                &read,
            )
            .map_err(debug)?;
            assert_eq!(
                send(&restarted, &mut admitted, 57, &wrong_world).await?.1,
                wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
            );
        }
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
            send(&restarted, &mut admitted, 56, &intent(3, &[]))
                .await?
                .1,
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
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 40, 49, 99])
                .ok_or("Guildhall classification capability")?;
        let guildhall_page = send(
            &restarted,
            &mut admitted,
            57,
            &query_for("oteryn:content.area.city.port_hope"),
        )
        .await?
        .1;
        assert!(
            guildhall_page
                .windows(GUILDHALL.len())
                .any(|w| w == GUILDHALL.as_bytes())
        );
        // Field 11 carries the qualified Guildhall class (enum value 3).
        assert!(guildhall_page.windows(2).any(|field| field == [88, 3]));
        verify_guild_acl(&h, &restarted, &mut admitted).await?;
        if patterns {
            verify_door_target(&restarted, &mut admitted).await?;
            verify_legacy_acl_bound(&h, &restarted, &mut admitted).await?;
            verify_ordered_acl(&h, &restarted, &mut admitted, id(91)).await?;
            verify_pattern_door(&h, &restarted, &mut admitted).await?;
        }
        drop(replayed);
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// Both principals pass through command ingress and the canonical House grant reader.
async fn verify_guild_acl(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'House Guild',2,9999999999999,1,1)")
        .bind(id(90).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,1,'Leader'),(encode($1,'hex')::uuid,2,'Member')")
        .bind(id(90).as_slice()).execute(&mut *tx).await?;
    let actor = id(91);
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,name,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision) SELECT encode($1,'hex')::uuid,account_id,world_id,'House Member',lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
        .bind(actor.as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1)")
        .bind(actor.as_slice()).bind(id(90).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 49, 515])
            .ok_or("House guild ACL capability")?;
    for (revision, rule, expected_role) in [
        (5, "@House Guild", Some("GUEST")),
        (6, "Leader@House Guild", None),
        (7, "Member@House Guild", Some("GUEST")),
    ] {
        let mut replacement = Vec::new();
        scalar(&mut replacement, 1, 1);
        scalar(&mut replacement, 2, revision);
        bytes(&mut replacement, 3, rule.as_bytes());
        assert_eq!(
            send(owner, admitted, 6252, &intent(4, &replacement)).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_result(wire::HouseDisposition::Ok, revision + 1)
            )
        );
        assert_eq!(guild_house_role(h, actor).await?.as_deref(), expected_role);
    }

    let mut position = Vec::new();
    scalar(&mut position, 1, 6);
    scalar(&mut position, 2, 0);
    scalar(&mut position, 3, 14);
    let mut door = Vec::new();
    bytes(&mut door, 1, &position);
    scalar(&mut door, 2, 8);
    let mut selectors = Vec::new();
    bytes(&mut selectors, 1, b"Member@House Guild");
    bytes(&mut door, 4, &selectors);
    assert_eq!(
        send(owner, admitted, 56, &intent(5, &door)).await?,
        (
            CommandStatus::Accepted,
            wire::encode_house_result(wire::HouseDisposition::Ok, 9)
        )
    );
    assert_eq!(guild_house_lists(h, actor).await?, ["door:6:0:-7", "guest"]);

    let mut tx = h.pool.begin().await?;
    // Administrative fixture edits reproduce a rename and a replacement at
    // the same rank level without involving a second guild transport session.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_guild_ranks SET name='Veteran' WHERE guild_id=encode($1,'hex')::uuid AND level=2")
        .bind(id(90).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(guild_house_role(h, actor).await?.as_deref(), Some("GUEST"));
    assert_eq!(guild_house_lists(h, actor).await?, ["door:6:0:-7", "guest"]);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid AND level=2")
        .bind(id(90).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,2,'Veteran')")
        .bind(id(90).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(guild_house_role(h, actor).await?, None);
    assert!(guild_house_lists(h, actor).await?.is_empty());
    verify_guild_acl_write_fence(h, admitted).await
}

async fn verify_legacy_acl_bound(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // This used to be an exact rank selector. The pattern grammar may not
    // turn old bytes into a grant to all guild members.
    let mut old = Vec::new();
    scalar(&mut old, 1, 1);
    scalar(&mut old, 2, 9);
    bytes(&mut old, 3, b"*@House Guild");
    assert_eq!(
        send(owner, admitted, 6252, &intent(4, &old)).await?.1,
        wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
    );

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let guilds: Vec<String> = sqlx::query_scalar(
        "INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at)
         SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,
          repeat('a',23) || chr(97 + n / 26) || chr(97 + n % 26),2,9999999999999,1,1
         FROM generate_series(0,79) n RETURNING name",
    ).bind(id(42).as_slice()).fetch_all(&mut *tx).await?;
    let mut lines: Vec<_> = guilds.iter().map(|guild| format!("@{guild}")).collect();
    lines.push("@House Guild".to_owned());
    lines.sort_unstable();
    // These records have the pre-3354 identity-only shape and a valid existing
    // receipt. An upgrade must leave their active grants and list revision intact.
    sqlx::query(
        "UPDATE game_house_acl_guild_ranks a SET raw_lines=NULL, rules=(
          SELECT jsonb_agg(jsonb_build_object('guild',g.guild_id,'rank',NULL,'identity',NULL) ORDER BY g.name COLLATE \"C\")
          FROM game_guilds g WHERE g.world_id=a.world_id AND g.name=ANY($2))
         WHERE a.house_key=$1 AND a.list_key='guest'",
    ).bind(HOUSE).bind(guilds.iter().map(String::as_str).chain(["House Guild"]).collect::<Vec<_>>())
     .execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(lines.join("\n").chars().count() > 1999);
    assert_eq!(guild_house_role(h, id(91)).await?.as_deref(), Some("GUEST"));
    assert_eq!(
        read_acl_rules(owner, admitted, wire::HouseAclSelector::Guest).await?,
        (9, lines.clone(), false)
    );

    let mut old = Vec::new();
    scalar(&mut old, 1, 1);
    scalar(&mut old, 2, 9);
    for line in &lines {
        bytes(&mut old, 3, line.as_bytes());
    }
    assert_eq!(
        send(owner, admitted, 6252, &intent(4, &old)).await?.1,
        wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
    );
    assert_eq!(
        read_acl_rules(owner, admitted, wire::HouseAclSelector::Guest).await?,
        (9, lines, false)
    );

    let selected = admitted.continuity.selected_capabilities;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 49])
            .ok_or("House without guild ACL capability")?;
    let body = textual_acl(1, 9, &["*"]);
    assert_eq!(
        send(owner, admitted, 6252, &body).await?,
        (CommandStatus::Rejected, vec![])
    );
    admitted.continuity.selected_capabilities = selected;
    Ok(())
}

async fn read_acl_rules(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    selector: wire::HouseAclSelector,
) -> TestResult<(u64, Vec<String>, bool)> {
    let query = wire::encode_house_acl_rules_query(
        id(42),
        "oteryn:content.area.city.liberty_bay",
        &wire::HouseAclRead {
            house: HOUSE.to_owned(),
            selector,
        },
    )
    .map_err(debug)?;
    let (status, reply) = send(owner, admitted, 57, &query).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let (snapshot, rules) = wire::decode_house_acl_rules_snapshot(&reply).map_err(debug)?;
    Ok((snapshot.revision, rules.lines, rules.ordered))
}

async fn verify_ordered_acl(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: [u8; 16],
) -> TestResult {
    let comment = format!("#{}", "a".repeat(1998));
    let overlong = format!("{comment}a");
    let too_many = vec!["#"; 101];
    let max_lines = vec!["#"; 100];
    let cases: &[(u64, &[&str], Option<&str>, bool)] = &[
        (9, &["# ignored", "!House Member", "House M*"], None, true),
        (10, &["ho?se m*b?r", "!House Member"], Some("GUEST"), true),
        (11, &["House Member?"], None, true),
        (12, &["House Member*"], Some("GUEST"), true),
        (13, &["House", "%", "House Membe_"], None, true),
        (14, &["House Member"], Some("GUEST"), true),
        (15, &["*", "!House Member"], Some("GUEST"), true),
        (16, &["!House Member", "*@House Guild"], None, true),
        (17, &["*@House Guild", "!House Member"], Some("GUEST"), true),
        (18, &["Veteran@House Guild"], Some("GUEST"), true),
        (19, &max_lines, None, true),
        (20, &too_many, None, false),
        (20, &[&overlong], None, false),
        (20, &[&comment, "#"], None, false),
        (20, &["# embedded\nLF"], None, false),
        (20, &["# embedded\rCR"], None, false),
        (20, &["# embedded\0NUL"], None, false),
        (20, &[&comment], None, true),
        (21, &[""], None, true),
    ];
    for &(revision, lines, role, accepted) in cases {
        let body = textual_acl(1, revision, lines);
        let (status, reply) = send(owner, admitted, 6252, &body).await?;
        if accepted {
            assert_eq!(
                (status, reply),
                (
                    CommandStatus::Accepted,
                    wire::encode_house_result(wire::HouseDisposition::Ok, revision + 1)
                ),
                "House list {lines:?}"
            );
            assert_eq!(
                read_acl_rules(owner, admitted, wire::HouseAclSelector::Guest).await?,
                (
                    revision + 1,
                    lines.iter().map(|line| (*line).to_owned()).collect(),
                    true
                )
            );
        } else {
            assert_ne!(
                reply,
                wire::encode_house_result(wire::HouseDisposition::Ok, revision + 1)
            );
        }
        assert_eq!(guild_house_role(h, actor).await?.as_deref(), role);
    }
    assert_eq!(
        send(owner, admitted, 6252, &textual_acl(2, 22, &["House M*"]))
            .await?
            .1,
        wire::encode_house_result(wire::HouseDisposition::Ok, 23)
    );
    assert_eq!(
        guild_house_role(h, actor).await?.as_deref(),
        Some("SUBOWNER")
    );
    assert_eq!(
        read_acl_rules(owner, admitted, wire::HouseAclSelector::Subowner).await?,
        (23, vec!["House M*".to_owned()], true)
    );
    Ok(())
}

async fn verify_door_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let bundle = owner.bundle_map().ok_or("House door bundle")?;
    let bundle_digest = bundle.overlay.base().digest();
    let artifact_digest = owner
        .runtime
        .lock()
        .await
        .content_pin()
        .client_artifact_digest();
    assert_ne!(bundle_digest, artifact_digest);
    assert_eq!(
        use_house_door(owner, admitted, artifact_digest, 0).await?,
        UseDisposition::StaleState
    );
    assert_eq!(
        use_house_door(owner, admitted, bundle_digest, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_house_door(owner, admitted, bundle_digest, 1).await?,
        UseDisposition::Committed
    );
    Ok(())
}

async fn use_house_door(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    digest: [u8; 32],
    revision: u64,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use crate::map::boot::tests::door_key;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, WorldObjectTarget, decode_use_result, encode_use_intent,
    };
    let mut placement = digest.to_vec();
    placement.extend_from_slice(&door_key().to_be_bytes());
    let target = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
    .map_err(debug)?;
    tokio::time::sleep(crate::gameplay_transport::map_door::WORLD_ACTION_COOLDOWN).await;
    let (_, reply) = send(owner, admitted, COMMAND_TYPE_USE_INTENT, &target).await?;
    decode_use_result(&reply).map_err(|error| format!("{error:?}").into())
}

async fn verify_pattern_door(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let exclude = format!("!{name}");
    let bundle = owner.bundle_map().ok_or("House door bundle")?;
    let digest = bundle.overlay.base().digest();
    let cases: &[(u64, u64, &[&str], UseDisposition)] = &[
        (23, 2, &["*", exclude.as_str()], UseDisposition::Committed),
        (24, 3, &[exclude.as_str(), "*"], UseDisposition::Locked),
        (25, 3, &["No Matching Name"], UseDisposition::Committed),
    ];
    for &(revision, door_revision, rules, disposition) in cases {
        let mut position = Vec::new();
        scalar(&mut position, 1, 6);
        scalar(&mut position, 2, 0);
        scalar(&mut position, 3, 14);
        let mut door = Vec::new();
        bytes(&mut door, 1, &position);
        scalar(&mut door, 2, revision);
        bytes(&mut door, 3, &id(41));
        let mut selectors = Vec::new();
        scalar(&mut selectors, 2, 1);
        for rule in rules {
            bytes(&mut selectors, 1, rule.as_bytes());
        }
        bytes(&mut door, 4, &selectors);
        assert_eq!(
            send(owner, admitted, 56, &intent(5, &door)).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Ok, revision + 1)
        );

        assert_eq!(
            read_acl_rules(
                owner,
                admitted,
                wire::HouseAclSelector::Door {
                    x: 6,
                    y: 0,
                    floor: 7
                }
            )
            .await?,
            (
                revision + 1,
                rules.iter().map(|rule| (*rule).to_owned()).collect(),
                true
            )
        );
        // Use the admitted test character as a visitor, so ownership does not
        // bypass the door rules. Both fixture owners have the same account.
        set_fixture_owner(h, id(91)).await?;
        assert_eq!(
            use_house_door(owner, admitted, digest, door_revision).await?,
            disposition
        );
        set_fixture_owner(h, id(41)).await?;
    }
    Ok(())
}

async fn set_fixture_owner(h: &Harness, actor: [u8; 16]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for statement in [
        "UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid WHERE house_key=$2",
        "UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid WHERE house_key=$2",
    ] {
        sqlx::query(statement)
            .bind(actor.as_slice())
            .bind(HOUSE)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn textual_acl(kind: u64, revision: u64, lines: &[&str]) -> Vec<u8> {
    let mut replacement = Vec::new();
    scalar(&mut replacement, 1, kind);
    scalar(&mut replacement, 2, revision);
    scalar(&mut replacement, 4, 1);
    for line in lines {
        bytes(&mut replacement, 3, line.as_bytes());
    }
    intent(4, &replacement)
}

async fn guild_house_role(h: &Harness, actor: [u8; 16]) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT (SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid))",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(actor.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn guild_house_lists(h: &Harness, actor: [u8; 16]) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT list_key FROM game_house_acl_member_lists(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) ORDER BY list_key")
        .bind(id(42).as_slice()).bind(HOUSE).bind(actor.as_slice()).fetch_all(&h.pool).await?)
}

async fn verify_guild_acl_write_fence(h: &Harness, admitted: &AdmittedSession) -> TestResult {
    let writable: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('oteryn_game_runtime','game_house_acl_guild_ranks','INSERT,UPDATE,DELETE')",
    ).fetch_one(&h.pool).await?;
    assert!(!writable);
    let fence = admitted.item_fence.ok_or("house ACL item fence")?;
    let actor = id(41);
    let current = fence.connection_generation.get();
    for (generation, revision, expected) in [
        (
            current + 1,
            10_i64,
            "House guild ACL needs its original current session",
        ),
        (current, 9, "House ACL lacks current owner receipt"),
    ] {
        let mut tx = h.pool.begin().await?;
        let session = admitted.game_session_id.as_bytes().as_slice();
        sqlx::query("INSERT INTO game_house_management_receipts(game_session_id,command_id,character_id,world_id,house_key,request_binding,result_revision,action_kind,list_key) VALUES(encode($1,'hex')::uuid,10000,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,decode(repeat('01',32),'hex'),$5,1,'guest')")
            .bind(session).bind(actor.as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(revision)
            .execute(&mut *tx).await?;
        let error = sqlx::query("SELECT game_house_management_guild_acl_advance(encode($1,'hex')::uuid,10000,$2::text::numeric,'[]'::jsonb)")
            .bind(session).bind(generation.to_string()).execute(&mut *tx).await
            .expect_err("unfenced House guild ACL");
        let error = error
            .as_database_error()
            .ok_or("expected House ACL database refusal")?;
        assert_eq!(error.code().as_deref(), Some("23514"));
        assert_eq!(error.message(), expected);
        tx.rollback().await?;
    }
    Ok(())
}
