//! NX-1 account bans: actual signed wire ingress, captured live sessions and expiry.
use super::*;
use crate::domain::AccountId;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::monk_save::postgres_tests::appearance_change_postgres::observer_visibility_tests as observer;

fn clock() -> Option<(u64, u64)> {
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    Some((now, now))
}
async fn account(h: &Harness, character: u8) -> TestResult<AccountId> {
    let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(character).as_slice()).fetch_one(&h.pool).await?;
    AccountId::from_bytes(crate::durability::character_authority::uuid_text(&account)?)
        .map_err(debug)
}
async fn submit_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    envelope: &wire::SignedEnvelope,
    receipt: bool,
) -> TestResult<Vec<u8>> {
    let command = if receipt {
        protocol::staff_tools::COMMAND_TYPE_STAFF_TOOLS_RECEIPT
    } else {
        protocol::staff_tools::COMMAND_TYPE_STAFF_TOOLS_REQUEST
    };
    let (status, bytes) = exchange_raw(owner, admitted, command, &wire::encode_envelope(envelope)?)
        .await?
        .ok_or("missing staff tools reply")?;
    assert_eq!(status, protocol::CommandStatus::Accepted);
    let reply = protocol::staff_tools::decode_reply(&bytes)?;
    wire::encode_result(&reply.result).map_err(Into::into)
}
struct Chain<'a> {
    signing: &'a Signing,
    pool: &'a sqlx::PgPool,
    sequence: u64,
    previous: String,
}
impl Chain<'_> {
    async fn action(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        admitted: AdmittedSession,
        operation: u8,
        action: rules::Action,
    ) -> TestResult<(wire::SignedEnvelope, Vec<u8>)> {
        // Ban visibility and expiry are evaluated by this database's clock.
        // Keep signed request time and the fixture's trusted clock on that source.
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
        )
        .fetch_one(self.pool)
        .await?;
        let now = u64::try_from(now)?;
        let request = self.signing.request(
            self.sequence + 1,
            &self.previous,
            operation,
            now,
            action,
        );
        let signed = self.signing.sign(&canonical(&request)?, false)?;
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        let outcome = submit_wire(owner, admitted, &signed, false).await?;
        let result = wire::decode_result(&outcome)?;
        self.sequence += 1;
        self.previous = sha(&signed.canonical);
        if result.disposition == 1 && request.action.kind() != "INSPECT" {
            let receipt = rules::Receipt {
                activation_sha256: self.signing.config.newest_activation_sha256.clone(),
                control_seq: (self.sequence + 1).to_string(),
                disposition: rules::ReceiptDisposition::Committed,
                environment: self.signing.config.environment.clone(),
                operation_id: request.operation_id,
                outcome_sha256: sha(&outcome),
                previous_link_sha256: self.previous.clone(),
                request_sha256: sha(&signed.canonical),
                roster_sha256: self.signing.config.allowed_signers_sha256.clone(),
            };
            let receipt = self.signing.sign(&canonical(&receipt)?, true)?;
            let acknowledged = submit_wire(owner, admitted, &receipt, true).await?;
            assert_eq!(wire::decode_result(&acknowledged)?.disposition, 3);
            self.sequence += 1;
            self.previous = sha(&receipt.canonical);
        }
        Ok((signed, outcome))
    }
}
#[test]
fn nx1_staff_ban_unban_account_scope_live_guard_replay_and_expiry_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NX-1 ban case panicked")??;
    eprintln!("NX-1 ban PostgreSQL duration: {:?}", began.elapsed());
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let signing = Signing::new()?;
        let h = Harness::create_with_admission_seed(admin, "nx1_ban_wire", true, false).await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        bind_tools_actor(&owner,deps.actor,deps.admitted.game_session_id).await?;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let target_actor = observer::initialize(&deps, &owner).await?;
        let target_session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|_| "issuer fence")?
            .ok_or("issuer fence")?
            .item_fence();
        let target_fence = owner
            .current_quest_fence(target_session)
            .await
            .map_err(|_| "target fence")?
            .ok_or("target fence")?
            .item_fence();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                wire::CAPABILITY_STAFF_V1,
                protocol::staff_tools::CAPABILITY_STAFF_TOOLS_V1,
            ],
        )
        .ok_or("staff tools selection")?;
        let mut target_admitted = admitted;
        target_admitted.game_session_id = target_session;
        target_admitted.runtime_actor = Some(target_actor);
        target_admitted.item_fence = Some(target_fence);
        target_admitted.continuity.connection_generation = target_fence.connection_generation.get();
        target_admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
        target_admitted.controller = None;
        assert!(
            owner.staff_refresh(target_admitted).await,
            "normal player initially passes live guard"
        );
        // A victim can obtain an operator's signed inspection of that victim.
        // The signed issuer prevents transporting it through the victim's own
        // session from turning that ordinary character into a GM.
        let stolen = signing.request(2, &signing.config.newest_activation_sha256, 89, clock().ok_or("clock")?.0,
            rules::Action::Inspect { character_id: hex(&id(101)) });
        assert!(h.root.submit_staff_tools(&authority, owner.holder, target_fence,
            signing.sign(&canonical(&stolen)?,false)?,false,clock).await?.is_none());
        assert_eq!(h.root.staff_live_status(&authority,owner.holder,target_fence).await?.group,0);
        let issuer_account = account(&h, 41).await?;
        let target_account = account(&h, 101).await?;
        assert_ne!(issuer_account, target_account);
        crate::gameplay_transport::staff::test_clock::set(None);
        let unavailable = signing.request(
            2,
            &signing.config.newest_activation_sha256,
            90,
            clock().ok_or("clock")?.0,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        let unavailable = signing.sign(&canonical(&unavailable)?, false)?;
        assert_eq!(
            exchange_raw(
                &owner,
                admitted,
                protocol::staff_tools::COMMAND_TYPE_STAFF_TOOLS_REQUEST,
                &wire::encode_envelope(&unavailable)?,
            )
            .await?,
            Some((protocol::CommandStatus::Rejected, Vec::new())),
            "an unavailable trusted clock fails closed without consuming the signed chain",
        );
        assert_eq!(counts(&h).await?, (0, 0, 0));
        let mut chain = Chain {
            signing: &signing,
            pool: &h.pool,
            sequence: 1,
            previous: signing.config.newest_activation_sha256.clone(),
        };
        let (_, inspected) = chain
            .action(
                &owner,
                admitted,
                90,
                rules::Action::Inspect {
                    character_id: hex(&id(41)),
                },
            )
            .await?;
        assert_eq!(wire::decode_result(&inspected)?.code, wire::StaffCode::Ok);
        let (_, protected) = chain
            .action(
                &owner,
                admitted,
                91,
                rules::Action::Ban {
                    character_id: hex(&id(41)),
                    duration_ms: "60000".into(),
                    reason: "Protected staff regression".into(),
                },
            )
            .await?;
        assert_eq!(wire::decode_result(&protected)?.disposition, 2);
        assert!(
            !h.root
                .staff_account_banned(&authority, issuer_account)
                .await?
        );
        let (ban, first) = chain
            .action(
                &owner,
                admitted,
                92,
                rules::Action::Ban {
                    character_id: hex(&id(101)),
                    duration_ms: "60000".into(),
                    reason: "Rule violation".into(),
                },
            )
            .await?;
        assert_eq!(wire::decode_result(&first)?.code, wire::StaffCode::Ok);
        let before = counts(&h).await?;
        assert_eq!(submit_wire(&owner, admitted, &ban, false).await?, first);
        assert_eq!(counts(&h).await?, before);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_bans")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1);
        let stored_ban: (String, i64, i64, Option<i64>, i64) = sqlx::query_as(
            "SELECT account_id::text,starts_at_ms,ends_at_ms,lifted_at_ms,\
             floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_staff_bans LIMIT 1",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stored_ban.0.replace('-', ""), hex(target_account.as_bytes()));
        assert!(
            stored_ban.1 <= stored_ban.4 && stored_ban.4 < stored_ban.2,
            "committed ban must be current on its owning clock: {stored_ban:?}",
        );
        assert!(
            h.root
                .staff_account_banned(&authority, target_account)
                .await?,
            "stored ban={stored_ban:?}; host clock={:?}; fixture clock={:?}",
            clock(),
            crate::gameplay_transport::staff::test_clock::get(),
        );
        assert!(
            !h.root
                .staff_account_banned(&authority, issuer_account)
                .await?
        );
        let target = h
            .root
            .staff_live_status(&authority, owner.holder, target_fence)
            .await?;
        assert!(target.banned && target.kicked);
        assert!(
            !owner.staff_refresh(target_admitted).await,
            "ban reaches the real live connection guard"
        );
        let (_, unbanned) = chain
            .action(
                &owner,
                admitted,
                93,
                rules::Action::Unban {
                    character_id: hex(&id(101)),
                },
            )
            .await?;
        assert_eq!(wire::decode_result(&unbanned)?.code, wire::StaffCode::Ok);
        assert!(
            !h.root
                .staff_account_banned(&authority, target_account)
                .await?
        );
        assert_eq!(
            submit_wire(&owner, admitted, &ban, false).await?,
            first,
            "old ban replay cannot resurrect a lifted ban"
        );
        assert!(
            !h.root
                .staff_account_banned(&authority, target_account)
                .await?
        );
        let target = h
            .root
            .staff_live_status(&authority, owner.holder, target_fence)
            .await?;
        assert!(
            !target.banned && target.kicked,
            "unban does not resurrect captured sessions"
        );
        let future_captured:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_disconnects WHERE game_session_id=encode($1,'hex')::uuid)").bind(id(111).as_slice()).fetch_one(&h.pool).await?;
        assert!(
            !future_captured,
            "disconnect receipts cannot target a future login"
        );
        let (_, repeated) = chain
            .action(
                &owner,
                admitted,
                94,
                rules::Action::Unban {
                    character_id: hex(&id(101)),
                },
            )
            .await?;
        assert_eq!(
            wire::decode_result(&repeated)?.code,
            wire::StaffCode::AlreadyReleased
        );
        let (_, expired) = chain
            .action(
                &owner,
                admitted,
                95,
                rules::Action::Ban {
                    character_id: hex(&id(101)),
                    duration_ms: "1".into(),
                    reason: "Expiry regression".into(),
                },
            )
            .await?;
        assert_eq!(wire::decode_result(&expired)?.code, wire::StaffCode::Ok);
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while h
                .root
                .staff_account_banned(&authority, target_account)
                .await?
            {
                tokio::task::yield_now().await;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await??;
        assert!(
            !h.root
                .staff_account_banned(&authority, target_account)
                .await?
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
