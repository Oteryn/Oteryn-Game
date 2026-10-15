//! Real native recovery/session checks plus explicitly test-only proposed VIP tables/columns.
//! The fixture is not an installed migration, SECURITY DEFINER/grant or gameplay consent producer.
#![allow(clippy::expect_used)]
use super::*;
use crate::bestiary_postgres_harness as harness;
use crate::foundation::{CommandId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use harness::{CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id};
use sqlx::Executor;
use std::future::Future;
use std::task::Poll;

const PRIVATE_SCHEMA: &str = r#"
ALTER TABLE game_character_social_settings ADD COLUMN vip_status SMALLINT NOT NULL DEFAULT 1 CHECK(vip_status IN(1,2));
ALTER TABLE game_character_social_settings ADD COLUMN vip_consent_revision NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(vip_consent_revision BETWEEN 0 AND 18446744073709551615);
CREATE TABLE game_vip_lists (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),world_id UUID NOT NULL,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 next_entry_no NUMERIC(20,0) NOT NULL CHECK(next_entry_no BETWEEN 1 AND 18446744073709551615),
 rate_tokens SMALLINT NOT NULL CHECK(rate_tokens BETWEEN 0 AND 20),rate_at BIGINT NOT NULL CHECK(rate_at>=0),PRIMARY KEY(account_id,world_id));
CREATE TABLE game_vip_entries (
 account_id UUID NOT NULL,world_id UUID NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 entry_no NUMERIC(20,0) NOT NULL CHECK(entry_no>0),description TEXT NOT NULL DEFAULT'',icon SMALLINT NOT NULL DEFAULT 0,notify BOOLEAN NOT NULL DEFAULT FALSE,added_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(account_id,world_id,character_id),UNIQUE(account_id,world_id,entry_no),FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE);
CREATE SEQUENCE game_vip_invitation_id NO CYCLE;
CREATE TABLE game_vip_consents (
 account_id UUID NOT NULL,world_id UUID NOT NULL,inviter_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),character_id UUID NOT NULL,
 invitation_id BIGINT NOT NULL DEFAULT nextval('game_vip_invitation_id') UNIQUE CHECK(invitation_id>0),consent SMALLINT NOT NULL CHECK(consent IN(1,2)),invite_expires_at TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(account_id,world_id,inviter_character_id,character_id),CHECK(inviter_character_id<>character_id),
 FOREIGN KEY(account_id,world_id,character_id) REFERENCES game_vip_entries ON DELETE CASCADE);
CREATE TABLE game_vip_group_members (
 account_id UUID NOT NULL,world_id UUID NOT NULL,character_id UUID NOT NULL,group_no SMALLINT NOT NULL CHECK(group_no BETWEEN 1 AND 8),
 PRIMARY KEY(account_id,world_id,character_id,group_no),FOREIGN KEY(account_id,world_id,character_id) REFERENCES game_vip_entries ON DELETE CASCADE);
CREATE TABLE game_vip_command_receipts (
 account_id UUID NOT NULL,world_id UUID NOT NULL,game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),kind SMALLINT NOT NULL CHECK(kind IN(1,2,3)),binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 result SMALLINT NOT NULL CHECK(result IN(1,2,3)),seconds SMALLINT,revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(account_id,world_id,game_session_id,command_id),FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE,
 CHECK((result=3 AND seconds IS NOT NULL AND seconds BETWEEN 1 AND 3) OR(result<>3 AND seconds IS NULL)));
CREATE FUNCTION vip_fixture_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN
 IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM game_vip_lists WHERE account_id=OLD.account_id AND world_id=OLD.world_id) THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'immutable VIP receipt' USING ERRCODE='23514';END;$$;
CREATE TRIGGER vip_fixture_receipt_immutable BEFORE UPDATE OR DELETE ON game_vip_command_receipts FOR EACH ROW EXECUTE FUNCTION vip_fixture_receipt_immutable();
REVOKE ALL ON game_vip_lists,game_vip_entries,game_vip_consents,game_vip_group_members,game_vip_command_receipts FROM PUBLIC;
"#;

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
        h.pool.execute(PRIVATE_SCHEMA).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn fixture_character(h: &Harness, seed: u8, owner: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(owner).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(owner).as_slice()).bind(format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26))).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
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
        "account_id":uuid(id(owner)),"target_world_id":uuid(id(WORLD)),
        "requested_name":format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26)),
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"
    }))?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(owner).as_slice()).bind(id(seed).as_slice()).bind(id(WORLD).as_slice())
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
        let mut wrong_recovery = authority.record_for(&h.root).map_err(debug)?;
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
        let recovery = authority.record_for(&h.root).map_err(debug)?;
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
