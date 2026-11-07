-- BANK-3: Henricus is a cause-3 blessing purchase with one fee and a bounded grant set.
-- Preserve historical receipts, fee-source guards and the Character revision chain.
ALTER TABLE game_character_npc_purchase_receipts ADD COLUMN blessing_keys TEXT[];
DO $$
DECLARE name TEXT; body TEXT;
BEGIN
 SELECT conname,pg_get_constraintdef(oid) INTO STRICT name,body FROM pg_constraint
 WHERE conrelid='game_character_npc_purchase_receipts'::regclass AND contype='c'
   AND pg_get_constraintdef(oid) LIKE '%service_key%' AND pg_get_constraintdef(oid) LIKE '%blessing_key%';
 EXECUTE format('ALTER TABLE game_character_npc_purchase_receipts DROP CONSTRAINT %I',name);
 EXECUTE 'ALTER TABLE game_character_npc_purchase_receipts ADD CONSTRAINT game_npc_purchase_service_shape CHECK ((('
   ||substr(body,7)||') AND blessing_keys IS NULL) OR (cause_kind=3 AND npc_key=''oteryn:npc.henricus''
      AND service_key=''oteryn:service.blessing.henricus'' AND blessing_key IS NULL
      AND blessing_keys IS NOT NULL AND cardinality(blessing_keys)=5
      AND array_ndims(blessing_keys)=1 AND array_lower(blessing_keys,1)=1
      AND array_position(blessing_keys,NULL) IS NULL
      AND blessing_keys <@ ARRAY[''oteryn:blessing.the_spiritual_shielding'',
        ''oteryn:blessing.the_embrace_of_tibia'',''oteryn:blessing.the_fire_of_the_suns'',
        ''oteryn:blessing.the_spark_of_the_phoenix'',''oteryn:blessing.the_wisdom_of_solitude'']::text[]))';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 -- 0298 retains historical pre-repricing receipts with a NOT VALID constraint.
 -- Keep that historical validation state while checking every new row.
 body:=regexp_replace(body,' NOT VALID$','');
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
   ||substr(body,7)||') OR (cause_kind=3 AND charm_key IS NULL AND purchase_service_key=''oteryn:service.blessing.henricus'')) NOT VALID';
END;
$$;

CREATE OR REPLACE FUNCTION game_character_npc_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
          JOIN game_character_roots c ON c.character_id=f.character_id
         WHERE f.transaction_id=NEW.fee_transaction_id AND f.character_id=NEW.character_id
           AND f.cause_kind=NEW.cause_kind AND f.cause_occurrence_id=NEW.purchase_occurrence_id
           AND f.purchase_service_key=NEW.service_key AND f.fee_gold_units=NEW.fee_gold_units
           AND f.committed_character_revision=NEW.committed_character_revision
           AND f.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
           AND c.account_id=NEW.account_id AND c.character_revision>=NEW.committed_character_revision)
       OR (NEW.cause_kind=2 AND NOT EXISTS (
           SELECT 1 FROM game_character_promotions p WHERE p.character_id=NEW.character_id
             AND p.purchase_occurrence_id=NEW.purchase_occurrence_id))
       OR (NEW.cause_kind=3 AND EXISTS (
           SELECT 1 FROM unnest(COALESCE(NEW.blessing_keys,ARRAY[NEW.blessing_key])) AS k(key)
           WHERE NOT EXISTS (SELECT 1 FROM game_character_blessings b
             WHERE b.character_id=NEW.character_id AND b.blessing_key=k.key
               AND b.provenance=NEW.purchase_occurrence_id::text))) THEN
        RAISE EXCEPTION 'NPC purchase must commit with fee, receipt and effect' USING ERRCODE='23514';
    END IF;
    IF NEW.blessing_keys IS NOT NULL AND (
        cardinality(NEW.blessing_keys)<>(SELECT count(DISTINCT k) FROM unnest(NEW.blessing_keys) k)
        OR NOT EXISTS (SELECT 1 FROM game_character_quest_tracks q
            WHERE q.character_id=NEW.character_id AND q.quest_key='oteryn:quest.the_inquisition_quest'
              AND q.track_key='oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline' AND q.value=25)
        OR NEW.fee_gold_units<>(CASE WHEN NEW.level_before<=30 THEN 2000
              WHEN NEW.level_before<120 THEN 2000+200*(NEW.level_before-30)
              ELSE 20000+75*(NEW.level_before-120) END)*cardinality(NEW.blessing_keys)*11/10
        OR EXISTS (SELECT 1 FROM unnest(ARRAY['oteryn:blessing.the_spiritual_shielding',
            'oteryn:blessing.the_embrace_of_tibia','oteryn:blessing.the_fire_of_the_suns',
            'oteryn:blessing.the_spark_of_the_phoenix','oteryn:blessing.the_wisdom_of_solitude']) k(key)
            WHERE NOT EXISTS (SELECT 1 FROM game_character_blessings b
                WHERE b.character_id=NEW.character_id AND b.blessing_key=k.key))) THEN
        RAISE EXCEPTION 'Henricus requires quest completion, exact fee and complete regular held set' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_character_blessing_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_npc_purchase_receipts r
        WHERE r.character_id=NEW.character_id AND NEW.blessing_key=ANY(COALESCE(r.blessing_keys,ARRAY[r.blessing_key]))
          AND r.purchase_occurrence_id::text=NEW.provenance AND r.cause_kind=3
          AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'blessing requires this transaction purchase' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Re-establish function hardening explicitly after replacing the bodies.
DO $$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_character_npc_purchase_guard()',
     'game_character_blessing_purchase_guard()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
   EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
 END LOOP;
END;
$$;
