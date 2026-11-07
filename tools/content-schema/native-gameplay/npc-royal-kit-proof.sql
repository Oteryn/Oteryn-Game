-- Only the old shop-only transaction FK is replaced by the existing strict
-- deferred shop proof OR the closed paid Royal Inbox MINT proof below.
-- Keep item/catalogue FKs and unique mint_transaction_id unchanged.
DO $royal_origin_fk$ DECLARE fk_name TEXT; fk_count INTEGER;BEGIN
 SELECT count(*),min(constraint_row.conname) INTO fk_count,fk_name FROM pg_constraint constraint_row
 WHERE constraint_row.conrelid='game_house_furniture_kit_origins'::regclass
  AND constraint_row.contype='f' AND constraint_row.confrelid='game_npc_trades'::regclass
  AND constraint_row.conkey=ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid=constraint_row.conrelid AND attname='mint_transaction_id')]::smallint[];
 IF fk_count<>1 THEN RAISE EXCEPTION 'expected exactly one legacy shop-only kit origin FK';END IF;
 EXECUTE format('ALTER TABLE game_house_furniture_kit_origins DROP CONSTRAINT %I',fk_name);
END $royal_origin_fk$;
-- Royal base purchase is a distinct source class, never an NPCtrade claim.
ALTER TABLE game_house_furniture_kit_catalogue DROP CONSTRAINT game_house_furniture_kit_catalogue_source_row_check,
 ADD CONSTRAINT game_house_furniture_kit_catalogue_source_row_check
 CHECK(source_row BETWEEN 0 AND 1674 OR source_row BETWEEN 1000000 AND 1000904 OR source_row BETWEEN 2000000 AND 2000002);
