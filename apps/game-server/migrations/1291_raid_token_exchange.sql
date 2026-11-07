-- CRAFT1 C8: source-bound Jorge exchange, no new protocol surface or quest state.
-- Rollback: withdraw the dialogue producer; committed trades remain immutable evidence.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_side_check,
 ADD CONSTRAINT game_npc_trades_side_check CHECK (side IN (1,2,3));
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_npc_trades'::regclass AND conname='game_npc_trades_conservation';
 ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_conservation;
 EXECUTE format('ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_conservation CHECK ((side IN (1,2) AND (%s)) OR (side=3 AND npc_key=''oteryn:npc.jorge'' AND catalogue_revision=1 AND quantity=1 AND item_units=1 AND unit_price=0 AND total_gold_units=0 AND burned_gold_units=0 AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND line_count=2 AND house_key IS NULL))',previous);
END $migration$;

CREATE FUNCTION game_jorge_exchange_proven(t game_npc_trades) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE b game_npc_trade_lines%ROWTYPE; m game_npc_trade_lines%ROWTYPE;
        expected INTEGER[] := ARRAY[19151,28897,23682,28895,43511,25981,23679,22027,31265,25979,25980,23678,19153,19156,28896,23681,34210,23676,23683,31268,19152,37058,31267,37059,25982,23680,23677,22028,19150,28894,19155,22026,31266,43517,37060,19154,43510];
BEGIN
 SELECT * INTO STRICT b FROM game_npc_trade_lines WHERE transaction_id=t.transaction_id AND line_ordinal=1;
 SELECT * INTO STRICT m FROM game_npc_trade_lines WHERE transaction_id=t.transaction_id AND line_ordinal=2;
 IF t.created_xact_id<>pg_current_xact_id() OR b.created_xact_id<>t.created_xact_id OR m.created_xact_id<>t.created_xact_id
  OR t.side<>3 OR t.npc_key<>'oteryn:npc.jorge' OR t.catalogue_revision<>1 OR t.offer_index NOT BETWEEN 0 AND 36
  OR t.item_definition_family<>'Item' OR t.item_definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i'||expected[t.offer_index+1]::text
  OR t.line_count<>2 OR (SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=t.transaction_id)<>2
  OR b.line_kind<>3 OR m.line_kind<>4 OR b.coin_worth IS NOT NULL OR m.coin_worth IS NOT NULL
  OR b.quantity_before-b.quantity_after<>20 OR m.quantity_before<>0 OR m.quantity_after<>1
  OR b.item_instance_id=m.item_instance_id OR m.item_instance_id<>t.item_instance_id
  OR NOT EXISTS(SELECT 1 FROM game_item_container_slots WHERE character_id=t.character_id AND item_instance_id=t.backpack_item_instance_id)
  OR m.placement_ordinal IS DISTINCT FROM 1+greatest(b.placement_ordinal,
       (SELECT max(e.placement_ordinal) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
        WHERE e.parent_item_instance_id=t.backpack_item_instance_id AND i.minted_transaction_id<>t.transaction_id),0)
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=t.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=t.transaction_id AND a.event_id=t.event_id
       AND a.item_instance_id=t.item_instance_id AND a.occurred_at=t.occurred_at AND a.envelope_sha256=t.envelope_sha256
       AND a.created_xact_id=t.created_xact_id AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'Jorge requires one exact token take and replica give with its audit' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_npc_trade_lines l LEFT JOIN game_item_instances i USING(item_instance_id)
   WHERE l.transaction_id=t.transaction_id AND (
    i.item_instance_id IS NULL OR i.world_id<>t.world_id OR i.quantity<>l.quantity_after
    OR i.lifecycle<>CASE WHEN l.quantity_after=0 THEN 2 ELSE 1 END
    OR i.definition_family<>'Item'
    OR (l.line_kind=3 AND (i.definition_production_key<>'oteryn:item.tibia.i19083'
      OR i.last_transaction_id IS DISTINCT FROM t.transaction_id OR i.minted_transaction_id=t.transaction_id
      OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev WHERE ev.item_instance_id=i.item_instance_id
         AND ev.transaction_id=t.transaction_id AND ev.quantity_before=l.quantity_before)))
    OR (l.line_kind=4 AND ((i.definition_family,i.definition_production_key,i.definition_revision_ref)
       IS DISTINCT FROM (t.item_definition_family,t.item_definition_production_key,t.item_definition_revision_ref)
       OR i.minted_transaction_id<>t.transaction_id OR i.last_transaction_id IS NOT NULL))
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR (l.quantity_after=0 AND (
       EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=i.item_instance_id)))
    OR (l.quantity_after>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e
        WHERE e.item_instance_id=i.item_instance_id AND e.character_id=t.character_id
         AND e.parent_item_instance_id=t.backpack_item_instance_id AND e.placement_ordinal=l.placement_ordinal
         AND (l.line_kind=3 OR e.placed_transaction_id=t.transaction_id)))
   )) OR EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
     WHERE e.parent_item_instance_id=t.backpack_item_instance_id AND e.placement_ordinal>b.placement_ordinal
       AND i.item_instance_id<>b.item_instance_id AND i.lifecycle=1 AND i.quantity>=20
       AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i19083'
       AND i.definition_revision_ref=(SELECT definition_revision_ref FROM game_item_instances WHERE item_instance_id=b.item_instance_id)
       AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
       AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
       AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
       AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id))
 THEN RAISE EXCEPTION 'Jorge item before-state, source order or custody is unproven' USING ERRCODE='23514'; END IF;
END $$;
REVOKE ALL ON FUNCTION game_jorge_exchange_proven(game_npc_trades) FROM PUBLIC;
DO $migration$
DECLARE previous TEXT; at INTEGER;
BEGIN
 SELECT pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure) INTO previous;
 at:=strpos(previous,'BEGIN');
 IF at=0 THEN RAISE EXCEPTION 'NPC trade guard body missing'; END IF;
 previous:=overlay(previous placing E'BEGIN\n    IF NEW.side=3 THEN PERFORM game_jorge_exchange_proven(NEW); RETURN NULL; END IF;\n' from at for 5);
 EXECUTE previous;
END $migration$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_jorge_exchange_proven(game_npc_trades) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
GRANT EXECUTE ON FUNCTION game_jorge_exchange_proven(game_npc_trades) TO oteryn_game_runtime;
