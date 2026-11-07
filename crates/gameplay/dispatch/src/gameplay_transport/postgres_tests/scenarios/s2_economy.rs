//! S2: sell retained loot, convert carried coin to bank gold, market to another Character,
//! retrieve that delivery from Inbox and deposit it without changing total gold or items.
use super::*;
use crate::durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankOperationOutcome, BankResult,
};
use oteryn_protocol_oteryn::market::{COMMAND_TYPE_MARKET, MarketRequest, encode_request};

#[test]
fn s2_economy_npc_bank_market_inbox_depot_conserves_and_relogs() -> TestResult {
    timed("s2-economy", scenario)
}

fn scenario() -> TestResult {
    world_scenario!("xmech_s2", |h,
                                 request,
                                 seal,
                                 authority,
                                 world_id,
                                 native,
                                 world,
                                 cells,
                                 room,
                                 pin,
                                 deps,
                                 actor,
                                 session,
                                 owner,
                                 fence,
                                 admitted| {
        step_west(&mut owner, actor, session).await?;
        step_west(&mut owner, actor, session).await?;
        let (npc, window) = merchant(&mut owner, admitted, NPC).await?;
        let index = native.item_index().ok_or("index")?;
        let (offer_index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, o)| {
                o.side == NpcTradeSide::Sell
                    && std::num::NonZeroU32::new(o.item_definition_ref)
                        .and_then(|r| {
                            crate::gameplay_transport::item_look::typed_definition(index, r)
                        })
                        .is_some_and(|d| generation_item_facts(native, &d).is_ok())
            })
            .ok_or("merchant buys loot")?;
        let definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            std::num::NonZeroU32::new(offer.item_definition_ref).ok_or("offer ref")?,
        )
        .ok_or("offer definition")?;
        mint_carried(
            &h,
            &owner,
            admitted,
            &definition.production_key,
            offer.count,
            140,
        )
        .await?;
        let initial = gold(&ledger(&h).await?);
        let sell = NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: window.open.ok_or("window")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: NpcTradeSide::Sell,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        };
        assert!(matches!(
            owner.npc_trade(actor, command(admitted, 22), sell).await,
            NpcTradeStep::Traded
        ));
        assert_eq!(
            gold(&ledger(&h).await?),
            initial + i64::from(offer.unit_price)
        );
        let sold = snapshot(&h).await?;
        assert!(matches!(
            owner.npc_trade(actor, command(admitted, 22), sell).await,
            NpcTradeStep::Traded
        ));
        assert_eq!(snapshot(&h).await?, sold);
        let amount = u64::try_from(gold(&ledger(&h).await?))?;
        deposit_bank(&h, &owner, admitted, amount).await?;
        let (ware_definition, ware_reference, client_id) = market_ware(native)?;
        let ware = mint_carried(
            &h,
            &owner,
            admitted,
            &ware_definition.production_key,
            10,
            146,
        )
        .await?;
        at_locker(&mut owner, admitted).await?;
        deposit(&h, &owner, admitted, ware, 30).await?;
        let mut other = peer(&h, &mut owner, admitted).await?;
        let other_actor = other.runtime_actor.ok_or("peer")?;
        step_west(&mut owner, other_actor, other.game_session_id).await?;
        step_west(&mut owner, other_actor, other.game_session_id).await?;
        // Retained peer bag and funding are initial fixture data, never settlement writers.
        seed_peer_bank_and_bag(&h, other).await?;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), session);
        assert!(owner.session_premium_gate(session).await.premium_current());
        for a in [&mut admitted, &mut other] {
            a.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37, 63])
                    .ok_or("market capability")?;
        }
        let total = count(
            &h,
            "SELECT sum(balance)::bigint FROM game_account_bank_balances",
        )
        .await?;
        let initial_quantity = quantity(&h, &ware_definition.production_key).await?;
        let sell = MarketRequest {
            action: 6,
            definition: ware_reference,
            offer: [0; 16],
            amount: 10,
            price: 2,
            anonymous: false,
            page: 0,
        };
        let create_id = admitted.continuity.next_command_id;
        let created = exchange(
            &owner,
            &mut admitted,
            COMMAND_TYPE_MARKET,
            &encode_request(sell).map_err(debug)?,
        )
        .await?;
        assert_eq!(created.0, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(created.1.get(1), Some(&1), "{created:?}");
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_market_escrow_locations").await?,
            1
        );
        let after_create = snapshot(&h).await?;
        assert_eq!(
            owner
                .market_command(admitted, create_id, &encode_request(sell).map_err(debug)?)
                .await
                .ok_or("market replay")?,
            created.1
        );
        assert_eq!(snapshot(&h).await?, after_create);
        let offer: Vec<u8> = sqlx::query_scalar(
            "SELECT decode(replace(offer_id::text,'-',''),'hex') FROM game_market_offers",
        )
        .fetch_one(&h.pool)
        .await?;
        let accept = MarketRequest {
            action: 8,
            definition: 0,
            offer: offer.as_slice().try_into()?,
            amount: 10,
            price: 0,
            anonymous: false,
            page: 0,
        };
        let accept_id = other.continuity.next_command_id;
        let accepted = exchange(
            &owner,
            &mut other,
            COMMAND_TYPE_MARKET,
            &encode_request(accept).map_err(debug)?,
        )
        .await?;
        assert_eq!(accepted.1.get(1), Some(&1), "{accepted:?}");
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_market_escrow_locations").await?,
            0
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_item_character_inbox_locations"
            )
            .await?,
            1
        );
        let settled = snapshot(&h).await?;
        assert_eq!(
            owner
                .market_command(other, accept_id, &encode_request(accept).map_err(debug)?)
                .await
                .ok_or("accept replay")?,
            accepted.1
        );
        assert_eq!(snapshot(&h).await?, settled);
        // The placement fee is an explicit gold sink; the price is a transfer between accounts.
        let fee: i64 =
            sqlx::query_scalar("SELECT coalesce(sum(fee),0)::bigint FROM game_market_settlements")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(fee, 20, "one placement fee, no fee on accept or replay");
        assert_eq!(
            count(
                &h,
                "SELECT sum(balance)::bigint FROM game_account_bank_balances"
            )
            .await?
                + fee,
            total
        );
        assert_eq!(
            quantity(&h, &ware_definition.production_key).await?,
            initial_quantity
        );
        let retrieve = oteryn_protocol_oteryn::depot_search::Request::Retrieve {
            client_id,
            all: true,
        };
        let result = owner
            .depot_search_command(other_actor, other, 40, retrieve)
            .await;
        assert_eq!(
            (result.outcome, result.retrieved),
            (oteryn_protocol_oteryn::depot_search::Outcome::Retrieved, 10)
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_item_character_inbox_locations"
            )
            .await?,
            0
        );
        let retrieved = snapshot(&h).await?;
        assert_eq!(
            owner
                .depot_search_command(other_actor, other, 40, retrieve)
                .await,
            result
        );
        assert_eq!(snapshot(&h).await?, retrieved);
        let delivered:Vec<u8>=sqlx::query_scalar("SELECT decode(replace(item_instance_id::text,'-',''),'hex') FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(other.item_fence.ok_or("peer fence")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        deposit(&h, &owner, other, delivered.as_slice().try_into()?, 41).await?;
        logout_relogin(&h, &mut owner, admitted).await?;
        let other = logout_relogin(&h, &mut owner, other).await?;
        let page = owner
            .depot(
                other.runtime_actor.ok_or("relogged peer")?,
                command(other, 1),
                DepotIntent {
                    town: Some(0),
                    box_number: DEPOT_BOX,
                    page: 0,
                    action: DepotAction::Browse,
                },
                None,
            )
            .await
            .ok_or("peer depot after relogin")?;
        assert_eq!(page.reply.entries.len(), 1);
        assert_eq!(page.reply.entries[0].count, 10);
        assert_eq!(
            count(
                &h,
                "SELECT sum(balance)::bigint FROM game_account_bank_balances"
            )
            .await?
                + fee,
            total
        );
        assert_eq!(
            quantity(&h, &ware_definition.production_key).await?,
            initial_quantity
        );
        drop(owner);
        drop(authority);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}

