-- All Ice Delivery sources use the same absolute Item clock. Prove the
-- schedule against the reward source and the exact sample minted with it.
DO $migration$
DECLARE body TEXT; begin_at INT;
    movement_arm CONSTANT TEXT := 'AND NOT game_temple_scroll_moves(NEW.item_instance_id)';
BEGIN
    body := pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n' || $proof$
    IF NEW.definition_production_key='oteryn:item.tibia.i4837' THEN
        IF NEW.definition_family='Item' AND NEW.source_ordinal=1
            AND NEW.duration_millis=600000
            AND num_nonnulls(NEW.target_family,NEW.target_production_key,NEW.target_revision)=0
            AND EXISTS (
                SELECT 1 FROM game_quest_reward_sources s
                JOIN game_reward_claim_mint_receipts r ON r.transaction_id=s.mint_transaction_id
                JOIN game_item_instances i USING(item_instance_id)
                WHERE s.transition_key IN ('oteryn:quest-callback/explorer-society-ice-delivery-1',
                                           'oteryn:quest-callback/explorer-society-ice-delivery-2',
                                           'oteryn:quest-callback/explorer-society/cut-ice',
                                           'oteryn:quest-callback/explorer-society/cut-ice-second-site')
                  AND s.plan #>> '{give,item}'=NEW.definition_production_key
                  AND (r.character_id,r.game_session_id,r.command_id)=(s.character_id,s.cause_id,s.cause_ordinal)
                  AND r.transaction_id=NEW.source_transaction_id AND r.item_instance_id=NEW.item_instance_id
                  AND r.claim_revision_ref='sha256:'||encode(NEW.content_digest,'hex')
                  AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                    =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
                  AND i.minted_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.quantity=1
                  AND NEW.expires_at_unix_ms=r.occurred_at+NEW.duration_millis
                  AND r.created_xact_id=s.created_xact_id
                  AND NEW.created_xact_id=pg_current_xact_id()
            ) THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'ice sample deadline requires its original quest mint'
          USING ERRCODE='23514';
    END IF;
$proof$ from begin_at for 5);
    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    IF (length(body)-length(replace(body,movement_arm,'')))/length(movement_arm)<>1 THEN
        RAISE EXCEPTION 'missing absolute Item timer movement proof';
    END IF;
    EXECUTE replace(body,movement_arm,movement_arm || $movement$
        AND NOT EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules p
                          WHERE p.item_instance_id=NEW.item_instance_id
                            AND p.definition_production_key='oteryn:item.tibia.i4837')
$movement$);
END $migration$;

-- Carry forward live samples already in custody. The original mint time
-- starts the clock, even for an overdue sample awaiting normal retirement.
INSERT INTO game_spell_item_temporal_schedules (
    item_instance_id,source_transaction_id,source_ordinal,duration_millis,
    expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest
)
SELECT i.item_instance_id,r.transaction_id,1,600000,r.occurred_at+600000,
       i.definition_family,i.definition_production_key,i.definition_revision_ref,
       decode(substr(r.claim_revision_ref,8),'hex')
FROM game_quest_reward_sources s
JOIN game_reward_claim_mint_receipts r ON r.transaction_id=s.mint_transaction_id
JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
WHERE s.transition_key IN ('oteryn:quest-callback/explorer-society-ice-delivery-1',
                           'oteryn:quest-callback/explorer-society-ice-delivery-2',
                           'oteryn:quest-callback/explorer-society/cut-ice')
  AND s.plan #>> '{give,item}'='oteryn:item.tibia.i4837'
  AND (r.character_id,r.game_session_id,r.command_id)=(s.character_id,s.cause_id,s.cause_ordinal)
  AND r.created_xact_id=s.created_xact_id
  AND r.claim_revision_ref ~ '^sha256:[0-9a-f]{64}$'
  AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i4837'
  AND i.minted_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.quantity=1
ON CONFLICT (item_instance_id) DO NOTHING;

-- Retire due icicles in the same unattended custodies as terminal scrolls.
-- Keep the exact custody, retirement and audit proofs on every removal path.
DO $migration$
DECLARE patch RECORD; body TEXT; old_predicate TEXT; new_predicate TEXT;
    scroll CONSTANT TEXT := 'oteryn:item.tibia.i25718';
    sample CONSTANT TEXT := 'oteryn:item.tibia.i4837';
BEGIN
    FOR patch IN SELECT * FROM (VALUES
        ('game_temple_scroll_capture(game_item_instances,uuid)', true),
        ('game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts)', true),
        ('game_temple_scroll_expire_due(uuid,uuid,numeric,integer)', false),
        ('game_character_inbox_remove()', false),
        ('game_character_depot_remove()', false),
        ('game_house_location_proven()', false),
        ('game_item_corpse_entry_removal_proven()', false)
    ) AS patches(signature, negated)
    LOOP
        body := pg_get_functiondef(patch.signature::regprocedure);
        IF patch.negated THEN
            old_predicate := format('p.definition_production_key<>%L', scroll);
            new_predicate := format('p.definition_production_key NOT IN (%L,%L)', scroll, sample);
        ELSE
            old_predicate := format('p.definition_production_key=%L', scroll);
            new_predicate := format('p.definition_production_key IN (%L,%L)', scroll, sample);
        END IF;
        IF (length(body)-length(replace(body,old_predicate,'')))/length(old_predicate)<>1 THEN
            RAISE EXCEPTION 'missing unattended Item deadline predicate in %', patch.signature;
        END IF;
        EXECUTE replace(body,old_predicate,new_predicate);
    END LOOP;
END $migration$;
