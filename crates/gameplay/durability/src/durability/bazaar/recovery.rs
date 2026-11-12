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

const ESCROWS: &str = "SELECT game_bazaar_escrows_invalid()";

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

const HOLDS: &str = "SELECT game_bazaar_holds_invalid()";

const BANK: &str = "SELECT game_bazaar_bank_invalid()";

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
