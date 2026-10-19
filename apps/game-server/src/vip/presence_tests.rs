//! Pure state fixtures are synthetic data, not native current-authority proofs.
//! PostgreSQL cases below use the unchanged real native Root/recovery/session/node fixtures.
#![allow(clippy::expect_used, reason = "Explicit synthetic pure-state fixtures")]
use super::*;
// Every root that path-loads durability compiles this module; each mounts its own copy of the
// shared native harness, so no root needs a crate-level harness registration.
#[allow(
    dead_code,
    reason = "shared PostgreSQL support; this mount uses a subset"
)]
#[allow(
    clippy::duplicate_mod,
    reason = "roots that also mount the harness keep their own copy"
)]
#[path = "../../tests/support/bestiary_postgres_harness.rs"]
mod native;
use native::{CHARACTER, Harness, TestResult, WORLD, debug, id, item_fence};
use sqlx::Executor;

fn character(seed: u8) -> CharacterId {
    CharacterId::from_bytes(id(seed)).expect("native CharacterId")
}
fn world() -> WorldId {
    WorldId::decode(&id(WORLD)).expect("native WorldId")
}
fn data(visible: Visibility, online: bool) -> EntryRead {
    EntryRead {
        target: character(61),
        entry: 1,
        invitation: Some(1),
        visibility: visible,
        online,
        notify: true,
    }
}
fn read(entry: EntryRead) -> WatcherRead {
    WatcherRead {
        fence: item_fence().expect("native IDs only"),
        account: AccountId::from_bytes(id(40)).expect("account"),
        world: world(),
        list_revision: 1,
        consent_revision: 0,
        complete: true,
        requested: BTreeSet::new(),
        entries: vec![entry],
    }
}
fn state() -> State {
    let mut s = State::new(world(), 2).expect("bounded state");
    s.listener = true;
    s
}
fn install(s: &mut State, read: WatcherRead, at: u64, reason: Refresh) {
    let session = read.fence.game_session_id;
    let (t, e, _) = s.reserve(read.fence, 0, at).expect("reserve");
    s.finish(session, t, e, read, at, at, reason)
        .expect("finish");
}
fn acknowledge(s: &mut State, at: u64) -> StatusUpdate {
    let session = item_fence().expect("fence").game_session_id;
    let update = s.next(session, at).expect("next").expect("candidate");
    s.delivered(&update, at).expect("data ack");
    update
}

