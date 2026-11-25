use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{
    FakePlatform, Ledger, SlowClaimReads, purchase, purchase_request, serve, uuid,
};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SWORD: &str = "store:exercise-sword";

#[derive(Clone, Copy)]
enum Case {
    FullBackpack,
    SlowDiscovery,
}

#[test]
fn full_backpack_refuses_claim_until_space_is_available() -> TestResult {
    claim_case("store-full-backpack-claim", Case::FullBackpack)
}

#[test]
fn slow_uncached_claim_leaves_time_for_a_queued_balance_read() -> TestResult {
    claim_case("store-bounded-claim-discovery", Case::SlowDiscovery)
}

fn claim_case(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "store_b2_claim", true, false).await?;
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let (generation, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    deps.admitted.world_id,
                    None,
                );
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation.active().ok_or("active generation")?);
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                harness.root.clone(),
            )
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
            let account: sqlx::types::Uuid =
                sqlx::query_scalar("SELECT account_id FROM game_character_roots")
                    .fetch_one(&harness.pool)
                    .await?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence =
                Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
            admitted.controller.as_mut().ok_or("controller")?.account_id = *account.as_bytes();

            let ledger = Arc::new(Mutex::new(Ledger {
                balance: 60,
                world: uuid(admitted.world_id.as_bytes()),
                advertise_claims: true,
                ..Ledger::default()
            }));
            owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
                Box::new(FakePlatform(ledger.clone())),
                "oteryn-game/store-consumer".into(),
            ))));
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[wire::CAPABILITY_STORE_V1],
            )
            .ok_or("Store selection")?;
            let order = purchase(&owner, &mut admitted, purchase_request(80, SWORD, 25)?).await?;
            assert_eq!(order.code, wire::StorePurchaseCode::Accepted);
            let delivery = order.operation_id.ok_or("delivery")?;
            let claim = (
                wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
                wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
                    fulfilment_unit_id: uuid(&delivery),
                })
                .map_err(debug)?,
            );

            match case {
                Case::FullBackpack => {
                    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
                        .bind(admitted.world_id.as_bytes().as_slice())
                        .bind(admitted.channel_id.as_bytes().as_slice())
                        .execute(&harness.pool).await?;
                    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                        .bind(admitted.world_id.as_bytes().as_slice())
                        .bind(admitted.channel_id.as_bytes().as_slice())
                        .bind(pin.identity().server_artifact_digest().as_slice())
                        .bind(pin.identity().client_artifact_digest().as_slice())
                        .bind(pin.frame_binding().digest().as_slice())
                        .execute(&harness.pool).await?;
                    test_capacity(&owner, &mut admitted, &harness, &claim).await?;
                }
                Case::SlowDiscovery => {
                    test_discovery(&owner, &mut admitted, &ledger, &claim).await?;
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

async fn test_capacity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    claim: &(u32, Vec<u8>),
) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(harness, &[("oteryn:item.tibia.i6120", 1); 20])
        .await?;
    let claim = std::slice::from_ref(claim);
    let refused = serve(owner, admitted, claim).await?;
    assert_eq!(
        refused,
        [(CommandStatus::Rejected, Vec::new())],
        "full backpack must give a retryable command refusal"
    );

    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=$1")
        .bind(sqlx::types::Uuid::from_bytes(id(90)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=$1")
        .bind(sqlx::types::Uuid::from_bytes(id(90)))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let accepted = serve(owner, admitted, claim).await?;
    let [(status, bytes)] = accepted.as_slice() else {
        return Err("expected a claim reply".into());
    };
    assert_eq!(*status, CommandStatus::Accepted);
    let receipt = wire::decode_claim_receipt(bytes).map_err(debug)?;
    assert_eq!(receipt.result, wire::StoreClaimCode::Committed);
    assert!(receipt.decided_at_unix_ms > 0);
    assert_eq!(serve(owner, admitted, claim).await?, accepted);
    let moves: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_loop_move_lines WHERE from_kind=6 AND to_kind=1",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(moves, 1);
    Ok(())
}

async fn test_discovery(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Mutex<Ledger>,
    claim: &(u32, Vec<u8>),
) -> TestResult {
    let slow = Arc::new(SlowClaimReads::default());
    {
        let mut ledger = ledger.lock().map_err(debug)?;
        let claim = ledger
            .intents
            .values()
            .find(|intent| intent["operation_kind"] == "CLAIM")
            .ok_or("Platform claim")?
            .clone();
        // The buyer's claim is beyond the first page presented to this command.
        ledger
            .intents
            .retain(|_, intent| intent["operation_kind"] != "CLAIM");
        for ordinal in 0..32 {
            let mut unrelated = claim.clone();
            let operation = uuid(&id(150 + ordinal));
            unrelated["operation_id"] = operation.clone().into();
            unrelated["account_id"] = uuid(&id(190)).into();
            unrelated["source_revision"] = "1".into();
            ledger.intents.insert(operation, unrelated);
        }
        ledger.slow_claim_reads = Some(slow.clone());
    }
    let account = uuid(&admitted.controller.ok_or("controller")?.account_id);
    let platform = owner.store.as_ref().ok_or("Store client")?;
    let bound = Duration::from_secs(4);
    let request = serve(owner, admitted, std::slice::from_ref(claim));
    let balance = async {
        slow.first_started.notified().await;
        platform.lock().await.balance(&account).await.map_err(debug)
    };
    let (reply, balance) = tokio::join!(
        tokio::time::timeout(bound, request),
        tokio::time::timeout(bound, balance)
    );
    assert!(
        reply.is_ok() && balance.is_ok(),
        "a stalled claim blocked command 186 or the queued balance read"
    );
    assert_eq!(reply??, [(CommandStatus::Rejected, Vec::new())]);
    let balance = balance??;
    let [coins] = balance.balances.as_slice() else {
        return Err("expected coin balance".into());
    };
    assert_eq!(coins.available, "35");
    Ok(())
}
