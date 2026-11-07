-- Run in a task-owned failed native test database containing the actual mature
-- reservation that reached +1 charging. This is a trigger component proof;
-- the native settlement scenario remains the integration proof.
-- The caller owns BEGIN/ROLLBACK; no real reservation is written.
-- Only clone work-unit counters start at0 to keep all mutation checks in-budget.
CREATE TEMP TABLE mint_guard_actual_row_probe
 (LIKE game_item_mint_reservations INCLUDING DEFAULTS INCLUDING GENERATED INCLUDING CONSTRAINTS);
CREATE TRIGGER actual_guard BEFORE UPDATE OR DELETE ON mint_guard_actual_row_probe
 FOR EACH ROW EXECUTE FUNCTION game_item_mint_reservation_guard();
DO $$
DECLARE columns TEXT; original JSONB; variant JSONB; column_name TEXT;
BEGIN
 SELECT to_jsonb(r) INTO original FROM game_item_mint_reservations r
 WHERE work_units_used<3 ORDER BY occurred_at LIMIT 1;
 IF original IS NULL THEN RAISE EXCEPTION 'requires actual retained native MINT reservation';END IF;
 SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) INTO columns
 FROM pg_attribute WHERE attrelid='mint_guard_actual_row_probe'::regclass
 AND attnum>0 AND NOT attisdropped AND attgenerated='';
 -- All original opaque envelope, binding, identity and fence bytes are retained.
 -- Each namespace is tested in this isolated clone, with DB-computed key.
 FOREACH column_name IN ARRAY ARRAY['channel','instance'] LOOP
  variant:=original||jsonb_build_object('work_units_used',0)||CASE WHEN column_name='channel'
   THEN jsonb_build_object('death_channel_id','01900000-0000-7000-8000-000000000002','death_instance_id',NULL)
   ELSE jsonb_build_object('death_channel_id',NULL,'death_instance_id','01900000-0000-7000-8000-000000000003') END;
  EXECUTE format('INSERT INTO mint_guard_actual_row_probe(%s) SELECT %s FROM jsonb_populate_record(NULL::mint_guard_actual_row_probe,$1)',columns,columns) USING variant;
 END LOOP;
END $$;
CREATE TEMP TABLE mint_guard_expected AS SELECT death_scope_key,work_units_used FROM mint_guard_actual_row_probe;
UPDATE mint_guard_actual_row_probe SET work_units_used=work_units_used+1;
DO $$
DECLARE attr RECORD; changed TEXT; failure TEXT; attempts INTEGER:=0;rejects INTEGER:=0;
BEGIN
 IF (SELECT count(*) FROM mint_guard_actual_row_probe r JOIN mint_guard_expected e USING(death_scope_key)
     WHERE r.work_units_used=e.work_units_used+1)<>2 THEN
  RAISE EXCEPTION 'actual full Channel and Instance rows must preserve generated key and charge once';END IF;
 FOR attr IN SELECT attname,atttypid::regtype::text AS kind FROM pg_attribute
 WHERE attrelid='mint_guard_actual_row_probe'::regclass AND attnum>0 AND NOT attisdropped
 AND attgenerated='' AND attname<>'work_units_used' ORDER BY attnum LOOP
  changed:=CASE
   WHEN attr.kind='uuid' THEN format('CASE WHEN %I IS DISTINCT FROM %L::uuid THEN %L::uuid ELSE %L::uuid END',attr.attname,'01900000-0000-7000-8000-000000000099','01900000-0000-7000-8000-000000000099','01900000-0000-7000-8000-000000000098')
   WHEN attr.kind='bytea' THEN format('COALESCE(%I,%L::bytea)||%L::bytea',attr.attname,'','\x01')
   WHEN attr.kind IN('smallint','bigint','integer','numeric') THEN format('COALESCE(%I,0)+1',attr.attname)
   WHEN attr.kind='boolean' THEN format('NOT COALESCE(%I,false)',attr.attname)
   WHEN attr.kind='text' THEN format('COALESCE(%I,%L)||%L',attr.attname,'','_mutation')
   ELSE NULL END;
  IF changed IS NULL THEN RAISE EXCEPTION 'uncovered writable reservation column % type %',attr.attname,attr.kind;END IF;
  -- NULL scope alternatives must become a real changed value, never NULL+1.
  attempts:=attempts+1;
  BEGIN
   EXECUTE format('UPDATE mint_guard_actual_row_probe SET work_units_used=work_units_used+1,%I=%s',attr.attname,changed);
   RAISE EXCEPTION 'writable reservation column % accepted mutation',attr.attname;
  EXCEPTION WHEN check_violation THEN
   GET STACKED DIAGNOSTICS failure=MESSAGE_TEXT;
   IF failure<>'DUR-03 item MINT reservation is immutable except its work-unit charge' THEN
    RAISE EXCEPTION 'column % refused by a different constraint: %',attr.attname,failure;
   END IF;
   rejects:=rejects+1;
  END;
 END LOOP;
 IF attempts=0 OR rejects<>attempts THEN RAISE EXCEPTION 'all writable immutable columns require refusal';END IF;
 BEGIN
  UPDATE mint_guard_actual_row_probe SET death_scope_key='\x01';
  RAISE EXCEPTION 'generated scope key directly writable';
 EXCEPTION WHEN generated_always THEN NULL;END;
 BEGIN
  DELETE FROM mint_guard_actual_row_probe;
  RAISE EXCEPTION 'reservation deletion accepted';
 EXCEPTION WHEN check_violation THEN
  GET STACKED DIAGNOSTICS failure=MESSAGE_TEXT;
  IF failure<>'DUR-03 item MINT reservation is immutable except its work-unit charge' THEN
   RAISE EXCEPTION 'delete refused by a different constraint: %',failure;
  END IF;
 END;
