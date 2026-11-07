//! GUILDHALL: actual House command -> composed owner -> native split ledger/custody.
use super::fixture::{Harness, debug, id, run};
use super::guild_bank::{bank_seed, transport};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as seed;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::guild as guild_wire;
use oteryn_protocol_oteryn::house as wire;
use oteryn_protocol_oteryn::{CommandStatus, guild_bank::*};
use transport::exchange;
const HOUSE: &str = "oteryn:content.house.bamboo_fortress";

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
fn bid(maximum: u64) -> Vec<u8> {
    let mut amount = vec![8];
    let mut n = maximum;
    while n >= 128 {
        amount.push(n as u8 | 128);
        n >>= 7;
    }
    amount.push(n as u8);
    let mut body = Vec::new();
    bytes(&mut body, 1, &amount);
    let mut target = Vec::new();
    bytes(&mut target, 1, &id(42));
    bytes(&mut target, 2, HOUSE.as_bytes());
    bytes(&mut body, 8, &target);
    body
}
async fn house(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    maximum: u64,
    disposition: wire::HouseDisposition,
) -> TestResult {
    let command = s.continuity.next_command_id;
    assert_eq!(
        exchange(owner, s, command, 56, &bid(maximum)).await?,
        (
            CommandStatus::Accepted,
            wire::encode_house_result(disposition, 0)
        ),
        "house bid {maximum}, command {command}"
    );
    Ok(())
}
async fn guild_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    intent: guild_wire::GuildIntent,
) -> TestResult<guild_wire::GuildIntentResult> {
    let command = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        command,
        guild_wire::COMMAND_TYPE_GUILD_INTENT,
        &guild_wire::encode_guild_intent(&intent).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let result = guild_wire::decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!(result.code, 1);
    Ok(result)
}
async fn bank_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    guild: [u8; 16],
    action: GuildBankAction,
) -> TestResult<GuildBankReply> {
    let command = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        command,
        COMMAND_TYPE_GUILD_BANK_INTENT,
        &encode_guild_bank_intent(&GuildBankIntent { guild, action }).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let result = decode_guild_bank_reply(&bytes).map_err(debug)?;
    assert_eq!(result.code, 1);
    Ok(result)
}
async fn access(h: &Harness, character: [u8; 16]) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(character.as_slice())
    .fetch_optional(&h.pool)
    .await?)
}
#[test]
fn guildhall_postgres_real_dispatch_split_escrow() -> TestResult {
    run_scenario(0, false)
}