/// VIP0 §8.2, the production hint path: a relay hint recorded on the owner (no explicit
/// `Refresh::PresenceHint`) makes the watcher's next delivery read announce the transition.
#[test]
fn a_recorded_hint_announces_on_the_next_delivery_read() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    assert!(!acknowledge(&mut s, 100).announce);
    s.hint(character(61), false).expect("hint");
    let session = item_fence().expect("fence").game_session_id;
    assert!(s.sessions[&session].dirty);
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        200,
        Refresh::Delivery,
    );
    let out = acknowledge(&mut s, 200);
    assert_eq!((out.status, out.announce), (Status::Offline, true));
    assert!(s.sessions[&session].hints.is_empty());
    // A hint whose advisory value the read does not confirm is consumed without a notice.
    s.hint(character(61), false).expect("stale hint");
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        300,
        Refresh::Delivery,
    );
    let out = acknowledge(&mut s, 300);
    assert_eq!((out.status, out.announce), (Status::Online, false));
}
/// A hint that arrives while a read is in flight is not consumed by it: the watcher stays dirty
/// and the next read announces.
#[test]
fn a_hint_during_a_read_survives_it_and_announces_on_the_next() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    let f = item_fence().expect("fence");
    let (t, e, _) = s.reserve(f, 0, 200).expect("reserve");
    s.hint(character(61), false).expect("hint");
    s.finish(
        f.game_session_id,
        t,
        e,
        read(data(Visibility::Shown, true)),
        200,
        200,
        Refresh::Delivery,
    )
    .expect("finish");
    assert!(s.sessions[&f.game_session_id].dirty);
    assert_eq!(s.sessions[&f.game_session_id].hints.len(), 1);
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        300,
        Refresh::Delivery,
    );
    let out = acknowledge(&mut s, 300);
    assert_eq!((out.status, out.announce), (Status::Offline, true));
}
#[test]
fn initial_statuses_use_explicit_consent_visibility_and_coarse_truth() {
    for (visibility, online, expected) in [
        (Visibility::Unconfirmed, true, Status::Unconfirmed),
        (Visibility::Hidden, true, Status::Offline),
        (Visibility::Shown, false, Status::Offline),
        (Visibility::Shown, true, Status::Online),
    ] {
        let mut s = state();
        install(
            &mut s,
            read(data(visibility, online)),
            100,
            Refresh::Reconcile,
        );
        let out = acknowledge(&mut s, 100);
        assert_eq!(out.status, expected);
        assert!(!out.announce);
    }
}
#[test]
fn online_expires_at_ten_seconds_and_late_read_cannot_renew_it() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    s.expire(10_099).expect("before expiry");
    assert!(
        s.sessions[&item_fence().expect("fence").game_session_id]
            .pending
            .is_empty()
    );
    s.expire(10_100).expect("expiry");
    assert_eq!(acknowledge(&mut s, 10_100).status, Status::Unconfirmed);
    let f = item_fence().expect("fence");
    let (t, e, _) = s.reserve(f, 0, 11_000).expect("reserve");
    assert!(matches!(
        s.finish(
            f.game_session_id,
            t,
            e,
            read(data(Visibility::Shown, true)),
            11_000,
            21_001,
            Refresh::Reconcile,
        ),
        Err(PresenceError::StaleWork)
    ));
    assert_ne!(
        s.sessions[&f.game_session_id].entries[&1].desired,
        Status::Online
    );
}
/// Codex P1 on #2164: the write of an ONLINE update is bounded by its confirmation deadline and
/// refused once the listener is down; other statuses carry no bound.
#[test]
fn an_online_write_is_bounded_by_its_confirmation_deadline() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    let session = item_fence().expect("fence").game_session_id;
    let online = s.next(session, 100).expect("next").expect("candidate");
    assert_eq!(online.status, Status::Online);
    assert_eq!(
        s.write_bound(&online, 3_100),
        Some(std::time::Duration::from_millis(7_000))
    );
    assert_eq!(
        s.write_bound(&online, 10_100),
        Some(std::time::Duration::ZERO)
    );
    assert_eq!(
        s.write_bound(&online, 20_000),
        Some(std::time::Duration::ZERO)
    );
    s.listener_down(101).expect("outage");
    assert_eq!(s.write_bound(&online, 102), Some(std::time::Duration::ZERO));
    let withdrawal = acknowledge(&mut s, 102);
    assert_eq!(withdrawal.status, Status::Unconfirmed);
    assert_eq!(s.write_bound(&withdrawal, 102), None);
}
#[test]
fn timely_privacy_read_invalidates_inflight_online_before_deadline() {
    for visibility in [Visibility::Hidden, Visibility::Unconfirmed] {
        let mut s = state();
        install(
            &mut s,
            read(data(Visibility::Shown, true)),
            100,
            Refresh::Reconcile,
        );
        let session = item_fence().expect("fence").game_session_id;
        let old = s
            .next(session, 101)
            .expect("next")
            .expect("online candidate");
        install(&mut s, read(data(visibility, true)), 102, Refresh::Delivery);
        assert!(matches!(
            s.delivered(&old, 103),
            Err(PresenceError::StaleWork)
        ));
        assert_ne!(acknowledge(&mut s, 104).status, Status::Online);
    }
}
#[test]
fn notify_revocation_clears_pending_logout_notice_without_changing_status() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        101,
        Refresh::PresenceHint {
            target: character(61),
            online: false,
        },
    );
    let session = item_fence().expect("fence").game_session_id;
    assert!(s.sessions[&session].pending[&1].announce);
    let mut latest = data(Visibility::Shown, false);
    latest.notify = false;
    install(&mut s, read(latest), 102, Refresh::Delivery);
    let out = acknowledge(&mut s, 103);
    assert_eq!(out.status, Status::Offline);
    assert!(!out.announce);
}
#[test]
fn changed_invitation_invalidates_inflight_even_when_status_remains_online() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    let session = item_fence().expect("fence").game_session_id;
    let old = s.next(session, 101).expect("next").expect("candidate");
    let mut replacement = data(Visibility::Shown, true);
    replacement.invitation = Some(2);
    install(&mut s, read(replacement), 102, Refresh::Delivery);
    assert!(matches!(
        s.delivered(&old, 103),
        Err(PresenceError::StaleWork)
    ));
    assert!(!acknowledge(&mut s, 104).announce);
}
#[test]
fn hiding_unhiding_consent_and_reconcile_never_create_false_login() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    install(
        &mut s,
        read(data(Visibility::Hidden, true)),
        101,
        Refresh::PresenceHint {
            target: character(61),
            online: true,
        },
    );
    assert!(!acknowledge(&mut s, 101).announce);
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        102,
        Refresh::PresenceHint {
            target: character(61),
            online: true,
        },
    );
    assert!(!acknowledge(&mut s, 102).announce);
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        103,
        Refresh::Reconcile,
    );
    assert!(!acknowledge(&mut s, 103).announce);
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        104,
        Refresh::PresenceHint {
            target: character(61),
            online: true,
        },
    );
    assert!(acknowledge(&mut s, 104).announce);
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        105,
        Refresh::PresenceHint {
            target: character(61),
            online: false,
        },
    );
    assert!(
        s.next(item_fence().expect("fence").game_session_id, 105)
            .expect("no duplicate")
            .is_none()
    );
}
#[test]
fn refresh_generation_and_listener_epoch_reject_delayed_results() {
    let mut s = state();
    let f = item_fence().expect("fence");
    let (old, epoch, _) = s.reserve(f, 0, 100).expect("old");
    let (new, _, _) = s.reserve(f, 0, 101).expect("new");
    s.finish(
        f.game_session_id,
        new,
        epoch,
        read(data(Visibility::Shown, false)),
        101,
        102,
        Refresh::Reconcile,
    )
    .expect("new result");
    assert!(matches!(
        s.finish(
            f.game_session_id,
            old,
            epoch,
            read(data(Visibility::Shown, true)),
            100,
            103,
            Refresh::Reconcile
        ),
        Err(PresenceError::StaleWork)
    ));
    let (ticket, e, _) = s.reserve(f, 0, 104).expect("before outage");
    s.listener_down(105).expect("outage");
    assert!(matches!(
        s.finish(
            f.game_session_id,
            ticket,
            e,
            read(data(Visibility::Shown, true)),
            104,
            106,
            Refresh::Reconcile
        ),
        Err(PresenceError::StaleWork)
    ));
}
#[test]
fn listener_outage_demotes_online_and_drops_other_pending_changes() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    s.listener_down(101).expect("outage");
    let out = acknowledge(&mut s, 102);
    assert_eq!(out.status, Status::Unconfirmed);
    assert!(!out.announce);
    assert!(s.reserve(item_fence().expect("fence"), 0, 103).is_err());
    s.listener_reconnected(104).expect("reconnect");
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        104,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 104);
    s.listener_down(105).expect("second outage");
    let f = item_fence().expect("fence");
    let old = s
        .withdrawal(f.game_session_id, 100, 105)
        .expect("data")
        .expect("withdrawal");
    let new = s
        .withdrawal(f.game_session_id, 200, 106)
        .expect("floorjump")
        .expect("reissued");
    assert!(new.revision > 200);
    assert!(matches!(
        s.delivered(&old, 106),
        Err(PresenceError::StaleWork)
    ));
    s.delivered(&new, 106).expect("new data");
    s.listener_down(107).expect("epoch");
    s.listener_reconnected(108).expect("reconnect");
    install(
        &mut s,
        read(data(Visibility::Unconfirmed, true)),
        108,
        Refresh::Reconcile,
    );
    // No obsolete epoch is retained when status stays UNCONFIRMED across reconnect.
    assert!(s.next(f.game_session_id, 108).expect("next").is_none());
}
#[test]
fn revision_floor_reconnect_prevents_old_pending_delta_reuse() {
    let mut s = state();
    let f = item_fence().expect("fence");
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    let (t, e, _) = s.reserve(f, 100, 101).expect("reconnect floor");
    s.finish(
        f.game_session_id,
        t,
        e,
        read(data(Visibility::Shown, true)),
        101,
        101,
        Refresh::Delivery,
    )
    .expect("fresh read");
    let new = s
        .next(f.game_session_id, 102)
        .expect("next")
        .expect("new candidate");
    assert!(new.revision > 100);
    let (t, e, _) = s.reserve(f, 200, 103).expect("inflight floor jump");
    s.finish(
        f.game_session_id,
        t,
        e,
        read(data(Visibility::Shown, true)),
        103,
        103,
        Refresh::Delivery,
    )
    .expect("fresh");
    assert!(matches!(
        s.delivered(&new, 104),
        Err(PresenceError::StaleWork)
    ));
    assert!(acknowledge(&mut s, 104).revision > 200);
}
#[test]
fn pending_updates_dequeue_in_revision_order_not_entry_order() {
    let mut s = state();
    let pair = |first_online: bool| {
        let mut r = read(data(Visibility::Shown, first_online));
        let mut second = data(Visibility::Shown, true);
        second.entry = 2;
        second.target = character(62);
        r.entries.push(second);
        r
    };
    install(&mut s, pair(true), 100, Refresh::Reconcile);
    // Entry 1 is re-coalesced with a newer revision; entry 2 keeps its older one.
    install(&mut s, pair(false), 101, Refresh::Reconcile);
    let first = acknowledge(&mut s, 102);
    let second = acknowledge(&mut s, 102);
    assert_eq!((first.entry, first.status), (2, Status::Online));
    assert_eq!((second.entry, second.status), (1, Status::Offline));
    assert!(first.revision < second.revision);
    let session = item_fence().expect("fence").game_session_id;
    assert!(s.next(session, 103).expect("next").is_none());
}
#[test]
fn state_capacity_queue_coalescing_and_index_replacement_are_bounded() {
    let mut s = state();
    let mut snapshot = read(data(Visibility::Shown, true));
    snapshot.entries = (1..=100)
        .map(|n| {
            let mut e = data(Visibility::Shown, true);
            e.entry = n;
            e.target = character(u8::try_from(n).expect("id"));
            e
        })
        .collect();
    install(&mut s, snapshot, 100, Refresh::Reconcile);
    let session = item_fence().expect("fence").game_session_id;
    assert_eq!(s.sessions[&session].pending.len(), 100);
    assert_eq!(s.index.len(), 100);
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        101,
        Refresh::Reconcile,
    );
    assert_eq!(s.index.len(), 1);
    assert_eq!(s.sessions[&session].entries.len(), 1);
    assert!(s.sessions[&session].pending.len() <= 1);
    let mut f = item_fence().expect("fence");
    f.game_session_id = GameSessionId::decode(&id(99)).expect("session");
    s.reserve(f, 0, 102).expect("second slot");
    f.game_session_id = GameSessionId::decode(&id(98)).expect("session");
    assert!(matches!(s.reserve(f, 0, 103), Err(PresenceError::Capacity)));
}
#[test]
fn malformed_read_or_revision_overflow_cannot_partially_replace_index() {
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    let f = item_fence().expect("fence");
    let (t, e, _) = s.reserve(f, 0, 101).expect("reserve");
    let mut invalid = read(data(Visibility::Hidden, true));
    invalid.entries.push(invalid.entries[0].clone());
    assert!(
        s.finish(
            f.game_session_id,
            t,
            e,
            invalid,
            101,
            102,
            Refresh::Reconcile
        )
        .is_err()
    );
    assert_eq!(
        s.sessions[&f.game_session_id].entries[&1].desired,
        Status::Online
    );
    let (t, e, _) = s.reserve(f, u64::MAX, 103).expect("floor");
    assert!(
        s.finish(
            f.game_session_id,
            t,
            e,
            read(data(Visibility::Hidden, true)),
            103,
            104,
            Refresh::Reconcile
        )
        .is_err()
    );
    assert_eq!(
        s.sessions[&f.game_session_id].entries[&1].desired,
        Status::Online
    );
    assert!(s.index[&character(61)].contains(&f.game_session_id));
}

