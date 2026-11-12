-- S10: source-bound nonpremium blessing vendors. Historical receipts remain immutable.
DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_character_npc_purchase_receipts'::regclass AND conname='game_npc_purchase_service_shape';
 ALTER TABLE game_character_npc_purchase_receipts DROP CONSTRAINT game_npc_purchase_service_shape;
 EXECUTE 'ALTER TABLE game_character_npc_purchase_receipts ADD CONSTRAINT game_npc_purchase_service_shape CHECK (('
 ||substr(body,7)||') OR (cause_kind=3 AND blessing_keys IS NULL AND blessing_key IS NOT NULL
 AND blessing_key=''oteryn:blessing.''||substr(service_key,25) AND ((service_key=''oteryn:service.blessing.the_spiritual_shielding'' AND npc_key IN (''oteryn:npc.norf'')) OR (service_key=''oteryn:service.blessing.the_embrace_of_tibia'' AND npc_key IN (''oteryn:npc.humphrey'')) OR (service_key=''oteryn:service.blessing.the_fire_of_the_suns'' AND npc_key IN (''oteryn:npc.edala'')) OR (service_key=''oteryn:service.blessing.the_spark_of_the_phoenix'' AND npc_key IN (''oteryn:npc.pydar'')) OR (service_key=''oteryn:service.blessing.twist_of_fate'' AND npc_key IN (''oteryn:npc.alia'',''oteryn:npc.amanda'',''oteryn:npc.azalea'',''oteryn:npc.brewster'',''oteryn:npc.cedrik'',''oteryn:npc.ferus'',''oteryn:npc.iptar_sin'',''oteryn:npc.kasmir'',''oteryn:npc.kjesse'',''oteryn:npc.lorietta'',''oteryn:npc.maealil'',''oteryn:npc.ninev'',''oteryn:npc.prezil'',''oteryn:npc.quentin'',''oteryn:npc.yberius'',''oteryn:npc.zedrulon_the_fallen'')))))';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 body:=regexp_replace(body,' NOT VALID$','');
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
 ||substr(body,7)||') OR (cause_kind=3 AND charm_key IS NULL AND purchase_service_key IN (''oteryn:service.blessing.the_spiritual_shielding'',''oteryn:service.blessing.the_embrace_of_tibia'',''oteryn:service.blessing.the_fire_of_the_suns'',''oteryn:service.blessing.the_spark_of_the_phoenix'',''oteryn:service.blessing.twist_of_fate''))) NOT VALID';
