//! T135 ownership selection uses the admitted command stream, not a backend-only shortcut.
use super::*;
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
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T135 sale thread failed")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let mut h =
            Harness::create_with_admission_seed(admin.clone(), "t135_house_sale_wire", true, false)
                .await?;
        prepare_current_session(&h).await?;
        let buyer_fence = setup::prepare(&h).await?;
        seed::house_and_items(&h, setup::buyer(), 3).await?;
        sqlx::query(
            "UPDATE game_house_properties SET rent_gold=50,revision=revision+1 WHERE house_key=$1",
        )
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
        let wrapped = seed::furniture_item(&h).await?;
        let charged = charged::seed(&h,wrapped).await?;
        h.root = seed::runtime_root(&h, &admin).await?;
        let result=async {
            let deps=Dependencies::new()?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let premium=crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(Default::default(),None,None,Some(h.root.clone()));
            let mut owner=deps.owner(&h,&authority);
            owner.premium_coordinator=Some(&premium);
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
            mutations(&h,&owner,&mut admitted,&mut buyer,reply).await?;
            setup::advance_calendar(&h).await?;
            owner.drain_source_house_dispositions().await.map_err(debug)?;
            owner.drain_source_house_dispositions().await.map_err(debug)?;
            let owner_id:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(owner_character_id) FROM game_house_properties WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
            assert_eq!(owner_id,setup::buyer());
            assert_eq!(setup::balance(&h,id(40)).await?,300);
            assert_eq!(setup::balance(&h,setup::account()).await?,750);
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
