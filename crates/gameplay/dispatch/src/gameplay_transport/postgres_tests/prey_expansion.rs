//! PREY-DIFF permanent Store delivery: current authority, immutable replay and atomic proof.
//! These cases exercise the trusted delivery/reservation owners while the price remains an
//! explicitly disabled owner decision; they do not manufacture a production coin quote.
use super::*;
use crate::durability::prey::{PreyExpansionError, PreyStoreDelivery};
use crate::durability::store::{PurchaseRequest, PurchaseRequestState, StoreDurabilityError};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_protocol_oteryn::prey as wire;
use oteryn_protocol_oteryn::CommandStatus;

#[derive(Clone, Copy)]
enum Case {
    Reservation,
    Release,
    MalformedRelease,
    Replay,
    Binding,
    Atomicity,
    Reload,
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED route_less");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "prey expansion case panicked")??;
    eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED");
    Ok(())
}

#[test]
fn prey_diff_expansion_purchase_reservation_serializes_ids_and_requires_current_fence() -> TestResult {
    run_case(Case::Reservation)
}

#[test]
fn prey_diff_expansion_purchase_release_requires_retained_definitive_refusal() -> TestResult {
    run_case(Case::Release)
}

#[test]
fn prey_diff_expansion_malformed_refusal_cannot_delete_reservation_as_runtime_role() -> TestResult {
    run_case(Case::MalformedRelease)
}

#[test]
fn prey_diff_expansion_grant_replays_exact_receipt_and_refuses_another_operation() -> TestResult {
    run_case(Case::Replay)
}

#[test]
fn prey_diff_expansion_grant_rejects_forged_binding_and_stale_replay_authority() -> TestResult {
    run_case(Case::Binding)
}

#[test]
fn prey_diff_expansion_entitlement_receipts_and_outbox_commit_or_roll_back_together() -> TestResult {
    run_case(Case::Atomicity)
}

