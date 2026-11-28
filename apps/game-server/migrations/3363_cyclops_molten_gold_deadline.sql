-- The original Cyclops exchange starts this cup's 10-hour terminal deadline.
DO $migration$
DECLARE
    body TEXT;
    old_predicate TEXT;
    new_predicate TEXT;
    patch RECORD;
    ice CONSTANT TEXT := 'oteryn:item.tibia.i4837';
    cup CONSTANT TEXT := 'oteryn:item.tibia.i12804';
    scroll CONSTANT TEXT := 'oteryn:item.tibia.i25718';
BEGIN
    body := pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
    old_predicate := format('IF NEW.definition_production_key=%L THEN', ice);
    IF (length(body)-length(replace(body,old_predicate,'')))/length(old_predicate)<>1 THEN
        RAISE EXCEPTION 'missing quest mint schedule proof';
    END IF;
    EXECUTE replace(body, old_predicate, $proof$
    IF NEW.definition_production_key='oteryn:item.tibia.i12804' THEN
        IF NEW.definition_family='Item' AND NEW.source_ordinal=1
            AND NEW.duration_millis=36000000
            AND num_nonnulls(NEW.target_family,NEW.target_production_key,NEW.target_revision)=0
            AND EXISTS (
                SELECT 1 FROM game_quest_reward_sources s
                JOIN game_reward_claim_mint_receipts r ON r.transaction_id=s.mint_transaction_id
                JOIN game_item_instances i USING(item_instance_id)
                WHERE s.transition_key='oteryn:quest-reward/qc-friends-melt-gold'
                  AND s.plan #>> '{give,item}'=NEW.definition_production_key
                  AND (r.character_id,r.game_session_id,r.command_id)=(s.character_id,s.cause_id,s.cause_ordinal)
                  AND r.transaction_id=NEW.source_transaction_id AND r.item_instance_id=NEW.item_instance_id
                  AND r.claim_revision_ref='sha256:'||encode(NEW.content_digest,'hex')
                  AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                    =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
                  AND i.minted_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.quantity=1
                  AND NEW.expires_at_unix_ms=r.occurred_at+NEW.duration_millis
                  AND r.created_xact_id=s.created_xact_id AND NEW.created_xact_id=pg_current_xact_id()
            ) THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'molten gold deadline requires its original quest mint' USING ERRCODE='23514';
    END IF;
$proof$ || old_predicate);

    -- Retain the absolute clock across moves and retire the cup in the same
    -- custodies as other terminal quest items.
    FOR patch IN SELECT * FROM (VALUES
        ('game_item_loop_line_admit()', '='),
        ('game_temple_scroll_capture(game_item_instances,uuid)', ' NOT IN '),
        ('game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts)', ' NOT IN '),
        ('game_temple_scroll_expire_due(uuid,uuid,numeric,integer)', ' IN '),
        ('game_character_inbox_remove()', ' IN '),
        ('game_character_depot_remove()', ' IN '),
        ('game_house_location_proven()', ' IN '),
        ('game_item_corpse_entry_removal_proven()', ' IN ')
    ) AS patches(signature, comparison)
    LOOP
        body := pg_get_functiondef(patch.signature::regprocedure);
        IF patch.comparison='=' THEN
            old_predicate := format('p.definition_production_key=%L', ice);
            new_predicate := format('p.definition_production_key IN (%L,%L)', ice, cup);
        ELSE
            old_predicate := format('p.definition_production_key%s(%L,%L)', patch.comparison, scroll, ice);
            new_predicate := format('p.definition_production_key%s(%L,%L,%L)', patch.comparison, scroll, ice, cup);
        END IF;
        IF (length(body)-length(replace(body,old_predicate,'')))/length(old_predicate)<>1 THEN
            RAISE EXCEPTION 'missing quest expiry predicate in %', patch.signature;
        END IF;
        EXECUTE replace(body,old_predicate,new_predicate);
    END LOOP;
END $migration$;
