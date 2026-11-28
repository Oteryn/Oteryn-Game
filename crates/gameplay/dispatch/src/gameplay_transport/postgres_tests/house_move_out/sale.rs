//! T135 ownership selection uses the admitted command stream, not a backend-only shortcut.
use super::*;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::premium::{IngestOutcome, PremiumConsumer};
use oteryn_protocol_oteryn::house_sale as sale;
#[path = "sale_fixture.rs"]
mod setup;
#[path = "charged.rs"]
mod charged;
#[test]
fn t135_house_transfer_catalogue_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T135 sale thread failed")??;
    Ok(())
}
#[test]
fn first_house_transfer_offer_accepts_and_settles_at_server_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "house transfer thread failed")??;
    Ok(())
}

fn scenario(first_offer: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let mut h =
            Harness::create_with_admission_seed(admin.clone(), "t135_house_sale_wire", true, false)
                .await?;
        prepare_current_session(&h).await?;
        let (rent, buyer_gold) = if first_offer { (100_000, 100_200) } else { (50, 1_000) };
        let buyer_fence = setup::prepare(&h, buyer_gold).await?;
        seed::house_and_items(&h, setup::buyer(), 3).await?;
        sqlx::query(
            "UPDATE game_house_properties SET rent_gold=$2,revision=revision+1 WHERE house_key=$1",
        )
        .bind(HOUSE)
        .bind(rent)
        .execute(&h.pool)
        .await?;
        let wrapped = seed::furniture_item(&h).await?;
        let charged = charged::seed(&h,wrapped).await?;
        h.root = seed::runtime_root(&h, &admin).await?;
        let result=async {
            let deps=Dependencies::new()?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let consumer = std::sync::Arc::<PremiumConsumer>::default();
            let (clock, activation) = if first_offer {
                let (_, clock, activation, _) = purchase_fixture::current(setup::account());
                (Some(clock), Some(activation))
            } else {
                (None, None)
            };
            let premium = SpellPremiumCoordinator::new(
                consumer.clone(), clock, activation, Some(h.root.clone())
            );
            let mut owner=deps.owner(&h,&authority);
            owner.premium_coordinator=Some(&premium);
            owner.premium = crate::premium::refresh::PremiumRefresher::new(consumer, h.root.clone(), None);
            deps.initialize(&owner).await;
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut admitted=deps.admitted;
            admitted.first_entry=FirstEntryOutcome::Positioned;
            let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id).await?.0;
            admitted.item_fence=item_fence_of(current);
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(owner.offered_capabilities(),&[sale::CAPABILITY,CAPABILITY_HOUSE_MOVE_OUT_V1])
                .ok_or("house capability")?;
            let request=sale::encode_request(&sale::HouseSaleRequest::catalogue(String::new())).map_err(debug)?;
            let (status,payload)=exchange(&owner,&mut admitted,1,sale::COMMAND,&request).await?;
            if status!=CommandStatus::Accepted {return Err(format!("T135 catalogue caller expected Accepted, got {status:?}").into());}
            let reply=sale::decode_reply(&payload).map_err(debug)?;
            assert_eq!(reply.rows.len(),1);
            assert_eq!(reply.rows[0].house,HOUSE);
            assert!(!reply.rows[0].name.is_empty());
            assert_eq!(reply.rows[0].allowed, (1<<1)|(1<<3));
            let actor={
                let mut runtime=owner.runtime.lock().await;
                let reservation=runtime.reserve_fresh_session(buyer_fence.game_session_id).map_err(debug)?;
                let actor=runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime.initialize_first_entry_position(actor).map_err(debug)?;
                runtime.bind_attacker_lease(actor,buyer_fence.game_session_id,crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(buyer_fence.character_id.as_bytes()).map_err(debug)?,buyer_fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
                actor
            };
            deps.initialize_player(&owner,actor,buyer_fence.game_session_id).await;
            let mut buyer=AdmittedSession {game_session_id:buyer_fence.game_session_id,runtime_actor:Some(actor),item_fence:Some(buyer_fence),controller:None,..admitted};
            buyer.continuity.next_command_id=1;
            if first_offer {
                first_transfer(&h,&owner,&mut admitted,&mut buyer,reply).await?;
            } else {
                mutations(&h,&owner,&mut admitted,&mut buyer,reply).await?;
            }
            setup::advance_calendar(&h).await?;
            owner.drain_source_house_dispositions().await.map_err(debug)?;
            owner.drain_source_house_dispositions().await.map_err(debug)?;
            let owner_id:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(owner_character_id) FROM game_house_properties WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
            assert_eq!(owner_id,setup::buyer());
            assert_eq!(setup::balance(&h,id(40)).await?,300);
            assert_eq!(setup::balance(&h,setup::account()).await?,buyer_gold - rent - 200);
            seed::assert_wrapped(&h,wrapped).await?;
            charged::delivered(&h,charged).await?;
            let left:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_house_interior_locations WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
            assert_eq!(left,0,"buyer receives the empty house");
            owner.drain_source_house_dispositions().await.map_err(debug)?;
            assert_eq!(setup::balance(&h,id(40)).await?,300,"World-job replay cannot pay twice");
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: &mut AdmittedSession,
    r: &sale::HouseSaleRequest,
) -> TestResult<sale::HouseSaleReply> {
    let id = actor.continuity.next_command_id;
    let payload = sale::encode_request(r).map_err(debug)?;
    let (status, payload) = exchange(owner, actor, id, sale::COMMAND, &payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "house action {} expected Accepted, got {status:?}",
            r.action
        )
        .into());
    }
    sale::decode_reply(&payload).map_err(debug)
}
fn action(kind: u8, row: &sale::HouseSaleRow) -> sale::HouseSaleRequest {
    sale::HouseSaleRequest {
        action: kind,
        house: row.house.clone(),
        expected_revision: row.revision,
        due_at_unix_ms: 0,
        recipient: String::new(),
        price: 0,
        after_house: String::new(),
    }
}
async fn first_transfer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    seller: &mut AdmittedSession,
    buyer: &mut AdmittedSession,
    page: sale::HouseSaleReply,
) -> TestResult {
    let mut offer = action(3, &page.rows[0]);
    offer.due_at_unix_ms = page.now_unix_ms + 2 * 86_400_000;
    offer.recipient = "House Recipient".into();
    offer.price = 200;
    let account = setup::account();
    let session = buyer.game_session_id;
    let consumer = owner.premium.consumer();
    owner.admit_premium(account, session);
    prove_buyer_premium(h, consumer, "before-relog", IngestOutcome::Accepted).await;
    assert!(owner.premium_gate(account).await.premium_current());
    // Use the same Premium registration boundary as logout and fresh entry.
    owner.release_premium(account, session);
    owner.admit_premium(account, session);
    consumer.load(&h.root, account).await?;
    let payload = sale::encode_request(&offer).map_err(debug)?;
    let command_id = seller.continuity.next_command_id;
    let (status, _) = exchange(owner, seller, command_id, sale::COMMAND, &payload).await?;
    assert_eq!(
        status,
        CommandStatus::Rejected,
        "durable Premium alone cannot authorize the offer"
    );
    assert_eq!(
        send(owner, seller, &sale::HouseSaleRequest::catalogue(String::new())).await?.rows,
        page.rows
    );
    prove_buyer_premium(h, consumer, "after-relog", IngestOutcome::Replayed).await;
    let offered = send(owner, seller, &offer).await?;
    assert_eq!(offered.rows[0].transfer_state, 1, "seller's first offer is recorded");
    assert_eq!(setup::balance(h, account).await?, 100_200);

    let incoming = send(owner, buyer, &sale::HouseSaleRequest::catalogue(String::new())).await?;
    let row = &incoming.rows[0];
    assert_eq!((row.due_at_unix_ms, row.price, row.rent), (offer.due_at_unix_ms, 200, 100_000));
    let accepted = send(owner, buyer, &action(5, row)).await?;
    assert_eq!(accepted.rows[0].transfer_state, 2);
    assert_eq!(setup::balance(h, account).await?, 0);
    owner.drain_source_house_dispositions().await.map_err(debug)?;
    assert_eq!(setup::balance(h, id(40)).await?, 100, "seller is paid only at execution");
    Ok(())
}