#[test]
fn prey_diff_expansion_survives_cold_admission_and_only_next_draw_expands() -> TestResult {
    run_case(Case::Reload)
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "prey_expansion", true, false).await?;
        prepare_current_session(&h).await?;
        super::daily_reward::epoch(&h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner, deps.admitted.game_session_id, false,
        ).await.ok_or("current character fence")?;
        let formula = owner.active_generation.and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula()).ok_or("training formula")?;
        let choice = crate::gameplay_transport::vocation_choice::vocation_choice_request(
            crate::durability::character_build::BuildOccurrence::from_bytes(id(94)).map_err(debug)?,
            crate::durability::character_build::DurableBuildState::default(),
            "monk", formula, None,
        ).map_err(debug)?;
        owner.revision_sequencer.acquire(current.character_id).await.commit_build(
            owner.root, owner.character, owner.holder, current, choice, formula, None,
        ).await.map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let base = crate::gameplay_transport::daily_reward::offered(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES, true,
        );
        let offered = crate::gameplay_transport::prey::offered(&base, true);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[1, 6, 182, wire::CAPABILITY_PREY_V1])
                .ok_or("Prey capability selection")?;
        assert!(owner.admit_prey(admitted).await);
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let account: [u8; 16] = account.try_into().map_err(|_| "account UUID length")?;
        let delivery = PreyStoreDelivery {
            operation_id: id(220),
            account_id: account,
            world_id: *admitted.world_id.as_bytes(),
            fulfilment_unit_id: "prey-expansion-unit-1".into(),
        };
        assert_eq!(proof_counts(&h, &delivery).await?, (0, 0, 0, 0, 0));
        match case {
            Case::Reservation => reservation(&owner, &h, admitted, account).await?,
            Case::Release => release(&owner, &h, admitted, account).await?,
            Case::MalformedRelease => malformed_release(&owner, &h, admitted, account).await?,
            Case::Replay => replay(&owner, &h, admitted, delivery).await?,
            Case::Binding => binding(&owner, &h, admitted, delivery).await?,
            Case::Atomicity => atomicity(&owner, &h, admitted, delivery).await?,
            Case::Reload => {
                let initial = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert_eq!(initial.list_size, 9);
                assert!(!initial.expansion_owned);
                assert_eq!(initial.slots[0].creatures.len(), 9);
                let tier = initial.slots[0].allowed_tier;
                let chosen = send(&owner, &mut admitted, &wire::Request::SetTier { slot: 1, tier }).await?;
                owner.deliver_prey_expansion(admitted, delivery.clone()).await.ok_or("Store grant")?;

                // Preserve the admitted connection's command history while rebuilding the
                // transient spell/Prey state, as in the existing daily_boost reload fixture.
                let commands = std::mem::take(
                    &mut std::sync::Arc::get_mut(&mut owner.states)
                        .ok_or("isolated owner state")?.command_ingress,
                );
                drop(owner);
                owner = deps.owner(&h, &authority);
                owner.active_generation = generation.active();
                std::sync::Arc::get_mut(&mut owner.states)
                    .ok_or("isolated reloaded state")?.command_ingress = commands;
                deps.initialize(&owner).await;
                assert!(owner.admit_prey(admitted).await);
                let after = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert!(after.expansion_owned);
                assert_eq!(after.list_size, 12);
                assert_eq!(after.slots, chosen.slots, "grant must retain every existing slot");
                assert_eq!(after.slots[1].state, 0, "expansion is not a premium slot unlock");
                assert_eq!(after.slots[2].state, 0, "expansion is not a permanent slot unlock");
                let next = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert_eq!(next.status, 0);
                assert_eq!(next.slots[0].creatures.len(), 12);
                assert_eq!(next.slots[0].chosen_tier, tier);
                assert!(next.slots[0].creatures.iter().all(|key| !initial.slots[0].creatures.contains(key)));
                for slot in [2_i16, 3] {
                    sqlx::query("INSERT INTO game_account_prey_unlocks(account_id,world_id,unlock_kind,target,source,claiming_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'PREY_SLOT_PERMANENT',$4,'STORE_CLAIM',encode($3,'hex')::uuid)")
                        .bind(account.as_slice()).bind(admitted.world_id.as_bytes().as_slice())
                        .bind(current.character_id.as_bytes().as_slice()).bind(slot).execute(&h.pool).await?;
                }
                assert!(owner.admit_prey(admitted).await);
                let three = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert!(three.expansion_owned);
                assert!(three.slots.iter().all(|slot| slot.creatures.len() == 12));
                let current_keys: std::collections::BTreeSet<_> = three.slots.iter()
                    .flat_map(|slot| slot.creatures.iter().cloned()).collect();
                assert_eq!(current_keys.len(), 36, "expanded slots must never share a creature");
                let first_keys: std::collections::BTreeSet<_> = initial.slots[0].creatures.iter().cloned().collect();
                assert!(current_keys.is_disjoint(&first_keys), "today's old nine must not be offered again");
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_offered WHERE character_id=encode($1,'hex')::uuid AND reset_epoch=1")
                    .bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, 45);
                assert_eq!(proof_counts(&h, &delivery).await?, (1, 1, 1, 1, 1));
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn purchase(admitted: AdmittedSession, account: [u8; 16], seed: u8) -> TestResult<PurchaseRequest> {
    Ok(PurchaseRequest {
        purchase_request_id: id(seed),
        account_id: account,
        character_id: *admitted.item_fence.ok_or("item fence")?.character_id.as_bytes(),
        // This durable boundary accepts the immutable binding digest, not catalogue prices.
        binding: b"prey-expansion-reservation-test:v1".to_vec(),
    })
}

async fn reservation(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, account: [u8; 16],
) -> TestResult {
    let first = purchase(admitted, account, 221)?;
    let second = purchase(admitted, account, 222)?;
    for request in [&first, &second] {
        assert_eq!(owner.root.record_purchase_request(request.clone()).await
            .map_err(|error| format!("reservation journal: {error:?}"))?, PurchaseRequestState::Sent);
    }
    let current = current(owner, admitted).await?;
    // Each Root admits one semantic pass at a time. Race independent owners against the
    // same database so this qualifies the purchase lock instead of local pass admission.
    let other = h.independent_root().map_err(|error| format!("reservation independent root: {error}"))?;
    assert!(other.maintain_ready_once().await
        .map_err(|error| format!("reservation independent readiness: {error:?}"))?);
    let other_seal = h.recovery.seal_current()
        .map_err(|error| format!("reservation independent seal: {error:?}"))?;
    let other_authority = other.open_character_authority(&other_seal).await
        .map_err(|error| format!("reservation independent authority: {error:?}"))?;
    let started = std::time::Instant::now();
    let (a, b) = {
        let mut first_call = std::pin::pin!(owner.root.reserve_prey_expansion_purchase(
            owner.character, owner.holder, current, first.clone(),
        ));
        let mut second_call = std::pin::pin!(other.reserve_prey_expansion_purchase(
            &other_authority, owner.holder, current, second.clone(),
        ));
        let (mut first_result, mut second_result) = (None, None);
        // Poll both independent owners on every wake until each completes. This keeps
        // the database race without enabling Tokio's optional macros feature.
        std::future::poll_fn(|cx| {
            if first_result.is_none()
                && let std::task::Poll::Ready(result) = std::future::Future::poll(first_call.as_mut(), cx)
            { first_result = Some(result); }
            if second_result.is_none()
                && let std::task::Poll::Ready(result) = std::future::Future::poll(second_call.as_mut(), cx)
            { second_result = Some(result); }
            if first_result.is_some() && second_result.is_some() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        }).await;
        let first_result = first_result.ok_or("first reservation did not finish")?;
        let second_result = second_result.ok_or("second reservation did not finish")?;
        match (first_result, second_result) {
            (Ok(first), Ok(second)) => (first, second),
            results => return Err(format!("concurrent Prey reservations after {:?}: {results:?}",
                started.elapsed()).into()),
        }
    };
    assert_ne!(a, b, "exactly one concurrent purchase may pass to Platform");
    let (winner, loser) = if a { (&first, &second) } else { (&second, &first) };
    assert!(owner.root.reserve_prey_expansion_purchase(owner.character, owner.holder, current, winner.clone()).await
        .map_err(|error| format!("reservation winner replay: {error:?}"))?);
    assert!(!owner.root.reserve_prey_expansion_purchase(owner.character, owner.holder, current, loser.clone()).await
        .map_err(|error| format!("reservation loser refusal: {error:?}"))?);
    let stored: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(purchase_id) FROM game_prey_expansion_purchases")
        .fetch_one(&h.pool).await?;
    assert_eq!(stored, winner.purchase_request_id);
    assert_eq!(reservation_count(h).await?, 1);

    let mut stale = current;
    stale.character_lease_generation += 1;
    assert!(matches!(owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, stale, winner.clone(),
    ).await, Err(StoreDurabilityError::AuthorityRejected)));
    let mut forged = winner.clone();
    forged.binding.push(0);
    assert!(matches!(owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, current, forged,
    ).await, Err(StoreDurabilityError::ConflictingBinding)));
    let mut wrong_character = winner.clone();
    wrong_character.character_id = id(239);
    assert!(matches!(owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, current, wrong_character,
    ).await, Err(StoreDurabilityError::AuthorityRejected)));
    assert_eq!(reservation_count(h).await?, 1);
    Ok(())
}

