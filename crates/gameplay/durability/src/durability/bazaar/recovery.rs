//! Restore admission checks bind retained domain state to its own immutable
//! decisions. Imported qualification receipts alone do not create domain state.
//! Historical delivery proofs remain valid after normal Inbox/item movement;
//! these checks never authorize a new disposition across recovery generations.
use super::{DurabilityError, Tx};

pub(in crate::durability) async fn verify(tx: &mut Tx<'_>) -> Result<(), DurabilityError> {
    for query in [ESCROWS, LOCKS, HOLDS, FUNDING, BANK] {
        if sqlx::query_scalar::<_, bool>(query)
            .fetch_one(&mut **tx)
            .await?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

const ESCROWS: &str = r#"
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_escrows e
 LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
 LEFT JOIN game_item_tiers tier ON tier.item_instance_id=e.item_instance_id
 CROSS JOIN LATERAL (SELECT convert_from(e.item,'UTF8')::jsonb AS item) j
 WHERE i.item_instance_id IS NULL
 OR e.attributes_hash IS DISTINCT FROM sha256(e.attributes)
 OR j.item->>'item_instance_id' IS DISTINCT FROM e.item_instance_id::text
 OR j.item->>'world_id' IS DISTINCT FROM e.world_id::text
 OR j.item->'definition'->>'family' IS DISTINCT FROM 'item'
 OR j.item->>'attributes_hash' IS DISTINCT FROM encode(e.attributes_hash,'hex')
 OR NOT EXISTS(
  SELECT 1 FROM game_bazaar_item_transitions p JOIN game_bazaar_decisions d
   ON d.issuer=p.issuer AND d.operation_id=p.operation_id
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
   CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE p.escrow_id=e.escrow_id AND p.kind='ESCROW' AND p.item_instance_id=e.item_instance_id
   AND p.source_character_id=e.seller_character_id AND d.command='EscrowBazaarItem'
   AND d.result_code='CHAR_CMD_OK' AND d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
   AND d.receipt_digest=e.escrow_receipt_digest AND d.world_id=e.world_id
   AND q.intent->>'listing_id'=e.listing_id::text
   AND q.intent->>'seller_account_id'=e.seller_account_id::text
   AND q.intent->>'seller_character_id'=e.seller_character_id::text
   AND q.receipt->'result_data'->>'escrow_id'=e.escrow_id::text
   AND q.receipt->'result_data'->>'escrow_revision'='1'
   AND q.receipt->'result_data'->'item'->>'item_instance_id'=e.item_instance_id::text
   AND q.receipt->'result_data'->'item'->'definition'=j.item->'definition'
   AND q.receipt->'result_data'->'item'->'definition'->>'production_key'=p.before_item->>'definition_production_key'
   AND q.receipt->'result_data'->'item'->'definition'->>'revision_ref'=p.before_item->>'definition_revision_ref'
   AND (q.receipt->'result_data'->'item'->>'quantity')::bigint=p.quantity
   AND q.receipt->'result_data'->'item'->>'attributes_hash'=encode(e.attributes_hash,'hex')
   AND q.intent->>'item_instance_id'=p.source_item_instance_id::text
   AND q.intent->>'expected_state_revision'=p.before_item->>'state_revision'
   AND (e.state IN('DELIVERED','RETURNED') OR (
    (to_jsonb(i)-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision'])
      =(p.before_item-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision'])
    AND ((p.split AND i.state_revision=1 AND i.minted_transaction_id=p.transaction_id AND i.last_transaction_id IS NULL)
     OR (NOT p.split AND i.state_revision=(p.before_item->>'state_revision')::numeric+1
       AND i.last_transaction_id=p.transaction_id AND i.minted_transaction_id=(p.before_item->>'minted_transaction_id')::uuid))
   ))
 )
 OR (e.state IN('ESCROWED','INVALID') AND (
  i.lifecycle<>1 OR i.world_id IS DISTINCT FROM e.world_id
  OR j.item->'definition'->>'production_key' IS DISTINCT FROM i.definition_production_key
  OR j.item->'definition'->>'revision_ref' IS DISTINCT FROM i.definition_revision_ref
  OR i.definition_family NOT IN('Item','item')
  OR i.quantity IS DISTINCT FROM (j.item->>'quantity')::bigint
  OR i.state_revision IS DISTINCT FROM (j.item->>'state_revision')::numeric
  OR e.attributes IS DISTINCT FROM convert_to(CASE WHEN coalesce(tier.tier,0)=0
    THEN '{}' ELSE '{"tier":"'||tier.tier::text||'"}' END,'UTF8')
  OR j.item->'attributes' IS DISTINCT FROM CASE WHEN coalesce(tier.tier,0)=0
    THEN '[]'::jsonb ELSE jsonb_build_array(jsonb_build_object('key','tier','value',tier.tier::text)) END
  OR NOT EXISTS(SELECT 1 FROM game_bazaar_item_locations l
     WHERE l.item_instance_id=e.item_instance_id AND l.escrow_id=e.escrow_id)
  OR NOT EXISTS(SELECT 1 FROM game_bazaar_decisions d
    WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.operation_id=e.last_operation_id
     AND d.command='EscrowBazaarItem' AND d.result_code='CHAR_CMD_OK'
     AND convert_from(d.receipt,'UTF8')::jsonb->'result_data'->'item'=j.item)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_market_escrow_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_bindings WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_text_revisions WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=e.item_instance_id)
 ))
 OR (e.state IN('DELIVERED','RETURNED') AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_item_transitions p JOIN game_bazaar_decisions d
   ON d.issuer=p.issuer AND d.operation_id=p.operation_id
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
   CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  JOIN game_character_inbox_deliveries inbox ON inbox.item_instance_id=p.item_instance_id
   AND inbox.cause_ref=p.operation_id::text AND inbox.character_id=p.destination_character_id
   AND inbox.world_id=e.world_id
  WHERE p.escrow_id=e.escrow_id AND p.item_instance_id=e.item_instance_id
   AND p.operation_id=e.last_operation_id AND p.destination_character_id=e.destination_character_id
   AND p.source_character_id=e.seller_character_id AND p.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
   AND p.kind=CASE e.state WHEN 'DELIVERED' THEN 'DELIVER' ELSE 'RETURN' END
   AND d.command=CASE e.state WHEN 'DELIVERED' THEN 'DeliverBazaarItem' ELSE 'ReturnBazaarItem' END
   AND inbox.cause_kind=CASE e.state WHEN 'DELIVERED' THEN 'BAZAAR_DELIVERY' ELSE 'BAZAAR_RETURN' END
   AND d.result_code='CHAR_CMD_OK' AND d.world_id=e.world_id
   AND q.receipt->'result_data'->>'escrow_id'=e.escrow_id::text
   AND q.receipt->'result_data'->>'escrow_revision'=e.revision::text
   AND q.receipt->'result_data'->>'destination_account_id'=e.destination_account_id::text
   AND q.receipt->'result_data'->>'destination_character_id'=e.destination_character_id::text
   AND q.receipt->'result_data'->'item'=j.item
   AND q.intent->>'escrow_receipt_digest'=encode(e.escrow_receipt_digest,'hex')
   AND q.intent->>'listing_id'=e.listing_id::text
   AND q.intent->>'seller_account_id'=e.seller_account_id::text
   AND (e.state='RETURNED' OR (q.intent->>'sale_id'=e.sale_id::text
      AND q.intent->'funding'=convert_from(e.funding,'UTF8')::jsonb))
   AND (p.before_item->>'state_revision')::numeric+1=(j.item->>'state_revision')::numeric
   AND p.quantity=(j.item->>'quantity')::bigint
 ))
) OR EXISTS(
 SELECT 1 FROM game_bazaar_item_locations l LEFT JOIN game_bazaar_escrows e USING(escrow_id)
 WHERE e.escrow_id IS NULL OR e.item_instance_id<>l.item_instance_id OR e.state NOT IN('ESCROWED','INVALID')
) OR EXISTS(
 SELECT 1 FROM game_bazaar_item_transitions t JOIN game_bazaar_escrows e USING(escrow_id)
 WHERE t.kind IN('DELIVER','RETURN') AND (e.last_operation_id<>t.operation_id
   OR e.state<>CASE t.kind WHEN 'DELIVER' THEN 'DELIVERED' ELSE 'RETURNED' END)
)
"#;

const LOCKS: &str = r#"
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_locks l JOIN game_character_roots r USING(character_id)
 WHERE l.snapshot_hash IS DISTINCT FROM sha256(l.snapshot)
 OR (l.state IN('locked','INVALID') AND (r.lifecycle<>2 OR r.world_id<>l.world_id
     OR r.account_id IS DISTINCT FROM l.terminal_owner
     OR NOT EXISTS(SELECT 1 FROM game_bazaar_character_transitions t
       WHERE t.character_id=l.character_id AND t.new_revision=r.character_revision
       AND t.new_account=r.account_id AND t.new_lifecycle=r.lifecycle)))
 OR NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
    CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.command='LockBazaarCharacter'
   AND d.result_code='CHAR_CMD_OK' AND d.world_id=l.world_id
   AND q.intent->>'listing_id'=l.listing_id::text
   AND q.intent->>'seller_account_id'=l.seller_account_id::text
   AND q.receipt->'result_data'->>'lock_id'=l.lock_id::text
   AND q.receipt->'result_data'->>'character_id'=l.character_id::text
   AND q.receipt->'result_data'->'signed_snapshot'=convert_from(l.signed_snapshot,'UTF8')::jsonb
 )
 OR NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt) q
  WHERE d.operation_id=l.last_operation_id AND d.result_code='CHAR_CMD_OK' AND d.world_id=l.world_id
   AND ((d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
     AND d.command IN('LockBazaarCharacter','BindBazaarCharacterTransfer','UnlockBazaarCharacter')
     AND q.receipt->'result_data'->>'lock_id'=l.lock_id::text
     AND q.receipt->'result_data'->>'lock_revision'=l.revision::text
     AND q.receipt->'result_data'->>'character_id'=l.character_id::text)
    OR (d.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND d.command='TransferCharacterOwnership'
     AND q.receipt->>'character_id'=l.character_id::text
     AND q.receipt->>'to_account_id'=l.terminal_owner::text
     AND EXISTS(SELECT 1 FROM game_bazaar_character_transitions t WHERE t.issuer=d.issuer
       AND t.operation_id=d.operation_id AND t.character_id=l.character_id
       AND t.new_account=l.terminal_owner AND t.new_revision=(q.receipt->>'character_revision')::numeric)))
 )
 OR (l.transfer_operation_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
    CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.operation_id=l.last_operation_id AND d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
   AND d.command='BindBazaarCharacterTransfer' AND d.result_code='CHAR_CMD_OK'
   AND q.receipt->'result_data'->>'transfer_operation_id'=l.transfer_operation_id::text
   AND q.receipt->'result_data'->>'transfer_binding_digest'=encode(l.transfer_digest,'hex')
   AND q.intent->>'purpose'=l.transfer_purpose
 ))
) OR EXISTS(SELECT 1 FROM game_character_roots r WHERE r.lifecycle=2
 AND NOT EXISTS(SELECT 1 FROM game_bazaar_locks l WHERE l.character_id=r.character_id AND l.state IN('locked','INVALID')))
 OR EXISTS(
 SELECT 1 FROM game_bazaar_character_transitions t
 WHERE NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt) q
  WHERE d.issuer=t.issuer AND d.operation_id=t.operation_id AND d.result_code='CHAR_CMD_OK'
   AND ((d.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND d.command='TransferCharacterOwnership'
     AND q.receipt->>'character_id'=t.character_id::text
     AND q.receipt->>'from_account_id'=t.old_account::text
     AND q.receipt->>'to_account_id'=t.new_account::text
     AND q.receipt->>'character_revision'=t.new_revision::text AND t.old_lifecycle=t.new_lifecycle)
    OR (d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND t.old_account=t.new_account
     AND q.receipt->'result_data'->>'character_id'=t.character_id::text
     AND ((d.command='LockBazaarCharacter' AND t.old_lifecycle=1 AND t.new_lifecycle=2
       AND EXISTS(SELECT 1 FROM game_bazaar_locks l
        WHERE l.lock_id=(q.receipt->'result_data'->>'lock_id')::uuid
        AND l.character_id=t.character_id
        AND convert_from(l.snapshot,'UTF8')::jsonb->>'character_revision'=t.new_revision::text))
      OR (d.command='UnlockBazaarCharacter' AND t.old_lifecycle=2 AND t.new_lifecycle=1
       AND q.receipt->'result_data'->>'current_account_id'=t.new_account::text))))
 ))