#[test]
fn guildhall_postgres_real_dispatch_disband_owned() -> TestResult {
    run_scenario(1, false)
}
#[test]
fn guildhall_postgres_real_dispatch_disposition_release_routing() -> TestResult {
    run_scenario(1, true)
}
#[test]
fn guildhall_postgres_real_dispatch_disband_bid() -> TestResult {
    run_scenario(2, false)
}
#[test]
fn guildhall_postgres_real_dispatch_catalogue_activation() -> TestResult {
    run_scenario(3, false)
}
fn run_scenario(ending: u8, reject_ordinary_release: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(ending, reject_ordinary_release).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guildhall thread panicked")??;
    Ok(())
}
fn scenario(ending: u8, reject_ordinary_release: bool) -> TestResult {
    run(|admin| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guildhall_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let mut guests = Vec::new();
        for n in 90..94 {
            let name = format!("Hall Vice {}", char::from(b'A' + (n - 90) as u8));
            seed::historical_character(
                &h.pool,
                n,
                CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?,
                seed::sid(n, 2),
                id(42),
                &name,
            )
            .await?;
            guests.push((
                name,
                seed::historical_session(&h.pool, n, super::fixture::fence(1)?.item_fence())
                    .await?,
            ));
        }
        bank_seed::balance(
            &h,
            id(40),
            id(41),
            if ending == 3 { 1_000_000 } else { 1000 },
            160,
        )
        .await?;
        bank_seed::retained_market_custody(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let deps = Dependencies::with_content_and_capacity(
            super::ChannelContentPin::test(world),
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            6,
        )?;
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
        let digest = owner
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        if ending != 3 {
            let mut tx = h.pool.begin().await?;
            sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,$2,'guildhall',1,100,$3)")
            .bind(id(42).as_slice()).bind(HOUSE).bind(digest.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,NULL,1,encode($3,'hex')::uuid)")
            .bind(id(42).as_slice()).bind(HOUSE).bind(id(91).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut leader = deps.admitted;
        leader.first_entry = FirstEntryOutcome::Positioned;
        leader.item_fence = item_fence_of(current);
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 40, 49, 55, 59, 99])
                .ok_or("hall capability")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                leader.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let guild = crate::domain::guild::GuildId::from_bytes(
            guild_command(
                &owner,
                &mut leader,
                guild_wire::GuildIntent::Found {
                    name: "Hall Guild".into(),
                },
            )
            .await?
            .guild,
        )
        .map_err(debug)?;
        let mut members = Vec::new();
        for (name, fence) in guests {
            let actor = {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime
                    .reserve_fresh_session(fence.game_session_id)
                    .map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        actor,
                        fence.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                actor
            };
            deps.initialize_player(&owner, actor, fence.game_session_id)
                .await;
            let mut member = AdmittedSession {
                game_session_id: fence.game_session_id,
                runtime_actor: Some(actor),
                item_fence: Some(fence),
                controller: None,
                ..leader
            };
            member.continuity.next_command_id = 1;
            guild_command(
                &owner,
                &mut leader,
                guild_wire::GuildIntent::Invite {
                    guild: *guild.as_bytes(),
                    name,
                },
            )
            .await?;
            let joined = guild_command(
                &owner,
                &mut member,
                guild_wire::GuildIntent::Join {
                    guild: *guild.as_bytes(),
                },
            )
            .await?;
            guild_command(
                &owner,
                &mut leader,
                guild_wire::GuildIntent::SetRank {
                    guild: *guild.as_bytes(),
                    character: *fence.character_id.as_bytes(),
                    level: 2,
                    expected_revision: joined.revision.ok_or("joined revision")?,
                },
            )
            .await?;
            members.push(member);
        }
        let state: i16 = sqlx::query_scalar(
            "SELECT state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(state, 2);
        let command = leader.continuity.next_command_id;
        let deposit = encode_guild_bank_intent(&GuildBankIntent {
            guild: *guild.as_bytes(),
            action: GuildBankAction::Deposit(600),
        })
        .map_err(debug)?;
        let (status, reply) = exchange(
            &owner,
            &mut leader,
            command,
            COMMAND_TYPE_GUILD_BANK_INTENT,
            &deposit,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(decode_guild_bank_reply(&reply).map_err(debug)?.code, 1);
        let selected = leader.continuity.selected_capabilities;
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 40, 49, 55, 59])
                .ok_or("legacy capabilities")?;
        let before = bank_seed::financial_snapshot(&h).await?;
        house(&owner, &mut leader, 700, wire::HouseDisposition::Rejected).await?;
        assert_eq!(before, bank_seed::financial_snapshot(&h).await?);
        leader.continuity.selected_capabilities = selected;
        house(
            &owner,
            &mut members[1],
            700,
            wire::HouseDisposition::NotGuildLeader,
        )
        .await?;
        let bid_command = leader.continuity.next_command_id;
        house(&owner, &mut leader, 700, wire::HouseDisposition::Ok).await?;
        let before = bank_seed::financial_snapshot(&h).await?;
        assert_eq!(
            exchange(&owner, &mut leader, bid_command, 56, &bid(700)).await?,
            (
                CommandStatus::DuplicateReplay,
                wire::encode_house_result(wire::HouseDisposition::Ok, 0)
            )
        );
        assert_eq!(before, bank_seed::financial_snapshot(&h).await?);
        let query = encode_guild_bank_query(&GuildBankQuery {
            guild: Some(*guild.as_bytes()),
            before: None,
            after_claim: None,
        })
        .map_err(debug)?;
        let command = leader.continuity.next_command_id;
        let (status, reply) = exchange(
            &owner,
            &mut leader,
            command,
            COMMAND_TYPE_GUILD_BANK_QUERY,
            &query,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(
            decode_guild_bank_reply(&reply)
                .map_err(debug)?
                .entries
                .iter()
                .any(|e| e.kind == 4)
        );
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 40, 49, 55, 59])
                .ok_or("legacy capabilities")?;
        let command = leader.continuity.next_command_id;
        let (status, reply) = exchange(
            &owner,
            &mut leader,
            command,
            COMMAND_TYPE_GUILD_BANK_QUERY,
            &query,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(
            decode_guild_bank_reply(&reply)
                .map_err(debug)?
                .entries
                .iter()
                .all(|e| e.kind <= 3)
        );
        leader.continuity.selected_capabilities = selected;
        // The new classification field is sent only after capability negotiation.
        let mut query = Vec::new();
        bytes(&mut query, 1, &id(42));
        bytes(&mut query, 2, b"oteryn:content.area.city.port_hope");
        let command = leader.continuity.next_command_id;
        let (status, classified) = exchange(&owner, &mut leader, command, 57, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(classified.windows(2).any(|field| field == [88, 3]));
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 40, 49, 55, 59])
                .ok_or("legacy capabilities")?;
        let command = leader.continuity.next_command_id;
        let (status, legacy) = exchange(&owner, &mut leader, command, 57, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(!legacy.windows(2).any(|field| field == [88, 3]));
        assert_eq!(legacy.len() + 2, classified.len());
        leader.continuity.selected_capabilities = selected;
        let audits:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.kind LIKE 'GUILDHALL_%' AND (NOT EXISTS(SELECT 1 FROM game_guild_bank_event_outbox e WHERE e.operation_id=o.operation_id AND e.owner='HOUSE') OR (SELECT count(*) FROM game_guild_bank_event_outbox e WHERE e.operation_id=o.operation_id)<>CASE WHEN o.bank_before<>o.bank_after THEN 3 ELSE 2 END))").fetch_one(&h.pool).await?;
        assert!(audits);
        let custody:(i64,i64)=sqlx::query_as("SELECT escrow_guild_gold,escrow_gold-escrow_guild_gold FROM game_house_bids WHERE guild_id=encode($1,'hex')::uuid").bind(guild.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        if ending == 3 {
            assert_eq!(custody, (600, 500100));
            let current_fence = leader.item_fence.ok_or("leader fence")?;
            leader.item_fence = Some(
                crate::durability::item_transfer::CurrentCharacterItemFence {
                    connection_generation: crate::foundation::ConnectionGeneration::new(2)
                        .map_err(debug)?,
                    ..current_fence
                },
            );
            let snapshot = bank_seed::financial_snapshot(&h).await?;
            let command = leader.continuity.next_command_id;
            assert!(
                exchange(&owner, &mut leader, command, 56, &bid(700))
                    .await
                    .is_err(),
                "stale generation must close the connection"
            );
            assert_eq!(snapshot, bank_seed::financial_snapshot(&h).await?);

            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        assert_eq!(custody, (600, 200));
        house(&owner, &mut leader, 550, wire::HouseDisposition::Ok).await?;
        let custody:(i64,i64)=sqlx::query_as("SELECT escrow_guild_gold,escrow_gold-escrow_guild_gold FROM game_house_bids WHERE guild_id=encode($1,'hex')::uuid").bind(guild.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(custody, (600, 50));
        let snapshot = bank_seed::financial_snapshot(&h).await?;
        house(
            &owner,
            &mut leader,
            1001,
            wire::HouseDisposition::InsufficientFunds,
        )
        .await?;
        assert_eq!(bank_seed::financial_snapshot(&h).await?, snapshot);
        let bank:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(bank, 350);

        // The existing Guild owner performs the leader change; the Hall bid keeps
        // its original contributors and item provenance is not rewritten.
        let revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let successor = *members[0]
            .item_fence
            .ok_or("successor fence")?
            .character_id
            .as_bytes();
        bank_seed::balance(&h, seed::sid(90, 2), successor, 500, 180).await?;
        let request = wire::GuildhallLeadershipIntent {
            guild: *guild.as_bytes(),
            successor,
            expected_revision: revision.parse()?,
        };
        let leadership_command = leader.continuity.next_command_id;
        let (status, result) = exchange(
            &owner,
            &mut leader,
            leadership_command,
            wire::COMMAND_TYPE_GUILDHALL_LEADERSHIP,
            &wire::encode_guildhall_leadership(request).map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            guild_wire::decode_guild_intent_result(&result)
                .map_err(debug)?
                .code,
            1
        );
        house(
            &owner,
            &mut leader,
            600,
            wire::HouseDisposition::NotGuildLeader,
        )
        .await?;
        let new_leader = &mut members[0];
        house(&owner, new_leader, 600, wire::HouseDisposition::StaleLeader).await?;
        bank_command(
            &owner,
            new_leader,
            *guild.as_bytes(),
            GuildBankAction::Deposit(100),
        )
        .await?;
        house(&owner, new_leader, 600, wire::HouseDisposition::Ok).await?;
        house(&owner, new_leader, 400, wire::HouseDisposition::Ok).await?;
        let old_bank:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(old_bank, 400);
        if ending == 2 {
            bank_command(
                &owner,
                &mut members[0],
                *guild.as_bytes(),
                GuildBankAction::Disband("Hall Guild".into()),
            )
            .await?;
            // The auction is still open: disband must refund without waiting seven days.
            for _ in 0..4 {
                owner
                    .drain_source_house_dispositions()
                    .await
                    .map_err(debug)?;
            }
            let custody:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_bids WHERE guild_id=encode($1,'hex')::uuid AND state=1").bind(guild.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(custody, 0);
            let old_bank:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(old_bank, 400);
            for _ in 0..6 {
                owner
                    .root
                    .advance_guild_disband(
                        &authority,
                        &h.node,
                        crate::durability::guild::GuildDisbandJobRequest {
                            world_id: world,
                            guild_id: guild,
                        },
                    )
                    .await
                    .map_err(debug)?;
            }
            let state: i16 = sqlx::query_scalar(
                "SELECT state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
            )
            .bind(guild.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(state, 4);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        // Exercise the actual background owner after an immutable retained due
        // auction. Only the test clock/deadline input is moved into the past.
        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second' WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
        owner
            .drain_source_house_dispositions()
            .await
            .map_err(debug)?;
        let property:(i16,Option<Vec<u8>>,Option<Vec<u8>>)=sqlx::query_as("SELECT state,uuid_send(owner_guild_id),uuid_send(owner_character_id) FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(42).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(property, (3, Some(guild.as_bytes().to_vec()), None));
        let slots: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_housing_slots WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(slots, 0);
        assert_eq!(access(&h, successor).await?.as_deref(), Some("OWNER"));
        assert_eq!(access(&h, id(41)).await?.as_deref(), Some("SUBOWNER"));
        // Rent reads the current leader, not the bid's captured funding Account.
        sqlx::query("UPDATE game_house_properties SET paid_until=clock_timestamp()-interval '1 second',revision=revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
        for _ in 0..2 {
            owner
                .drain_source_house_dispositions()
                .await
                .map_err(debug)?;
        }
        let guild_balance: i64 = sqlx::query_scalar(
            "SELECT balance FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(guild_balance, 500);
        let snapshot = bank_seed::financial_snapshot(&h).await?;
        owner
            .drain_source_house_dispositions()
            .await
            .map_err(debug)?;
        assert_eq!(bank_seed::financial_snapshot(&h).await?, snapshot);
        // Demotion changes the live entry role; a kick revokes it without copied
        // ACL entries or a reconnect. Both mutations arrive through the router.
        let revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let target = *members[3]
            .item_fence
            .ok_or("member fence")?
            .character_id
            .as_bytes();
        guild_command(
            &owner,
            &mut members[0],
            guild_wire::GuildIntent::SetRank {
                guild: *guild.as_bytes(),
                character: target,
                level: 3,
                expected_revision: revision.parse()?,
            },
        )
        .await?;
        assert_eq!(access(&h, target).await?.as_deref(), Some("GUEST"));
        guild_command(
            &owner,
            &mut members[0],
            guild_wire::GuildIntent::Kick {
                guild: *guild.as_bytes(),
                character: target,
            },
        )
        .await?;
        assert_eq!(access(&h, target).await?, None);
        // Explicit guests are scoped to this tenure and must disappear on release.
        let mut acl = vec![8, 1, 16, 1];
        bytes(&mut acl, 3, &target);
        let mut request = Vec::new();
        bytes(&mut request, 4, &acl);
        let mut house_target = Vec::new();
        bytes(&mut house_target, 1, &id(42));
        bytes(&mut house_target, 2, HOUSE.as_bytes());
        bytes(&mut request, 8, &house_target);
        let command = members[0].continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut members[0], command, 56, &request).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_result(wire::HouseDisposition::Ok, 2)
            )
        );
        assert_eq!(access(&h, target).await?.as_deref(), Some("GUEST"));
        // Exercise rent fallback through the current leader's Account ledger.
        bank_command(
            &owner,
            &mut members[0],
            *guild.as_bytes(),
            GuildBankAction::Withdraw(450),
        )
        .await?;
        sqlx::query("UPDATE game_house_properties SET paid_until=clock_timestamp()-interval '1 second',revision=revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&h.pool).await?;
        for _ in 0..2 {
            owner
                .drain_source_house_dispositions()
                .await
                .map_err(debug)?;
        }
        let balance:(i64,i64)=sqlx::query_as("SELECT g.balance,b.balance FROM game_guild_bank_balances g CROSS JOIN game_account_bank_balances b WHERE g.guild_id=encode($1,'hex')::uuid AND b.account_id=encode($2,'hex')::uuid AND b.world_id=encode($3,'hex')::uuid")
            .bind(guild.as_bytes().as_slice()).bind(seed::sid(90,2).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(balance, (0, 800));
        // Retained interior fixture: provenance deliberately belongs to the old
        // leader, not the current payer. Runtime settlement must preserve it.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE TEMP TABLE hall_items ON COMMIT DROP AS SELECT game_character_uuid_v7() AS item,game_character_uuid_v7() AS mint,game_character_uuid_v7() AS placement").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT item,encode($1,'hex')::uuid,'Item','oteryn:content.item.vase','r1',1,1,mint FROM hall_items").bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations SELECT item,encode($1,'hex')::uuid,$2,'\\x01'::bytea,1,placement FROM hall_items").bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) SELECT item,encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,placement,1 FROM hall_items").bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        if reject_ordinary_release {
            // Fail even on an early-state call: guild work must never enter the
            // personal release function, including before it reaches DRAINED.
            sqlx::query("CREATE OR REPLACE FUNCTION game_house_move_out_release(op UUID) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN RAISE EXCEPTION 'ordinary release called for guild disposition' USING ERRCODE='23514'; END $$")
                .execute(&h.pool).await?;
        }
        if ending == 1 {
            bank_command(
                &owner,
                &mut members[0],
                *guild.as_bytes(),
                GuildBankAction::Disband("Hall Guild".into()),
            )
            .await?;
        } else {
            // Test clock input: the seven-day rent grace has already elapsed.
            sqlx::query("UPDATE game_house_properties SET paid_until=clock_timestamp()-interval '8 days',rent_gold=2000,revision=revision+1 WHERE house_key=$1")
                .bind(HOUSE).execute(&h.pool).await?;
        }
        let mut saw_drained_owned = false;
        for _ in 0..8 {
            owner
                .drain_source_house_dispositions()
                .await
                .map_err(debug)?;
            if reject_ordinary_release {
                let disposition: Option<(i16, bool, Option<Vec<u8>>)> = sqlx::query_as(
                    "SELECT d.state,d.released_at IS NOT NULL,uuid_send(p.owner_guild_id) FROM game_house_dispositions d JOIN game_house_properties p USING(world_id,house_key) WHERE d.house_key=$1 AND d.state<>4",
                )
                .bind(HOUSE)
                .fetch_optional(&h.pool)
                .await?;
                if let Some((3, false, owner_guild)) = disposition {
                    assert_eq!(owner_guild.as_deref(), Some(guild.as_bytes().as_slice()));
                    saw_drained_owned = true;
                }
            }
        }
        if reject_ordinary_release {
            assert!(saw_drained_owned, "guild custody drains before its own release pass");
        }
        let release: (i16, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT state,uuid_send(owner_guild_id) FROM game_house_properties WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(release, (1, None));
        let acl_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_house_acl_entries WHERE house_key=$1")
                .bind(HOUSE)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(acl_count, 0);
        let cause:String=sqlx::query_scalar("SELECT cause FROM game_house_dispositions WHERE house_key=$1 AND released_at IS NOT NULL").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(
            cause,
            if ending == 1 {
                "GUILD_DISBAND"
            } else {
                "EVICTION"
            }
        );
        let returned:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_inbox_deliveries WHERE character_id=encode($1,'hex')::uuid AND cause_kind='HOUSE_DISPOSITION'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(returned, 1);
        let banned:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_house_bans WHERE guild_id=encode($1,'hex')::uuid)").bind(guild.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(banned, ending == 0);
        if ending == 1 {
            for _ in 0..6 {
                owner
                    .root
                    .advance_guild_disband(
                        &authority,
                        &h.node,
                        crate::durability::guild::GuildDisbandJobRequest {
                            world_id: world,
                            guild_id: guild,
                        },
                    )
                    .await
                    .map_err(debug)?;
            }
            let state: i16 = sqlx::query_scalar(
                "SELECT state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
            )
            .bind(guild.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(state, 4);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