async fn release(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, account: [u8; 16],
) -> TestResult {
    let first = purchase(admitted, account, 223)?;
    let second = purchase(admitted, account, 224)?;
    for request in [&first, &second] {
        owner.root.record_purchase_request(request.clone()).await.map_err(debug)?;
    }
    let current = current(owner, admitted).await?;
    assert!(owner.root.reserve_prey_expansion_purchase(owner.character, owner.holder, current, first.clone()).await.map_err(debug)?);
    assert!(matches!(owner.root.release_refused_prey_expansion_purchase(owner.character, first.clone()).await,
        Err(StoreDurabilityError::ConflictingBinding)));
    assert_eq!(reservation_count(h).await?, 1, "an unknown debit outcome must retain its reservation");
    let refused = serde_json::to_vec(&serde_json::json!({
        "result": "INSUFFICIENT_BALANCE", "order_id": null, "operation_ids": [],
    }))?;
    owner.root.record_store_answer(first.clone(), refused.clone()).await.map_err(debug)?;
    assert_eq!(owner.root.record_purchase_request(first.clone()).await.map_err(debug)?, PurchaseRequestState::Answered(refused));
    owner.root.release_refused_prey_expansion_purchase(owner.character, first.clone()).await.map_err(debug)?;
    assert_eq!(reservation_count(h).await?, 0);
    // Retry of the same release is harmless; a fresh request can now reach Platform.
    owner.root.release_refused_prey_expansion_purchase(owner.character, first).await.map_err(debug)?;
    assert!(owner.root.reserve_prey_expansion_purchase(owner.character, owner.holder, current, second.clone()).await.map_err(debug)?);
    let accepted = serde_json::to_vec(&serde_json::json!({
        "result": "ACCEPTED", "order_id": "accepted-order", "operation_ids": ["accepted-operation"],
    }))?;
    owner.root.record_store_answer(second.clone(), accepted).await.map_err(debug)?;
    assert!(matches!(owner.root.release_refused_prey_expansion_purchase(owner.character, second).await,
        Err(StoreDurabilityError::ConflictingBinding)));
    assert_eq!(reservation_count(h).await?, 1);
    Ok(())
}