async fn prove_buyer_premium(
    h: &Harness,
    consumer: &PremiumConsumer,
    nonce: &str,
    expected: IngestOutcome,
) {
    let account = setup::account();
    let body = serde_json::json!({
        "schema": crate::premium::SNAPSHOT_SCHEMA,
        "producer_revision": "1".repeat(40),
        "producer_profile": crate::premium::PRODUCER_PROFILE,
        "nonce": nonce,
        "account_id": crate::premium::snapshot::canonical_uuid(account),
        "product_id": crate::premium::PRODUCT_ID,
        "product_version": crate::premium::PRODUCT_VERSION,
        "entitlement_id": "01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70",
        "entitlement_state": "ACTIVE",
        "lifecycle_revision": 1,
        "authority_revision": 7,
        "effective_from": "2026-09-01T00:00:00Z",
        "effective_until": "2026-10-02T00:00:00Z",
        "authority_issued_at": "2026-10-01T00:00:00Z",
        "authority_valid_until": "2026-10-01T01:00:00Z",
        "refresh_after": "2026-10-01T00:40:00Z",
    });
    assert_eq!(
        consumer.ingest(&h.root, account, nonce, body.to_string().as_bytes()).await,
        expected
    );
}

async fn mutations(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    seller: &mut AdmittedSession,
    buyer: &mut AdmittedSession,
    mut page: sale::HouseSaleReply,
) -> TestResult {
    let due = page.now_unix_ms + 2 * 86_400_000;
    let mut notice = action(1, &page.rows[0]);
    notice.due_at_unix_ms = due;
    page = send(owner, seller, &notice).await?;
    assert_eq!(page.rows[0].move_out_state, 1);
    page = send(owner, seller, &action(2, &page.rows[0])).await?;
    assert_eq!(page.rows[0].move_out_state, 0);
    let mut offer = action(3, &page.rows[0]);
    offer.due_at_unix_ms = due;
    offer.recipient = "House Recipient".into();
    offer.price = 200;
    page = send(owner, seller, &offer).await?;
    assert_eq!(page.rows[0].transfer_state, 1);
    assert_eq!(
        setup::balance(h, setup::account()).await?,
        1000,
        "offering cannot spend buyer gold"
    );
    page = send(owner, seller, &action(4, &page.rows[0])).await?;
    assert_eq!(page.rows[0].transfer_state, 0);
    for refund_action in [4, 6] {
        offer.expected_revision = page.rows[0].revision;
        page = send(owner, seller, &offer).await?;
        assert_eq!(page.rows[0].transfer_state, 1);
        let incoming = send(
            owner,
            buyer,
            &sale::HouseSaleRequest::catalogue(String::new()),
        )
        .await?;
        assert_eq!((incoming.rows[0].price, incoming.rows[0].rent), (200, 50));
        let accepted = send(owner, buyer, &action(5, &incoming.rows[0])).await?;
        assert_eq!(accepted.rows[0].transfer_state, 2);
        assert_eq!(setup::balance(h, setup::account()).await?, 750);
        if refund_action == 4 {
            page = send(owner, seller, &action(4, &accepted.rows[0])).await?;
        } else {
            assert!(
                send(owner, buyer, &action(6, &accepted.rows[0]))
                    .await?
                    .rows
                    .is_empty()
            );
            page = send(
                owner,
                seller,
                &sale::HouseSaleRequest::catalogue(String::new()),
            )
            .await?;
        }
        assert_eq!(page.rows[0].transfer_state, 0);
        assert_eq!(
            setup::balance(h, setup::account()).await?,
            1000,
            "accepted cancellation/rejection refunds price plus rent"
        );
    }
    offer.expected_revision = page.rows[0].revision;
    send(owner, seller, &offer).await?;
    let incoming = send(
        owner,
        buyer,
        &sale::HouseSaleRequest::catalogue(String::new()),
    )
    .await?;
    send(owner, buyer, &action(5, &incoming.rows[0])).await?;
    let legacy = encode_house_move_out_request(&HouseMoveOutRequest {
        house: HOUSE.into(),
        action: 2,
        due_at_unix_ms: 0,
    })
    .map_err(debug)?;
    let id = seller.continuity.next_command_id;
    let (status, _) = exchange(owner, seller, id, COMMAND_TYPE_HOUSE_MOVE_OUT, &legacy).await?;
    assert_eq!(
        status,
        CommandStatus::Rejected,
        "legacy cancellation cannot bypass transfer escrow"
    );
    Ok(())
}
