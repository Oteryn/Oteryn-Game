//! VIP-1b (CP D1444): the VIP writers against the installed migration `0130_vip_lists.sql`,
//! with the real native recovery and session checks. Mounted by the library's test target only
//! (`gameplay_transport::vip_intent`), which owns the real Premium coordinator; without a
//! configured PostgreSQL route each test returns without running.
#![allow(clippy::expect_used)]
use crate::bestiary_postgres_harness as harness;
use crate::domain::vip::*;
use crate::domain::{AccountId, CharacterId, WorldId};
use crate::durability::DurabilityRoot;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::vip::*;
use crate::foundation::{CommandId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use harness::{CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id};
use sqlx::{Executor, Row};
use std::future::Future;
use std::task::Poll;

fn account(seed: u8) -> AccountId {
    AccountId::from_bytes(id(seed)).expect("account")
}
fn character(seed: u8) -> CharacterId {
    CharacterId::from_bytes(id(seed)).expect("character")
}
fn list(seed: u8) -> VipListKey {
    VipListKey {
        account_id: account(seed),
        world_id: WorldId::from_bytes(id(WORLD)).expect("world"),
    }
}
fn request(number: u64, operation: VipEndOperation) -> VipEndRequest {
    VipEndRequest {
        list: list(40),
        command: CommandRef::new(
            GameSessionId::decode(&id(SESSION)).expect("session"),
            CommandId::new(number).expect("command"),
        ),
        operation,
    }
}
fn withdraw(number: u64) -> VipEndRequest {
    request(number, VipEndOperation::Withdraw { entry_no: 1 })
}
fn invitation(inviter: u8, owner: u8, invitation_id: u64) -> VipInvitationBinding {
    VipInvitationBinding {
        watcher_account: account(owner),
        inviter: character(inviter),
        invitation_id,
    }
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, false).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn fixture_character(h: &Harness, seed: u8, owner: u8) -> TestResult {
    fixture_character_world(h, seed, owner, WORLD).await
}
async fn fixture_character_world(h: &Harness, seed: u8, owner: u8, world: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(owner).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($5,'hex')::uuid,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(owner).as_slice()).bind(format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26))).bind(id(CHARACTER).as_slice()).bind(id(world).as_slice()).execute(&h.pool).await?;
    // Complete historical creation fixture through the existing native decoder/binder.
    // This is not an actor authority constructor: the real sealed recovery and full
    // integrity checks below must still succeed after these historical rows exist.
    let distinct = |tag| {
        let mut value = id(seed);
        value[15] ^= tag;
        value
    };
    let operation = distinct(0x80);
    let decision = distinct(0x81);
    let uuid = |bytes: [u8; 16]| {
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    };
    let raw = serde_json::to_vec(&serde_json::json!({
        "contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),
        "source_revision":seed.to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":uuid(id(owner)),"target_world_id":uuid(id(world)),
        "requested_name":format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26)),
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"
    }))?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(owner).as_slice()).bind(id(seed).as_slice()).bind(id(world).as_slice())
        .bind(distinct(0x82).as_slice()).bind(distinct(0x83).as_slice()).bind(decision.as_slice()).bind(i64::from(seed)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding")
        .execute(&h.pool).await?;
    Ok(())
}
async fn fixture_list(h: &Harness, owner: u8) -> TestResult {
    sqlx::query("INSERT INTO game_vip_lists VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,2,20,floor(extract(epoch FROM clock_timestamp())*1000000)::bigint) ON CONFLICT DO NOTHING")
        .bind(id(owner).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    Ok(())
}
// SQL-only relationship fixture. It never supplies the session/recovery/node proof checked by writer.
async fn fixture_consent(
    h: &Harness,
    owner: u8,
    inviter: u8,
    target: u8,
    state: i16,
) -> TestResult<u64> {
    fixture_list(h, owner).await?;
    sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(owner).as_slice()).bind(id(WORLD).as_slice()).bind(id(target).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT DO NOTHING")
        .bind(id(target).as_slice()).execute(&h.pool).await?;
    let number:String=sqlx::query_scalar("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,clock_timestamp()+interval '7days') RETURNING invitation_id::text")
        .bind(id(owner).as_slice()).bind(id(WORLD).as_slice()).bind(id(inviter).as_slice()).bind(id(target).as_slice()).bind(state).fetch_one(&h.pool).await?;
    sqlx::query("UPDATE game_character_social_settings SET vip_consent_revision=vip_consent_revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(target).as_slice()).execute(&h.pool).await?;
    Ok(number.parse()?)
}
async fn apply(
    h: &Harness,
    request: VipEndRequest,
    fence: CurrentCharacterItemFence,
) -> TestResult<VipReceipt> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .end_vip_consent(&authority, &h.node, fence, request)
        .await
        .map_err(|e| debug(e).into())
}
async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64, String)> {
    Ok((
        h.count("game_vip_entries").await?,
        h.count("game_vip_consents").await?,
        h.count("game_vip_command_receipts").await?,
        h.root_revision().await?,
    ))
}

#[test]
fn vip_consent_state_and_expiry_matrix() {
    use ConsentEnd::*;
    for (action, expected) in [(Withdraw, true), (Reject, true), (Revoke, false)] {
        assert_eq!(
            can_end_consent(action, ConsentState::Pending { expires_at_us: 100 }, 99),
            expected
        );
        assert!(!can_end_consent(
            action,
            ConsentState::Pending { expires_at_us: 100 },
            100
        ));
        assert!(!can_end_consent(
            action,
            ConsentState::Pending { expires_at_us: 100 },
            101
        ));
    }
    for (action, expected) in [(Withdraw, true), (Reject, false), (Revoke, true)] {
        assert_eq!(
            can_end_consent(action, ConsentState::Accepted, i64::MAX),
            expected
        );
    }
}
#[test]
fn vip_persisted_rate_arithmetic_bounds_rounding_and_clock_refusal() {
    let empty = VipRateBucket {
        tokens: 0,
        at_us: 10_000_000,
    };
    assert_eq!(
        charge_vip_bucket(empty, 10_000_000)
            .expect("bucket")
            .exhausted_seconds,
        Some(3)
    );
    assert_eq!(
        charge_vip_bucket(empty, 12_000_001)
            .expect("bucket")
            .exhausted_seconds,
        Some(1)
    );
    let one = charge_vip_bucket(empty, 13_000_000).expect("refill");
    assert_eq!(one.exhausted_seconds, None);
    assert_eq!(one.bucket.tokens, 0);
    assert_eq!(one.bucket.at_us, 13_000_000);
    assert_eq!(
        charge_vip_bucket(empty, 1_000_000_000)
            .expect("bounded refill")
            .bucket
            .tokens,
        19
    );
    assert_eq!(charge_vip_bucket(empty, 9_999_999), None);
    assert_eq!(
        charge_vip_bucket(
            VipRateBucket {
                tokens: 21,
                ..empty
            },
            13_000_000
        ),
        None
    );
}

#[test]
fn vip_withdraw_keeps_shared_entry_groups_and_other_inviter_consent() -> TestResult {
    run("vipwithdraw", async |h| {
        fixture_character(h, 60, 40).await?;
        fixture_character(h, 61, 70).await?;
        fixture_consent(h, 40, CHARACTER, 61, 2).await?;
        fixture_consent(h, 40, 60, 61, 2).await?;
        sqlx::query("INSERT INTO game_vip_group_members SELECT account_id,world_id,character_id,1 FROM game_vip_entries").execute(&h.pool).await?;
        let result = apply(h, withdraw(1), harness::item_fence()?).await?;
        assert_eq!(result.result, VipResult::Ok);
        assert_eq!(result.revision, 2);
        assert_eq!(footprint(h).await?, (1, 1, 1, "1".into()));
        assert_eq!(h.count("game_vip_group_members").await?, 1);
        let inviter: Vec<u8> =
            sqlx::query_scalar("SELECT uuid_send(inviter_character_id) FROM game_vip_consents")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(inviter, id(60));
        let view:String=sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid").bind(id(61).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(view, "3");
        Ok(())
    })
}

