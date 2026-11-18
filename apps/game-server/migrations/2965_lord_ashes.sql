-- Crystal 201189f8 data/items/items.xml:22845-22861, and skinning.lua:259-268.
-- Reuse the original corpse MINT and ItemType timers with a Character-local
-- claim receipt. The item craft transaction leaves both the ashes and knife intact.
DO $$
DECLARE body TEXT; anchor TEXT;
BEGIN
 body:=pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
 anchor:='WHEN ''oteryn:item.tibia.i54678'' THEN';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing source corpse registry'; END IF;
 EXECUTE replace(body,anchor,'WHEN ''oteryn:item.tibia.i8181'' THEN ARRAY[8181,8182,8183,8184] '||anchor);
END $$;

-- Schedules are created with the original corpse MINT and continue to refer to
-- that same ItemInstance as the definition changes.
CREATE FUNCTION game_lord_ashes_phase(p_item UUID) RETURNS SMALLINT
LANGUAGE sql STABLE STRICT AS $$
 SELECT s.source_stage::smallint
 FROM game_spell_field_temporal_schedules s
 JOIN game_item_instances i USING(item_instance_id)
 WHERE s.item_instance_id=p_item AND s.source_ordinal=0
   AND (s.source_stage,s.definition_production_key) IN (
       (1,'oteryn:item.tibia.i8181'),(2,'oteryn:item.tibia.i8182'))
   AND s.expires_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
   AND i.lifecycle=1 AND i.quantity=1
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
       (s.definition_family,s.definition_production_key,s.definition_revision)
$$;

CREATE FUNCTION game_lord_ashes_loot_open(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT NOT EXISTS (
    SELECT 1 FROM game_spell_field_temporal_schedules s
    WHERE s.item_instance_id=p_item AND s.source_ordinal=0 AND s.source_stage=1
      AND s.definition_production_key='oteryn:item.tibia.i8181'
      AND s.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint)
$$;

CREATE TABLE game_lord_ashes_claims (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_craft_records(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 corpse_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 phase SMALLINT NOT NULL CHECK (phase IN (1,2)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_lord_ashes_claims_character ON game_lord_ashes_claims(character_id);
CREATE INDEX game_lord_ashes_claims_corpse ON game_lord_ashes_claims(corpse_item_instance_id);
-- Donor storage remains 1 after the loot form and becomes 2 after the final
-- eligible form. A Character at 2 never receives another grant.
CREATE UNIQUE INDEX game_lord_ashes_final_claim ON game_lord_ashes_claims(character_id) WHERE phase=2;

CREATE FUNCTION game_lord_ashes_claim_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 IF game_lord_ashes_phase(NEW.corpse_item_instance_id) IS DISTINCT FROM NEW.phase
    OR EXISTS(SELECT 1 FROM game_lord_ashes_claims WHERE character_id=NEW.character_id AND phase=2)
 THEN RAISE EXCEPTION 'lord ashes claim has expired or already finished' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_lord_ashes_claim_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(
   SELECT 1 FROM game_item_craft_records c
   JOIN game_item_ground_locations g ON g.item_instance_id=NEW.corpse_item_instance_id
   JOIN game_durability_reconnect_sessions s USING(game_session_id)
   WHERE c.transaction_id=NEW.transaction_id AND c.character_id=NEW.character_id
     AND c.created_xact_id=pg_current_xact_id() AND NOT c.consume_source
     AND c.recipe='oteryn:skinning.lord-ashes'
     AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i5908'
     AND g.world_id=c.world_id AND g.channel_id=s.runtime_scope_channel_id AND g.scope_kind=1
     AND g.runtime_scope_ownership_generation=s.scope_ownership_generation
     AND jsonb_array_length(c.outputs)=1
     AND c.outputs->0->>'key'='oteryn:item.tibia.i954'
     AND c.outputs->0->>'revision'='definition-r1' AND c.outputs->0->>'count'='1'
 ) THEN RAISE EXCEPTION 'lord ashes claim requires an exact carried reward and scoped corpse' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

CREATE TRIGGER game_lord_ashes_claim_admit BEFORE INSERT ON game_lord_ashes_claims
 FOR EACH ROW EXECUTE FUNCTION game_lord_ashes_claim_admit();
CREATE CONSTRAINT TRIGGER game_lord_ashes_claim_complete AFTER INSERT ON game_lord_ashes_claims
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_lord_ashes_claim_complete();
CREATE TRIGGER game_lord_ashes_claim_immutable BEFORE UPDATE OR DELETE ON game_lord_ashes_claims
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_lord_ashes_claim_no_truncate BEFORE TRUNCATE ON game_lord_ashes_claims
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$
DECLARE body TEXT; anchor TEXT; at INTEGER; signature TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 anchor:='AND NEW.recipe<>''oteryn:skinning.crystal'' THEN';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing skinning craft guard'; END IF;
 body:=replace(body,anchor,'AND NEW.recipe NOT IN (''oteryn:skinning.crystal'',''oteryn:skinning.lord-ashes'') THEN');
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing craft admission'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
 IF NEW.recipe='oteryn:skinning.lord-ashes' AND (
   NEW.consume_source OR NOT EXISTS(SELECT 1 FROM game_lord_ashes_claims a
     WHERE a.transaction_id=NEW.transaction_id AND a.character_id=NEW.character_id
       AND a.created_xact_id=pg_current_xact_id())
 ) THEN RAISE EXCEPTION 'lord ashes craft requires a current claim' USING ERRCODE='23514'; END IF;
 $arm$ from at for 5);
 FOREACH signature IN ARRAY ARRAY['game_lord_ashes_phase(uuid)','game_lord_ashes_loot_open(uuid)',
     'game_lord_ashes_claim_admit()','game_lord_ashes_claim_complete()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $$;
REVOKE ALL ON game_lord_ashes_claims FROM PUBLIC;
GRANT SELECT,INSERT ON game_lord_ashes_claims TO oteryn_game_runtime;
GRANT SELECT ON game_lord_ashes_claims TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_lord_ashes_phase(uuid),game_lord_ashes_loot_open(uuid) TO oteryn_game_runtime,oteryn_game_control;