pub(super) async fn seed_peer_bank_and_bag(h: &Harness, peer: AdmittedSession) -> TestResult {
    let fence = peer.item_fence.ok_or("peer fence")?;
    let account = crate::durability::guild_chat_fixture::sid(90, 2);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)").bind(id(180).as_slice()).bind(id(42).as_slice()).bind(id(181).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(fence.character_id.as_bytes().as_slice()).bind(id(180).as_slice()).bind(id(42).as_slice()).bind(id(181).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,100,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)").bind(id(182).as_slice()).bind(id(183).as_slice()).bind(id(184).as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(185).as_slice()).bind(id(186).as_slice()).bind(id(180).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,100,0,100,encode($5,'hex')::uuid)").bind(id(187).as_slice()).bind(id(183).as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100,encode($3,'hex')::uuid)").bind(account.as_slice()).bind(id(42).as_slice()).bind(id(187).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn deposit_bank(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    amount: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let typed = |key: &str| -> TestResult<TypedDefinitionRef> {
        crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("ref")?,
        )
        .ok_or_else(|| "typed".into())
    };
    let bank = BankDepositRequest {
        identity: BankOperationIdentity {
            occurrence: BankOperationOccurrence::from_bytes(id(141)).map_err(debug)?,
            transaction_id: id(142),
            event_id: id(143),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "test/1".into(),
        },
        amount,
        coins: BankCoinFacts {
            gold: typed(GOLD)?,
            platinum: typed("oteryn:item.tibia.i3035")?,
            crystal: typed("oteryn:item.tibia.i3043")?,
            backpack: generation_item_facts(native, &typed(BACKPACK)?).map_err(debug)?,
        },
        change_item_instance_ids: [id(144), id(145)],
    };
    assert!(
        matches!(h.root.bank_deposit(owner.character,owner.holder,admitted.item_fence.ok_or("fence")?,bank.clone()).await.map_err(debug)?,BankOperationOutcome::Committed(r) if r.result==BankResult::Ok)
    );
    assert_eq!(gold(&ledger(h).await?), 0);
    assert_eq!(
        count(
            h,
            "SELECT sum(balance)::bigint FROM game_account_bank_balances"
        )
        .await?,
        i64::try_from(amount)?
    );
    let deposited = snapshot(h).await?;
    assert!(
        matches!(h.root.bank_deposit(owner.character,owner.holder,admitted.item_fence.ok_or("fence")?,bank).await.map_err(debug)?,BankOperationOutcome::Replayed(r) if r.result==BankResult::Ok)
    );
    assert_eq!(snapshot(h).await?, deposited);
    Ok(())
}