#[test]
fn vip_reject_and_revoke_bind_exact_invitation_and_state() -> TestResult {
    run("viptarget", async |h| {
        fixture_character(h, 60, 70).await?;
        let first = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let stale = request(1, VipEndOperation::Reject(invitation(60, 70, first + 1)));
        assert_eq!(
            apply(h, stale, harness::item_fence()?).await?.result,
            VipResult::NotFound
        );
        assert_eq!(h.count("game_vip_consents").await?, 1);
        assert_eq!(
            apply(
                h,
                request(2, VipEndOperation::Revoke(invitation(60, 70, first))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::NotFound
        );
        assert_eq!(
            apply(
                h,
                request(3, VipEndOperation::Reject(invitation(60, 70, first))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::Ok
        );
        let second = fixture_consent(h, 70, 60, CHARACTER, 2).await?;
        assert!(second > first);
        assert_eq!(
            apply(
                h,
                request(4, VipEndOperation::Revoke(invitation(60, 70, first))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::NotFound
        );
        assert_eq!(
            apply(
                h,
                request(5, VipEndOperation::Reject(invitation(60, 70, second))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::NotFound
        );
        sqlx::query(
            "UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()-interval '1day'",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(
            apply(
                h,
                request(6, VipEndOperation::Revoke(invitation(60, 70, second))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::Ok
        );
        assert_eq!(footprint(h).await?, (1, 0, 6, "1".into()));
        let watcher: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(70).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(watcher, "3");
        Ok(())
    })
}

#[test]
fn vip_historical_replay_precedes_ended_session_but_not_recovery_or_binding() -> TestResult {
    run("vipreplay", async |h| {
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 1).await?;
        let original = apply(h, withdraw(1), harness::item_fence()?).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&h.pool)
            .await?;
        let before = footprint(h).await?;
        let replay = apply(h, withdraw(1), harness::item_fence()?).await?;
        assert!(replay.replayed);
        assert_eq!(replay.result, original.result);
        assert_eq!(replay.revision, original.revision);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut changed = withdraw(1);
        changed.operation = VipEndOperation::Withdraw { entry_no: 2 };
        assert!(matches!(
            h.root
                .end_vip_consent(&authority, &h.node, harness::item_fence()?, changed)
                .await,
            Err(VipError::ConflictingCommand)
        ));
        let mut wrong_recovery = recovery_record(&authority, &h.root).map_err(debug)?;
        wrong_recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            end_in_transaction(
                &mut tx,
                &wrong_recovery,
                &h.node,
                harness::item_fence()?,
                withdraw(1)
            )
            .await,
            Err(VipError::Unavailable(_))
        ));
        tx.rollback().await?;
        assert_eq!(footprint(h).await?, before);
        Ok(())
    })
}

#[test]
fn vip_new_writes_reject_each_independently_stale_native_fence() -> TestResult {
    run("vipfences", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let baseline = harness::item_fence()?;
        let mut cases = [baseline; 7];
        cases[0].connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        cases[1].character_lease_generation = 2;
        cases[2].scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        cases[3].game_session_id = GameSessionId::decode(&id(90)).map_err(debug)?;
        cases[4].character_id = character(90);
        cases[5].runtime_scope = RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(90)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
        );
        cases[6].runtime_scope = RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(90)).map_err(debug)?,
        );
        let before = footprint(h).await?;
        for fence in cases {
            assert!(matches!(
                h.root
                    .end_vip_consent(&authority, &h.node, fence, withdraw(1))
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(footprint(h).await?, before);
        }
        let other = harness::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .end_vip_consent(&authority, &other, baseline, withdraw(1))
                .await,
            Err(VipError::AuthorityRejected)
        ));
        sqlx::query("UPDATE game_durability_admission_runtime_guards SET ready=FALSE")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .end_vip_consent(&authority, &h.node, baseline, withdraw(1))
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        Ok(())
    })
}

#[test]
fn vip_account_world_isolation_and_foreign_invitation_fail_closed() -> TestResult {
    run("vipisolation", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = footprint(h).await?;
        for foreign in [
            VipListKey {
                account_id: account(70),
                ..list(40)
            },
            VipListKey {
                world_id: WorldId::from_bytes(id(90)).map_err(debug)?,
                ..list(40)
            },
        ] {
            let mut changed = withdraw(1);
            changed.list = foreign;
            assert!(matches!(
                h.root
                    .end_vip_consent(&authority, &h.node, harness::item_fence()?, changed)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(footprint(h).await?, before);
        }
        let bad = request(1, VipEndOperation::Reject(invitation(60, 80, number)));
        assert_eq!(
            apply(h, bad, harness::item_fence()?).await?.result,
            VipResult::NotFound
        );
        assert_eq!(h.count("game_vip_consents").await?, 1);
        let foreign_revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(70).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(foreign_revision, "1");
        for (number, operation) in [
            (2, VipEndOperation::Withdraw { entry_no: 0 }),
            (3, VipEndOperation::Withdraw { entry_no: u64::MAX }),
            (4, VipEndOperation::Reject(invitation(60, 70, 0))),
            (5, VipEndOperation::Revoke(invitation(60, 70, u64::MAX))),
        ] {
            assert_eq!(
                apply(h, request(number, operation), harness::item_fence()?)
                    .await?
                    .result,
                VipResult::NotFound
            );
        }
        Ok(())
    })
}

#[test]
fn vip_expired_pending_is_absent_but_entry_is_retained() -> TestResult {
    run("vipexpired", async |h| {
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 1).await?;
        sqlx::query(
            "UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()-interval '1day'",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(
            apply(h, withdraw(1), harness::item_fence()?).await?.result,
            VipResult::NotFound
        );
        assert_eq!(footprint(h).await?, (1, 1, 1, "1".into()));
        let view:String=sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(view, "1");
        Ok(())
    })
}

#[test]
fn vip_missing_target_view_rolls_back_erase_rate_and_receipt() -> TestResult {
    run("viprollback", async |h| {
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 2).await?;
        sqlx::query(
            "DELETE FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&h.pool)
        .await?;
        let before = footprint(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .end_vip_consent(&authority, &h.node, harness::item_fence()?, withdraw(1))
                .await,
            Err(VipError::Unavailable(_))
        ));
        assert_eq!(footprint(h).await?, before);
        let state: (String, i16) =
            sqlx::query_as("SELECT revision::text,rate_tokens FROM game_vip_lists")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(state, ("1".into(), 20));
        Ok(())
    })
}

#[test]
fn vip_refusal_receipt_is_immutable_and_reconnect_does_not_reset_rate() -> TestResult {
    run("viprate", async |h| {
        fixture_list(h, 40).await?;
        let result = apply(h, withdraw(1), harness::item_fence()?).await?;
        assert_eq!(result.result, VipResult::NotFound);
        let before: (String, i16, i64) =
            sqlx::query_as("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists")
                .fetch_one(&h.pool)
                .await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2")
            .execute(&h.pool)
            .await?;
        let mut reconnected = harness::item_fence()?;
        reconnected.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let replay = apply(h, withdraw(1), reconnected).await?;
        assert!(replay.replayed);
        let after: (String, i16, i64) =
            sqlx::query_as("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(before, after);
        let error = sqlx::query("UPDATE game_vip_command_receipts SET result=1")
            .execute(&h.pool)
            .await
            .expect_err("immutable receipt");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        sqlx::query("UPDATE game_vip_lists SET rate_tokens=0,rate_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint").execute(&h.pool).await?;
        let exhausted = apply(h, withdraw(2), reconnected).await?;
        assert!(matches!(
            exhausted.result,
            VipResult::Exhausted { seconds: 1..=3 }
        ));
        assert_eq!(
            apply(h, withdraw(2), reconnected).await?.result,
            exhausted.result
        );
        assert_eq!(h.count("game_vip_command_receipts").await?, 2);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_concurrent_commands_serialize_one_erase_and_receipt_each_outcome() -> TestResult {
    run("vipconcurrent", async |h| {
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 2).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let recovery = recovery_record(&authority, &h.root).map_err(debug)?;
        let operation = async |number| -> TestResult<VipReceipt> {
            let mut tx = h.pool.begin().await?;
            let result = end_in_transaction(
                &mut tx,
                &recovery,
                &h.node,
                harness::item_fence()?,
                withdraw(number),
            )
            .await
            .map_err(debug)?;
            tx.commit().await?;
            Ok(result)
        };
        for (first_number, second_number, duplicate) in [(1, 2, false), (3, 3, true)] {
            if duplicate {
                fixture_consent(h, 40, CHARACTER, 60, 2).await?;
            }
            let mut first = std::pin::pin!(operation(first_number));
            let mut second = std::pin::pin!(operation(second_number));
            let mut first_result = None;
            let mut second_result = None;
            std::future::poll_fn(|context| {
                if first_result.is_none()
                    && let Poll::Ready(result) = first.as_mut().poll(context)
                {
                    first_result = Some(result);
                }
                if second_result.is_none()
                    && let Poll::Ready(result) = second.as_mut().poll(context)
                {
                    second_result = Some(result);
                }
                if first_result.is_some() && second_result.is_some() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            let receipts = [
                first_result.expect("first completed")?,
                second_result.expect("second completed")?,
            ];
            let results = receipts.map(|receipt| receipt.result);
            if duplicate {
                assert_eq!(results, [VipResult::Ok, VipResult::Ok]);
                assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
            } else {
                assert_eq!(results.iter().filter(|r| **r == VipResult::Ok).count(), 1);
                assert_eq!(
                    results
                        .iter()
                        .filter(|r| **r == VipResult::NotFound)
                        .count(),
                    1
                );
            }
        }
        assert_eq!(footprint(h).await?, (1, 0, 3, "1".into()));
        let view:String=sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(view, "4");
        Ok(())
    })
}

fn accept_request(number: u64, binding: VipInvitationBinding) -> VipAcceptRequest {
    let base = request(number, VipEndOperation::Accept(binding));
    VipAcceptRequest {
        list: base.list,
        command: base.command,
        invitation: binding,
    }
}
async fn accept(h: &Harness, request: VipAcceptRequest) -> TestResult<VipReceipt> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .accept_vip_invitation(&authority, &h.node, harness::item_fence()?, request)
        .await
        .map_err(|e| debug(e).into())
}
async fn accepted_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE character_id=encode($1,'hex')::uuid AND consent=2")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
async fn consent_view(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
async fn both<A: Future, B: Future>(first: A, second: B) -> (A::Output, B::Output) {
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut a = None;
    let mut b = None;
    std::future::poll_fn(|cx| {
        if a.is_none()
            && let Poll::Ready(value) = first.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let Poll::Ready(value) = second.as_mut().poll(cx)
        {
            b = Some(value);
        }
        if a.is_some() && b.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    (a.expect("first completed"), b.expect("second completed"))
}

#[test]
fn vip_accept_decision_expiry_and_cap_boundaries() {
    let pending = ConsentState::Pending { expires_at_us: 100 };
    assert_eq!(accept_consent(pending, 99, 99), AcceptDecision::Accept);
    for count in [100, 101, u64::MAX] {
        assert_eq!(accept_consent(pending, 99, count), AcceptDecision::ListFull);
    }
    for now in [100, 101, i64::MAX] {
        assert_eq!(accept_consent(pending, now, 0), AcceptDecision::NotFound);
        assert_eq!(accept_consent(pending, now, 100), AcceptDecision::NotFound);
    }
    assert_eq!(
        accept_consent(ConsentState::Accepted, 0, 0),
        AcceptDecision::NotFound
    );
}

#[test]
fn vip_accept_exact_key_old_handle_replacement_and_single_revision_effects() -> TestResult {
    run("vipacceptbinding", async |h| {
        fixture_character(h, 60, 70).await?;
        fixture_character(h, 61, 70).await?;
        let old = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let before_view = consent_view(h).await?;
        for (number, binding) in [
            (1, invitation(60, 70, old + 1)),
            (2, invitation(61, 70, old)),
            (3, invitation(60, 80, old)),
        ] {
            assert_eq!(
                accept(h, accept_request(number, binding)).await?.result,
                VipResult::NotFound
            );
        }
        assert_eq!(accepted_count(h).await?, 0);
        assert_eq!(consent_view(h).await?, before_view);
        assert_eq!(
            accept(h, accept_request(4, invitation(60, 70, old)))
                .await?
                .result,
            VipResult::Ok
        );
        assert_eq!(accepted_count(h).await?, 1);
        assert_eq!(consent_view(h).await?, "2");
        let watcher_revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(70).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(watcher_revision, "2");
        assert_eq!(
            accept(h, accept_request(5, invitation(60, 70, old)))
                .await?
                .result,
            VipResult::NotFound
        );
        assert_eq!(consent_view(h).await?, "2");
        // A genuine target revoke then a private invitation-producer fixture obtains a new
        // sequence identity. The old delivered handle cannot act on the replacement.
        assert_eq!(
            apply(
                h,
                request(6, VipEndOperation::Revoke(invitation(60, 70, old))),
                harness::item_fence()?
            )
            .await?
            .result,
            VipResult::Ok
        );
        let fresh = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        assert!(fresh > old);
        assert_eq!(
            accept(h, accept_request(7, invitation(60, 70, old)))
                .await?
                .result,
            VipResult::NotFound
        );
        assert_eq!(accepted_count(h).await?, 0);
        assert_eq!(
            accept(h, accept_request(8, invitation(60, 70, fresh)))
                .await?
                .result,
            VipResult::Ok
        );
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_accept_receipt_precedes_live_fence_but_not_recovery_and_binding() -> TestResult {
    run("vipacceptreplay", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        let committed = accept(h, req).await?;
        assert_eq!(committed.result, VipResult::Ok);
        let before = footprint(h).await?;
        let bucket: (String, i16, i64) = sqlx::query_as("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid")
            .bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&h.pool)
            .await?;
        let replay = accept(h, req).await?;
        assert!(replay.replayed);
        assert_eq!(
            (replay.result, replay.revision),
            (committed.result, committed.revision)
        );
        assert_eq!(footprint(h).await?, before);
        assert_eq!(consent_view(h).await?, "2");
        let after: (String, i16, i64) = sqlx::query_as("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid")
            .bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(bucket, after);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut conflict = req;
        conflict.invitation.invitation_id += 1;
        assert!(matches!(
            h.root
                .accept_vip_invitation(&authority, &h.node, harness::item_fence()?, conflict)
                .await,
            Err(VipError::ConflictingCommand)
        ));
        let mut recovery = recovery_record(&authority, &h.root).map_err(debug)?;
        recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            end_in_transaction(
                &mut tx,
                &recovery,
                &h.node,
                harness::item_fence()?,
                request(1, VipEndOperation::Accept(req.invitation))
            )
            .await,
            Err(VipError::Unavailable(_))
        ));
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn vip_accept_current_native_authority_and_account_world_matrix() -> TestResult {
    run("vipacceptfence", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let baseline = harness::item_fence()?;
        let mut cases = [baseline; 7];
        cases[0].connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        cases[1].character_lease_generation = 2;
        cases[2].scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        cases[3].game_session_id = GameSessionId::decode(&id(90)).map_err(debug)?;
        cases[4].character_id = character(60); // Valid different root, wrong acting session.
        cases[5].runtime_scope = RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(90)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
        );
        cases[6].runtime_scope = RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(90)).map_err(debug)?,
        );
        let before = footprint(h).await?;
        for fence in cases {
            assert!(matches!(
                h.root
                    .accept_vip_invitation(&authority, &h.node, fence, req)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(footprint(h).await?, before);
            assert_eq!(accepted_count(h).await?, 0);
            assert_eq!(consent_view(h).await?, "1");
        }
        for foreign in [
            VipListKey {
                account_id: account(70),
                ..list(40)
            },
            VipListKey {
                world_id: WorldId::from_bytes(id(90)).map_err(debug)?,
                ..list(40)
            },
        ] {
            assert!(matches!(
                h.root
                    .accept_vip_invitation(
                        &authority,
                        &h.node,
                        baseline,
                        VipAcceptRequest {
                            list: foreign,
                            ..req
                        }
                    )
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(footprint(h).await?, before);
            assert_eq!(accepted_count(h).await?, 0);
            assert_eq!(consent_view(h).await?, "1");
        }
        let other = harness::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .accept_vip_invitation(&authority, &other, baseline, req)
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        assert_eq!(accepted_count(h).await?, 0);
        assert_eq!(consent_view(h).await?, "1");
        // The genuine actor remains valid until this independently mutated current DB fact.
        sqlx::query("UPDATE game_durability_admission_runtime_guards SET ready=FALSE")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .accept_vip_invitation(&authority, &h.node, baseline, req)
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        assert_eq!(accepted_count(h).await?, 0);
        assert_eq!(consent_view(h).await?, "1");
        Ok(())
    })
}

#[test]
fn vip_accept_expired_refusal_rate_and_missing_view_rollback() -> TestResult {
    run("vipacceptrollback", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        sqlx::query(
            "UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()-interval '1day'",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(accept(h, req).await?.result, VipResult::NotFound);
        assert!(accept(h, req).await?.replayed);
        assert_eq!(accepted_count(h).await?, 0);
        sqlx::query(
            "UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()+interval '7days'",
        )
        .execute(&h.pool)
        .await?;
        sqlx::query(
            "DELETE FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .execute(&h.pool)
        .await?;
        let before = footprint(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .accept_vip_invitation(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    accept_request(2, req.invitation)
                )
                .await,
            Err(VipError::Unavailable(_))
        ));
        assert_eq!(footprint(h).await?, before);
        assert_eq!(accepted_count(h).await?, 0);
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1')").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        sqlx::query("UPDATE game_vip_lists SET rate_tokens=0,rate_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).execute(&h.pool).await?;
        let exhausted = accept(h, accept_request(2, req.invitation)).await?;
        assert!(matches!(
            exhausted.result,
            VipResult::Exhausted { seconds: 1..=3 }
        ));
        assert_eq!(
            accept(h, accept_request(2, req.invitation)).await?.result,
            exhausted.result
        );
        assert_eq!(accepted_count(h).await?, 0);
        assert_eq!(consent_view(h).await?, "0");
        Ok(())
    })
}

#[test]
fn vip_accept_two_real_roots_duplicate_and_cross_list_cap_race() -> TestResult {
    run("vipacceptconcurrent", async |h| {
        fixture_character(h, 60, 70).await?;
        let first = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let second_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let req = accept_request(1, invitation(60, 70, first));
        let (left, right) = both(
            h.root
                .accept_vip_invitation(&a, &h.node, harness::item_fence()?, req),
            second_root.accept_vip_invitation(&b, &h.node, harness::item_fence()?, req),
        )
        .await;
        let receipts = [left.map_err(debug)?, right.map_err(debug)?];
        assert!(receipts.iter().all(|r| r.result == VipResult::Ok));
        assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
        assert_eq!(accepted_count(h).await?, 1);
        // Synthetic historical relationship DATA; each inviter is a distinct, valid native
        // root with its complete bootstrap history, not an invented active actor/session.
        for inviter in 61..=158 {
            fixture_character(h, inviter, 70).await?;
            fixture_consent(h, 70, inviter, CHARACTER, 2).await?;
        }
        assert_eq!(accepted_count(h).await?, 99);
        fixture_character(h, 159, 70).await?;
        fixture_character(h, 160, 80).await?;
        let one = fixture_consent(h, 70, 159, CHARACTER, 1).await?;
        let two = fixture_consent(h, 80, 160, CHARACTER, 1).await?;
        let before_view: u64 = consent_view(h).await?.parse()?;
        let req1 = accept_request(2, invitation(159, 70, one));
        let req2 = accept_request(3, invitation(160, 80, two));
        let (left, right) = both(
            h.root
                .accept_vip_invitation(&a, &h.node, harness::item_fence()?, req1),
            second_root.accept_vip_invitation(&b, &h.node, harness::item_fence()?, req2),
        )
        .await;
        let receipts = [left.map_err(debug)?, right.map_err(debug)?];
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.result == VipResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.result == VipResult::ListFull)
                .count(),
            1
        );
        assert_eq!(accepted_count(h).await?, 100);
        assert_eq!(consent_view(h).await?.parse::<u64>()?, before_view + 1);
        assert_eq!(h.count("game_vip_command_receipts").await?, 3);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

// Add qualification uses actual native Premium snapshot validation+durable fencing+consumer,
// and the genuine closed coordinator. Its explicit registered clock is a TEST time source,
// never reconstructed from a persisted record or passed to Add as a benefit/current bool.
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, clock_registration,
};
use crate::premium::{IngestOutcome, PremiumConsumer, TrustedNow};
use std::sync::{Arc, Mutex};
const PREMIUM_TEST_NOW: i64 = 1_790_769_600_000_000;
struct AddClock(Mutex<Option<TrustedNow>>);
impl clock_registration::Registered for AddClock {}
impl RegisteredPremiumClock for AddClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        *self.0.lock().expect("test clock")
    }
}
fn add_clock() -> Arc<AddClock> {
    Arc::new(AddClock(Mutex::new(TrustedNow::new(PREMIUM_TEST_NOW, 1))))
}
fn add_gate(
    h: &Harness,
    consumer: Arc<PremiumConsumer>,
    clock: Option<Arc<AddClock>>,
    activated: bool,
) -> SpellPremiumCoordinator {
    SpellPremiumCoordinator::new(
        consumer,
        clock.map(|value| value as Arc<dyn RegisteredPremiumClock>),
        activated.then_some(PremiumActivation {
            activation_id: "vip-add-native-test".into(),
            switch_over_us: PREMIUM_TEST_NOW - 60_000_000,
        }),
        Some(h.root.clone()),
    )
}
async fn prove_add_premium(h: &Harness, consumer: &PremiumConsumer) -> TestResult {
    let nonce = "6b1f0c2d9e8a47f3a1b2c3d4e5f60718";
    let response = crate::premium::test_producer::snapshot(
        &crate::premium::test_producer::ProducerRequest {
            account_id: crate::premium::snapshot::canonical_uuid(id(40)),
            nonce: nonce.into(),
        },
        7,
        &[],
    );
    assert!(matches!(
        consumer.ingest(&h.root, id(40), nonce, &response).await,
        IngestOutcome::Accepted | IngestOutcome::Replayed
    ));
    Ok(())
}
fn add_request(number: u64, name: &str) -> VipAddRequest {
    VipAddRequest::new(
        list(40),
        request(number, VipEndOperation::Withdraw { entry_no: 1 }).command,
        name,
    )
}
fn fixture_name(seed: u8) -> String {
    format!(
        "Vip Fixture {}{}",
        char::from(b'A' + seed / 26),
        char::from(b'A' + seed % 26)
    )
}
async fn add_target_fixture(h: &Harness, seed: u8, owner: u8) -> TestResult {
    fixture_character(h, seed, owner).await?;
    // Independently installed historical PARTY baseline DATA, not watcher-derived target
    // authority or a production admission/default fallback. Add itself cannot create this row.
    sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1')")
        .bind(id(seed).as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn add(
    h: &Harness,
    premium: &SpellPremiumCoordinator,
    request: VipAddRequest,
) -> TestResult<VipAddReceipt> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovered = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .add_vip(
            &recovered,
            &h.node,
            harness::item_fence()?,
            premium,
            request,
        )
        .await
        .map_err(|error| debug(error).into())
}
fn run_add<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, false).await?;
        let result = async {
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(version, "170006");
            eprintln!("VIP-ADD-PG server_version_num={version}");
            body(&h).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}
async fn target_revision(h: &Harness, seed: u8) -> TestResult<u64> {
    let value: String = sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(seed).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}
async fn seed_add_entry(h: &Harness, seed: u8, entry: u64) -> TestResult {
    add_target_fixture(h, seed, 70).await?;
    sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric)")
        .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(seed).as_slice()).bind(entry.to_string()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_vip_lists SET next_entry_no=$3::text::numeric WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind((entry+1).to_string()).execute(&h.pool).await?;
    Ok(())
}

#[test]
fn vip_add_native_offline_identity_unknown_privacy_and_receipted_invalid_names() -> TestResult {
    run_add("vipaddnames", async |h| {
        add_target_fixture(h, 60, 70).await?;
        fixture_character_world(h, 61, 70, 90).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        // No target session exists; one native offline root is sufficient, never online lookup.
        let result = add(h, &gate, add_request(1, &fixture_name(60))).await?;
        assert_eq!(
            (result.result, result.entry_no),
            (VipAddResult::Ok, Some(1))
        );
        assert_eq!(target_revision(h, 60).await?, 1);
        let consent = sqlx::query("SELECT consent,invite_expires_at>clock_timestamp()+interval '6 days 23 hours' AS long_enough,invite_expires_at<=clock_timestamp()+interval '7 days' AS bounded FROM game_vip_consents")
            .fetch_one(&h.pool).await?;
        assert_eq!(consent.try_get::<i16, _>("consent")?, 1);
        assert!(
            consent.try_get::<bool, _>("long_enough")? && consent.try_get::<bool, _>("bounded")?
        );
        for (number, name) in [(2, "Unknown Name".to_owned()), (3, fixture_name(61))] {
            let request = add_request(number, &name);
            assert_eq!(
                add(h, &gate, request.clone()).await?.result,
                VipAddResult::UnknownName
            );
            assert!(add(h, &gate, request).await?.replayed);
        }
        for (number, name) in [(4, "x"), (5, "Invalid\nName"), (6, " Name"), (7, "Name1")] {
            let request = add_request(number, name);
            assert_eq!(
                add(h, &gate, request.clone()).await?.result,
                VipAddResult::Rejected
            );
            assert!(add(h, &gate, request).await?.replayed);
        }
        let self_name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            add(h, &gate, add_request(8, &self_name)).await?.result,
            VipAddResult::Rejected
        );
        assert_eq!(h.count("game_vip_command_receipts").await?, 8);
        let tokens: i16 = sqlx::query_scalar("SELECT rate_tokens FROM game_vip_lists")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(tokens, 12);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(h.count("game_vip_consents").await?, 1);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_add_native_free_current_lapse_and_unproven_history_caps() -> TestResult {
    run_add("vipaddpremium", async |h| {
        fixture_list(h, 40).await?;
        for (entry, seed) in (60..79).enumerate() {
            seed_add_entry(h, seed, entry as u64 + 1).await?;
        }
        add_target_fixture(h, 79, 70).await?;
        add_target_fixture(h, 80, 70).await?;
        add_target_fixture(h, 81, 70).await?;
        let consumer = Arc::new(PremiumConsumer::default());
        let clock = add_clock();
        let gate = add_gate(h, Arc::clone(&consumer), Some(Arc::clone(&clock)), true);
        assert_eq!(
            add(h, &gate, add_request(1, &fixture_name(79)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(
            add(h, &gate, add_request(2, &fixture_name(80)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        prove_add_premium(h, &consumer).await?;
        // A durable ACTIVE snapshot loaded on another consumer is not proven in-process.
        let historical = Arc::new(PremiumConsumer::default());
        historical.load(&h.root, id(40)).await?;
        let historical_gate = add_gate(h, historical, Some(Arc::clone(&clock)), true);
        assert_eq!(
            add(h, &historical_gate, add_request(3, &fixture_name(80)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        assert_eq!(
            add(h, &gate, add_request(4, &fixture_name(80)))
                .await?
                .result,
            VipAddResult::Ok
        );
        let ticket = consumer.ticket();
        consumer.pull_failed(id(40), ticket);
        assert_eq!(
            add(h, &gate, add_request(5, &fixture_name(81)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        prove_add_premium(h, &consumer).await?;
        *clock.0.lock().expect("clock") = None;
        assert_eq!(
            add(h, &gate, add_request(6, &fixture_name(81)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        *clock.0.lock().expect("clock") = TrustedNow::new(PREMIUM_TEST_NOW + 7_200_000_000, 1);
        assert_eq!(
            add(h, &gate, add_request(7, &fixture_name(81)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        *clock.0.lock().expect("clock") = TrustedNow::new(PREMIUM_TEST_NOW, 1);
        assert_eq!(
            add(h, &gate, add_request(8, &fixture_name(81)))
                .await?
                .result,
            VipAddResult::Ok
        );
        for (entry, seed) in (82..159).enumerate() {
            seed_add_entry(h, seed, entry as u64 + 23).await?;
        }
        assert_eq!(h.count("game_vip_entries").await?, 99);
        add_target_fixture(h, 159, 70).await?;
        add_target_fixture(h, 160, 70).await?;
        assert_eq!(
            add(h, &gate, add_request(9, &fixture_name(159)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(
            add(h, &gate, add_request(10, &fixture_name(160)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        // No pruning on failure, loss of proof or lapsed authority, including an existing
        // Account-shared row's invitation (no list growth) when over the free cap.
        let ticket = consumer.ticket();
        consumer.pull_failed(id(40), ticket);
        assert_eq!(
            add(h, &gate, add_request(11, &fixture_name(60)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(h.count("game_vip_entries").await?, 100);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_add_native_pre_delivery_exact_100_boundary_uses_real_gate_transaction() -> TestResult {
    run_add("vipaddpredelivery", async |h| {
        fixture_list(h, 40).await?;
        for (entry, seed) in (60..159).enumerate() {
            seed_add_entry(h, seed, entry as u64 + 1).await?;
        }
        add_target_fixture(h, 159, 70).await?;
        add_target_fixture(h, 160, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        assert_eq!(
            add(h, &gate, add_request(1, &fixture_name(159)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(
            add(h, &gate, add_request(2, &fixture_name(160)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        assert_eq!(h.count("game_vip_entries").await?, 100);
        assert_eq!(h.count("game_premium_activation_latch").await?, 0);
        Ok(())
    })
}

#[test]
fn vip_add_native_shared_entry_expired_id_and_block_cap_privacy() -> TestResult {
    run_add("vipaddinvites", async |h| {
        add_target_fixture(h, 60, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        // Existing shared entry through another Character is historical DATA, not our actor's
        // grant. Our own invitation must be produced independently by actual Add.
        fixture_character(h, 61, 40).await?;
        let other = fixture_consent(h, 40, 61, 60, 1).await?;
        let added = add(h, &gate, add_request(1, &fixture_name(60))).await?;
        assert_eq!(added.result, VipAddResult::Ok);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(h.count("game_vip_consents").await?, 2);
        assert_eq!(
            add(h, &gate, add_request(2, &fixture_name(60)))
                .await?
                .result,
            VipAddResult::AlreadyListed
        );
        let old: i64 = sqlx::query_scalar("SELECT invitation_id FROM game_vip_consents WHERE inviter_character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()-interval '1day' WHERE inviter_character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            add(h, &gate, add_request(3, &fixture_name(60)))
                .await?
                .result,
            VipAddResult::Ok
        );
        let new: i64 = sqlx::query_scalar("SELECT invitation_id FROM game_vip_consents WHERE inviter_character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert!(new > old);
        let unchanged: i64 = sqlx::query_scalar("SELECT invitation_id FROM game_vip_consents WHERE inviter_character_id=encode($1,'hex')::uuid")
            .bind(id(61).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(unchanged as u64, other);
        // Same Account's other root is a permitted target, but is never autoaccepted.
        add_target_fixture(h, 62, 40).await?;
        assert_eq!(
            add(h, &gate, add_request(4, &fixture_name(62)))
                .await?
                .result,
            VipAddResult::Ok
        );
        add_target_fixture(h, 63, 70).await?;
        sqlx::query("INSERT INTO game_character_social_blocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(id(63).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            add(h, &gate, add_request(5, &fixture_name(63)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(target_revision(h, 63).await?, 0);
        add_target_fixture(h, 64, 70).await?;
        for inviter in 70..90 {
            fixture_character(h, inviter, 80).await?;
            fixture_consent(h, 80, inviter, 64, 1).await?;
        }
        let before = target_revision(h, 64).await?;
        assert_eq!(
            add(h, &gate, add_request(6, &fixture_name(64)))
                .await?
                .result,
            VipAddResult::Ok
        );
        assert_eq!(target_revision(h, 64).await?, before);
        let own: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND inviter_character_id=encode($3,'hex')::uuid")
            .bind(id(63).as_slice()).bind(id(64).as_slice()).bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(own, 0);
        Ok(())
    })
}

#[test]
fn vip_add_native_callback_rollback_missing_baseline_and_replay_before_new_fences() -> TestResult {
    run_add("vipaddrollback", async |h| {
        fixture_character(h, 60, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = add_request(1, &fixture_name(60));
        let before = footprint(h).await?;
        // The missing baseline is now created atomically with the add (PARTYPVP0 §3).
        let baseline_count = || async {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(60).as_slice()).fetch_one(&h.pool).await
        };
        assert_eq!(baseline_count().await?, 0);
        h.pool.execute("CREATE FUNCTION vip_add_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected after entry' USING ERRCODE='23514';END;$$;CREATE TRIGGER vip_add_fault AFTER INSERT ON game_vip_consents FOR EACH ROW EXECUTE FUNCTION vip_add_fault()").await?;
        assert!(
            h.root
                .add_vip(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    &gate,
                    request.clone()
                )
                .await
                .is_err()
        );
        assert_eq!(footprint(h).await?, before);
        assert_eq!(h.count("game_vip_lists").await?, 0);
        // An absent baseline has revision zero; inspecting it must not create the row.
        let target_revision: String = sqlx::query_scalar("SELECT COALESCE((SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid),'0')")
            .bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(target_revision, "0");
        assert_eq!(baseline_count().await?, 0);
        h.pool
            .execute(
                "DROP TRIGGER vip_add_fault ON game_vip_consents;DROP FUNCTION vip_add_fault()",
            )
            .await?;
        let committed = h
            .root
            .add_vip(
                &authority,
                &h.node,
                harness::item_fence()?,
                &gate,
                request.clone(),
            )
            .await
            .map_err(debug)?;
        let baseline: (i16, i16, String) = sqlx::query_as("SELECT party_invites,channel_visibility,source_policy FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(baseline, (1, 1, "PARTYPVP0-PARTIES-AND-PVP-V1".to_owned()));
        let after = footprint(h).await?;
        sqlx::query("UPDATE game_durability_admission_runtime_guards SET ready=FALSE")
            .execute(&h.pool)
            .await?;
        let mut stale = harness::item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(2)?;
        // An undelivered/missing-Premium node cannot prevent historical receipt replay.
        let unread =
            SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, None);
        let replay = h
            .root
            .add_vip(&authority, &h.node, stale, &unread, request.clone())
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.result, committed.result);
        assert_eq!(replay.revision, committed.revision);
        assert_eq!(footprint(h).await?, after);
        let changed = add_request(1, "Another Name");
        assert!(matches!(
            h.root
                .add_vip(&authority, &h.node, stale, &unread, changed)
                .await,
            Err(VipError::ConflictingCommand)
        ));
        assert_eq!(footprint(h).await?, after);
        Ok(())
    })
}

#[test]
fn vip_add_native_current_fences_and_actual_assignment_revocation_write_nothing() -> TestResult {
    run_add("vipaddfences", async |h| {
        add_target_fixture(h, 60, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = add_request(1, &fixture_name(60));
        let before = footprint(h).await?;
        for mutation in 0..6 {
            let mut f = harness::item_fence()?;
            let mut req = request.clone();
            match mutation {
                0 => f.connection_generation = ConnectionGeneration::new(2)?,
                1 => f.character_lease_generation = 2,
                2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => req.list.account_id = account(70),
                4 => req.list.world_id = WorldId::from_bytes(id(90)).map_err(debug)?,
                _ => {
                    req.command =
                        CommandRef::new(GameSessionId::decode(&id(51))?, CommandId::new(1)?)
                }
            }
            assert!(matches!(
                h.root.add_vip(&authority, &h.node, f, &gate, req).await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(footprint(h).await?, before);
        }
        let wrong_node = harness::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .add_vip(
                    &authority,
                    &wrong_node,
                    harness::item_fence()?,
                    &gate,
                    request.clone()
                )
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        // Keep the caller/fence/proof untouched; revoke independently current assignment via
        // its genuine control writer, with actual authenticated role and actual predecessor.
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        };
        let scope = harness::item_fence()?.runtime_scope;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope)
            .await
            .map_err(debug)?
            .ok_or("native current predecessor missing")?;
        let role: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        let revoked = h
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([88; 32]),
                actor: ControlActor::new(&role).map_err(debug)?,
                command: AssignmentCommand::Revoke { scope, predecessor },
            })
            .await
            .map_err(debug)?;
        assert!(
            matches!(revoked, AssignmentOutcome::Committed(_)),
            "revoke={revoked:?}"
        );
        assert!(matches!(
            h.root
                .add_vip(&authority, &h.node, harness::item_fence()?, &gate, request)
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        assert_eq!(h.count("game_vip_lists").await?, 0);
        Ok(())
    })
}

#[test]
fn vip_add_native_two_root_duplicate_and_free_cap_races() -> TestResult {
    run_add("vipaddraces", async |h| {
        add_target_fixture(h, 60, 70).await?;
        let second = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first_authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let gate1 = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let gate2 = SpellPremiumCoordinator::new(
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            None,
            Some(second.clone()),
        );
        let req = add_request(1, &fixture_name(60));
        let (a, b) = both(
            h.root.add_vip(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                &gate1,
                req.clone(),
            ),
            second.add_vip(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                &gate2,
                req,
            ),
        )
        .await;
        let results = [a.map_err(debug)?, b.map_err(debug)?];
        assert!(results.iter().all(|r| r.result == VipAddResult::Ok));
        assert_eq!(results.iter().filter(|r| r.replayed).count(), 1);
        assert_eq!(footprint(h).await?, (1, 1, 1, "1".into()));
        assert_eq!(target_revision(h, 60).await?, 1);
        for (entry, seed) in (61..79).enumerate() {
            seed_add_entry(h, seed, entry as u64 + 2).await?;
        }
        add_target_fixture(h, 79, 70).await?;
        add_target_fixture(h, 80, 70).await?;
        let gate1 = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            true,
        );
        let gate2 = SpellPremiumCoordinator::new(
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            Some(PremiumActivation {
                activation_id: "vip-add-native-test".into(),
                switch_over_us: PREMIUM_TEST_NOW - 60_000_000,
            }),
            Some(second.clone()),
        );
        let (a, b) = both(
            h.root.add_vip(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                &gate1,
                add_request(2, &fixture_name(79)),
            ),
            second.add_vip(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                &gate2,
                add_request(3, &fixture_name(80)),
            ),
        )
        .await;
        let results = [a.map_err(debug)?, b.map_err(debug)?];
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == VipAddResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == VipAddResult::ListFull)
                .count(),
            1
        );
        assert_eq!(h.count("game_vip_entries").await?, 20);
        assert_eq!(h.count("game_vip_command_receipts").await?, 3);
        assert_eq!(
            target_revision(h, 79).await? + target_revision(h, 80).await?,
            1
        );
        Ok(())
    })
}

async fn advisory_waiter(h: &Harness, key: i64, mode: &str) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            // Other isolated fixture databases use the same native Premium key. Only an
            // actual waiter in this fixture database establishes this race's barrier.
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND locktype='advisory' AND classid=$1::bigint::oid AND objid=$2::bigint::oid AND objsubid=1 AND mode=$3 AND NOT granted)")
                .bind((key as u64>>32) as i64).bind((key as u64&0xffff_ffff)as i64).bind(mode)
                .fetch_one(&h.pool).await?;
            if waiting { return Ok::<(),sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    Ok(())
}

#[test]
fn vip_add_native_pre_delivery_write_and_actual_latch_writer_serialize() -> TestResult {
    run_add("vipaddlatch", async |h| {
        add_target_fixture(h, 60, 70).await?;
        const BARRIER: i64 = 0x5649_5041_4444;
        use crate::durability::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY as KEY;
        let mut barrier = h.pool.acquire().await?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(BARRIER)
            .execute(&mut *barrier)
            .await?;
        // Validation-only observer: refuses if the actual entry writer is not inside the
        // physical backend/TX holding the genuine Premium shared latch lock.
        let observer = format!(
            "CREATE FUNCTION vip_add_barrier() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='advisory' AND classid={}::oid AND objid={}::oid AND objsubid=1 AND mode='ShareLock' AND granted) THEN RAISE EXCEPTION 'Add lost native latch TX' USING ERRCODE='23514';END IF;PERFORM pg_advisory_xact_lock({BARRIER});RETURN NEW;END;$$;CREATE TRIGGER vip_add_barrier AFTER INSERT ON game_vip_entries FOR EACH ROW EXECUTE FUNCTION vip_add_barrier()",
            (KEY as u64) >> 32,
            (KEY as u64) & 0xffff_ffff
        );
        // The only interpolated values are numeric fixture/native lock constants, never
        // request data. Pinned SQLx requires its explicit owned dynamic-SQL marker.
        sqlx::raw_sql(sqlx::AssertSqlSafe(observer))
            .execute(&h.pool)
            .await?;
        let latch_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(latch_root.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let latch_flow = async {
            advisory_waiter(h, BARRIER, "ExclusiveLock").await?;
            let release = async {
                advisory_waiter(h, KEY, "ExclusiveLock").await?;
                sqlx::query("SELECT pg_advisory_unlock($1)")
                    .bind(BARRIER)
                    .execute(&mut *barrier)
                    .await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let (written, released) = both(
                latch_root.latch_premium_activation(
                    "vip-add-native-test",
                    PREMIUM_TEST_NOW - 60_000_000,
                    PREMIUM_TEST_NOW,
                ),
                release,
            )
            .await;
            released?;
            written.map_err(debug)?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let (added, latched) = both(
            h.root.add_vip(
                &authority,
                &h.node,
                harness::item_fence()?,
                &gate,
                add_request(1, &fixture_name(60)),
            ),
            latch_flow,
        )
        .await;
        latched?;
        assert_eq!(added.map_err(debug)?.result, VipAddResult::Ok);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(h.count("game_vip_consents").await?, 1);
        assert_eq!(h.count("game_vip_command_receipts").await?, 1);
        assert_eq!(h.count("game_premium_activation_latch").await?, 1);
        // A subsequent actual free command cannot still use the earlier empty-latch bypass.
        h.pool
            .execute(
                "DROP TRIGGER vip_add_barrier ON game_vip_entries;DROP FUNCTION vip_add_barrier()",
            )
            .await?;
        for (entry, seed) in (61..80).enumerate() {
            seed_add_entry(h, seed, entry as u64 + 2).await?;
        }
        add_target_fixture(h, 80, 70).await?;
        assert_eq!(
            add(h, &gate, add_request(2, &fixture_name(80)))
                .await?
                .result,
            VipAddResult::ListFull
        );
        assert_eq!(h.count("game_vip_entries").await?, 20);
        Ok(())
    })
}

// Remove fixtures append after the complete, unchanged original 25-test tail.
// run_add enforces the real configured PG17.6 route even for these non-Premium commands.
fn remove_request(number: u64, entry_no: u64) -> VipRemoveRequest {
    VipRemoveRequest {
        list: list(40),
        command: withdraw(number).command,
        entry_no,
    }
}
async fn remove(h: &Harness, request: VipRemoveRequest) -> TestResult<VipReceipt> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .remove_vip_entry(&authority, &h.node, harness::item_fence()?, request)
        .await
        .map_err(|error| debug(error).into())
}
async fn remove_snapshot(h: &Harness) -> TestResult<(String, String)> {
    // Complete retained rows, not just a count: includes accepted state, target view,
    // list allocator/rate/revision, memberships and receipt bindings/timestamps.
    let rows: String = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('entry',to_jsonb(t)) x FROM game_vip_entries t UNION ALL SELECT jsonb_build_array('consent',to_jsonb(t)) FROM game_vip_consents t UNION ALL SELECT jsonb_build_array('list',to_jsonb(t)) FROM game_vip_lists t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_vip_group_members t UNION ALL SELECT jsonb_build_array('receipt',to_jsonb(t)) FROM game_vip_command_receipts t UNION ALL SELECT jsonb_build_array('view',to_jsonb(t)) FROM game_character_social_settings t) r")
        .fetch_one(&h.pool).await?;
    Ok((rows, h.root_revision().await?))
}
async fn own_entry_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn vip_remove_native_account_cascade_retains_groups_other_entries_and_foreign_consent() -> TestResult
{
    run_add("viprmcascade", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        add_target_fixture(h, 60, 70).await?;
        add_target_fixture(h, 61, 70).await?;
        fixture_character(h, 62, 40).await?;
        fixture_character(h, 63, 80).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        assert_eq!(
            add(h, &gate, add_request(1, &fixture_name(60)))
                .await?
                .entry_no,
            Some(1)
        );
        assert_eq!(
            add(h, &gate, add_request(2, &fixture_name(61)))
                .await?
                .entry_no,
            Some(2)
        );
        fixture_consent(h, 40, 62, 60, 2).await?;
        fixture_consent(h, 80, 63, 60, 2).await?;
        // A custom group, to prove the entry FK cascades memberships, not group definitions.
        sqlx::query("INSERT INTO game_vip_groups VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,4,'Test Friends')")
            .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
        for seed in [60, 61] {
            sqlx::query("INSERT INTO game_vip_group_members VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,4)")
                .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(seed).as_slice()).execute(&h.pool).await?;
        }
        let before_view = target_revision(h, 60).await?;
        let other_view = target_revision(h, 61).await?;
        let before_revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(40).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let accepted_before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE consent=2")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(accepted_before, 2);
        let committed = remove(h, remove_request(3, 1)).await?;
        assert_eq!(committed.result, VipResult::Ok);
        assert_eq!(committed.revision, before_revision.parse::<u64>()? + 1);
        assert_eq!(target_revision(h, 60).await?, before_view + 1);
        assert_eq!(target_revision(h, 61).await?, other_view);
        assert_eq!(own_entry_count(h).await?, 1);
        assert_eq!(h.count("game_vip_groups").await?, 1);
        let retained_group: (i16, String) =
            sqlx::query_as("SELECT group_no,name FROM game_vip_groups")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(retained_group, (4, "Test Friends".into()));
        assert_eq!(h.count("game_vip_group_members").await?, 1);
        let retained: Vec<(String, String, i16)> = sqlx::query_as("SELECT account_id::text,character_id::text,consent FROM game_vip_consents ORDER BY account_id,character_id").fetch_all(&h.pool).await?;
        assert_eq!(retained.len(), 2);
        assert_eq!(retained.iter().filter(|row| row.2 == 2).count(), 1);
        let untouched_foreign: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND consent=2")
            .bind(id(80).as_slice()).bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(untouched_foreign, 1);
        let retained_member: Vec<u8> = sqlx::query_scalar(
            "SELECT decode(replace(character_id::text,'-',''),'hex') FROM game_vip_group_members",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(retained_member.as_slice(), id(61).as_slice());
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_remove_native_zero_consent_expired_blocked_and_overcap_need_no_premium() -> TestResult {
    run_add("viprmcaps", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        fixture_character(h, 60, 70).await?;
        fixture_list(h, 40).await?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)")
            .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        let settings = h.count("game_character_social_settings").await?;
        assert_eq!(remove(h, remove_request(1, 1)).await?.result, VipResult::Ok);
        assert_eq!(h.count("game_character_social_settings").await?, settings);
        fixture_character(h, 61, 70).await?;
        fixture_consent(h, 40, CHARACTER, 61, 1).await?;
        sqlx::query(
            "UPDATE game_vip_consents SET invite_expires_at=clock_timestamp()-interval '1 day'",
        )
        .execute(&h.pool)
        .await?;
        // HIDDEN is visibility DATA, not a PARTY block. Install the actual independent
        // PARTY block too, and prove Remove neither consults it nor deletes it.
        sqlx::query("UPDATE game_character_social_settings SET vip_status=2 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(61).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_social_blocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(id(61).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        let before = target_revision(h, 61).await?;
        assert_eq!(remove(h, remove_request(2, 1)).await?.result, VipResult::Ok);
        assert_eq!(target_revision(h, 61).await?, before + 1);
        assert_eq!(h.count("game_character_social_blocks").await?, 1);
        assert_eq!(h.count("game_vip_consents").await?, 0);
        for (index, seed) in (62..83).enumerate() {
            seed_add_entry(h, seed, index as u64 + 2).await?;
        }
        assert_eq!(own_entry_count(h).await?, 21);
        assert_eq!(remove(h, remove_request(3, 2)).await?.result, VipResult::Ok);
        assert_eq!(own_entry_count(h).await?, 20);
        assert_eq!(h.count("game_premium_activation_latch").await?, 0);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_remove_native_unknown_rate_replay_and_never_reused_entry_identity() -> TestResult {
    run_add("viprmreplay", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        add_target_fixture(h, 60, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let original = add(h, &gate, add_request(1, &fixture_name(60))).await?;
        let old_invitation: i64 = sqlx::query_scalar("SELECT invitation_id FROM game_vip_consents")
            .fetch_one(&h.pool)
            .await?;
        let removed = remove(
            h,
            remove_request(2, original.entry_no.ok_or("entry missing")?),
        )
        .await?;
        let removed_state = remove_snapshot(h).await?;
        let replay = remove(h, remove_request(2, 1)).await?;
        assert!(replay.replayed);
        assert_eq!(
            (replay.result, replay.revision),
            (removed.result, removed.revision)
        );
        let old_add = add(h, &gate, add_request(1, &fixture_name(60))).await?;
        assert!(old_add.replayed);
        assert_eq!(old_add.entry_no, original.entry_no);
        assert_eq!(remove_snapshot(h).await?, removed_state);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(2, 2)
                )
                .await,
            Err(VipError::ConflictingCommand)
        ));
        assert!(matches!(
            h.root
                .end_vip_consent(&authority, &h.node, harness::item_fence()?, withdraw(2))
                .await,
            Err(VipError::ConflictingCommand)
        ));
        let fresh = add(h, &gate, add_request(3, &fixture_name(60))).await?;
        assert!(fresh.entry_no > original.entry_no);
        let new_invitation: i64 = sqlx::query_scalar("SELECT invitation_id FROM game_vip_consents")
            .fetch_one(&h.pool)
            .await?;
        assert!(new_invitation > old_invitation);
        for (command, entry) in [(4, 1), (5, 0), (6, u64::MAX)] {
            assert_eq!(
                remove(h, remove_request(command, entry)).await?.result,
                VipResult::NotFound
            );
            assert_eq!(own_entry_count(h).await?, 1);
        }
        let view = target_revision(h, 60).await?;
        sqlx::query("UPDATE game_vip_lists SET rate_tokens=0,rate_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint").execute(&h.pool).await?;
        let exhausted = remove(
            h,
            remove_request(7, fresh.entry_no.ok_or("new entry missing")?),
        )
        .await?;
        assert!(matches!(
            exhausted.result,
            VipResult::Exhausted { seconds: 1..=3 }
        ));
        assert_eq!(own_entry_count(h).await?, 1);
        assert_eq!(target_revision(h, 60).await?, view);
        let before = remove_snapshot(h).await?;
        let replay = remove(
            h,
            remove_request(7, fresh.entry_no.ok_or("new entry missing")?),
        )
        .await?;
        assert!(replay.replayed);
        assert_eq!(replay.result, exhausted.result);
        assert_eq!(remove_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn vip_remove_native_missing_view_overflow_and_after_delete_fault_roll_back_all_rows() -> TestResult
{
    run_add("viprmrollback", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 2).await?;
        sqlx::query("INSERT INTO game_vip_group_members VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)")
            .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        sqlx::query(
            "DELETE FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&h.pool)
        .await?;
        let before = remove_snapshot(h).await?;
        assert!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(1, 1)
                )
                .await
                .is_err()
        );
        assert_eq!(remove_snapshot(h).await?, before);
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy,vip_consent_revision) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1',18446744073709551615)")
            .bind(id(60).as_slice()).execute(&h.pool).await?;
        let before = remove_snapshot(h).await?;
        assert!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(1, 1)
                )
                .await
                .is_err()
        );
        assert_eq!(remove_snapshot(h).await?, before);
        sqlx::query("UPDATE game_character_social_settings SET vip_consent_revision=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).execute(&h.pool).await?;
        h.pool.execute("CREATE FUNCTION vip_remove_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected after delete' USING ERRCODE='23514';END;$$;CREATE TRIGGER vip_remove_fault AFTER DELETE ON game_vip_entries FOR EACH ROW EXECUTE FUNCTION vip_remove_fault()").await?;
        let before = remove_snapshot(h).await?;
        assert!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(1, 1)
                )
                .await
                .is_err()
        );
        assert_eq!(remove_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER vip_remove_fault ON game_vip_entries;DROP FUNCTION vip_remove_fault()").await?;
        assert_eq!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(1, 1)
                )
                .await
                .map_err(debug)?
                .result,
            VipResult::Ok
        );
        assert_eq!(target_revision(h, 60).await?, 2);
        assert_eq!(h.count("game_vip_consents").await?, 0);
        assert_eq!(h.count("game_vip_group_members").await?, 0);
        assert_eq!(h.count("game_vip_command_receipts").await?, 1);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_remove_native_current_fences_assignment_revoke_and_recovery_before_replay() -> TestResult {
    run_add("viprmfences", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        fixture_character(h, 60, 70).await?;
        fixture_consent(h, 40, CHARACTER, 60, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = remove_snapshot(h).await?;
        for mutation in 0..8 {
            let mut fence = harness::item_fence()?;
            let mut request = remove_request(1, 1);
            match mutation {
                0 => fence.connection_generation = ConnectionGeneration::new(2)?,
                1 => fence.character_lease_generation = 2,
                2 => fence.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => request.list.account_id = account(70),
                4 => request.list.world_id = WorldId::from_bytes(id(90)).map_err(debug)?,
                5 => {
                    request.command =
                        CommandRef::new(GameSessionId::decode(&id(51))?, CommandId::new(1)?)
                }
                6 => fence.game_session_id = GameSessionId::decode(&id(51))?,
                _ => {
                    fence.runtime_scope = RuntimeScopeRefV1::channel(
                        crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
                        crate::foundation::ChannelId::decode(&id(90)).map_err(debug)?,
                    )
                }
            }
            assert!(matches!(
                h.root
                    .remove_vip_entry(&authority, &h.node, fence, request)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(remove_snapshot(h).await?, before);
        }
        let wrong_node = harness::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &wrong_node,
                    harness::item_fence()?,
                    remove_request(1, 1)
                )
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(remove_snapshot(h).await?, before);
        // Actual existing entry in another Account cannot be selected by its entry number.
        fixture_character(h, 61, 70).await?;
        fixture_consent(h, 70, 61, 60, 1).await?;
        let committed = remove(h, remove_request(1, 1)).await?;
        assert_eq!(committed.result, VipResult::Ok);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(
            remove(h, remove_request(2, 1)).await?.result,
            VipResult::NotFound
        );
        // Keep an independently stored, actually removable own entry present across
        // current assignment loss; historical receipt must not erase this replacement.
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,2)")
            .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        sqlx::query("UPDATE game_vip_lists SET next_entry_no=3 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(id(40).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
        let after = remove_snapshot(h).await?;
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        };
        let scope = harness::item_fence()?.runtime_scope;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope)
            .await
            .map_err(debug)?
            .ok_or("current predecessor missing")?;
        let role: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        let revoked = h
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([89; 32]),
                actor: ControlActor::new(&role).map_err(debug)?,
                command: AssignmentCommand::Revoke { scope, predecessor },
            })
            .await
            .map_err(debug)?;
        assert!(
            matches!(revoked, AssignmentOutcome::Committed(_)),
            "revoke={revoked:?}"
        );
        assert!(matches!(
            h.root
                .remove_vip_entry(
                    &authority,
                    &h.node,
                    harness::item_fence()?,
                    remove_request(3, 2)
                )
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(remove_snapshot(h).await?, after);
        let historical = h
            .root
            .remove_vip_entry(
                &authority,
                &h.node,
                harness::item_fence()?,
                remove_request(1, 1),
            )
            .await
            .map_err(debug)?;
        assert!(historical.replayed);
        assert_eq!(
            (historical.result, historical.revision),
            (committed.result, committed.revision)
        );
        let mut wrong_recovery = recovery_record(&authority, &h.root).map_err(debug)?;
        wrong_recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            end_in_transaction(
                &mut tx,
                &wrong_recovery,
                &h.node,
                harness::item_fence()?,
                request(1, VipEndOperation::Remove { entry_no: 1 })
            )
            .await,
            Err(VipError::Unavailable(_))
        ));
        tx.rollback().await?;
        assert_eq!(remove_snapshot(h).await?, after);
        Ok(())
    })
}

#[test]
fn vip_remove_native_two_root_duplicate_distinct_and_add_remove_serialization() -> TestResult {
    run_add("viprmraces", async |h| {
        eprintln!("VIP-REMOVE-PG server_version_num=170006");
        add_target_fixture(h, 60, 70).await?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        add(h, &gate, add_request(1, &fixture_name(60))).await?;
        let second = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        // Provision real recovery sequentially before racing the two ready native Roots.
        let first_authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (a, b) = both(
            h.root.remove_vip_entry(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                remove_request(2, 1),
            ),
            second.remove_vip_entry(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                remove_request(2, 1),
            ),
        )
        .await;
        let results = [a.map_err(debug)?, b.map_err(debug)?];
        assert!(results.iter().all(|r| r.result == VipResult::Ok));
        assert_eq!(results.iter().filter(|r| r.replayed).count(), 1);
        assert_eq!(h.count("game_vip_command_receipts").await?, 2);
        assert_eq!(target_revision(h, 60).await?, 2);
        let fresh = add(h, &gate, add_request(3, &fixture_name(60)))
            .await?
            .entry_no
            .ok_or("fresh entry missing")?;
        let view = target_revision(h, 60).await?;
        let (a, b) = both(
            h.root.remove_vip_entry(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                remove_request(4, fresh),
            ),
            second.remove_vip_entry(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                remove_request(5, fresh),
            ),
        )
        .await;
        let results = [a.map_err(debug)?, b.map_err(debug)?];
        assert_eq!(
            results.iter().filter(|r| r.result == VipResult::Ok).count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == VipResult::NotFound)
                .count(),
            1
        );
        assert!(results.iter().all(|r| !r.replayed));
        assert_eq!(target_revision(h, 60).await?, view + 1);
        let entry = add(h, &gate, add_request(6, &fixture_name(60)))
            .await?
            .entry_no
            .ok_or("race entry missing")?;
        let view = target_revision(h, 60).await?;
        let second_gate = SpellPremiumCoordinator::new(
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            None,
            Some(second.clone()),
        );
        let (removed, added) = both(
            h.root.remove_vip_entry(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                remove_request(7, entry),
            ),
            second.add_vip(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                &second_gate,
                add_request(8, &fixture_name(60)),
            ),
        )
        .await;
        let removed = removed.map_err(debug)?;
        let added = added.map_err(debug)?;
        assert_eq!(removed.result, VipResult::Ok);
        match added.result {
            VipAddResult::Ok => {
                assert!(added.entry_no.is_some_and(|new| new > entry));
                assert_eq!(own_entry_count(h).await?, 1);
                assert_eq!(h.count("game_vip_consents").await?, 1);
                assert_eq!(target_revision(h, 60).await?, view + 2);
            }
            VipAddResult::AlreadyListed => {
                assert_eq!(added.entry_no, Some(entry));
                assert_eq!(own_entry_count(h).await?, 0);
                assert_eq!(h.count("game_vip_consents").await?, 0);
                assert_eq!(target_revision(h, 60).await?, view + 1);
            }
            other => return Err(format!("unexpected serialized Add: {other:?}").into()),
        }
        assert_eq!(h.count("game_vip_command_receipts").await?, 8);
        assert_eq!(h.root_revision().await?, "1");
        // Native admission-relation locks serialize these real semantic transactions;
        // this is not a claim of isolated list-row contention or production load testing.
        Ok(())
    })
}

fn run_list<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    run_add(tag, body)
}
fn list_request(number: u64, operation: VipListOperation) -> VipListRequest {
    VipListRequest {
        list: list(40),
        command: withdraw(number).command,
        operation,
    }
}
async fn modify(
    h: &Harness,
    premium: &SpellPremiumCoordinator,
    request: VipListRequest,
) -> TestResult<VipListReceipt> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .modify_vip_list(
            &authority,
            &h.node,
            harness::item_fence()?,
            premium,
            request,
        )
        .await
        .map_err(|error| debug(error).into())
}
async fn list_snapshot(h: &Harness) -> TestResult<((String, String), String)> {
    let groups: String = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(g) ORDER BY account_id,world_id,group_no),'[]'::jsonb)::text FROM game_vip_groups g").fetch_one(&h.pool).await?;
    Ok((remove_snapshot(h).await?, groups))
}
fn unread_gate() -> SpellPremiumCoordinator {
    SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, None)
}
#[test]
fn vip_list_text_and_group_validation_boundaries() {
    use crate::domain::vip::{vip_edit_description, vip_group_name};
    assert_eq!(
        vip_group_name("  Friends  "),
        Some(("Friends".into(), "friends".into()))
    );
    for name in ["", "   ", "bad\nname", "12345678901234567890123456"] {
        assert_eq!(vip_group_name(name), None);
    }
    assert!(vip_group_name(&"ą".repeat(25)).is_some());
    assert_eq!(
        vip_edit_description(" trim ", 10, &[1, 4, 8]),
        Some("trim".into())
    );
    assert!(vip_edit_description(&"🦀".repeat(128), 0, &[]).is_some());
    assert!(vip_edit_description(&"a".repeat(129), 0, &[]).is_none());
    for (text, icon, groups) in [
        ("bad\ntext", 0, vec![]),
        ("", 11, vec![]),
        ("", 0, vec![4, 4]),
        ("", 0, vec![0]),
        ("", 0, vec![9]),
    ] {
        assert!(vip_edit_description(text, icon, &groups).is_none());
    }
}
#[test]
fn vip_list_native_groups_fixed_bounds_casefold_cap_and_remove_memberships() -> TestResult {
    run_list("viplistgroups", async |h| {
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(
                    1,
                    VipListOperation::GroupAdd {
                        name: "Friends".into()
                    }
                )
            )
            .await?
            .group_no,
            Some(4)
        );
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(
                    2,
                    VipListOperation::GroupAdd {
                        name: "FRIENDS".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::NameTaken
        );
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(
                    3,
                    VipListOperation::GroupRename {
                        group_no: 4,
                        name: "New name".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::Ok
        );
        for (command, group_no) in [(4, 1), (5, 3), (6, 9)] {
            assert_eq!(
                modify(
                    h,
                    &gate,
                    list_request(command, VipListOperation::GroupRemove { group_no })
                )
                .await?
                .result,
                VipListResult::Rejected
            );
        }
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(7, VipListOperation::GroupRemove { group_no: 8 })
            )
            .await?
            .result,
            VipListResult::NotFound
        );
        for number in 5..=8 {
            assert_eq!(
                modify(
                    h,
                    &gate,
                    list_request(
                        u64::from(number) + 3,
                        VipListOperation::GroupAdd {
                            name: format!("Group {number}")
                        }
                    )
                )
                .await?
                .group_no,
                Some(number)
            );
        }
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(
                    12,
                    VipListOperation::GroupAdd {
                        name: "Overflow".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::GroupLimit
        );
        // VIP0 §6 says rename waits until under the current cap. This pins the
        // conservative count==5 interpretation pending clarification of its "Over" heading.
        assert_eq!(
            modify(
                h,
                &gate,
                list_request(
                    16,
                    VipListOperation::GroupRename {
                        group_no: 4,
                        name: "At capacity".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::GroupLimit
        );
        add_target_fixture(h, 60, 70).await?;
        add(h, &gate, add_request(13, &fixture_name(60))).await?;
        assert_eq!(
            modify(
                h,
                &unread_gate(),
                list_request(
                    14,
                    VipListOperation::Edit {
                        entry_no: 1,
                        description: " note ".into(),
                        icon: 10,
                        notify: true,
                        groups: vec![1, 4, 8]
                    }
                )
            )
            .await?
            .result,
            VipListResult::Ok
        );
        let view = target_revision(h, 60).await?;
        let removed = modify(
            h,
            &unread_gate(),
            list_request(15, VipListOperation::GroupRemove { group_no: 4 }),
        )
        .await?;
        assert_eq!(removed.result, VipListResult::Ok);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        assert_eq!(h.count("game_vip_consents").await?, 1);
        assert_eq!(h.count("game_vip_group_members").await?, 2);
        assert_eq!(target_revision(h, 60).await?, view);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}
#[test]
fn vip_list_native_free_retains_data_edits_removes_and_replays_without_premium() -> TestResult {
    run_list("viplistfree", async |h| {
        let pre = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        modify(
            h,
            &pre,
            list_request(
                1,
                VipListOperation::GroupAdd {
                    name: "Retained".into(),
                },
            ),
        )
        .await?;
        add_target_fixture(h, 60, 70).await?;
        add(h, &pre, add_request(2, &fixture_name(60))).await?;
        let free = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            true,
        );
        assert_eq!(
            modify(
                h,
                &free,
                list_request(3, VipListOperation::GroupAdd { name: "New".into() })
            )
            .await?
            .result,
            VipListResult::GroupLimit
        );
        assert_eq!(
            modify(
                h,
                &free,
                list_request(
                    4,
                    VipListOperation::GroupRename {
                        group_no: 4,
                        name: "Renamed".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::GroupLimit
        );
        assert_eq!(h.count("game_vip_groups").await?, 1);
        let edit = list_request(
            5,
            VipListOperation::Edit {
                entry_no: 1,
                description: "Changed".into(),
                icon: 2,
                notify: false,
                groups: vec![4],
            },
        );
        let committed = modify(h, &unread_gate(), edit.clone()).await?;
        assert_eq!(committed.result, VipListResult::Ok);
        let after = list_snapshot(h).await?;
        let replay = modify(h, &unread_gate(), edit).await?;
        assert!(replay.replayed);
        assert_eq!(replay.revision, committed.revision);
        assert_eq!(list_snapshot(h).await?, after);
        assert_eq!(
            modify(
                h,
                &unread_gate(),
                list_request(6, VipListOperation::GroupRemove { group_no: 4 })
            )
            .await?
            .result,
            VipListResult::Ok
        );
        assert_eq!(h.count("game_vip_group_members").await?, 0);
        assert_eq!(h.count("game_vip_entries").await?, 1);
        Ok(())
    })
}
#[test]
fn vip_list_native_edit_unknown_group_and_injected_fault_are_atomic() -> TestResult {
    run_list("viplistatomic", async |h| {
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        add_target_fixture(h, 60, 70).await?;
        add(h, &gate, add_request(1, &fixture_name(60))).await?;
        modify(
            h,
            &gate,
            list_request(
                2,
                VipListOperation::GroupAdd {
                    name: "Stored".into(),
                },
            ),
        )
        .await?;
        assert_eq!(
            modify(
                h,
                &unread_gate(),
                list_request(
                    3,
                    VipListOperation::Edit {
                        entry_no: 1,
                        description: "Will not apply".into(),
                        icon: 1,
                        notify: true,
                        groups: vec![1, 8]
                    }
                )
            )
            .await?
            .result,
            VipListResult::NotFound
        );
        let description: String = sqlx::query_scalar("SELECT description FROM game_vip_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(description, "");
        h.pool.execute("CREATE FUNCTION vip_edit_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected membership fault' USING ERRCODE='23514';END;$$;CREATE TRIGGER vip_edit_fault AFTER INSERT ON game_vip_group_members FOR EACH ROW EXECUTE FUNCTION vip_edit_fault()").await?;
        let before = list_snapshot(h).await?;
        assert!(
            modify(
                h,
                &unread_gate(),
                list_request(
                    4,
                    VipListOperation::Edit {
                        entry_no: 1,
                        description: "Must roll back".into(),
                        icon: 2,
                        notify: true,
                        groups: vec![1, 4]
                    }
                )
            )
            .await
            .is_err()
        );
        assert_eq!(list_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER vip_edit_fault ON game_vip_group_members;DROP FUNCTION vip_edit_fault()").await?;
        // A receipt insertion fault after GroupAdd also rolls back its pre-delivery savepoint.
        h.pool.execute("CREATE FUNCTION vip_receipt_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected receipt fault' USING ERRCODE='23514';END;$$;CREATE TRIGGER vip_receipt_fault AFTER INSERT ON game_vip_command_receipts FOR EACH ROW EXECUTE FUNCTION vip_receipt_fault()").await?;
        assert!(
            modify(
                h,
                &gate,
                list_request(
                    5,
                    VipListOperation::GroupAdd {
                        name: "Must roll back".into()
                    }
                )
            )
            .await
            .is_err()
        );
        assert_eq!(list_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn vip_list_native_status_uses_party_baseline_and_rolls_back_if_missing() -> TestResult {
    run_list("vipliststatus", async |h| {
        let req = list_request(1, VipListOperation::Status { hidden: true });
        let before = list_snapshot(h).await?;
        assert!(modify(h, &unread_gate(), req.clone()).await.is_err());
        assert_eq!(list_snapshot(h).await?, before);
        // Explicit historical PARTY baseline DATA, not a VIP fallback/default producer.
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1')")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        let hidden = modify(h, &unread_gate(), req.clone()).await?;
        assert_eq!(hidden.result, VipListResult::Ok);
        let setting: (i16, String) = sqlx::query_as(
            "SELECT vip_status,vip_consent_revision::text FROM game_character_social_settings",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(setting, (2, "0".into()));
        let after = list_snapshot(h).await?;
        assert!(modify(h, &unread_gate(), req).await?.replayed);
        assert_eq!(list_snapshot(h).await?, after);
        assert_eq!(
            modify(
                h,
                &unread_gate(),
                list_request(2, VipListOperation::Status { hidden: false })
            )
            .await?
            .result,
            VipListResult::Ok
        );
        let shown: i16 =
            sqlx::query_scalar("SELECT vip_status FROM game_character_social_settings")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(shown, 1);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}
#[test]
fn vip_list_native_authority_rejection_replay_binding_and_rate_refusal() -> TestResult {
    run_list("viplistfences", async |h| {
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let req = list_request(
            1,
            VipListOperation::GroupAdd {
                name: "Original".into(),
            },
        );
        let before = list_snapshot(h).await?;
        for mutation in 0..9 {
            let mut fence = harness::item_fence()?;
            let mut request = req.clone();
            match mutation {
                0 => fence.connection_generation = ConnectionGeneration::new(2)?,
                1 => fence.character_lease_generation = 2,
                2 => fence.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => request.list.account_id = account(70),
                4 => request.list.world_id = WorldId::from_bytes(id(90)).map_err(debug)?,
                5 => {
                    request.command =
                        CommandRef::new(GameSessionId::decode(&id(51))?, CommandId::new(1)?)
                }
                6 => fence.game_session_id = GameSessionId::decode(&id(51))?,
                7 => fence.character_id = character(60),
                _ => {
                    fence.runtime_scope = RuntimeScopeRefV1::channel(
                        crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
                        crate::foundation::ChannelId::decode(&id(90)).map_err(debug)?,
                    )
                }
            }
            assert!(matches!(
                h.root
                    .modify_vip_list(&authority, &h.node, fence, &gate, request)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(list_snapshot(h).await?, before);
        }
        let wrong_node = harness::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .modify_vip_list(
                    &authority,
                    &wrong_node,
                    harness::item_fence()?,
                    &gate,
                    req.clone()
                )
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(list_snapshot(h).await?, before);
        let committed = h
            .root
            .modify_vip_list(
                &authority,
                &h.node,
                harness::item_fence()?,
                &gate,
                req.clone(),
            )
            .await
            .map_err(debug)?;
        let after = list_snapshot(h).await?;
        let mut stale = harness::item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(2)?;
        let replay = h
            .root
            .modify_vip_list(&authority, &h.node, stale, &unread_gate(), req)
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.revision, committed.revision);
        assert_eq!(list_snapshot(h).await?, after);
        assert!(matches!(
            h.root
                .modify_vip_list(
                    &authority,
                    &h.node,
                    stale,
                    &unread_gate(),
                    list_request(
                        1,
                        VipListOperation::GroupAdd {
                            name: "Changed".into()
                        }
                    )
                )
                .await,
            Err(VipError::ConflictingCommand)
        ));
        let mut wrong_recovery = recovery_record(&authority, &h.root).map_err(debug)?;
        wrong_recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            modify_list_in_transaction(
                &mut tx,
                &wrong_recovery,
                &h.node,
                harness::item_fence()?,
                &list_request(
                    1,
                    VipListOperation::GroupAdd {
                        name: "Original".into()
                    }
                ),
                5
            )
            .await,
            Err(VipError::Unavailable(_))
        ));
        tx.rollback().await?;
        assert_eq!(list_snapshot(h).await?, after);
        sqlx::query("UPDATE game_vip_lists SET rate_tokens=0,rate_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint").execute(&h.pool).await?;
        let limited = modify(
            h,
            &unread_gate(),
            list_request(2, VipListOperation::GroupRemove { group_no: 4 }),
        )
        .await?;
        assert!(matches!(
            limited.result,
            VipListResult::Exhausted { seconds: 1..=3 }
        ));
        assert_eq!(h.count("game_vip_groups").await?, 1);
        assert_eq!(
            modify(
                h,
                &unread_gate(),
                list_request(2, VipListOperation::GroupRemove { group_no: 4 })
            )
            .await?
            .result,
            limited.result
        );
        Ok(())
    })
}
#[test]
fn vip_list_native_two_roots_duplicate_and_case_insensitive_name_race() -> TestResult {
    run_list("viplistconcurrent", async |h| {
        let second = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first_authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let gate = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        let other_gate = SpellPremiumCoordinator::new(
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            None,
            Some(second.clone()),
        );
        let req = list_request(
            1,
            VipListOperation::GroupAdd {
                name: "Duplicate".into(),
            },
        );
        let (a, b) = both(
            h.root.modify_vip_list(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                &gate,
                req.clone(),
            ),
            second.modify_vip_list(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                &other_gate,
                req,
            ),
        )
        .await;
        let receipts = [a.map_err(debug)?, b.map_err(debug)?];
        assert!(
            receipts
                .iter()
                .all(|r| r.result == VipListResult::Ok && r.group_no == Some(4))
        );
        assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
        let (a, b) = both(
            h.root.modify_vip_list(
                &first_authority,
                &h.node,
                harness::item_fence()?,
                &gate,
                list_request(
                    2,
                    VipListOperation::GroupAdd {
                        name: "Racing".into(),
                    },
                ),
            ),
            second.modify_vip_list(
                &second_authority,
                &h.node,
                harness::item_fence()?,
                &other_gate,
                list_request(
                    3,
                    VipListOperation::GroupAdd {
                        name: "RACING".into(),
                    },
                ),
            ),
        )
        .await;
        let receipts = [a.map_err(debug)?, b.map_err(debug)?];
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.result == VipListResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.result == VipListResult::NameTaken)
                .count(),
            1
        );
        assert_eq!(h.count("game_vip_groups").await?, 2);
        assert_eq!(h.count("game_vip_command_receipts").await?, 3);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn vip_list_native_delivered_and_bypass_require_live_session_and_current_assignment() -> TestResult
{
    run_list("viplistcurrent", async |h| {
        let pre = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            false,
        );
        modify(
            h,
            &pre,
            list_request(
                1,
                VipListOperation::GroupAdd {
                    name: "Existing".into(),
                },
            ),
        )
        .await?;
        sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1')").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        let free = add_gate(
            h,
            Arc::new(PremiumConsumer::default()),
            Some(add_clock()),
            true,
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Materialize the real durable activation latch through its own coordinator.
        assert_eq!(
            modify(
                h,
                &free,
                list_request(
                    2,
                    VipListOperation::GroupRename {
                        group_no: 4,
                        name: "Free rename".into()
                    }
                )
            )
            .await?
            .result,
            VipListResult::GroupLimit
        );
        let before = list_snapshot(h).await?;
        let delivered = list_request(
            3,
            VipListOperation::GroupRename {
                group_no: 4,
                name: "Fresh rename".into(),
            },
        );
        let bypass = list_request(4, VipListOperation::Status { hidden: true });
        // Each request crosses a different coordinator branch into the same native mutation
        // fence; neither the delivered benefit decision nor the non-growth bypass grants it.
        for request in [delivered.clone(), bypass.clone()] {
            let mut stale = harness::item_fence()?;
            stale.connection_generation = ConnectionGeneration::new(2)?;
            assert!(matches!(
                h.root
                    .modify_vip_list(&authority, &h.node, stale, &free, request)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(list_snapshot(h).await?, before);
        }
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&h.pool)
            .await?;
        for request in [delivered.clone(), bypass.clone()] {
            assert!(matches!(
                h.root
                    .modify_vip_list(&authority, &h.node, harness::item_fence()?, &free, request)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(list_snapshot(h).await?, before);
        }
        let replay = modify(
            h,
            &unread_gate(),
            list_request(
                1,
                VipListOperation::GroupAdd {
                    name: "Existing".into(),
                },
            ),
        )
        .await?;
        assert!(replay.replayed);
        assert_eq!(list_snapshot(h).await?, before);
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1")
            .execute(&h.pool)
            .await?;
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        };
        let scope = harness::item_fence()?.runtime_scope;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope)
            .await
            .map_err(debug)?
            .ok_or("current predecessor missing")?;
        let role: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        let revoked = h
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([91; 32]),
                actor: ControlActor::new(&role).map_err(debug)?,
                command: AssignmentCommand::Revoke { scope, predecessor },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(revoked, AssignmentOutcome::Committed(_)));
        for request in [delivered, bypass] {
            assert!(matches!(
                h.root
                    .modify_vip_list(&authority, &h.node, harness::item_fence()?, &free, request)
                    .await,
                Err(VipError::AuthorityRejected)
            ));
            assert_eq!(list_snapshot(h).await?, before);
        }
        Ok(())
    })
}

#[test]
fn vip_receipt_probe_finds_only_this_sessions_committed_command() -> TestResult {
    run("vipreceiptprobe", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        let committed = accept(h, req).await?;
        assert_eq!(committed.result, VipResult::Ok);
        let before = footprint(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = harness::item_fence()?;
        assert!(
            h.root
                .vip_receipt_recorded(&authority, fence, req.list, req.command)
                .await
                .map_err(debug)?
        );
        assert_eq!(footprint(h).await?, before);
        let unknown = CommandRef::new(req.command.game_session_id(), CommandId::new(2)?);
        assert!(
            !h.root
                .vip_receipt_recorded(&authority, fence, req.list, unknown)
                .await
                .map_err(debug)?
        );
        assert_eq!(footprint(h).await?, before);
        let foreign = CommandRef::new(GameSessionId::decode(&id(51))?, req.command.command_id());
        assert!(matches!(
            h.root
                .vip_receipt_recorded(&authority, fence, req.list, foreign)
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        Ok(())
    })
}

/// The debug-build `serve_admitted` future is deeper than the default 2 MiB test stack, so the
/// dispatch scenario runs on a dedicated thread with an explicit stack (as the kill reward e2e).
const DISPATCH_STACK_BYTES: usize = 64 << 20;

#[test]
fn vip_listener_down_replays_a_committed_accept_through_dispatch() -> TestResult {
    std::thread::Builder::new()
        .name("vip-dispatch-replay".into())
        .stack_size(DISPATCH_STACK_BYTES)
        .spawn(|| listener_down_dispatch_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "VIP dispatch replay thread panicked")?
        .map_err(Into::into)
}

fn listener_down_dispatch_scenario() -> TestResult {
    use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MessageType, NodeId};
    use crate::gameplay_transport::ComposedFreshAdmission;
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use crate::gameplay_transport::connection::{
        AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, SessionContinuity,
        serve_admitted,
    };
    use crate::gameplay_transport::fresh_evidence::FreshEvidenceSource;
    use oteryn_protocol_oteryn::vip::{
        CAPABILITY_VIP_V1, COMMAND_TYPE_VIP_INTENT, VipIntent, VipResultCode,
        decode_vip_intent_result, encode_vip_intent,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, decode_command_result, decode_wire_envelope, encode_client_command,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Mutex;

    run("vipdispatchreplay", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = harness::item_fence()?;
        let world = fence.runtime_scope.world_id();
        let channel = crate::foundation::ChannelId::decode(&id(harness::CHANNEL))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        let content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence = FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let owner = ComposedFreshAdmission {
            root: &h.root,
            character: &authority,
            holder: &h.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                fence.runtime_scope,
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .map_err(debug)?,
            runtime: &runtime,
            movement_cells: room.movement_cells(),
            door: &door,
            chest: room.door(),
            spells: &spells,
            active_generation: None,
            premium_coordinator: None,
            rooms: crate::gameplay_transport::SeamRooms::new(None, None),
            achievements: &achievements,
            imported_charms: &charms,
            spell_states: Mutex::default(),
            world_gate: std::sync::Mutex::default(),
            attack: Mutex::default(),
            clock_origin: std::time::Instant::now(),
            lost: std::sync::Mutex::default(),
            revision_sequencer:
                crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
            quest_catalogue: None,
            quest_sessions: std::sync::Mutex::default(),
            chat: crate::gameplay_transport::chat_intent::ChatRuntime::default(),
            npc_conversations: Default::default(),
            wheel_ruleset: None,
            wheel_sessions: std::sync::Mutex::default(),
            progression_sessions: std::sync::Mutex::default(),
            last_positions: crate::gameplay_transport::last_position::LastPositions::default(),
            input_gate: Default::default(),
            premium: crate::premium::refresh::PremiumRefresher::new(
                Default::default(),
                h.root.clone(),
                None,
            ),
            premium_sessions: std::sync::Mutex::default(),
            fence_holders: Default::default(),
            spell_lane: crate::durability::spell_owner_commit::SpellLane::new(world, channel),
            loop_ground: Default::default(),
        };
        let presence = Arc::new(
            crate::durability::vip_presence::PresenceOwner::new(h.root.clone(), world, 1)
                .map_err(debug)?,
        );
        presence.listener_reconnected().map_err(debug)?;
        assert!(owner.chat.vip.install_presence(presence.clone()));
        let snapshot = owner
            .vip_open(fence.game_session_id, Some(fence))
            .await
            .ok_or("VIP snapshot")?;
        let handle = snapshot
            .consent
            .invitations
            .first()
            .ok_or("invitation handle")?
            .handle;
        let committed = accept(h, req).await?;
        assert_eq!(committed.result, VipResult::Ok);
        // The listener goes down after the commit whose acknowledgement was lost.
        presence.listener_down().map_err(debug)?;
        assert!(!presence.listening());
        // The replacement snapshot has lost the accepted handle.
        let snapshot = owner
            .vip_open(fence.game_session_id, Some(fence))
            .await
            .ok_or("VIP snapshot")?;
        assert!(snapshot.consent.invitations.is_empty());
        assert!(
            snapshot
                .consent
                .watchers
                .first()
                .ok_or("watcher handle")?
                .handle
                > handle
        );
        let before = footprint(h).await?;
        let admitted = AdmittedSession {
            game_session_id: fence.game_session_id,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity {
                selected_capabilities: SelectedCapabilities::select(
                    &[OfferedCapability {
                        id: CAPABILITY_VIP_V1,
                        requires: &[],
                    }],
                    &[CAPABILITY_VIP_V1],
                )
                .ok_or("VIP capability selection")?,
                ..SessionContinuity::FRESH
            },
            item_fence: Some(fence),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        // The receipted command replays even without its old handle; a new command is refused.
        for command_id in 1..=2 {
            let payload =
                encode_vip_intent(&VipIntent::Accept { invitation: handle }).map_err(debug)?;
            let frame = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_VIP_INTENT,
                    payload: &payload,
                },
            )?;
            client
                .write_all(&(frame.len() as u32).to_be_bytes())
                .await?;
            client.write_all(&frame).await?;
        }
        client.shutdown().await?;
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut results = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            let envelope = decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                results.push((
                    result.command_id,
                    result.status,
                    decode_vip_intent_result(result.payload)
                        .map_err(debug)?
                        .code,
                ));
            }
            cursor += 4 + length;
        }
        assert_eq!(
            results,
            vec![
                (
                    1,
                    crate::foundation::CommandStatus::Accepted,
                    VipResultCode::Ok
                ),
                (
                    2,
                    crate::foundation::CommandStatus::Rejected,
                    VipResultCode::VipUnavailable,
                ),
            ]
        );
        assert_eq!(footprint(h).await?, before);
        // A receipt lookup that cannot finish leaves the outcome unknown: the CommandId stays
        // unsequenced, so a later resend can still replay the committed accept.
        let mut lock = h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_vip_command_receipts IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *lock)
            .await?;
        let payload =
            encode_vip_intent(&VipIntent::Accept { invitation: handle }).map_err(debug)?;
        let (outcome, deltas) = owner
            .vip_command(
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: fence.game_session_id,
                    command_id: 1,
                    item_fence: Some(fence),
                },
                &payload,
            )
            .await;
        lock.rollback().await?;
        assert!(outcome.unknown);
        assert_eq!(outcome.result.code, VipResultCode::VipUnavailable);
        assert!(deltas.is_empty());
        assert_eq!(footprint(h).await?, before);
        Ok(())
    })
}

#[test]
fn vip_end_replay_matches_the_receipt_binding_of_a_retired_handle() -> TestResult {
    run("vipendreplay", async |h| {
        fixture_character(h, 60, 70).await?;
        let number = fixture_consent(h, 70, 60, CHARACTER, 1).await?;
        let req = accept_request(1, invitation(60, 70, number));
        let committed = accept(h, req).await?;
        assert_eq!(committed.result, VipResult::Ok);
        let before = footprint(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = harness::item_fence()?;
        // VIP0 §6: a retired handle that still names the committed binding replays; one that
        // names another invitation is a different request.
        let named = h
            .root
            .replay_vip_end(
                &authority,
                fence,
                req.list,
                req.command,
                VipEndKind::Accept,
                req.invitation,
            )
            .await
            .map_err(debug)?
            .ok_or("missing named receipt")?;
        assert!(named.replayed);
        assert_eq!(
            (named.result, named.revision),
            (committed.result, committed.revision)
        );
        assert!(matches!(
            h.root
                .replay_vip_end(
                    &authority,
                    fence,
                    req.list,
                    req.command,
                    VipEndKind::Accept,
                    invitation(60, 70, number + 1)
                )
                .await,
            Err(VipError::ConflictingCommand)
        ));
        assert_eq!(footprint(h).await?, before);
        assert!(matches!(
            h.root
                .replay_vip_end(
                    &authority,
                    fence,
                    req.list,
                    req.command,
                    VipEndKind::Reject,
                    req.invitation
                )
                .await,
            Err(VipError::ConflictingCommand)
        ));
        assert_eq!(footprint(h).await?, before);
        let unknown = CommandRef::new(req.command.game_session_id(), CommandId::new(2)?);
        assert_eq!(
            h.root
                .replay_vip_end(
                    &authority,
                    fence,
                    req.list,
                    unknown,
                    VipEndKind::Accept,
                    req.invitation
                )
                .await
                .map_err(debug)?,
            None
        );
        assert_eq!(footprint(h).await?, before);
        let foreign = CommandRef::new(GameSessionId::decode(&id(51))?, req.command.command_id());
        assert!(matches!(
            h.root
                .replay_vip_end(
                    &authority,
                    fence,
                    req.list,
                    foreign,
                    VipEndKind::Accept,
                    req.invitation
                )
                .await,
            Err(VipError::AuthorityRejected)
        ));
        assert_eq!(footprint(h).await?, before);
        Ok(())
    })
}