async fn malformed_release(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, account: [u8; 16],
) -> TestResult {
    let request = purchase(admitted, account, 227)?;
    owner.root.record_purchase_request(request.clone()).await.map_err(debug)?;
    assert!(owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, current(owner, admitted).await?, request.clone(),
    ).await.map_err(debug)?);
    let malformed = serde_json::to_vec(&serde_json::json!({
        "result": "INSUFFICIENT_BALANCE", "order_id": "unexpected-order", "operation_ids": [],
    }))?;
    owner.root.record_store_answer(request.clone(), malformed).await.map_err(debug)?;
    assert!(matches!(owner.root.release_refused_prey_expansion_purchase(owner.character, request.clone()).await,
        Err(StoreDurabilityError::ConflictingBinding)));
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
    let error = sqlx::query("DELETE FROM game_prey_expansion_purchases WHERE purchase_id=encode($1,'hex')::uuid")
        .bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await
        .err().ok_or("runtime deleted a reservation after a malformed refusal")?;
    assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
    tx.rollback().await?;
    assert_eq!(reservation_count(h).await?, 1);
    Ok(())
}

async fn replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, delivery: PreyStoreDelivery,
) -> TestResult {
    let receipt = owner.deliver_prey_expansion(admitted, delivery.clone()).await.ok_or("initial grant")?;
    let encoded = serde_json::to_vec(&receipt)?;
    let stored: Vec<u8> = sqlx::query_scalar("SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid")
        .bind(delivery.operation_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(encoded, stored);
    let before_revision = super::revision(h).await?;
    for _ in 0..2 {
        let again = owner.deliver_prey_expansion(admitted, delivery.clone()).await.ok_or("same-operation replay")?;
        assert_eq!(serde_json::to_vec(&again)?, stored, "receipt timestamp and recovery binding must be stable");
    }
    assert_eq!(super::revision(h).await?, before_revision);
    assert_eq!(proof_counts(h, &delivery).await?, (1, 1, 1, 1, 1));
    let mut another = delivery.clone();
    another.operation_id = id(225);
    another.fulfilment_unit_id = "prey-expansion-unit-2".into();
    let catalogue = owner.prey_catalogue().ok_or("Prey catalogue")?;
    let fence = current(owner, admitted).await?;
    assert!(matches!(owner.root.grant_prey_list_expansion(
        owner.character, owner.holder, fence, another.clone(), &catalogue, &NoPremium,
    ).await, Err(PreyExpansionError::AlreadyOwned)));
    assert_eq!(proof_counts(h, &another).await?, (1, 0, 0, 0, 0));
    let purchase = purchase(admitted, delivery.account_id, 226)?;
    owner.root.record_purchase_request(purchase.clone()).await.map_err(debug)?;
    assert!(!owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, current(owner, admitted).await?, purchase,
    ).await.map_err(debug)?, "owned entitlement must refuse before a new debit");
    assert_eq!(reservation_count(h).await?, 0);
    Ok(())
}