INSERT INTO game_house_furniture_kit_catalogue(source_row,target_key,price_gold,source_line,npc_key) VALUES
(2000000,'oteryn:item.tibia.i31510',500000000,137,'oteryn:npc.king_tibianus'),
(2000001,'oteryn:item.tibia.i31510',500000000,84,'oteryn:npc.queen_eloise'),
(2000002,'oteryn:item.tibia.i31510',500000000,89,'oteryn:npc.emperor_kruzak');
CREATE TABLE game_npc_royal_kit_mints(
 parent_transaction_id UUID PRIMARY KEY REFERENCES game_npc_cosmetic_receipts(transaction_id),
 fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns(transaction_id),
 mint_transaction_id UUID NOT NULL UNIQUE REFERENCES game_reward_claim_inbox_mint_receipts(transaction_id),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 source_row INTEGER NOT NULL CHECK(source_row BETWEEN 2000000 AND 2000002),
 source_key TEXT NOT NULL,program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 plan_sha256 BYTEA NOT NULL CHECK(octet_length(plan_sha256)=32),
 target_key TEXT NOT NULL CHECK(target_key='oteryn:item.tibia.i31510'),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
REVOKE ALL ON game_npc_royal_kit_mints FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_npc_royal_kit_mints TO oteryn_game_runtime;
CREATE TRIGGER game_npc_royal_kit_stamp BEFORE INSERT ON game_npc_royal_kit_mints FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_npc_royal_kit_immutable BEFORE UPDATE OR DELETE ON game_npc_royal_kit_mints FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_npc_royal_kit_no_truncate BEFORE TRUNCATE ON game_npc_royal_kit_mints FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
-- Existing accepted R1 CharacterInbox delivery translation; source free-slot20
-- remains a separate admission rule, not the mint-output batch limit.
CREATE FUNCTION game_npc_royal_inbox_admit(p_parent UUID,p_source TEXT) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$ DECLARE royal_character UUID; royal_count BIGINT; BEGIN
 IF p_source NOT IN('oteryn:npc-cosmetic/king_tibianus/golden-buy-armor','oteryn:npc-cosmetic/queen_eloise/golden-buy-armor','oteryn:npc-cosmetic/emperor_kruzak/golden-buy-armor')
  OR NOT EXISTS(SELECT 1 FROM game_npc_source_context royal_context,
   LATERAL jsonb_array_elements(royal_context.expected_plan->'effects') royal_effect
   WHERE royal_context.npc_transaction_id=p_parent AND royal_context.source_key=p_source
    AND royal_effect.value->>'kind'='mint_store_decoration_kit' AND royal_effect.value->>'source_target'='31510'
    AND royal_effect.value->>'requires_free_store_inbox_entry'='true'
    AND game_npc_source_effect_current(p_parent,p_source,royal_effect.value)) THEN RETURN FALSE; END IF;
 SELECT character_id INTO STRICT royal_character FROM game_npc_source_context WHERE npc_transaction_id=p_parent
  AND backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 INSERT INTO game_character_inbox_counters(character_id) VALUES(royal_character) ON CONFLICT DO NOTHING;
 PERFORM 1 FROM game_character_inbox_counters WHERE character_id=royal_character FOR UPDATE;
 SELECT count(*) INTO royal_count FROM game_item_character_inbox_locations WHERE character_id=royal_character;
 RETURN royal_count<20;
END $$;
CREATE FUNCTION game_npc_royal_kit_proven(p_parent UUID,p_item UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_royal_kit_mints royal_child
 JOIN game_npc_cosmetic_receipts royal_parent ON royal_parent.transaction_id=royal_child.parent_transaction_id
 JOIN game_item_fee_burns royal_fee ON royal_fee.transaction_id=royal_child.fee_transaction_id
 JOIN game_house_furniture_kit_catalogue royal_catalogue ON royal_catalogue.source_row=royal_child.source_row
 JOIN game_reward_claim_inbox_mint_receipts royal_mint ON royal_mint.transaction_id=royal_child.mint_transaction_id
 JOIN game_reward_claim_inbox_mint_reservations royal_reservation ON royal_reservation.transaction_id=royal_mint.transaction_id
 JOIN game_reward_claim_inbox_mint_lines royal_line ON royal_line.transaction_id=royal_mint.transaction_id
 JOIN game_item_instances royal_item ON royal_item.item_instance_id=royal_child.item_instance_id
 JOIN game_item_character_inbox_locations royal_inbox ON royal_inbox.item_instance_id=royal_item.item_instance_id
 JOIN game_house_furniture_kit_origins royal_origin ON royal_origin.item_instance_id=royal_item.item_instance_id
 WHERE royal_child.parent_transaction_id=p_parent AND royal_child.item_instance_id=p_item
  AND game_npc_source_parent_current(royal_parent.transaction_id,royal_parent.source_key)
  AND EXISTS(SELECT 1 FROM game_npc_source_context royal_context,
    LATERAL jsonb_array_elements(royal_context.expected_plan->'effects') royal_effect
    WHERE royal_context.npc_transaction_id=royal_parent.transaction_id
     AND royal_effect.value->>'kind'='mint_store_decoration_kit'
     AND royal_effect.value->>'source_target'='31510'
     AND royal_effect.value->>'requires_free_store_inbox_entry'='true'
     AND game_npc_source_effect_current(royal_parent.transaction_id,royal_parent.source_key,royal_effect.value))
  AND royal_parent.source_key=royal_child.source_key
  AND royal_parent.source_key='oteryn:npc-cosmetic/'||substring(royal_catalogue.npc_key FROM 12)||'/golden-buy-armor'
  AND royal_parent.npc_key=royal_catalogue.npc_key AND royal_parent.fee_transaction_id=royal_fee.transaction_id
  AND royal_parent.program_sha256=royal_child.program_sha256 AND royal_parent.plan_sha256=royal_child.plan_sha256
  AND royal_fee.character_id=royal_parent.character_id AND royal_fee.fee_gold_units=500000000
  AND royal_child.created_xact_id=pg_current_xact_id() AND royal_fee.created_xact_id=pg_current_xact_id()
  AND royal_mint.created_xact_id=pg_current_xact_id() AND royal_line.created_xact_id=pg_current_xact_id()
  AND royal_mint.game_session_id=royal_parent.game_session_id AND royal_mint.command_id=royal_parent.command_id
  AND royal_mint.character_id=royal_parent.character_id AND royal_mint.item_count=1
  AND convert_from(royal_reservation.mint_spec,'UTF8')::jsonb->'claim'=jsonb_build_object('family','NpcCosmetic','key',royal_child.source_key,'revision',encode(royal_child.program_sha256,'hex'))
  AND royal_line.output_index=1 AND royal_line.source_line=0 AND royal_line.quantity=1
  AND royal_line.item_instance_id=royal_item.item_instance_id
  AND royal_line.definition_family='Item' AND royal_line.definition_production_key='oteryn:item.tibia.i23398'
  AND royal_line.definition_revision_ref=royal_origin.target_revision
  AND royal_item.definition_family='Item' AND royal_item.definition_production_key='oteryn:item.tibia.i23398'
  AND royal_item.definition_revision_ref=royal_line.definition_revision_ref AND royal_item.quantity=1
  AND royal_item.state_revision=1 AND royal_item.lifecycle=1 AND royal_item.last_transaction_id IS NULL
  AND royal_item.minted_transaction_id=royal_mint.transaction_id AND royal_item.world_id=royal_mint.world_id
  AND (SELECT count(*) FROM game_item_character_inbox_locations royal_slot WHERE royal_slot.character_id=royal_parent.character_id)<=20
  AND royal_inbox.character_id=royal_parent.character_id AND royal_inbox.world_id=royal_mint.world_id
  AND royal_origin.mint_transaction_id=royal_mint.transaction_id AND royal_origin.source_row=royal_child.source_row
  AND royal_origin.target_key=royal_child.target_key AND royal_origin.created_xact_id=pg_current_xact_id())
$$;
CREATE FUNCTION game_npc_royal_kit_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
 IF NOT game_npc_royal_kit_proven(NEW.parent_transaction_id,NEW.item_instance_id) THEN
  RAISE EXCEPTION 'royal display kit requires its exact paid NPC source and native Inbox mint' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_royal_kit_complete AFTER INSERT ON game_npc_royal_kit_mints
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_royal_kit_complete();
CREATE FUNCTION game_npc_royal_kit_parent_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
 IF NEW.source_key IN('oteryn:npc-cosmetic/king_tibianus/golden-buy-armor','oteryn:npc-cosmetic/queen_eloise/golden-buy-armor','oteryn:npc-cosmetic/emperor_kruzak/golden-buy-armor')
  AND NOT EXISTS(SELECT 1 FROM game_npc_royal_kit_mints royal_child WHERE royal_child.parent_transaction_id=NEW.transaction_id
   AND game_npc_royal_kit_proven(NEW.transaction_id,royal_child.item_instance_id)) THEN
  RAISE EXCEPTION 'royal base purchase requires its native Inbox display kit' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_royal_kit_parent_complete AFTER INSERT ON game_npc_cosmetic_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_royal_kit_parent_complete();
-- Existing shop-origin proof remains unchanged for every other source class.
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_kit_origin_proven()'::regprocedure);needle:=E'BEGIN\n';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing original kit proof'; END IF;
 EXECUTE replace(body,needle,needle||$royal$
 IF NEW.source_row BETWEEN 2000000 AND 2000002 THEN
  IF NOT EXISTS(SELECT 1 FROM game_npc_royal_kit_mints royal_child WHERE royal_child.item_instance_id=NEW.item_instance_id
   AND game_npc_royal_kit_proven(royal_child.parent_transaction_id,NEW.item_instance_id)) THEN
   RAISE EXCEPTION 'royal origin requires its same-transaction paid NPC source' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
$royal$);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_royal_inbox_admit(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_npc_royal_kit_proven(UUID,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_npc_royal_kit_parent_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_npc_royal_kit_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_npc_royal_kit_proven(UUID,UUID),game_npc_royal_kit_complete(),game_npc_royal_kit_parent_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

REVOKE ALL ON FUNCTION game_npc_royal_inbox_admit(UUID,TEXT) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_royal_inbox_admit(UUID,TEXT) TO oteryn_game_runtime;