// The VIP tables are the installed migration 0130; fixture rows are administrative DATA, not a
// production invitation/acceptance producer or a current-authority substitute.
async fn fixture_character(h: &Harness, seed: u8, owner: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(owner).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(owner).as_slice()).bind(format!("Vip Presence {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26))).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    let distinct = |tag| {
        let mut bytes = id(seed);
        bytes[15] ^= tag;
        bytes
    };
    let operation = distinct(0x80);
    let decision = distinct(0x81);
    let uuid = |bytes: [u8; 16]| {
        let s = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{}-{}",
            &s[..8],
            &s[8..12],
            &s[12..16],
            &s[16..20],
            &s[20..]
        )
    };
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),"source_revision":seed.to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":uuid(id(owner)),"target_world_id":uuid(id(WORLD)),"requested_name":format!("Vip Presence {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26)),"interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},"issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(owner).as_slice()).bind(id(seed).as_slice()).bind(id(WORLD).as_slice()).bind(distinct(0x82).as_slice()).bind(distinct(0x83).as_slice()).bind(decision.as_slice()).bind(i64::from(seed)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding").execute(&h.pool).await?;
    Ok(())
}
async fn fixture(h: &Harness) -> TestResult {
    fixture_character(h, 61, 60).await?;
    fixture_character(h, 62, 70).await?;
    fixture_character(h, 63, 40).await?;
    sqlx::query(
        "INSERT INTO game_vip_lists VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,3,20,0)",
    )
    .bind(id(40).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(&h.pool)
    .await?;
    for (entry, target) in [(1_i64, 61_u8), (2, 62)] {
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no,notify) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,TRUE)").bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(target).as_slice()).bind(entry).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT DO NOTHING").bind(id(target).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,clock_timestamp()-interval '1 day')").bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(CHARACTER).as_slice()).bind(id(target).as_slice()).execute(&h.pool).await?;
    }
    // Target state is explicit DB fixture data; all watcher authority still passes the actual
    // native Root/session/lease/node/scope checks, never a fabricated observation capability.
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($4,'hex')::uuid")
        .bind(id(91).as_slice()).bind(id(60).as_slice()).bind(id(61).as_slice()).bind(id(native::SESSION).as_slice()).execute(&h.pool).await?;
    Ok(())
}
/// The registered PostgreSQL target that runs the native cases (`pr-postgres-harness`).
const NATIVE_LANE: &str = "durability_postgres";
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    // The native cases run once, in the protected `durability_postgres` lane; the other roots
    // that path-load durability compile them but do not run them a second time.
    if env!("CARGO_CRATE_NAME") != NATIVE_LANE {
        return Ok(());
    }
    let Some(admin) = native::configured_admin() else {
        return Ok(());
    };
    let runtime = native::runtime()?;
    let h = runtime.block_on(Harness::create(admin, tag, false))?;
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(body(&h))));
    runtime.block_on(h.cleanup())?;
    match result {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
fn native_joined_reader_and_owner_follow_actual_consent_hide_session_and_inviter() -> TestResult {
    run("vip2reader", async |h| {
        fixture(h).await?;
        let types: (String, String) = sqlx::query_as("SELECT pg_typeof(COALESCE(NULL::smallint,1))::text,pg_typeof(COALESCE(NULL::smallint,1::smallint))::text").fetch_one(&h.pool).await?;
        assert_eq!(types, ("integer".to_owned(), "smallint".to_owned()));
        println!("real PG typed default probe: {types:?}");
        // Missing admitted target-settings row must use native SHOWN default without
        // changing the SMALLINT decoder or silently treating missing schema as default.
        sqlx::query(
            "DELETE FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(62).as_slice())
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let read = h
            .root
            .read_current_vip_presence(&authority, &h.node, item_fence()?, Selection::Full)
            .await
            .map_err(debug)?;
        assert_eq!(read.entries[0].status(), Status::Online);
        assert_eq!(read.entries[1].status(), Status::Offline);
        assert_eq!(read.account.as_bytes(), &id(40));
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        owner
            .refresh_current(&authority, &h.node, item_fence()?, 10, Refresh::Reconcile)
            .await
            .map_err(debug)?;
        let first = owner
            .next_current_update(&authority, &h.node, item_fence()?, 10)
            .await
            .map_err(debug)?
            .ok_or("missing first")?;
        assert_eq!(first.status(), Status::Online);
        assert!(first.revision() > 10);
        owner.delivered(&first).map_err(debug)?;
        let second = owner
            .next_current_update(&authority, &h.node, item_fence()?, 10)
            .await
            .map_err(debug)?
            .ok_or("missing second")?;
        owner.delivered(&second).map_err(debug)?;
        sqlx::query("UPDATE game_character_social_settings SET vip_status=2 WHERE character_id=encode($1,'hex')::uuid").bind(id(61).as_slice()).execute(&h.pool).await?;
        let hidden = owner
            .next_current_update(&authority, &h.node, item_fence()?, 10)
            .await
            .map_err(debug)?
            .ok_or("hidden delta")?;
        assert_eq!(hidden.status(), Status::Offline);
        assert!(!hidden.announce());
        let withdrawal = owner
            .next_withdrawal_candidate(item_fence()?.game_session_id, 100)
            .map_err(debug)?
            .ok_or("last-sent ONLINE withdrawal survives inflight OFFLINE")?;
        assert_eq!(withdrawal.status(), Status::Unconfirmed);
        assert!(!withdrawal.announce());
        assert!(matches!(
            owner.delivered(&hidden),
            Err(PresenceError::StaleWork)
        ));
        owner.delivered(&withdrawal).map_err(debug)?;
        let hidden = owner
            .next_current_update(&authority, &h.node, item_fence()?, 100)
            .await
            .map_err(debug)?
            .ok_or("fresh hidden")?;
        assert_eq!(hidden.status(), Status::Offline);
        owner.delivered(&hidden).map_err(debug)?;
        sqlx::query("UPDATE game_character_social_settings SET vip_status=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(61).as_slice()).execute(&h.pool).await?;
        let shown = owner
            .next_current_update(&authority, &h.node, item_fence()?, 10)
            .await
            .map_err(debug)?
            .ok_or("shown delta")?;
        assert_eq!(shown.status(), Status::Online);
        assert!(!shown.announce());
        owner.delivered(&shown).map_err(debug)?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(91).as_slice()).execute(&h.pool).await?;
        assert!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 10)
                .await
                .map_err(debug)?
                .is_none()
        );
        sqlx::query("UPDATE game_vip_consents SET inviter_character_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid").bind(id(63).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
        let lost = owner
            .next_current_update(&authority, &h.node, item_fence()?, 10)
            .await
            .map_err(debug)?
            .ok_or("inviter isolation")?;
        assert_eq!(lost.status(), Status::Unconfirmed);
        owner.delivered(&lost).map_err(debug)?;
        assert_eq!(h.count("game_character_roots").await?, 4);
        Ok(())
    })
}
/// VIP-2 (VIP0 §8.2) on the native rows: A (session 50, account 40) watches B (character 61).
/// B logs in (its session row enters state 1) and the relay's online hint makes A's owner
/// re-read and announce ONLINE; B logs out (TERMINAL) and the offline hint announces OFFLINE.
/// A hint for a character A does not watch changes nothing.
#[test]
fn native_presence_hint_announces_a_login_and_a_logout_to_the_watcher() -> TestResult {
    run("vip2hint", async |h| {
        fixture(h).await?;
        let set_state = async |state: i16| -> TestResult {
            sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=$1 WHERE game_session_id=encode($2,'hex')::uuid")
                .bind(state).bind(id(91).as_slice()).execute(&h.pool).await?;
            Ok(())
        };
        set_state(3).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        owner
            .refresh_current(&authority, &h.node, item_fence()?, 0, Refresh::Reconcile)
            .await
            .map_err(debug)?;
        for _ in 0..2 {
            let initial = owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await
                .map_err(debug)?
                .ok_or("initial status")?;
            assert_eq!(initial.status(), Status::Offline);
            assert!(!initial.announce());
            owner.delivered(&initial).map_err(debug)?;
        }
        let target = CharacterId::from_bytes(id(61)).map_err(debug)?;
        set_state(1).await?;
        owner
            .refresh_current(
                &authority,
                &h.node,
                item_fence()?,
                0,
                Refresh::PresenceHint {
                    target,
                    online: true,
                },
            )
            .await
            .map_err(debug)?;
        let login = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("login delta")?;
        assert_eq!((login.status(), login.announce()), (Status::Online, true));
        owner.delivered(&login).map_err(debug)?;
        set_state(3).await?;
        owner
            .refresh_current(
                &authority,
                &h.node,
                item_fence()?,
                0,
                Refresh::PresenceHint {
                    target,
                    online: false,
                },
            )
            .await
            .map_err(debug)?;
        let logout = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("logout delta")?;
        assert_eq!(
            (logout.status(), logout.announce()),
            (Status::Offline, true)
        );
        owner.delivered(&logout).map_err(debug)?;
        // An unwatched character's hint is dropped: nothing to send.
        owner
            .refresh_current(
                &authority,
                &h.node,
                item_fence()?,
                0,
                Refresh::PresenceHint {
                    target: CharacterId::from_bytes(id(63)).map_err(debug)?,
                    online: true,
                },
            )
            .await
            .map_err(debug)?;
        assert!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await
                .map_err(debug)?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn native_reader_rejects_independently_wrong_fences_and_uninstalled_schema() -> TestResult {
    run("vip2fences", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Without the VIP relation the read fails closed (a schema that is not installed).
        h.pool
            .execute("ALTER TABLE game_vip_consents RENAME TO game_vip_consents_absent")
            .await?;
        assert!(matches!(
            h.root
                .read_current_vip_presence(&authority, &h.node, item_fence()?, Selection::Full)
                .await,
            Err(PresenceError::Unavailable(_))
        ));
        h.pool
            .execute("ALTER TABLE game_vip_consents_absent RENAME TO game_vip_consents")
            .await?;
        for which in 0..5 {
            let mut f = item_fence()?;
            match which {
                0 => f.character_lease_generation += 1,
                1 => {
                    f.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                2 => {
                    f.scope_ownership_generation =
                        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
                3 => {
                    f.runtime_scope = RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(99)).map_err(debug)?,
                        crate::foundation::ChannelId::decode(&id(native::CHANNEL))
                            .map_err(debug)?,
                    )
                }
                // House seam (#2165): an Instance-scope (House) watcher fails closed.
                _ => {
                    f.runtime_scope = RuntimeScopeRefV1::instance(
                        WorldId::decode(&id(WORLD)).map_err(debug)?,
                        id(97),
                    )
                    .map_err(debug)?
                }
            }
            assert!(matches!(
                h.root
                    .read_current_vip_presence(&authority, &h.node, f, Selection::Full)
                    .await,
                Err(PresenceError::AuthorityRejected)
            ));
        }
        let bad_node = NodeIncarnationProof::new(
            h.node.fact(),
            super::super::runtime_scope_assignment::BootstrapSecret::from_bytes([77; 32]),
        );
        assert!(matches!(
            h.root
                .read_current_vip_presence(&authority, &bad_node, item_fence()?, Selection::Full)
                .await,
            Err(PresenceError::AuthorityRejected)
        ));
        assert!(
            h.root
                .read_current_vip_presence(&authority, &h.node, item_fence()?, Selection::Full)
                .await
                .map_err(debug)?
                .entries
                .is_empty()
        );
        let owner = PresenceOwner::new(h.root.clone(), world(), 1).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let mut absent = item_fence()?;
        absent.game_session_id = GameSessionId::decode(&id(98)).map_err(debug)?;
        assert!(matches!(
            owner
                .refresh_current(&authority, &h.node, absent, 0, Refresh::Reconcile)
                .await,
            Err(PresenceError::AuthorityRejected)
        ));
        // A never-qualified failed reservation cannot consume the only live session slot.
        assert!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await
                .map_err(debug)?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn timer_reconcile_visibility_budget_and_work_suppression_are_exact() {
    let mut s = state();
    let f = item_fence().expect("fence");
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    assert!(s.due(60_099, 1).expect("before60s").is_empty());
    assert!(s.due(60_100, 0).expect("budget0").is_empty());
    assert_eq!(s.due(60_100, 1).expect("at60s"), vec![f.game_session_id]);
    let (old_ticket, old_epoch, _) = s.reserve(f, 0, 60_100).expect("inflight read");
    assert!(s.due(60_101, 1).expect("reading").is_empty());
    // Dropped async work must not suppress reconcile forever. Reject its delayed result
    // both before another reserve and after a fresh reservation has superseded it.
    assert_eq!(
        s.due(70_100, 1).expect("abandoned work"),
        vec![f.game_session_id]
    );
    assert!(matches!(
        s.finish(
            f.game_session_id,
            old_ticket,
            old_epoch,
            read(data(Visibility::Shown, false)),
            60_100,
            70_100,
            Refresh::Reconcile
        ),
        Err(PresenceError::StaleWork)
    ));
    let (new_ticket, new_epoch, _) = s.reserve(f, 0, 70_101).expect("fresh work");
    assert!(matches!(
        s.finish(
            f.game_session_id,
            old_ticket,
            old_epoch,
            read(data(Visibility::Shown, false)),
            60_100,
            70_101,
            Refresh::Reconcile
        ),
        Err(PresenceError::StaleWork)
    ));
    s.finish(
        f.game_session_id,
        new_ticket,
        new_epoch,
        read(data(Visibility::Shown, false)),
        70_101,
        70_101,
        Refresh::Reconcile,
    )
    .expect("fresh completion");
    let mut unqualified = State::new(world(), 1).expect("one slot");
    unqualified.listener = true;
    let mut absent = f;
    absent.game_session_id = GameSessionId::decode(&id(98)).expect("native data");
    unqualified
        .reserve(absent, 0, 100)
        .expect("unqualified work");
    unqualified
        .due(10_100, 1)
        .expect("abandoned unqualified work");
    assert!(unqualified.sessions.is_empty());
    unqualified
        .reserve(f, 0, 10_100)
        .expect("real slot remains available");
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    assert!(s.due(10_099, 1).expect("before10s").is_empty());
    assert_eq!(s.due(10_100, 1).expect("at10s"), vec![f.game_session_id]);
    assert_eq!(acknowledge(&mut s, 10_100).status, Status::Unconfirmed);
    // Public update's debug representation contains none of its private identities/fences.
    let mut s = state();
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    assert_eq!(
        format!("{:?}", acknowledge(&mut s, 100)),
        "StatusUpdate { .. }"
    );
}

#[test]
fn reconnect_reconciles_at_once_after_a_failed_read_backoff() {
    let mut s = state();
    let f = item_fence().expect("fence");
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    assert_eq!(acknowledge(&mut s, 100).status, Status::Online);
    // A failed read backs the watcher off for CONFIRM_MS and queues its withdrawal.
    let (ticket, epoch, _) = s.reserve(f, 0, 200).expect("read");
    s.failed(f.game_session_id, ticket, epoch, 200)
        .expect("failed read");
    assert!(s.due(201, 1).expect("backoff").is_empty());
    // The reconnect clears that withdrawal; the watcher must be due at once, not at 10_200.
    s.listener_down(300).expect("outage");
    s.listener_reconnected(301).expect("reconnect");
    assert_eq!(s.due(301, 1).expect("reconcile"), vec![f.game_session_id]);
}

#[test]
fn native_public_owner_drains_only_fenced_withdrawals_during_listener_and_vip_query_failure()
-> TestResult {
    run("vip2outage", async |h| {
        fixture(h).await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        assert!(version.starts_with("17.6"));
        println!("native VIP owner PostgreSQL {version}");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let first = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("ONLINE")?;
        assert_eq!(first.status(), Status::Online);
        owner.delivered(&first).map_err(debug)?;
        // Repeated native delivery read cannot duplicate still-inflight candidates.
        let second = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("OFFLINE")?;
        assert!(matches!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await,
            Err(PresenceError::Busy)
        ));
        owner.delivered(&second).map_err(debug)?;
        owner.listener_down().map_err(debug)?;
        let withdrawn = owner
            .next_current_update(&authority, &h.node, item_fence()?, 100)
            .await
            .map_err(debug)?
            .ok_or("outage withdrawal")?;
        assert_eq!(withdrawn.status(), Status::Unconfirmed);
        assert!(withdrawn.revision() > 100);
        assert!(!withdrawn.announce());
        owner.delivered(&withdrawn).map_err(debug)?;
        assert!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await
                .map_err(debug)?
                .is_none()
        );
        owner.listener_reconnected().map_err(debug)?;
        let restored = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("read restores")?;
        assert_eq!(restored.status(), Status::Online);
        assert!(!restored.announce());
        owner.delivered(&restored).map_err(debug)?;
        owner.listener_down().map_err(debug)?;
        let data = owner
            .next_withdrawal_candidate(item_fence()?.game_session_id, 200)
            .map_err(debug)?
            .ok_or("DB independent withdrawal data")?;
        assert_eq!(data.status(), Status::Unconfirmed);
        assert!(data.revision() > 200);
        assert!(!data.announce());
        owner.delivered(&data).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let restored = owner
            .next_current_update(&authority, &h.node, item_fence()?, 200)
            .await
            .map_err(debug)?
            .ok_or("fresh after data")?;
        assert_eq!(restored.status(), Status::Online);
        owner.delivered(&restored).map_err(debug)?;
        // The public hint action must query a previously OFFLINE target too, without a
        // separate caller target_hint() call. Fixture target belongs to a different Account.
        sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,2 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($4,'hex')::uuid").bind(id(92).as_slice()).bind(id(70).as_slice()).bind(id(62).as_slice()).bind(id(native::SESSION).as_slice()).execute(&h.pool).await?;
        owner
            .refresh_current(
                &authority,
                &h.node,
                item_fence()?,
                0,
                Refresh::PresenceHint {
                    target: character(62),
                    online: true,
                },
            )
            .await
            .map_err(debug)?;
        let login = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("hinted offline target")?;
        assert_eq!(login.entry(), 2);
        assert_eq!(login.status(), Status::Online);
        assert!(login.announce());
        owner.delivered(&login).map_err(debug)?;
        h.pool.execute("DROP TABLE game_vip_lists CASCADE").await?;
        let withdrawal = owner
            .next_current_update(&authority, &h.node, item_fence()?, 0)
            .await
            .map_err(debug)?
            .ok_or("VIP query failure withdrawal")?;
        assert_eq!(withdrawal.status(), Status::Unconfirmed);
        assert!(!withdrawal.announce());
        owner.delivered(&withdrawal).map_err(debug)?;
        let mut wrong = item_fence()?;
        wrong.character_lease_generation += 1;
        assert!(matches!(
            owner
                .next_current_update(&authority, &h.node, wrong, 0)
                .await,
            Err(PresenceError::AuthorityRejected)
        ));
        // A deliberately unavailable native recipient relation prevents even RecipientOnly.
        // This is a relation-failure fixture, not a claim that a live full DB outage ran.
        h.pool.execute("ALTER TABLE game_durability_reconnect_sessions RENAME TO unavailable_vip2_sessions").await?;
        assert!(matches!(
            owner
                .next_current_update(&authority, &h.node, item_fence()?, 0)
                .await,
            Err(PresenceError::Unavailable(_))
        ));
        let data = owner
            .next_withdrawal_candidate(item_fence()?.game_session_id, 300)
            .map_err(debug)?
            .ok_or("withdrawal without native read")?;
        assert_eq!(data.status(), Status::Unconfirmed);
        assert!(data.revision() > 300);
        assert!(!data.announce());
        h.pool.execute("ALTER TABLE unavailable_vip2_sessions RENAME TO game_durability_reconnect_sessions").await?;
        Ok(())
    })
}

#[test]
fn withdrawal_replaces_offline_work_while_last_sent_online_without_duplicate_unconfirmed() {
    for inflight in [false, true] {
        let mut s = state();
        let f = item_fence().expect("fence");
        install(
            &mut s,
            read(data(Visibility::Shown, true)),
            100,
            Refresh::Reconcile,
        );
        acknowledge(&mut s, 100);
        install(
            &mut s,
            read(data(Visibility::Hidden, true)),
            101,
            Refresh::Delivery,
        );
        let old = inflight.then(|| {
            s.next(f.game_session_id, 101)
                .expect("next")
                .expect("OFFLINE")
        });
        let withdrawal = s
            .withdrawal(f.game_session_id, 100, 102)
            .expect("data")
            .expect("withdrawal");
        assert_eq!(withdrawal.status, Status::Unconfirmed);
        assert!(!withdrawal.announce);
        assert!(withdrawal.revision > 100);
        if let Some(old) = old {
            assert!(matches!(
                s.delivered(&old, 102),
                Err(PresenceError::StaleWork)
            ));
        }
        assert!(matches!(
            s.withdrawal(f.game_session_id, 100, 103),
            Err(PresenceError::Busy)
        ));
        assert!(s.sessions[&f.game_session_id].pending.is_empty());
        s.delivered(&withdrawal, 103).expect("new data ack");
        assert!(
            s.withdrawal(f.game_session_id, 100, 104)
                .expect("no duplicate")
                .is_none()
        );
    }
}

#[test]
fn incremental_index_preserves_shared_targets_and_other_watchers() {
    let mut s = state();
    let first = item_fence().expect("synthetic binding");
    let mut second = first;
    second.game_session_id = GameSessionId::decode(&id(94)).expect("synthetic session");
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    let mut other = read(data(Visibility::Hidden, true));
    other.fence = second;
    install(&mut s, other, 100, Refresh::Reconcile);
    assert_eq!(s.index[&character(61)].len(), 2);
    let mut changed = read(data(Visibility::Shown, false));
    changed.entries[0].target = character(62);
    install(&mut s, changed, 101, Refresh::Reconcile);
    assert_eq!(
        s.index[&character(61)],
        BTreeSet::from([second.game_session_id])
    );
    assert_eq!(
        s.index[&character(62)],
        BTreeSet::from([first.game_session_id])
    );
    s.replace_watcher(first.game_session_id, None);
    assert!(!s.index.contains_key(&character(62)));
    assert_eq!(
        s.index[&character(61)],
        BTreeSet::from([second.game_session_id])
    );
    s.replace_watcher(second.game_session_id, None);
    assert!(s.index.is_empty());
}
#[test]
fn reverse_index_holds_only_accepted_entries_and_drops_unconfirmed_consent() {
    let mut s = state();
    let first = item_fence().expect("synthetic binding");
    let mut second = first;
    second.game_session_id = GameSessionId::decode(&id(94)).expect("synthetic session");
    let mut pending = read(data(Visibility::Unconfirmed, true));
    pending.fence = second;
    install(&mut s, pending, 100, Refresh::Reconcile);
    assert!(
        !s.index.contains_key(&character(61)),
        "pending, rejected or absent consent is never indexed"
    );
    install(
        &mut s,
        read(data(Visibility::Hidden, false)),
        100,
        Refresh::Reconcile,
    );
    assert_eq!(
        s.index[&character(61)],
        BTreeSet::from([first.game_session_id]),
        "accepted-but-hidden consent stays indexed"
    );
    install(
        &mut s,
        read(data(Visibility::Unconfirmed, false)),
        101,
        Refresh::Reconcile,
    );
    assert!(
        s.index.is_empty(),
        "consent that became unconfirmed removes the session from the index"
    );
    assert!(s.sessions.contains_key(&first.game_session_id));
}
#[test]
fn batch_preparation_is_atomic_and_cancelled_tickets_cannot_complete() {
    let mut s = state();
    let first = item_fence().expect("synthetic binding");
    let mut second = first;
    second.game_session_id = GameSessionId::decode(&id(94)).expect("synthetic session");
    let mut other = read(data(Visibility::Shown, true));
    other.fence = second;
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    install(&mut s, other, 100, Refresh::Reconcile);
    let mut tickets = Vec::new();
    for fence in [first, second] {
        let (t, e, _) = s.reserve(fence, 0, 101).expect("reserve");
        tickets.push((fence.game_session_id, t, e));
    }
    let mut bad = read(data(Visibility::Hidden, true));
    bad.fence = second;
    bad.entries.push(bad.entries[0].clone());
    assert!(
        s.finish_batch(
            &tickets,
            vec![read(data(Visibility::Hidden, true)), bad],
            101,
            102
        )
        .is_err()
    );
    assert_eq!(
        s.sessions[&first.game_session_id].entries[&1].desired,
        Status::Online
    );
    assert_eq!(
        s.sessions[&second.game_session_id].entries[&1].desired,
        Status::Online
    );
    assert_eq!(s.index[&character(61)].len(), 2);
    s.abort_batch(&tickets, 103);
    assert!(s.sessions.values().all(|w| !w.reading && w.ticket == 0));
    let mut other = read(data(Visibility::Shown, true));
    other.fence = second;
    assert!(matches!(
        s.finish_batch(
            &tickets,
            vec![read(data(Visibility::Shown, true)), other],
            101,
            104
        ),
        Err(PresenceError::StaleWork)
    ));
    assert!(
        s.sessions
            .values()
            .all(|w| w.entries[&1].desired != Status::Online)
    );
}

// A second independent watcher uses the same historical native fixture format as Harness.
// These session/guard rows are explicit DATA; no predicate or cached fence bypasses native
// recovery/session/lease/node/scope/guard SQL at the reader boundary.
async fn second_watcher(h: &Harness) -> TestResult<CurrentCharacterItemFence> {
    fixture_character(h, 64, 80).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($4,'hex')::uuid")
        .bind(id(93).as_slice()).bind(id(80).as_slice()).bind(id(64).as_slice()).bind(id(native::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'account-current',1,0,'{}')")
        .bind(id(80).as_slice()).bind(id(64).as_slice()).bind(id(93).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')")
        .bind(id(64).as_slice()).bind(id(80).as_slice()).bind(id(WORLD).as_slice()).bind(id(93).as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_vip_lists VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,7,2,20,0)",
    )
    .bind(id(80).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no,notify) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,TRUE)").bind(id(80).as_slice()).bind(id(WORLD).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
    let mut fence = item_fence()?;
    fence.character_id = character(64);
    fence.game_session_id = GameSessionId::decode(&id(93)).map_err(debug)?;
    Ok(fence)
}
#[test]
fn native_batch_resolves_each_inviter_and_revision_with_shared_target_facts() -> TestResult {
    run("vip2batchprivacy", async |h| {
        fixture(h).await?;
        let second = second_watcher(h).await?;
        // Native SMALLINT missing-row default must also work on the bulk path.
        sqlx::query(
            "DELETE FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(61).as_slice())
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let watchers = [
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: item_fence()?,
                last_seen_revision: 0,
            },
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: second,
                last_seen_revision: 0,
            },
        ];
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        {
            let state = owner.state.lock().map_err(debug)?;
            assert_eq!(
                state.sessions[&item_fence()?.game_session_id].entries[&1].desired,
                Status::Online
            );
            assert_eq!(
                state.sessions[&second.game_session_id].entries[&1].desired,
                Status::Unconfirmed
            );
            assert_eq!(state.sessions[&second.game_session_id].list_revision, 7);
            // The second watcher's consent is unconfirmed, so only the first is indexed.
            assert_eq!(
                state.index[&character(61)],
                BTreeSet::from([item_fence()?.game_session_id])
            );
        }
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy,vip_status) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1',2)").bind(id(61).as_slice()).execute(&h.pool).await?;
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        {
            let state = owner.state.lock().map_err(debug)?;
            assert_eq!(
                state.sessions[&item_fence()?.game_session_id].entries[&1].desired,
                Status::Offline
            );
            assert_eq!(
                state.sessions[&second.game_session_id].entries[&1].desired,
                Status::Unconfirmed
            );
            assert!(
                state
                    .sessions
                    .values()
                    .all(|w| w.pending.values().all(|p| !p.announce))
            );
        }
        sqlx::query("UPDATE game_character_social_settings SET vip_status=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(61).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy,vip_consent_revision) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1',3)").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        assert_eq!(
            owner.state.lock().map_err(debug)?.sessions[&item_fence()?.game_session_id]
                .consent_revision,
            3
        );
        // Accepted consent for a different Character of the first Account grants it nothing.
        sqlx::query("UPDATE game_vip_consents SET inviter_character_id=encode($1,'hex')::uuid WHERE account_id=encode($2,'hex')::uuid").bind(id(63).as_slice()).bind(id(40).as_slice()).execute(&h.pool).await?;
        sqlx::query(
            "UPDATE game_vip_lists SET revision=revision+1 WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(40).as_slice())
        .execute(&h.pool)
        .await?;
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        {
            let state = owner.state.lock().map_err(debug)?;
            assert_eq!(
                state.sessions[&item_fence()?.game_session_id].entries[&1].desired,
                Status::Unconfirmed
            );
            assert_eq!(
                state.sessions[&item_fence()?.game_session_id].list_revision,
                2
            );
            assert!(
                state
                    .sessions
                    .values()
                    .all(|w| w.pending.values().all(|p| !p.announce))
            );
        }
        sqlx::query("DELETE FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid")
            .bind(id(80).as_slice())
            .execute(&h.pool)
            .await?;
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        let state = owner.state.lock().map_err(debug)?;
        assert!(state.sessions[&second.game_session_id].entries.is_empty());
        // The first watcher's consent became unconfirmed above, so no session indexes 61.
        assert!(!state.index.contains_key(&character(61)));
        Ok(())
    })
}
#[test]
fn native_batch_one_bad_watcher_rejects_every_positive_completion_and_releases_tickets()
-> TestResult {
    run("vip2batchfence", async |h| {
        fixture(h).await?;
        let second = second_watcher(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        for mutation in 0..5 {
            let mut wrong = second;
            match mutation {
                0 => wrong.character_lease_generation += 1,
                1 => {
                    wrong.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                2 => {
                    wrong.scope_ownership_generation =
                        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
                3 => wrong.character_id = character(63),
                _ => wrong.game_session_id = GameSessionId::decode(&id(99)).map_err(debug)?,
            }
            let bad = [
                BatchWatcher {
                    authority: &authority,
                    node: &h.node,
                    fence: item_fence()?,
                    last_seen_revision: 0,
                },
                BatchWatcher {
                    authority: &authority,
                    node: &h.node,
                    fence: wrong,
                    last_seen_revision: 0,
                },
            ];
            assert!(matches!(
                owner.refresh_batch_current(&bad).await,
                Err(PresenceError::AuthorityRejected)
            ));
            assert!(owner.state.lock().map_err(debug)?.sessions.is_empty());
        }
        let bad_node = NodeIncarnationProof::new(
            h.node.fact(),
            super::super::runtime_scope_assignment::BootstrapSecret::from_bytes([77; 32]),
        );
        let bad = [
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: item_fence()?,
                last_seen_revision: 0,
            },
            BatchWatcher {
                authority: &authority,
                node: &bad_node,
                fence: second,
                last_seen_revision: 0,
            },
        ];
        assert!(matches!(
            owner.refresh_batch_current(&bad).await,
            Err(PresenceError::AuthorityRejected)
        ));
        assert!(owner.state.lock().map_err(debug)?.sessions.is_empty());
        // Direct transaction-core stale-recovery control; not a fabricated entrypoint seal.
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let mut stale_recovery = recovery.clone();
        stale_recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            read_batch_in_transaction(
                &mut tx,
                vec![
                    BatchReadInput {
                        recovery,
                        node: h.node.clone(),
                        fence: item_fence()?
                    },
                    BatchReadInput {
                        recovery: stale_recovery,
                        node: h.node.clone(),
                        fence: second
                    }
                ]
            )
            .await,
            Err(PresenceError::Unavailable(_))
        ));
        tx.rollback().await?;
        let good = [
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: item_fence()?,
                last_seen_revision: 0,
            },
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: second,
                last_seen_revision: 0,
            },
        ];
        owner.refresh_batch_current(&good).await.map_err(debug)?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid").bind(second.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            owner.refresh_batch_current(&good).await,
            Err(PresenceError::AuthorityRejected)
        ));
        let state = owner.state.lock().map_err(debug)?;
        assert!(state.sessions.values().all(|w| {
            !w.reading
                && w.entries.values().all(|e| e.desired != Status::Online)
                && w.pending
                    .values()
                    .all(|p| p.status == Status::Unconfirmed && !p.announce)
        }));
        Ok(())
    })
}
#[test]
fn native_batch_drop_cancels_all_reservations_and_listener_replacement_rejects_old_work()
-> TestResult {
    run("vip2batchcancel", async |h| {
        fixture(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 1).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let watchers = [BatchWatcher {
            authority: &authority,
            node: &h.node,
            fence: item_fence()?,
            last_seen_revision: 0,
        }];
        // Poll until the real native query blocks behind the fixture's relation lock, then
        // drop the future. This tests the public future's cancellation guard, not a callback.
        let mut blocked = h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_vip_lists IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocked)
            .await?;
        let mut future = Box::pin(owner.refresh_batch_current(&watchers));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(500), future.as_mut())
                .await
                .is_err()
        );
        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%WITH watchers AS%')").fetch_one(&h.pool).await?;
        assert!(
            waiting,
            "real bulk query must reach the locked fixture relation"
        );
        assert!(
            owner
                .state
                .lock()
                .map_err(debug)?
                .sessions
                .values()
                .any(|w| w.reading)
        );
        drop(future);
        assert!(owner.state.lock().map_err(debug)?.sessions.is_empty());
        blocked.rollback().await?;
        // Cancellation drops the native max-one holder; SQLx returns or retires it
        // asynchronously. Active work must not open a holder or assume immediate reuse.
        // This genuine cancellation event permits one bounded maintenance window, then a
        // fresh root-bound reconciliation. No retry loop, deadline extension or guard bypass.
        h.root.request_ready();
        assert!(h.root.maintain_ready_once().await.map_err(|error| format!(
            "native holder maintenance after cancelled batch: {error:?}"
        ))?);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| {
                format!("fresh native reconciliation after cancelled batch: {error:?}")
            })?;
        let watchers = [BatchWatcher {
            authority: &authority,
            node: &h.node,
            fence: item_fence()?,
            last_seen_revision: 0,
        }];
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(|error| {
                format!("fresh native batch after bounded holder maintenance: {error:?}")
            })?;
        let (ticket, epoch, _) = {
            owner
                .state
                .lock()
                .map_err(debug)?
                .reserve(item_fence()?, 0, clock().map_err(debug)?)
                .map_err(debug)?
        };
        owner.listener_down().map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let observation = h
            .root
            .read_current_vip_presence(&authority, &h.node, item_fence()?, Selection::Full)
            .await
            .map_err(debug)?;
        let now = clock().map_err(debug)?;
        assert!(matches!(
            owner.state.lock().map_err(debug)?.finish_batch(
                &[(item_fence()?.game_session_id, ticket, epoch)],
                vec![observation],
                now,
                now
            ),
            Err(PresenceError::StaleWork)
        ));
        assert!(
            owner
                .state
                .lock()
                .map_err(debug)?
                .sessions
                .values()
                .all(|w| w.entries.values().all(|e| e.desired != Status::Online))
        );
        Ok(())
    })
}

