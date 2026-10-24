//! GUILD-BANK-1: admitted session -> real command/result bytes -> runtime-role PostgreSQL -> reload.
#[path = "guild_bank/seed.rs"]
mod bank_seed;
#[path = "guild_bank/transport.rs"]
mod transport;
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::{
    guild::{GuildDisbandJobRequest, GuildDisbandOutcome},
    guild_chat_fixture as seed,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::guild_bank::*;
use oteryn_protocol_oteryn::{CommandStatus, guild as guild_wire};
use sha2::Digest;
use transport::exchange;

#[test]
fn guild_bank_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild bank e2e panicked")??;
    Ok(())
}
async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    intent: GuildBankIntent,
    code: u8,
) -> TestResult<GuildBankReply> {
    let id = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        id,
        COMMAND_TYPE_GUILD_BANK_INTENT,
        &encode_guild_bank_intent(&intent).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let reply = decode_guild_bank_reply(&bytes).map_err(debug)?;
    assert_eq!(reply.code, code);
    Ok(reply)
}
async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    guild: [u8; 16],
) -> TestResult<GuildBankReply> {
    let id = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        id,
        COMMAND_TYPE_GUILD_BANK_QUERY,
        &encode_guild_bank_query(&GuildBankQuery {
            guild: Some(guild),
            before: None,
            after_claim: None,
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    decode_guild_bank_reply(&bytes).map_err(debug)
}
async fn open_claims(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
) -> TestResult<Vec<GuildBankClaim>> {
    let command_id = session.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        session,
        command_id,
        COMMAND_TYPE_GUILD_BANK_QUERY,
        &[],
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let result = decode_guild_bank_reply(&bytes).map_err(debug)?;
    assert_eq!(result.code, 1);
    Ok(result.claims)
}
async fn guild(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    action: guild_wire::GuildIntent,
) -> TestResult<guild_wire::GuildIntentResult> {
    let id = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        id,
        guild_wire::COMMAND_TYPE_GUILD_INTENT,
        &guild_wire::encode_guild_intent(&action).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let r = guild_wire::decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!(r.code, 1);
    Ok(r)
}
fn scenario() -> TestResult {
    run(|admin| async move {
        reset();
        let mut h =
            Harness::create_with_admission_seed(admin, "guild_bank_wire", true, false).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("GUILD-BANK-PG server_version_num={version}");
        prepare_current_session(&h).await?;
        let base = super::fixture::fence(1)?.item_fence();
        seed::historical_character(
            &h.pool,
            90,
            CharacterId::from_bytes(seed::sid(90, 1)).map_err(debug)?,
            seed::sid(90, 2),
            id(42),
            "Bank Member",
        )
        .await?;
        let guest = seed::historical_session(&h.pool, 90, base).await?;
        bank_seed::balance(&h, id(40), id(41), i64::try_from(HARD_MAX)? - 10, 160).await?;
        bank_seed::balance(&h, seed::sid(90, 2), seed::sid(90, 1), 1000, 180).await?;
        bank_seed::retained_market_custody(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Default::default(),
                None,
                None,
                Some(h.root.clone()),
            );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&coordinator);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut leader = deps.admitted;
        leader.first_entry = FirstEntryOutcome::Positioned;
        leader.item_fence = item_fence_of(current);
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[40, 55])
                .ok_or("bank capability")?;
        assert!(leader.continuity.selected_capabilities.contains(55));
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let r = runtime
                .reserve_fresh_session(guest.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(r).map_err(debug)?;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            actor
        };
        deps.initialize_player(&owner, actor, guest.game_session_id)
            .await;
        let mut member = AdmittedSession {
            game_session_id: guest.game_session_id,
            runtime_actor: Some(actor),
            item_fence: Some(guest),
            controller: None,
            ..leader
        };
        let g = guild(
            &owner,
            &mut leader,
            guild_wire::GuildIntent::Found {
                name: "Bank Guild".into(),
            },
        )
        .await?
        .guild;
        guild(
            &owner,
            &mut leader,
            guild_wire::GuildIntent::Invite {
                guild: g,
                name: "Bank Member".into(),
            },
        )
        .await?;
        guild(
            &owner,
            &mut member,
            guild_wire::GuildIntent::Join { guild: g },
        )
        .await?;
        // A first refusal must not leave even an empty Guild balance behind.
        let before_refusal = bank_seed::financial_snapshot(&h).await?;
        command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Deposit(1001),
            },
            4,
        )
        .await?;
        assert_eq!(bank_seed::financial_snapshot(&h).await?, before_refusal);
        let initial = member;
        let deposit = GuildBankIntent {
            guild: g,
            action: GuildBankAction::Deposit(500),
        };
        // Audit failure rolls back both ledgers and balances, and does not sequence the command.
        let before_failure = bank_seed::financial_snapshot(&h).await?;
        sqlx::raw_sql("CREATE FUNCTION guild_bank_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END $$; CREATE TRIGGER guild_bank_test_fail BEFORE INSERT ON game_guild_bank_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_bank_test_fail()")
            .execute(&h.pool).await?;
        assert!(
            exchange(
                &owner,
                &mut member,
                initial.continuity.next_command_id,
                COMMAND_TYPE_GUILD_BANK_INTENT,
                &encode_guild_bank_intent(&deposit).map_err(debug)?
            )
            .await
            .is_err()
        );
        assert_eq!(bank_seed::financial_snapshot(&h).await?, before_failure);
        sqlx::raw_sql("DROP TRIGGER guild_bank_test_fail ON game_guild_bank_event_outbox; DROP FUNCTION guild_bank_test_fail()")
            .execute(&h.pool).await?;
        member = initial;
        let paid = command(&owner, &mut member, deposit.clone(), 1).await?;
        assert_eq!((paid.bank_balance, paid.guild_balance), (500, 500));
        let mut replay = initial;
        assert_eq!(
            command(&owner, &mut replay, deposit.clone(), 1).await?,
            paid
        );
        let before = bank_seed::financial_snapshot(&h).await?;
        let refused_withdrawal = member;
        let denied = command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Withdraw(1),
            },
            2,
        )
        .await?;
        command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Deposit(501),
            },
            4,
        )
        .await?;
        command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Claim,
            },
            7,
        )
        .await?;
        assert_eq!(bank_seed::financial_snapshot(&h).await?, before);
        // Same occurrence with changed bytes is rejected; original receipt remains immutable.
        let mut conflict = initial;
        let conflict_id = conflict.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut conflict,
                conflict_id,
                COMMAND_TYPE_GUILD_BANK_INTENT,
                &encode_guild_bank_intent(&GuildBankIntent {
                    guild: g,
                    action: GuildBankAction::Deposit(499)
                })
                .map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        // Malformed, unnegotiated and stale-generation commands never write money or evidence.
        let mut old = member;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        let old_id = old.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut old,
                old_id,
                COMMAND_TYPE_GUILD_BANK_QUERY,
                &encode_guild_bank_query(&GuildBankQuery {
                    guild: Some(g),
                    before: None,
                    after_claim: None,
                })
                .map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let mut stale = member;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation += 1;
        let stale_id = stale.continuity.next_command_id;
        // The reconnect bank prelude independently disconnects an already stale socket.
        // Exercise this command's own handler as well, to prove its native fence rejects.
        assert!(matches!(
            owner
                .guild_command(
                    stale,
                    stale_id,
                    COMMAND_TYPE_GUILD_BANK_INTENT,
                    &encode_guild_bank_intent(&deposit).map_err(debug)?
                )
                .await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));
        assert!(
            exchange(
                &owner,
                &mut stale,
                stale_id,
                COMMAND_TYPE_GUILD_BANK_INTENT,
                &encode_guild_bank_intent(&deposit).map_err(debug)?
            )
            .await
            .is_err()
        );
        let malformed_id = member.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut member,
                malformed_id,
                COMMAND_TYPE_GUILD_BANK_INTENT,
                &[0xff]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        assert_eq!(bank_seed::financial_snapshot(&h).await?, before);
        let page = query(&owner, &mut member, g).await?;
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].amount, 500);
        // A vice may withdraw to its own Account (the high-balance leader cannot receive a soft-cap credit).
        let revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(g.as_slice())
        .fetch_one(&h.pool)
        .await?;
        guild(
            &owner,
            &mut leader,
            guild_wire::GuildIntent::SetRank {
                guild: g,
                character: *guest.character_id.as_bytes(),
                level: 2,
                expected_revision: revision.parse()?,
            },
        )
        .await?;
        let withdrawal = command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Withdraw(100),
            },
            1,
        )
        .await?;
        assert_eq!(
            (withdrawal.bank_balance, withdrawal.guild_balance),
            (600, 400)
        );
        let mut replay_refusal = refused_withdrawal;
        assert_eq!(
            command(
                &owner,
                &mut replay_refusal,
                GuildBankIntent {
                    guild: g,
                    action: GuildBankAction::Withdraw(1)
                },
                2
            )
            .await?,
            denied
        );
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Withdraw(1),
            },
            5,
        )
        .await?;
        // Two native writers racing the same Account cannot both spend its 600 gold.
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        let make_request = |tag, command_id| -> TestResult<_> {
            let intent = GuildBankIntent {
                guild: g,
                action: GuildBankAction::Deposit(400),
            };
            Ok(crate::durability::guild_bank::GuildBankRequest {
                operation_id: id(tag),
                command: crate::foundation::CommandRef::new(
                    member.game_session_id,
                    crate::foundation::CommandId::new(command_id).map_err(debug)?,
                ),
                occurred_at: now,
                wire_digest: sha2::Sha256::digest(
                    encode_guild_bank_intent(&intent).map_err(debug)?,
                )
                .into(),
                intent,
            })
        };
        let concurrent_root = fixture::tutorial_vitals_fixture::reopen_root(&h)?;
        assert!(concurrent_root.maintain_ready_once().await?);
        let concurrent_authority = concurrent_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (a, b) = fixture::join_two(
            h.root
                .commit_guild_bank(&authority, &h.node, guest, make_request(240, 10000)?),
            concurrent_root.commit_guild_bank(
                &concurrent_authority,
                &h.node,
                guest,
                make_request(241, 10001)?,
            ),
        )
        .await;
        let mut codes = [a.map_err(debug)?.code, b.map_err(debug)?.code];
        codes.sort_unstable();
        assert_eq!(codes, [1, 4]);
        drop(concurrent_authority);
        drop(concurrent_root);
        assert_eq!(query(&owner, &mut member, g).await?.guild_balance, 800);
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Deposit(BALANCE_MAX),
            },
            5,
        )
        .await?;
        let before_disband = leader;
        let disband = GuildBankIntent {
            guild: g,
            action: GuildBankAction::Disband("Bank Guild".into()),
        };
        sqlx::query("CREATE TABLE game_guildhall_uncomposed_probe(guild_id UUID)")
            .execute(&h.pool)
            .await?;
        let before_blocked = bank_seed::financial_snapshot(&h).await?;
        command(&owner, &mut leader, disband.clone(), 1).await?;
        assert_eq!(bank_seed::financial_snapshot(&h).await?, before_blocked);
        sqlx::query("DROP TABLE game_guildhall_uncomposed_probe")
            .execute(&h.pool)
            .await?;
        let job = GuildDisbandJobRequest {
            world_id: leader.world_id,
            guild_id: GuildId::from_bytes(g).map_err(debug)?,
        };
        h.root
            .advance_guild_disband(&authority, &h.node, job)
            .await
            .map_err(debug)?;

        let payout = query(&owner, &mut leader, g).await?;
        assert_eq!(
            (payout.bank_balance, payout.guild_balance, payout.claim),
            (HARD_MAX - 7, 0, 797)
        );
        command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Deposit(1),
            },
            3,
        )
        .await?;
        command(
            &owner,
            &mut member,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Claim,
            },
            7,
        )
        .await?;
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Claim,
            },
            5,
        )
        .await?;
        // Genuine World controller resumes after payout commit; only it frees membership/name.
        let job = GuildDisbandJobRequest {
            world_id: leader.world_id,
            guild_id: GuildId::from_bytes(g).map_err(debug)?,
        };
        for _ in 0..4 {
            if matches!(
                h.root
                    .advance_guild_disband(&authority, &h.node, job.clone())
                    .await
                    .map_err(debug)?,
                GuildDisbandOutcome::Disbanded { .. }
            ) {
                break;
            }
        }
        let state: i16 = sqlx::query_scalar(
            "SELECT state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(g.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(state, 4);
        // Account-bound custody survives membership cleanup and owner reconstruction.
        drop(owner);
        drop(authority);
        h.root = fixture::tutorial_vitals_fixture::reopen_root(&h)?;
        assert!(h.root.maintain_ready_once().await?);
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Default::default(),
                None,
                None,
                Some(h.root.clone()),
            );
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&coordinator);
        deps.initialize(&owner).await;
        assert_eq!(query(&owner, &mut leader, g).await?.claim, 797);
        assert_eq!(
            open_claims(&owner, &mut leader).await?,
            vec![GuildBankClaim {
                guild: g,
                amount: 797
            }]
        );
        assert!(open_claims(&owner, &mut member).await?.is_empty());
        let mut replay = initial;
        assert_eq!(command(&owner, &mut replay, deposit, 1).await?, paid);
        let mut replay_refusal = refused_withdrawal;
        assert_eq!(
            command(
                &owner,
                &mut replay_refusal,
                GuildBankIntent {
                    guild: g,
                    action: GuildBankAction::Withdraw(1)
                },
                2
            )
            .await?,
            denied
        );
        let mut replay_disband = before_disband;
        command(&owner, &mut replay_disband, disband, 1).await?;
        let new_guild = guild(
            &owner,
            &mut leader,
            guild_wire::GuildIntent::Found {
                name: "Bank Successor".into(),
            },
        )
        .await?
        .guild;
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: new_guild,
                action: GuildBankAction::Deposit(100),
            },
            1,
        )
        .await?;
        let before_claim = leader;
        let delivered = command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Claim,
            },
            1,
        )
        .await?;
        assert_eq!(
            (delivered.bank_balance, delivered.claim),
            (HARD_MAX - 7, 697)
        );
        let mut retry = before_claim;
        assert_eq!(
            command(
                &owner,
                &mut retry,
                GuildBankIntent {
                    guild: g,
                    action: GuildBankAction::Claim
                },
                1
            )
            .await?,
            delivered
        );
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: new_guild,
                action: GuildBankAction::Deposit(700),
            },
            1,
        )
        .await?;
        assert_eq!(
            command(
                &owner,
                &mut leader,
                GuildBankIntent {
                    guild: g,
                    action: GuildBankAction::Claim
                },
                1
            )
            .await?
            .claim,
            0
        );
        command(
            &owner,
            &mut leader,
            GuildBankIntent {
                guild: g,
                action: GuildBankAction::Claim,
            },
            7,
        )
        .await?;
        let payouts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_guild_bank_operations WHERE kind='GUILD_DISBAND_PAYOUT'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(payouts, 1);
        let terminal: Option<i64> = sqlx::query_scalar(
            "SELECT amount FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(g.as_slice())
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(terminal, Some(0));
        assert!(open_claims(&owner, &mut leader).await?.is_empty());
        let snapshot = bank_seed::financial_snapshot(&h).await?;
        // Production SQL rejects money without an exact ledger successor, and immutable custody binding.
        let mut tx = h.pool.begin().await?;
        sqlx::query("UPDATE game_guild_bank_balances SET balance=balance+1 WHERE guild_id=encode($1,'hex')::uuid").bind(new_guild.as_slice()).execute(&mut *tx).await?;
        let error = tx
            .commit()
            .await
            .err()
            .ok_or("orphan Guild balance update accepted")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        let error = sqlx::query(
            "UPDATE game_guild_disband_claims SET amount=1 WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(g.as_slice())
        .execute(&h.pool)
        .await
        .err()
        .ok_or("terminal claim increased")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        assert_eq!(bank_seed::financial_snapshot(&h).await?, snapshot);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