"#;

const HOLDS: &str = r#"
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_gold_holds h
 WHERE NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb->'result_data' AS facts,
    CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.result_code='CHAR_CMD_OK'
   AND d.command IN('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold')
   AND q.facts->>'hold_id'=h.hold_id::text AND q.facts->>'hold_revision'=h.revision::text
   AND coalesce(q.facts->>'held_amount',q.facts->>'remaining_held')=h.amount::text
   AND (d.command<>'ReleaseBazaarGold' OR q.facts->>'hold_state'=
      CASE WHEN h.state='RELEASED' THEN 'RELEASED' ELSE 'ACTIVE' END)
   AND q.intent->>'listing_id'=h.listing_id::text AND q.intent->>'buyer_account_id'=h.buyer_account_id::text
   AND q.intent->>'seller_account_id'=h.seller_account_id::text AND d.world_id=h.world_id
 )
 OR (h.state='RELEASED' AND h.incoming_net<>0)
 OR (h.state='ACTIVE' AND (h.sale_id IS NOT NULL OR h.gross_amount IS NOT NULL
    OR h.delivery_operation_id IS NOT NULL OR h.delivery_receipt_digest IS NOT NULL))
 OR ((h.state='PAID' OR (h.state='RELEASED' AND h.delivery_receipt_digest IS NOT NULL)) AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d JOIN game_bazaar_gold_burns b
   ON b.issuer=d.issuer AND b.operation_id=d.operation_id
  CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.command='PayoutBazaarGold'
   AND d.result_code='CHAR_CMD_OK' AND q.intent->>'hold_id'=h.hold_id::text
   AND q.intent->>'sale_id'=h.sale_id::text AND q.intent->>'delivery_operation_id'=h.delivery_operation_id::text
   AND q.intent->>'delivery_receipt_digest'=encode(h.delivery_receipt_digest,'hex')
 ))
 OR (h.state='DISPOSITION_PENDING' AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_locks l JOIN game_bazaar_decisions d
   ON d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.operation_id=l.last_operation_id
  WHERE l.transfer_operation_id=h.delivery_operation_id AND l.listing_id=h.listing_id
   AND l.transfer_sale_id=h.sale_id AND l.transfer_buyer=h.buyer_account_id
   AND l.seller_account_id=h.seller_account_id AND l.world_id=h.world_id
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'reservation_id'=h.hold_id::text
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'hold_revision'=h.revision::text
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'gross_amount'=h.gross_amount::text
   AND d.command='BindBazaarCharacterTransfer' AND d.result_code='CHAR_CMD_OK'
 ))
 OR ((h.state IN('AWAITING_PAYOUT','PAID') OR h.delivery_receipt_digest IS NOT NULL) AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
   convert_from(d.receipt,'UTF8')::jsonb AS receipt) q
  WHERE d.operation_id=h.delivery_operation_id AND d.receipt_digest=h.delivery_receipt_digest
   AND d.result_code='CHAR_CMD_OK' AND d.world_id=h.world_id
   AND ((d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.command='DeliverBazaarItem'
     AND q.intent->>'listing_id'=h.listing_id::text AND q.intent->>'sale_id'=h.sale_id::text
     AND q.intent->>'seller_account_id'=h.seller_account_id::text
     AND q.intent->>'buyer_account_id'=h.buyer_account_id::text
     AND q.intent->'funding'->>'reservation_id'=h.hold_id::text
     AND q.intent->'funding'->>'gross_amount'=h.gross_amount::text
     AND EXISTS(SELECT 1 FROM game_bazaar_escrows e WHERE e.last_operation_id=d.operation_id
        AND e.state='DELIVERED' AND e.sale_id=h.sale_id AND e.listing_id=h.listing_id))
    OR (d.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND d.command='TransferCharacterOwnership'
     AND q.receipt->>'to_account_id'=h.buyer_account_id::text
     AND EXISTS(SELECT 1 FROM game_bazaar_locks l WHERE l.transfer_sale_id=h.sale_id
       AND l.transfer_buyer=h.buyer_account_id AND l.listing_id=h.listing_id
       AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'reservation_id'=h.hold_id::text)))
 ))
) OR EXISTS(
 SELECT 1 FROM (
  SELECT buyer_account_id AS account_id,world_id FROM game_bazaar_gold_holds WHERE amount>0
  UNION SELECT seller_account_id,world_id FROM game_bazaar_gold_holds WHERE incoming_net>0
 ) a LEFT JOIN game_account_bank_balances b USING(account_id,world_id)
 WHERE b.account_id IS NULL
 OR coalesce((SELECT sum(h.amount) FROM game_bazaar_gold_holds h
    WHERE h.buyer_account_id=a.account_id AND h.world_id=a.world_id),0)>b.balance
 OR 9000000000000000-game_account_bank_credit_headroom_before_bazaar(a.account_id,a.world_id)
    +coalesce((SELECT sum(h.incoming_net) FROM game_bazaar_gold_holds h
     WHERE h.seller_account_id=a.account_id AND h.world_id=a.world_id),0)>999999999999
)
"#;

