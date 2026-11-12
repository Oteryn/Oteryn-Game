use super::*;
use oteryn_protocol_oteryn::coin_transfer::{self as gift, Request as Gift};
async fn call(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: &mut crate::gameplay_transport::connection::AdmittedSession,
    r: Gift,
) -> TestResult<Value> {
    let id = session.continuity.next_command_id;
    let bytes = gift::encode(&r).map_err(debug)?;
    let (status, result) =
        super::super::cyclopedia::support::exchange(owner, session, id, gift::COMMAND, &bytes)
            .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    Ok(serde_json::from_str(
        &gift::decode_reply(&result).map_err(debug)?,
    )?)
}
fn id_of(v: &Value) -> TestResult<[u8; 16]> {
    let s = v.as_str().ok_or("quote id")?.replace('-', "");
    let mut id = [0; 16];
    for (i, b) in id.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)?;
    }
    Ok(id)
}
pub(super) async fn exercise(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::{
        durability::guild_chat_fixture as chars, native_admission_source::store::StoreClient,
    };
    let recipient = chars::sid(91, 1);
    let recipient_account = chars::sid(91, 2);
    chars::historical_character(
        &h.pool,
        91,
        crate::domain::CharacterId::from_bytes(recipient).map_err(debug)?,
        recipient_account,
        id(99),
        "Gift Recipient",
    )
    .await?;
    let shared = std::sync::Arc::new(std::sync::Mutex::new(peer::State::default()));
    {
        let mut s = shared.lock().map_err(|_| "peer")?;
        s.wallets.insert(
            uuid(&id(40)),
            peer::Wallet {
                transferable: 100,
                non_transferable: 200,
                reserved: 0,
            },
        );
        s.wallets
            .insert(uuid(&recipient_account), peer::Wallet::default());
    }
    owner.store = Some(std::sync::Arc::new(tokio::sync::Mutex::new(
        StoreClient::new(Box::new(peer::NoStore), "platform-test".into())
            .with_coins(Some(peer::client(&shared))),
    )));
    let review = Gift {
        action: 1,
        recipient: "Gift Recipient".into(),
        amount: 25,
        ..Default::default()
    };
    let quote = call(owner, admitted, review.clone()).await?;
    assert_eq!(quote["result"], "ok");
    assert_eq!(quote["non_transferable"], "200");
    assert!(!quote.to_string().contains("account_id"));
    let confirm = Gift {
        action: 2,
        quote: id_of(&quote["quote"])?,
        operation: id(120),
        ..review.clone()
    };
    shared.lock().map_err(|_| "peer")?.dropped_ack = true;
    let result = call(owner, admitted, confirm.clone()).await?;
    assert_eq!(result["pending"], true);
    assert!(result["receipt"].is_null());
    assert_eq!(shared.lock().map_err(|_| "peer")?.mutations, 1);
    sqlx::query("UPDATE game_coin_operations SET next_attempt=clock_timestamp()")
        .execute(&h.pool)
        .await?;
    let worker = h
        .root
        .coin_owner(authority, &h.node, admitted.world_id, None)?;
    let mut restarted = peer::client(&shared);
    assert!(
        crate::gameplay_transport::coin_market::reconcile_coin_market(&worker, &mut restarted)
            .await
    );
    let receipt = call(owner, admitted, confirm.clone()).await?;
    assert_eq!(receipt["pending"], false);
    assert_eq!(receipt["receipt"]["receipt"]["kind"], "transfer");
    assert!(!receipt.to_string().contains("account_id"));
    assert_eq!(shared.lock().map_err(|_| "peer")?.mutations, 1);
    {
        let s = shared.lock().map_err(|_| "peer")?;
        assert_eq!(
            s.wallets
                .get(&uuid(&recipient_account))
                .ok_or("recipient")?
                .transferable,
            25
        );
        let w = s.wallets.get(&uuid(&id(40))).ok_or("sender")?;
        assert_eq!((w.transferable, w.non_transferable), (75, 200));
    }
    let conflict = call(
        owner,
        admitted,
        Gift {
            amount: 50,
            ..confirm.clone()
        },
    )
    .await?;
    assert_eq!(conflict["result"], "idempotency_conflict");
    let used = call(
        owner,
        admitted,
        Gift {
            operation: id(121),
            ..confirm
        },
    )
    .await?;
    assert_eq!(used["result"], "quote_changed_or_used");
    let name: String =
        sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=$1::uuid")
            .bind(uuid(&id(41)))
            .fetch_one(&h.pool)
            .await?;
    let own = call(
        owner,
        admitted,
        Gift {
            recipient: name,
            ..review.clone()
        },
    )
    .await?;
    assert_eq!(own["result"], "same_account");
    let quote = call(owner, admitted, review.clone()).await?;
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query("UPDATE game_coin_gift_quotes SET expires_at=clock_timestamp()-interval '1 second' WHERE quote_id=$1::uuid").bind(quote["quote"].as_str().ok_or("quote")?).execute(&mut *seed).await?;
    seed.commit().await?;
    let expired = call(
        owner,
        admitted,
        Gift {
            action: 2,
            quote: id_of(&quote["quote"])?,
            operation: id(122),
            ..review.clone()
        },
    )
    .await?;
    assert_eq!(expired["result"], "quote_expired");
    let quote = call(owner, admitted, review.clone()).await?;
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query(
        "UPDATE game_character_roots SET name='Renamed Recipient' WHERE character_id=$1::uuid",
    )
    .bind(uuid(&recipient))
    .execute(&mut *seed)
    .await?;
    seed.commit().await?;
    let changed = call(
        owner,
        admitted,
        Gift {
            action: 2,
            quote: id_of(&quote["quote"])?,
            operation: id(123),
            ..review.clone()
        },
    )
    .await?;
    assert_eq!(changed["result"], "recipient_changed");
    let review = Gift {
        recipient: "Renamed Recipient".into(),
        ..review
    };
    let reassigned = call(owner, admitted, review.clone()).await?;
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query("UPDATE game_character_roots SET account_id=$2::uuid WHERE character_id=$1::uuid")
        .bind(uuid(&recipient))
        .bind(uuid(&id(40)))
        .execute(&mut *seed)
        .await?;
    seed.commit().await?;
    let changed = call(
        owner,
        admitted,
        Gift {
            action: 2,
            quote: id_of(&reassigned["quote"])?,
            operation: id(125),
            ..review.clone()
        },
    )
    .await?;
    assert_eq!(changed["result"], "recipient_changed");
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query("UPDATE game_character_roots SET account_id=$2::uuid WHERE character_id=$1::uuid")
        .bind(uuid(&recipient))
        .bind(uuid(&recipient_account))
        .execute(&mut *seed)
        .await?;
    seed.commit().await?;
    let quote = call(owner, admitted, review.clone()).await?;
    shared
        .lock()
        .map_err(|_| "peer")?
        .wallets
        .get_mut(&uuid(&id(40)))
        .ok_or("sender")?
        .transferable = 0;
    let refused = call(
        owner,
        admitted,
        Gift {
            action: 2,
            quote: id_of(&quote["quote"])?,
            operation: id(124),
            ..review.clone()
        },
    )
    .await?;
    assert_eq!(refused["pending"], true);
    assert_eq!(refused["error"], "insufficient_transferable");
    let stale = admitted.item_fence.ok_or("fence")?;
    let stale = h.root.coin_owner(
        authority,
        &h.node,
        admitted.world_id,
        Some(
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: stale.character_lease_generation + 1,
                ..stale
            },
        ),
    )?;
    assert!(stale.gift_review(review).await.is_err());
    let s = shared.lock().map_err(|_| "peer")?;
    assert_eq!(s.mutations, 1);
    assert_eq!(
        s.wallets
            .get(&uuid(&id(40)))
            .ok_or("sender")?
            .non_transferable,
        200
    );
    assert_eq!(
        s.wallets
            .get(&uuid(&recipient_account))
            .ok_or("recipient")?
            .transferable,
        25
    );
    Ok(())
}
