//! Pure state fixtures are synthetic data, not native current-authority proofs.
//! PostgreSQL cases below use the unchanged real native Root/recovery/session/node fixtures.
#![allow(clippy::expect_used, reason = "Explicit synthetic pure-state fixtures")]
use super::*;
use crate::bestiary_postgres_harness as native;
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

// Exact private VIP1 table/column subset. Administrative fixture data is not a production
// invitation/acceptance producer, schema migration, grant or current-authority substitute.
const SCHEMA: &str = r"
ALTER TABLE game_character_social_settings ADD COLUMN vip_status SMALLINT NOT NULL DEFAULT 1 CHECK(vip_status IN(1,2));
ALTER TABLE game_character_social_settings ADD COLUMN vip_consent_revision NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(vip_consent_revision BETWEEN 0 AND 18446744073709551615);
CREATE TABLE game_vip_lists(account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),world_id UUID NOT NULL,revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),next_entry_no NUMERIC(20,0) NOT NULL,rate_tokens SMALLINT NOT NULL,rate_at BIGINT NOT NULL,PRIMARY KEY(account_id,world_id));
CREATE TABLE game_vip_entries(account_id UUID NOT NULL,world_id UUID NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),entry_no NUMERIC(20,0) NOT NULL CHECK(entry_no>0),description TEXT NOT NULL DEFAULT'',icon SMALLINT NOT NULL DEFAULT 0,notify BOOLEAN NOT NULL DEFAULT FALSE,added_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),PRIMARY KEY(account_id,world_id,character_id),UNIQUE(account_id,world_id,entry_no),FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE);
CREATE SEQUENCE game_vip_invitation_id NO CYCLE;
CREATE TABLE game_vip_consents(account_id UUID NOT NULL,world_id UUID NOT NULL,inviter_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),character_id UUID NOT NULL,invitation_id BIGINT NOT NULL DEFAULT nextval('game_vip_invitation_id') UNIQUE CHECK(invitation_id>0),consent SMALLINT NOT NULL CHECK(consent IN(1,2)),invite_expires_at TIMESTAMPTZ NOT NULL,PRIMARY KEY(account_id,world_id,inviter_character_id,character_id),CHECK(inviter_character_id<>character_id),FOREIGN KEY(account_id,world_id,character_id) REFERENCES game_vip_entries ON DELETE CASCADE);
";

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
    h.pool.execute(SCHEMA).await?;
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
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = native::configured_admin()
        .ok_or("canonical PostgreSQL17.6 route required; no silent skip")?;
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
#[test]
fn native_reader_rejects_independently_wrong_fences_and_uninstalled_schema() -> TestResult {
    run("vip2fences", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .read_current_vip_presence(&authority, &h.node, item_fence()?, Selection::Full)
                .await,
            Err(PresenceError::Unavailable(_))
        ));
        h.pool.execute(SCHEMA).await?;
        for which in 0..4 {
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
                _ => {
                    f.runtime_scope = RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(99)).map_err(debug)?,
                        crate::foundation::ChannelId::decode(&id(native::CHANNEL))
                            .map_err(debug)?,
                    )
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