END;
$$;
-- Donor StdModule.bless uses getItemById(3057,true): carried, including nested bags.
-- Reuse BAGS-1's existing depth-eight carried tree and physical equipment roots.
CREATE FUNCTION game_s10_has_carried_amulet(p_character UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 WITH RECURSIVE carried(item,depth) AS (
   (SELECT item_instance_id,0 FROM game_item_container_slots WHERE character_id=p_character
    UNION SELECT item_instance_id,0 FROM game_character_equipment_slots WHERE character_id=p_character)
   UNION ALL
   SELECT e.item_instance_id,c.depth+1 FROM carried c
   JOIN LATERAL (
     SELECT item_instance_id FROM game_item_container_entries
      WHERE parent_item_instance_id=c.item AND character_id=p_character
     UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=c.item
   ) e ON true WHERE c.depth<8
 )
 SELECT EXISTS(SELECT 1 FROM carried c JOIN game_item_instances i ON i.item_instance_id=c.item
   WHERE i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3057'
     AND i.lifecycle=1 AND i.quantity>0)
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_s10_has_carried_amulet(uuid) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_s10_has_carried_amulet(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_s10_has_carried_amulet(uuid) TO oteryn_game_runtime;

-- Quote at the receipt's locked level, using the accepted 2026-10-18 owner price ruling.
CREATE FUNCTION game_s10_blessing_quote_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.service_key IN ('oteryn:service.blessing.the_spiritual_shielding','oteryn:service.blessing.the_embrace_of_tibia','oteryn:service.blessing.the_fire_of_the_suns','oteryn:service.blessing.the_spark_of_the_phoenix','oteryn:service.blessing.twist_of_fate') AND NEW.fee_gold_units <>
    (CASE WHEN NEW.service_key='oteryn:service.blessing.twist_of_fate' THEN
      CASE WHEN NEW.level_before<=30 THEN 2000 WHEN NEW.level_before<270 THEN 2000+200*(NEW.level_before-30) ELSE 50000 END
    ELSE CASE WHEN NEW.level_before<=30 THEN 2000 WHEN NEW.level_before<120 THEN 2000+200*(NEW.level_before-30) ELSE 20000+75*(NEW.level_before-120) END END) THEN
  RAISE EXCEPTION 'S10 blessing requires exact locked-level quote' USING ERRCODE='23514';
 END IF;
 IF NEW.service_key='oteryn:service.blessing.twist_of_fate' AND NOT (
   EXISTS(SELECT 1 FROM game_character_blessings WHERE character_id=NEW.character_id AND blessing_key<>'oteryn:blessing.twist_of_fate') OR
   game_s10_has_carried_amulet(NEW.character_id)) THEN
  RAISE EXCEPTION 'Twist requires another blessing or carried amulet of loss' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_s10_blessing_quote AFTER INSERT ON game_character_npc_purchase_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_s10_blessing_quote_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_s10_blessing_quote_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_s10_blessing_quote_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_s10_blessing_quote_guard() TO oteryn_game_runtime;


-- S10: Kawill is a free prerequisite, not a paid blessing or quest mission.
CREATE TABLE game_character_npc_stage_receipts (
    purchase_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(purchase_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    command_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(command_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding)=32),
    npc_key TEXT NOT NULL CHECK (npc_key='oteryn:npc.kawill'),
    npc_identity BYTEA NOT NULL CHECK (octet_length(npc_identity)=16),
    npc_generation NUMERIC(20,0) NOT NULL CHECK (npc_generation BETWEEN 1 AND 18446744073709551615),
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision>=1),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision=original_character_revision+1
               AND committed_character_revision<=18446744073709551615),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after=level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before>=0),
    experience_after BIGINT NOT NULL CHECK (experience_after=experience_before),
    profile_revision TEXT NOT NULL,
    ruleset_revision TEXT NOT NULL,
    content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    reward_revision TEXT NOT NULL,
    committed_at BIGINT NOT NULL CHECK (committed_at>=0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, command_session_id, command_id),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (character_id, purchase_occurrence_id)
);

CREATE TABLE game_character_phoenix_prerequisites (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 grant_occurrence_id UUID NOT NULL REFERENCES game_character_npc_stage_receipts(purchase_occurrence_id),
 active BOOLEAN NOT NULL,
 consumed_by_purchase UUID REFERENCES game_character_npc_purchase_receipts(purchase_occurrence_id),
 CHECK (active=(consumed_by_purchase IS NULL))
);
CREATE INDEX game_phoenix_grant ON game_character_phoenix_prerequisites(grant_occurrence_id);
CREATE INDEX game_phoenix_consumed ON game_character_phoenix_prerequisites(consumed_by_purchase);
CREATE OR REPLACE VIEW game_character_premium_revision_chain WITH (security_invoker=true) AS
 SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_appearance_receipts
 UNION ALL SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_npc_purchase_receipts
 UNION ALL SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_npc_stage_receipts;