const BANK: &str = r#"
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_gold_burns burn
 LEFT JOIN game_bazaar_decisions d ON d.issuer=burn.issuer AND d.operation_id=burn.operation_id
 LEFT JOIN game_account_bank_entries debit ON debit.bazaar_issuer=burn.issuer
   AND debit.bazaar_operation_id=burn.operation_id AND debit.bazaar_kind='BAZAAR_DEBIT'
 LEFT JOIN game_account_bank_entries credit ON credit.bazaar_issuer=burn.issuer
   AND credit.bazaar_operation_id=burn.operation_id AND credit.bazaar_kind='BAZAAR_CREDIT'
 CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
   convert_from(d.receipt,'UTF8')::jsonb->'result_data' AS facts) q
 WHERE d.operation_id IS NULL OR d.command<>'PayoutBazaarGold' OR d.result_code<>'CHAR_CMD_OK'
 OR debit.entry_id IS NULL OR credit.entry_id IS NULL
 OR debit.account_id IS DISTINCT FROM (q.intent->>'buyer_account_id')::uuid
 OR credit.account_id IS DISTINCT FROM (q.intent->>'seller_account_id')::uuid
 OR debit.world_id IS DISTINCT FROM burn.world_id OR credit.world_id IS DISTINCT FROM burn.world_id
 OR d.world_id IS DISTINCT FROM burn.world_id
 OR debit.amount IS DISTINCT FROM (q.intent->>'gross_amount')::bigint
 OR credit.amount IS DISTINCT FROM (q.intent->>'seller_amount')::bigint
 OR burn.amount IS DISTINCT FROM (q.intent->>'fee_amount')::bigint
 OR debit.amount<>credit.amount+burn.amount OR burn.amount<>ceil(debit.amount::numeric*250/10000)
 OR q.facts->>'buyer_ledger_entry_id' IS DISTINCT FROM debit.entry_id::text
 OR q.facts->>'seller_ledger_entry_id' IS DISTINCT FROM credit.entry_id::text
 OR q.facts->>'burn_entry_id' IS DISTINCT FROM burn.burn_entry_id::text
 OR NOT EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.hold_id=(q.intent->>'hold_id')::uuid
    AND h.sale_id=(q.intent->>'sale_id')::uuid AND h.delivery_operation_id=(q.intent->>'delivery_operation_id')::uuid
    AND encode(h.delivery_receipt_digest,'hex')=q.intent->>'delivery_receipt_digest')
) OR EXISTS(
 SELECT 1 FROM game_account_bank_entries e WHERE e.bazaar_operation_id IS NOT NULL
 AND NOT EXISTS(SELECT 1 FROM game_bazaar_gold_burns b WHERE b.issuer=e.bazaar_issuer AND b.operation_id=e.bazaar_operation_id)
) OR EXISTS(
 SELECT 1 FROM game_account_bank_entries e
 WHERE EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.world_id=e.world_id
   AND (h.buyer_account_id=e.account_id OR h.seller_account_id=e.account_id))
 AND ((e.previous_entry_id IS NULL AND e.balance_before<>0)
  OR (e.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p
    WHERE p.entry_id=e.previous_entry_id AND p.account_id=e.account_id AND p.world_id=e.world_id AND p.balance_after=e.balance_before))
  OR NOT(EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=e.account_id
     AND b.world_id=e.world_id AND b.last_entry_id=e.entry_id AND b.balance=e.balance_after)
    OR EXISTS(SELECT 1 FROM game_account_bank_entries n WHERE n.previous_entry_id=e.entry_id
      AND n.account_id=e.account_id AND n.world_id=e.world_id AND n.balance_before=e.balance_after)))
) OR EXISTS(
 SELECT 1 FROM game_account_bank_balances b
 WHERE EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.world_id=b.world_id
   AND (h.buyer_account_id=b.account_id OR h.seller_account_id=b.account_id))
 AND ((b.last_entry_id IS NULL AND b.balance<>0)
  OR (b.last_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e
   WHERE e.entry_id=b.last_entry_id AND e.account_id=b.account_id AND e.world_id=b.world_id AND e.balance_after=b.balance)))
)
"#;