async fn binding(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, delivery: PreyStoreDelivery,
) -> TestResult {
    let mut wrong_account = delivery.clone();
    wrong_account.account_id = id(238);
    let mut wrong_world = delivery.clone();
    wrong_world.world_id = id(237);
    let mut bad_unit = delivery.clone();
    bad_unit.fulfilment_unit_id = String::new();
    let mut oversized_unit = delivery.clone();
    oversized_unit.fulfilment_unit_id = "ż".repeat(256) + "x"; // 513 UTF-8 bytes
    for forged in [wrong_account, wrong_world, bad_unit, oversized_unit] {
        assert!(owner.deliver_prey_expansion(admitted, forged).await.is_none());
        assert_eq!(proof_counts(h, &delivery).await?, (0, 0, 0, 0, 0));
    }
    let mut stale = admitted;
    stale.item_fence.as_mut().ok_or("item fence")?.character_lease_generation += 1;
    assert!(owner.deliver_prey_expansion(stale, delivery.clone()).await.is_none());
    assert_eq!(proof_counts(h, &delivery).await?, (0, 0, 0, 0, 0));

    let mut delivery = delivery;
    delivery.fulfilment_unit_id = "ż".repeat(256); // existing Store boundary is 512 bytes, not ASCII
    let receipt = owner.deliver_prey_expansion(admitted, delivery.clone()).await.ok_or("current grant")?;
    assert!(owner.deliver_prey_expansion(stale, delivery.clone()).await.is_none(), "replay must also check current authority");
    let mut different_unit = delivery.clone();
    different_unit.fulfilment_unit_id = "forged-unit".into();
    assert!(owner.deliver_prey_expansion(admitted, different_unit).await.is_none());
    assert_eq!(owner.deliver_prey_expansion(admitted, delivery.clone()).await, Some(receipt));
    assert_eq!(proof_counts(h, &delivery).await?, (1, 1, 1, 1, 1));

    // An authenticated Prey connection still has no public GrantExpansion action.
    let mut admitted = admitted;
    let command = admitted.continuity.next_command_id;
    let results = super::prey::exchange(owner, &mut admitted, &[(command, &[8, 9])]).await?;
    assert_eq!(results, [(CommandStatus::Rejected, Vec::new())]);
    assert_eq!(proof_counts(h, &delivery).await?, (1, 1, 1, 1, 1));
    Ok(())
}