CREATE FUNCTION game_s10_phoenix_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_TABLE_NAME='game_character_npc_stage_receipts' THEN
  IF NOT EXISTS(SELECT 1 FROM game_character_roots c JOIN game_character_phoenix_prerequisites p USING(character_id)
    WHERE c.character_id=NEW.character_id AND c.account_id=NEW.account_id
      AND c.character_revision>=NEW.committed_character_revision
      AND p.grant_occurrence_id=NEW.purchase_occurrence_id AND p.active
      AND NEW.created_xact_id=pg_current_xact_id())
    OR EXISTS(SELECT 1 FROM game_character_npc_purchase_receipts r WHERE r.character_id=NEW.character_id
      AND r.command_session_id=NEW.command_session_id AND r.command_id=NEW.command_id) THEN
   RAISE EXCEPTION 'Kawill stage requires its own Character receipt and active flag' USING ERRCODE='23514';
  END IF;
 ELSIF TG_TABLE_NAME='game_character_npc_purchase_receipts' THEN
  IF EXISTS(SELECT 1 FROM game_character_npc_stage_receipts r WHERE r.character_id=NEW.character_id
      AND r.command_session_id=NEW.command_session_id AND r.command_id=NEW.command_id) THEN
   RAISE EXCEPTION 'NPC command already consumed by free stage' USING ERRCODE='23514';
  END IF;
  IF NEW.service_key='oteryn:service.blessing.the_spark_of_the_phoenix' AND NOT EXISTS(
    SELECT 1 FROM game_character_phoenix_prerequisites p WHERE p.character_id=NEW.character_id
      AND NOT p.active AND p.consumed_by_purchase=NEW.purchase_occurrence_id) THEN
   RAISE EXCEPTION 'Pydar must consume Kawill prerequisite' USING ERRCODE='23514';
  END IF;
 ELSE
  IF TG_OP='UPDATE' AND NEW.character_id<>OLD.character_id THEN
   RAISE EXCEPTION 'Phoenix prerequisite Character identity is immutable' USING ERRCODE='23514';
  END IF;
  IF NEW.active THEN
   IF TG_OP='UPDATE' AND OLD.active THEN
    RAISE EXCEPTION 'Kawill prerequisite already active' USING ERRCODE='23514';
   END IF;
   IF NOT EXISTS(SELECT 1 FROM game_character_npc_stage_receipts r WHERE r.character_id=NEW.character_id
      AND r.purchase_occurrence_id=NEW.grant_occurrence_id AND r.created_xact_id=pg_current_xact_id()) THEN
    RAISE EXCEPTION 'Kawill flag requires this transaction stage receipt' USING ERRCODE='23514';
   END IF;
  ELSE
   IF TG_OP<>'UPDATE' OR NOT OLD.active OR NEW.character_id<>OLD.character_id
      OR NEW.grant_occurrence_id<>OLD.grant_occurrence_id OR NOT EXISTS(
       SELECT 1 FROM game_character_npc_purchase_receipts r WHERE r.character_id=NEW.character_id
        AND r.purchase_occurrence_id=NEW.consumed_by_purchase
        AND r.npc_key='oteryn:npc.pydar' AND r.service_key='oteryn:service.blessing.the_spark_of_the_phoenix'
        AND r.created_xact_id=pg_current_xact_id()) THEN
    RAISE EXCEPTION 'Kawill flag consumption requires this transaction Pydar purchase' USING ERRCODE='23514';
   END IF;
  END IF;
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_s10_stage_effect AFTER INSERT ON game_character_npc_stage_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_s10_phoenix_guard();
CREATE CONSTRAINT TRIGGER game_s10_spark_effect AFTER INSERT ON game_character_npc_purchase_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_s10_phoenix_guard();
CREATE CONSTRAINT TRIGGER game_s10_stage_chain AFTER INSERT ON game_character_npc_stage_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_s10_flag_effect AFTER INSERT OR UPDATE ON game_character_phoenix_prerequisites
 FOR EACH ROW EXECUTE FUNCTION game_s10_phoenix_guard();
CREATE TRIGGER game_s10_stage_stamp BEFORE INSERT ON game_character_npc_stage_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_s10_stage_immutable BEFORE UPDATE OR DELETE ON game_character_npc_stage_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_s10_flag_no_delete BEFORE DELETE ON game_character_phoenix_prerequisites
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_s10_stage_no_truncate BEFORE TRUNCATE ON game_character_npc_stage_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_s10_flag_no_truncate BEFORE TRUNCATE ON game_character_phoenix_prerequisites
 EXECUTE FUNCTION game_character_reject_truncate();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_s10_phoenix_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_s10_phoenix_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_s10_phoenix_guard() TO oteryn_game_runtime;
REVOKE ALL ON game_character_npc_stage_receipts,game_character_phoenix_prerequisites FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_npc_stage_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_phoenix_prerequisites TO oteryn_game_runtime;
GRANT SELECT ON game_character_npc_stage_receipts,game_character_phoenix_prerequisites TO oteryn_game_control;