#[test]
fn native_batch_bounds_and_schema_failure_never_preserve_online() -> TestResult {
    run("vip2batchbounds", async |h| {
        fixture(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = PresenceOwner::new(h.root.clone(), world(), 1).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        let watchers = [BatchWatcher {
            authority: &authority,
            node: &h.node,
            fence: item_fence()?,
            last_seen_revision: 0,
        }];
        let oversized = [
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: item_fence()?,
                last_seen_revision: 0,
            },
            BatchWatcher {
                authority: &authority,
                node: &h.node,
                fence: item_fence()?,
                last_seen_revision: 0,
            },
        ];
        assert!(matches!(
            owner.refresh_batch_current(&oversized).await,
            Err(PresenceError::Capacity)
        ));
        assert!(owner.state.lock().map_err(debug)?.sessions.is_empty());
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        h.pool
            .execute("ALTER TABLE game_character_social_settings DROP COLUMN vip_status")
            .await?;
        assert!(matches!(
            owner.refresh_batch_current(&watchers).await,
            Err(PresenceError::Unavailable(_))
        ));
        assert!(
            owner
                .state
                .lock()
                .map_err(debug)?
                .sessions
                .values()
                .all(|w| !w.reading && w.entries.values().all(|e| e.desired != Status::Online))
        );
        h.pool.execute("ALTER TABLE game_character_social_settings ADD COLUMN vip_status SMALLINT NOT NULL DEFAULT 1 CHECK(vip_status IN(1,2))").await?;
        owner
            .refresh_batch_current(&watchers)
            .await
            .map_err(debug)?;
        // Historical DATA carries complete native creation receipts. The query must refuse
        // list101, even though every entry has a valid target and absent consent stays private.
        for seed in 100_u8..199 {
            fixture_character(h, seed, 70).await?;
            sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(seed).as_slice()).bind(i64::from(seed)).execute(&h.pool).await?;
        }
        assert!(matches!(
            owner.refresh_batch_current(&watchers).await,
            Err(PresenceError::Capacity)
        ));
        assert!(
            owner
                .state
                .lock()
                .map_err(debug)?
                .sessions
                .values()
                .all(|w| !w.reading && w.entries.values().all(|e| e.desired != Status::Online))
        );
        Ok(())
    })
}