END $$;

-- Receipt sibling: require an ACTUAL successfully materialized native corpse
-- receipt. A failed transaction cannot supply this provenance. Only the clone
-- restores its pre-mark NULL for component testing; the real receipt is untouched.
CREATE TEMP TABLE mint_receipt_actual_row_probe
 (LIKE game_item_mint_receipts INCLUDING DEFAULTS INCLUDING GENERATED INCLUDING CONSTRAINTS);
CREATE TRIGGER actual_receipt_guard BEFORE UPDATE OR DELETE ON mint_receipt_actual_row_probe
 FOR EACH ROW EXECUTE FUNCTION game_item_mint_receipt_guard();
CREATE TEMP TABLE mint_receipt_expected(death_scope_key BYTEA,materialized_at BIGINT);
DO $$
DECLARE columns TEXT; original JSONB; variant JSONB; namespace TEXT;
BEGIN
 SELECT to_jsonb(r) INTO original FROM game_item_mint_receipts r
 WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL
 AND loot_table_production_key='oteryn:item.tibia.i43758' AND death_instance_id IS NOT NULL
 ORDER BY occurred_at LIMIT 1;
 IF original IS NULL THEN RAISE EXCEPTION 'requires actual committed Rakesh Instance corpse receipt';END IF;
 SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) INTO columns
 FROM pg_attribute WHERE attrelid='mint_receipt_actual_row_probe'::regclass
 AND attnum>0 AND NOT attisdropped AND attgenerated='';
 FOREACH namespace IN ARRAY ARRAY['channel','instance'] LOOP
  variant:=original||jsonb_build_object('materialized_at',NULL)||CASE WHEN namespace='channel'
   THEN jsonb_build_object('death_channel_id','01900000-0000-7000-8000-000000000002','death_instance_id',NULL)
   ELSE jsonb_build_object('death_channel_id',NULL,'death_instance_id','01900000-0000-7000-8000-000000000003') END;
  EXECUTE format('INSERT INTO mint_receipt_actual_row_probe(%s) SELECT %s FROM jsonb_populate_record(NULL::mint_receipt_actual_row_probe,$1)',columns,columns) USING variant;
 END LOOP;
 INSERT INTO mint_receipt_expected SELECT death_scope_key,(original->>'materialized_at')::bigint FROM mint_receipt_actual_row_probe;