// Reverse links matter as much as hold -> receipt links: restoring custody or
// a bound executable transfer without its consumed funding must close admission.
// Read immutable Bind decisions so unlock and a later binding cannot erase a
// prior settlement obligation. Imported receipts without a retained listing do
// not manufacture domain state.
const FUNDING: &str = r#"
WITH obligations AS (
 SELECT e.listing_id,e.seller_account_id,e.world_id,(q.intent->>'sale_id')::uuid AS sale_id,
  (q.intent->>'buyer_account_id')::uuid AS buyer_account_id,p.operation_id AS delivery_operation_id,
  q.intent->'funding' AS funding,'ITEM'::text AS asset,NULL::bytea AS binding_digest,d.recovery_generation AS binding_generation
 FROM game_bazaar_item_transitions p JOIN game_bazaar_escrows e USING(escrow_id)
 JOIN game_bazaar_decisions d ON d.issuer=p.issuer AND d.operation_id=p.operation_id
 CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK'
   THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
 WHERE p.kind='DELIVER' AND d.command='DeliverBazaarItem' AND d.result_code='CHAR_CMD_OK'
  AND q.intent->'funding'->>'currency'='GOLD'
 UNION ALL
 SELECT l.listing_id,l.seller_account_id,l.world_id,(q.intent->>'sale_id')::uuid,
  NULL::uuid,(q.intent->>'transfer_operation_id')::uuid,q.intent->'funding','CHARACTER',
  decode(q.intent->>'transfer_binding_digest','hex'),b.recovery_generation
 FROM game_bazaar_decisions b
 CROSS JOIN LATERAL (SELECT CASE WHEN b.result_code='CHAR_CMD_OK'
  THEN convert_from(b.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
  convert_from(b.receipt,'UTF8')::jsonb AS receipt) q
 JOIN game_bazaar_locks l ON l.lock_id::text=q.receipt->'result_data'->>'lock_id'
 WHERE b.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND b.command='BindBazaarCharacterTransfer'
  AND b.result_code='CHAR_CMD_OK' AND q.intent->>'purpose'='BAZAAR_SETTLEMENT'
  AND q.intent->'funding'->>'currency'='GOLD'
), evidence AS (
 SELECT o.*,d.result_code AS delivery_code,d.receipt_digest AS delivered_digest,
   d.recovery_generation AS delivery_generation,d.intent_digest AS delivered_intent_digest,
   convert_from(d.receipt,'UTF8')::jsonb AS delivered_receipt
 FROM obligations o LEFT JOIN game_bazaar_decisions d ON d.operation_id=o.delivery_operation_id
  AND d.issuer=CASE o.asset WHEN 'ITEM' THEN 'OTERYN_PLATFORM_BAZAAR_AUTHORITY'
    ELSE 'OTERYN_PLATFORM_CHARACTER_AUTHORITY' END
  AND d.command=CASE o.asset WHEN 'ITEM' THEN 'DeliverBazaarItem' ELSE 'TransferCharacterOwnership' END
)
SELECT EXISTS(SELECT 1 FROM evidence o WHERE NOT EXISTS(
 SELECT 1 FROM game_bazaar_gold_holds h
 WHERE h.hold_id::text=o.funding->>'reservation_id'
  AND h.listing_id=o.listing_id AND h.seller_account_id=o.seller_account_id AND h.world_id=o.world_id
  AND h.revision >= (o.funding->>'hold_revision')::numeric
  AND ((o.asset='ITEM' AND h.buyer_account_id=o.buyer_account_id)
    OR (o.asset='CHARACTER' AND (o.delivery_code IS DISTINCT FROM 'CHAR_CMD_OK'
       OR h.buyer_account_id::text=o.delivered_receipt->>'to_account_id')))
  AND (
   (o.delivery_code='CHAR_CMD_OK'
    AND (o.asset='ITEM' OR (o.delivered_intent_digest=o.binding_digest
      AND EXISTS(SELECT 1 FROM game_bazaar_character_transitions t
       WHERE t.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND t.operation_id=o.delivery_operation_id
       AND t.character_id::text=o.delivered_receipt->>'character_id'
       AND t.new_account=h.buyer_account_id
       AND t.new_revision=(o.delivered_receipt->>'character_revision')::numeric)))
    AND h.state IN('AWAITING_PAYOUT','PAID','RELEASED')
    AND h.sale_id=o.sale_id AND h.delivery_operation_id=o.delivery_operation_id
    AND h.delivery_receipt_digest=o.delivered_digest
    AND h.gross_amount=(o.funding->>'gross_amount')::bigint
    AND ((h.state='AWAITING_PAYOUT' AND h.revision=(o.funding->>'hold_revision')::numeric
       AND h.amount>=h.gross_amount AND h.incoming_net>=h.gross_amount-ceil(h.gross_amount::numeric*250/10000))
     OR (h.state IN('PAID','RELEASED') AND EXISTS(
      SELECT 1 FROM game_bazaar_decisions paid JOIN game_bazaar_gold_burns burn
       ON burn.issuer=paid.issuer AND burn.operation_id=paid.operation_id
      CROSS JOIN LATERAL (SELECT CASE WHEN paid.result_code='CHAR_CMD_OK'
       THEN convert_from(paid.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) p
      WHERE paid.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND paid.command='PayoutBazaarGold'
       AND paid.result_code='CHAR_CMD_OK' AND p.intent->>'hold_id'=h.hold_id::text
       AND p.intent->>'expected_hold_revision'=o.funding->>'hold_revision'
       AND p.intent->>'sale_id'=o.sale_id::text
       AND p.intent->>'delivery_operation_id'=o.delivery_operation_id::text
       AND p.intent->>'delivery_receipt_digest'=encode(o.delivered_digest,'hex')
    ))))
   OR (o.asset='CHARACTER' AND o.delivery_code IS DISTINCT FROM 'CHAR_CMD_OK'
    AND (
     (h.state='DISPOSITION_PENDING' AND h.sale_id=o.sale_id
      AND h.delivery_operation_id=o.delivery_operation_id AND h.delivery_receipt_digest IS NULL
      AND h.revision=(o.funding->>'hold_revision')::numeric
      AND h.gross_amount=(o.funding->>'gross_amount')::bigint
      AND h.amount>=h.gross_amount AND h.incoming_net>=h.gross_amount-ceil(h.gross_amount::numeric*250/10000))
     OR (o.delivery_code IS NOT NULL AND o.delivery_generation>=o.binding_generation AND EXISTS(
      SELECT 1 FROM game_bazaar_decisions released
      CROSS JOIN LATERAL (SELECT CASE WHEN released.result_code='CHAR_CMD_OK'
        THEN convert_from(released.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
        convert_from(released.receipt,'UTF8')::jsonb->'result_data' AS facts) r
      WHERE released.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND released.command='ReleaseBazaarGold'
       AND released.result_code='CHAR_CMD_OK' AND released.recovery_generation=o.delivery_generation
       AND r.intent->>'hold_id'=h.hold_id::text AND r.intent->>'release_reason'='TERMINAL_REFUSAL'
       AND r.intent->>'expected_hold_revision'=o.funding->>'hold_revision'
       AND r.facts->>'hold_id'=h.hold_id::text
       AND (r.facts->>'hold_revision')::numeric=(o.funding->>'hold_revision')::numeric+1
       AND h.revision>=(r.facts->>'hold_revision')::numeric
     ))
   ))
  )
))
"#;