/// The non-Premium gate of a node without Premium reads (free-account caps).
struct NotCurrentGate;
impl super::super::vip::VipPremiumGate for NotCurrentGate {
    async fn vip_gate<T, F>(
        &self,
        _account: [u8; 16],
        operation: F,
    ) -> std::result::Result<T, super::super::DurabilityError>
    where
        T: Send + 'static,
        F: super::super::vip::VipGateOperation<T>,
    {
        Ok(operation(crate::domain::premium::PremiumStatus::NotCurrent, None).await)
    }
}

/// VIP-2 end to end (CP D1444) on the installed migration 0130 and the real writers: the watcher
/// (character 64, session 93) sends the client `VIP_INTENT` add frame for the harness character;
/// the target accepts from its own admitted view; the watcher's owner then confirms ONLINE, and
/// the target's logout and login reach the watcher as domain 21 status deltas the client decodes.
/// RL-11 is measured from the target's session commit to the decoded delta.
#[test]
fn native_vip_add_accept_and_presence_reach_the_watcher_client_end_to_end() -> TestResult {
    use super::super::vip::{VipAcceptRequest, VipAddRequest, VipAddResult, VipListKey, VipResult};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_protocol_oteryn::vip::{
        VipDelta, VipIntent, VipStatus, VipStatusUpdate, decode_vip_delta, decode_vip_intent,
        encode_vip_delta, encode_vip_intent,
    };
    run("vip2e2e", async |h| {
        fixture(h).await?;
        let watcher = second_watcher(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let list = |account: u8| -> TestResult<VipListKey> {
            Ok(VipListKey {
                account_id: crate::domain::AccountId::from_bytes(id(account)).map_err(debug)?,
                world_id: crate::domain::WorldId::from_bytes(id(WORLD)).map_err(debug)?,
            })
        };
        let command = |session: u8, number: u64| -> TestResult<CommandRef> {
            Ok(CommandRef::new(
                GameSessionId::decode(&id(session)).map_err(debug)?,
                CommandId::new(number).map_err(debug)?,
            ))
        };
        // The target's admission makes its settings row and reads its (empty) consent view.
        let target_view = h
            .root
            .admit_vip(&authority, &h.node, item_fence()?)
            .await
            .map_err(debug)?;
        assert!(target_view.invitations.is_empty());
        let name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        // The client frame of the add, decoded by the server as the connection does.
        let frame = encode_vip_intent(&VipIntent::Add { name: name.clone() }).map_err(debug)?;
        let VipIntent::Add { name: decoded } = decode_vip_intent(&frame).map_err(debug)? else {
            return Err("not an add".into());
        };
        let added = h
            .root
            .add_vip(
                &authority,
                &h.node,
                watcher,
                &NotCurrentGate,
                VipAddRequest::new(list(80)?, command(93, 1)?, &decoded),
            )
            .await
            .map_err(debug)?;
        assert_eq!(added.result, VipAddResult::Ok);
        let entry_no = added.entry_no.ok_or("entry number")?;
        // The target accepts the invitation of its own view.
        let target_view = h
            .root
            .admit_vip(&authority, &h.node, item_fence()?)
            .await
            .map_err(debug)?;
        let (invitation, inviter) = target_view.invitations.first().ok_or("invitation")?;
        assert_eq!(inviter, "Vip Presence CM");
        let accepted = h
            .root
            .accept_vip_invitation(
                &authority,
                &h.node,
                item_fence()?,
                VipAcceptRequest {
                    list: list(40)?,
                    command: command(native::SESSION, 1)?,
                    invitation: invitation.binding,
                },
            )
            .await
            .map_err(debug)?;
        assert_eq!(accepted.result, VipResult::Ok);
        let watcher_view = h
            .root
            .admit_vip(&authority, &h.node, watcher)
            .await
            .map_err(debug)?;
        assert!(
            watcher_view
                .entries
                .iter()
                .any(|e| e.entry_no == entry_no && e.name == name)
        );
        // A native add stores notify off; the watcher turns login/logout notices on.
        let edited = h
            .root
            .modify_vip_list(
                &authority,
                &h.node,
                watcher,
                &NotCurrentGate,
                super::super::vip::VipListRequest {
                    list: list(80)?,
                    command: command(93, 2)?,
                    operation: super::super::vip::VipListOperation::Edit {
                        entry_no,
                        description: String::new(),
                        icon: 0,
                        notify: true,
                        groups: Vec::new(),
                    },
                },
            )
            .await
            .map_err(debug)?;
        assert_eq!(edited.result, super::super::vip::VipListResult::Ok);
        // The watcher's owner confirms the listed target ONLINE (no announce at first sight).
        let owner = PresenceOwner::new(h.root.clone(), world(), 2).map_err(debug)?;
        owner.listener_reconnected().map_err(debug)?;
        owner
            .refresh_current(&authority, &h.node, watcher, 0, Refresh::Reconcile)
            .await
            .map_err(debug)?;
        let mut shown = Vec::new();
        while let Some(update) = owner
            .next_current_update(&authority, &h.node, watcher, 0)
            .await
            .map_err(debug)?
        {
            owner.delivered(&update).map_err(debug)?;
            shown.push((update.entry(), update.status(), update.announce()));
        }
        assert!(
            shown.contains(&(entry_no, Status::Online, false)),
            "{shown:?}"
        );
        // The client decodes each status delta the connection would write.
        let client = |update: &StatusUpdate| -> TestResult<VipStatusUpdate> {
            let delta = VipDelta::Status(VipStatusUpdate {
                entry_no: update.entry(),
                status: match update.status() {
                    Status::Online => VipStatus::Online,
                    Status::Offline => VipStatus::Offline,
                    Status::Unconfirmed => VipStatus::Unconfirmed,
                },
                announce: update.announce(),
            });
            let payload = encode_vip_delta(&delta).map_err(debug)?;
            match decode_vip_delta(delta.delta_type(), &payload).map_err(debug)? {
                VipDelta::Status(status) => Ok(status),
                _ => Err("not a status".into()),
            }
        };
        let target = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let mut latencies = Vec::new();
        for (state, online, status) in [
            (3_i16, false, VipStatus::Offline),
            (1, true, VipStatus::Online),
        ] {
            let started = std::time::Instant::now();
            sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=$1 WHERE game_session_id=encode($2,'hex')::uuid")
                .bind(state).bind(id(native::SESSION).as_slice()).execute(&h.pool).await?;
            // The production path: the relay's sink records the hint (`PresenceSink::hint`
            // calls `target_hint`), and the connection's next delivery reads because the
            // watcher is due, announcing the transition the read confirms.
            owner.target_hint(target, online).map_err(debug)?;
            assert!(owner.is_due(watcher.game_session_id).map_err(debug)?);
            let update = owner
                .next_current_update(&authority, &h.node, watcher, 0)
                .await
                .map_err(debug)?
                .ok_or("status delta")?;
            let seen = client(&update)?;
            latencies.push(started.elapsed());
            assert_eq!(
                seen,
                VipStatusUpdate {
                    entry_no,
                    status,
                    announce: true
                }
            );
            owner.delivered(&update).map_err(debug)?;
        }
        println!(
            "VIP0-RL-11 end to end (session commit to decoded client delta, one node): logout {} us, login {} us",
            latencies[0].as_micros(),
            latencies[1].as_micros()
        );
        Ok(())
    })
}

#[test]
fn confirmation_left_tracks_only_shown_online_and_saturates() {
    let mut s = state();
    let session = item_fence().expect("fence").game_session_id;
    assert_eq!(s.confirmation_left(session, 100), None);
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    assert_eq!(s.confirmation_left(session, 100), None);
    acknowledge(&mut s, 100);
    assert_eq!(s.confirmation_left(session, 350), Some(CONFIRM_MS - 250));
    let mut older = s.sessions[&session].entries[&1].clone();
    older.confirmed_from = 50;
    s.sessions
        .get_mut(&session)
        .expect("watcher")
        .entries
        .insert(2, older);
    assert_eq!(s.confirmation_left(session, 350), Some(CONFIRM_MS - 300));
    s.sessions
        .get_mut(&session)
        .expect("watcher")
        .entries
        .remove(&2);
    assert_eq!(s.confirmation_left(session, 100 + CONFIRM_MS), Some(0));
    assert_eq!(s.confirmation_left(session, 101 + CONFIRM_MS), Some(0));
    install(
        &mut s,
        read(data(Visibility::Shown, false)),
        400,
        Refresh::Delivery,
    );
    acknowledge(&mut s, 400);
    assert_eq!(s.confirmation_left(session, 400), None);
}

#[test]
fn confirmation_lead_is_due_at_half_age_only_when_ready_to_read() {
    let mut s = state();
    let session = item_fence().expect("fence").game_session_id;
    install(
        &mut s,
        read(data(Visibility::Shown, true)),
        100,
        Refresh::Reconcile,
    );
    acknowledge(&mut s, 100);
    let at = 100 + CONFIRM_LEAD_MS;
    assert!(!s.confirmation_due(session, at - 1));
    assert!(s.confirmation_due(session, at));
    s.sessions.get_mut(&session).expect("watcher").reading = true;
    assert!(!s.confirmation_due(session, at));
    let watcher = s.sessions.get_mut(&session).expect("watcher");
    watcher.reading = false;
    watcher.retry_at = at + 1;
    assert!(!s.confirmation_due(session, at));
    assert!(s.confirmation_due(session, at + 1));
    s.listener = false;
    assert!(!s.confirmation_due(session, at + 1));
}