async fn atomicity(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    admitted: AdmittedSession, delivery: PreyStoreDelivery,
) -> TestResult {
    // Inject a failure after the entitlement and Store receipt have been inserted but before
    // the outbox can be written. The real owner must roll back all five durable proofs.
    sqlx::query("CREATE FUNCTION prey_test_refuse_outbox() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'PREY test outbox refusal' USING ERRCODE='23514'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER prey_test_outbox_failure BEFORE INSERT ON store_receipt_outbox FOR EACH ROW EXECUTE FUNCTION prey_test_refuse_outbox()")
        .execute(&h.pool).await?;
    let before_revision = super::revision(h).await?;
    assert!(owner.deliver_prey_expansion(admitted, delivery.clone()).await.is_none());
    assert_eq!(proof_counts(h, &delivery).await?, (0, 0, 0, 0, 0));
    assert_eq!(super::revision(h).await?, before_revision);
    sqlx::query("DROP TRIGGER prey_test_outbox_failure ON store_receipt_outbox").execute(&h.pool).await?;
    sqlx::query("DROP FUNCTION prey_test_refuse_outbox()").execute(&h.pool).await?;

    // A database caller cannot persist just the flag and its delivery pointer without the
    // fenced Prey receipt, Store receipt and outbox in that same transaction.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_prey_expansion_deliveries(operation_id,character_id,world_id,occurrence,fulfilment_unit_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($1,'hex')::uuid,$4)")
        .bind(delivery.operation_id.as_slice()).bind(id(41).as_slice())
        .bind(delivery.world_id.as_slice()).bind(&delivery.fulfilment_unit_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_prey_expansions(character_id,world_id,delivery_operation_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(delivery.world_id.as_slice())
        .bind(delivery.operation_id.as_slice()).execute(&mut *tx).await?;
    assert!(tx.commit().await.is_err(), "the incomplete expansion proof must not commit");
    assert_eq!(proof_counts(h, &delivery).await?, (0, 0, 0, 0, 0));

    owner.deliver_prey_expansion(admitted, delivery.clone()).await.ok_or("retry after rollback")?;
    assert_eq!(proof_counts(h, &delivery).await?, (1, 1, 1, 1, 1));
    let same_transaction: bool = sqlx::query_scalar(
        "SELECT d.created_xact_id=e.created_xact_id AND d.created_xact_id=r.created_xact_id AND r.store_expansion AND s.item_instance_id IS NULL AND s.entitlement_key='prey-list-expansion' AND s.fulfilment_unit_id=d.fulfilment_unit_id AND s.account_id=c.account_id AND s.world_id=c.world_id FROM game_prey_expansion_deliveries d JOIN game_character_prey_expansions e ON e.delivery_operation_id=d.operation_id JOIN game_character_prey_receipts r ON r.character_id=d.character_id AND r.occurrence=d.occurrence JOIN game_character_roots c ON c.character_id=d.character_id JOIN store_delivery_receipts s ON s.operation_id=d.operation_id JOIN store_receipt_outbox o ON o.operation_id=d.operation_id WHERE d.operation_id=encode($1,'hex')::uuid",
    ).bind(delivery.operation_id.as_slice()).fetch_one(&h.pool).await?;
    assert!(same_transaction, "grant must retain one account/world-bound atomic proof");
    Ok(())
}

async fn current(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession,
) -> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    crate::gameplay_transport::character_gameplay_fence::current(owner, admitted.game_session_id, true)
        .await.ok_or_else(|| "current character fence".into())
}

async fn reservation_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_prey_expansion_purchases").fetch_one(&h.pool).await?)
}

async fn proof_counts(h: &Harness, delivery: &PreyStoreDelivery) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_prey_expansions WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_prey_expansion_deliveries WHERE operation_id=encode($2,'hex')::uuid),(SELECT count(*) FROM game_character_prey_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid),(SELECT count(*) FROM store_delivery_receipts WHERE operation_id=encode($2,'hex')::uuid),(SELECT count(*) FROM store_receipt_outbox WHERE operation_id=encode($2,'hex')::uuid)",
    ).bind(id(41).as_slice()).bind(delivery.operation_id.as_slice()).fetch_one(&h.pool).await?)
}

async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, request: &wire::Request,
) -> TestResult<wire::Reply> {
    let bytes = wire::encode_request(request).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let results = super::prey::exchange(owner, admitted, &[(command, &bytes)]).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, CommandStatus::Accepted);
    wire::decode_reply(&results[0].1).map_err(debug)
}

struct NoPremium;
impl crate::durability::character_wheel::WheelPremiumGate for NoPremium {
    async fn allocation_permitted(
        &self, _: &mut sqlx::Transaction<'_, sqlx::Postgres>, _: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(false)
    }
}