END $$;
DO $$
DECLARE attr RECORD; changed TEXT; failure TEXT; attempts INTEGER:=0;rejects INTEGER:=0;
BEGIN
 FOR attr IN SELECT attname,atttypid::regtype::text AS kind FROM pg_attribute
 WHERE attrelid='mint_receipt_actual_row_probe'::regclass AND attnum>0 AND NOT attisdropped
 AND attgenerated='' AND attname<>'materialized_at' ORDER BY attnum LOOP
  changed:=CASE
   WHEN attr.kind='uuid' THEN format('CASE WHEN p.%I IS DISTINCT FROM %L::uuid THEN %L::uuid ELSE %L::uuid END',attr.attname,'01900000-0000-7000-8000-000000000099','01900000-0000-7000-8000-000000000099','01900000-0000-7000-8000-000000000098')
   WHEN attr.kind='xid8' THEN format('CASE WHEN p.%I IS DISTINCT FROM %L::xid8 THEN %L::xid8 ELSE %L::xid8 END',attr.attname,'1','1','2')
   WHEN attr.kind='bytea' THEN format('COALESCE(p.%I,%L::bytea)||%L::bytea',attr.attname,'','\x01')
   WHEN attr.kind IN('smallint','bigint','integer','numeric') THEN format('COALESCE(p.%I,0)+1',attr.attname)
   WHEN attr.kind='boolean' THEN format('NOT COALESCE(p.%I,false)',attr.attname)
   WHEN attr.kind='text' THEN format('COALESCE(p.%I,%L)||%L',attr.attname,'','_mutation')
   ELSE NULL END;
  IF changed IS NULL THEN RAISE EXCEPTION 'uncovered writable receipt column % type %',attr.attname,attr.kind;END IF;
  attempts:=attempts+1;
  BEGIN
   -- OLD is still NULL here; each negative attempts the otherwise valid mark.
   EXECUTE format('UPDATE mint_receipt_actual_row_probe p SET materialized_at=e.materialized_at,%I=%s FROM mint_receipt_expected e WHERE p.death_scope_key=e.death_scope_key',attr.attname,changed);
   RAISE EXCEPTION 'writable receipt column % accepted mutation',attr.attname;
  EXCEPTION WHEN check_violation THEN
   GET STACKED DIAGNOSTICS failure=MESSAGE_TEXT;
   IF failure<>'DUR-03 item MINT receipt is immutable except its one-way materialized_at mark' THEN
    RAISE EXCEPTION 'receipt column % refused by different constraint: %',attr.attname,failure;
   END IF;
   rejects:=rejects+1;
  END;
 END LOOP;
 IF attempts=0 OR rejects<>attempts THEN RAISE EXCEPTION 'all writable receipt input columns require refusal';END IF;
END $$;
UPDATE mint_receipt_actual_row_probe p SET materialized_at=e.materialized_at
 FROM mint_receipt_expected e WHERE p.death_scope_key=e.death_scope_key;
DO $$
DECLARE statement TEXT; failure TEXT;
BEGIN
 IF (SELECT count(*) FROM mint_receipt_actual_row_probe p JOIN mint_receipt_expected e USING(death_scope_key)
     WHERE p.materialized_at=e.materialized_at)<>2 THEN
  RAISE EXCEPTION 'actual Channel and Instance clones must retain computed keys and accept one mark';END IF;
 FOREACH statement IN ARRAY ARRAY[
  'UPDATE mint_receipt_actual_row_probe SET materialized_at=NULL',
  'UPDATE mint_receipt_actual_row_probe SET materialized_at=materialized_at+1',
  'DELETE FROM mint_receipt_actual_row_probe'
 ] LOOP
  BEGIN
   EXECUTE statement;
   RAISE EXCEPTION 'receipt one-way mark/deletion guard accepted mutation';
  EXCEPTION WHEN check_violation THEN
   GET STACKED DIAGNOSTICS failure=MESSAGE_TEXT;
   IF failure<>'DUR-03 item MINT receipt is immutable except its one-way materialized_at mark' THEN
    RAISE EXCEPTION 'receipt refused by different constraint: %',failure;
   END IF;
  END;
 END LOOP;
 BEGIN
  UPDATE mint_receipt_actual_row_probe SET death_scope_key='\x01';
  RAISE EXCEPTION 'receipt generated scope key directly writable';
 EXCEPTION WHEN generated_always THEN NULL;END;
END $$;
