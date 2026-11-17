-- PUBPROJ: statistics transition history changes during quarantine and CANCEL.
-- Preserve every canonical custody row and its original bounds; exclude only
-- the two non-custodial G-8 history/coverage tables introduced by migration1354.
CREATE OR REPLACE FUNCTION game_account_closure_custody_hash(ch UUID) RETURNS BYTEA LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$
DECLARE t RECORD; row_value JSONB; chain BYTEA:=sha256(''::bytea); used BIGINT:=0; owned UUID[]; n BIGINT; query TEXT;
BEGIN
 SELECT array_agg(item_instance_id),count(*) INTO owned,n FROM (
  SELECT item_instance_id FROM public.game_item_container_slots WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_container_entries WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_character_equipment_slots WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_character_depot_locations WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_character_inbox_locations WHERE character_id=ch LIMIT 65537
 ) items;
 IF n>65536 THEN RAISE EXCEPTION 'closure custody bound' USING ERRCODE='54000'; END IF;
 FOR t IN SELECT c.relname,
 EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname='character_id' AND NOT a.attisdropped) AS direct
 FROM pg_class c JOIN pg_namespace ns ON ns.oid=c.relnamespace
 WHERE ns.nspname='public' AND c.relkind='r'
 AND (c.relname LIKE 'game_character_%' OR c.relname LIKE 'game_item_%')
 AND c.relname NOT LIKE '%audit%' AND c.relname NOT LIKE '%receipt%'
 AND c.relname NOT LIKE '%reservation%' AND c.relname NOT LIKE '%history%'
 -- PUBPROJ: interval/history withdrawal is not Character/item value custody.
 AND c.relname NOT IN ('game_character_online_intervals','game_character_online_pending')
 AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname IN ('character_id','item_instance_id') AND NOT a.attisdropped)
 ORDER BY c.relname LOOP
 chain:=sha256(chain||convert_to(t.relname,'UTF8'));
 IF t.direct THEN
  query:=format('SELECT j FROM (SELECT to_jsonb(r) j FROM public.%I r WHERE character_id=$1 LIMIT 65537) bounded ORDER BY j::text',t.relname);
 ELSE
  query:=format('SELECT j FROM (SELECT to_jsonb(r) j FROM public.%I r WHERE item_instance_id=ANY($2) LIMIT 65537) bounded ORDER BY j::text',t.relname);
 END IF;
 FOR row_value IN EXECUTE query USING ch,owned LOOP
  used:=used+1;
  IF used>65536 OR octet_length(row_value::text)>1048576 THEN RAISE EXCEPTION 'closure custody bound' USING ERRCODE='54000'; END IF;
  chain:=sha256(chain||convert_to(row_value::text,'UTF8'));
 END LOOP;
 END LOOP;
 RETURN chain;
END $$;
