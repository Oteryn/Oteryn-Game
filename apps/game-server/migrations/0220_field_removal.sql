-- FIELD-RUNES-1: an entry or a machete removes one conjured barrier. This is
-- an Item retirement, not an early expiry or a second spell cast.
-- Field ticks retain their original cast CommandRef and have their own store
-- occurrence. Empty is the unchanged ordinary swing/spell namespace.
ALTER TABLE game_character_pvp_harm ADD COLUMN field_occurrence BYTEA NOT NULL DEFAULT ''::bytea
 CHECK(octet_length(field_occurrence) IN(0,32));
ALTER TABLE game_character_pvp_harm DROP CONSTRAINT game_character_pvp_harm_pkey;
ALTER TABLE game_character_pvp_harm ADD PRIMARY KEY(game_session_id,command_id,ordinal,victim,field_occurrence);
CREATE UNIQUE INDEX game_player_field_occurrence_once ON game_character_pvp_harm(field_occurrence)
 WHERE octet_length(field_occurrence)=32;
-- Even a self hit or a PZ-paused condition successor needs positive COMMIT
-- evidence. An empty list of PvP harm receipts cannot prove a transaction.
CREATE TABLE game_player_field_cycles (
 occurrence BYTEA PRIMARY KEY CHECK(octet_length(occurrence)=32),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 victim UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER game_player_field_cycles_immutable BEFORE UPDATE OR DELETE ON game_player_field_cycles
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_field_cycles_no_truncate BEFORE TRUNCATE ON game_player_field_cycles
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
REVOKE ALL ON game_player_field_cycles FROM PUBLIC;
GRANT SELECT,INSERT ON game_player_field_cycles TO oteryn_game_runtime;
GRANT SELECT ON game_player_field_cycles TO oteryn_game_control;
CREATE TABLE game_field_removals (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 transaction_id UUID NOT NULL UNIQUE,
 event_id UUID NOT NULL UNIQUE,
 source_transaction_id UUID NOT NULL REFERENCES game_spell_item_receipts(transaction_id),
 character_id UUID NOT NULL,
 game_session_id UUID NOT NULL,
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation>0),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before>0),
 source_stack_ordinal BIGINT NOT NULL,
 spatial_position BYTEA NOT NULL,
 map_revision TEXT NOT NULL,
 content_revision TEXT NOT NULL,
 placement_context BYTEA NOT NULL,
 tool_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
 command_id NUMERIC(20,0) CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 intent_binding BYTEA CHECK(octet_length(intent_binding)=32),
 CHECK((tool_item_instance_id IS NULL)=(command_id IS NULL)),
 CHECK((tool_item_instance_id IS NULL)=(intent_binding IS NULL)),
 occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE UNIQUE INDEX game_field_tool_command_once ON game_field_removals(game_session_id,command_id) WHERE command_id IS NOT NULL;

CREATE FUNCTION game_field_machete_carried(wanted_character UUID,wanted_world UUID,wanted_item UUID)
 RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 WITH RECURSIVE path(item,depth) AS (
  SELECT wanted_item,0
  UNION ALL
  SELECT e.parent_item_instance_id,path.depth+1 FROM game_item_nested_entries e
   JOIN path ON e.item_instance_id=path.item WHERE path.depth<16 AND e.world_id=wanted_world
 )
 SELECT EXISTS(SELECT 1 FROM game_item_instances tool
  WHERE tool.item_instance_id=wanted_item AND tool.lifecycle=1 AND tool.world_id=wanted_world
   AND tool.definition_family='Item' AND tool.definition_production_key='oteryn:item.tibia.i3308'
   AND EXISTS(SELECT 1 FROM path JOIN game_item_container_entries e ON e.item_instance_id=path.item
    JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id
    WHERE e.character_id=wanted_character AND b.character_id=wanted_character
     AND e.world_id=wanted_world AND b.world_id=wanted_world))
$$;

CREATE FUNCTION game_field_removal_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
  SELECT 1 FROM game_item_instances i
  JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id
   AND l.transaction_id=NEW.source_transaction_id AND l.operation_kind=1
  WHERE i.item_instance_id=NEW.item_instance_id AND i.minted_transaction_id=l.transaction_id
   AND i.world_id=NEW.world_id AND i.lifecycle=2 AND i.quantity=0
   AND i.last_transaction_id=NEW.transaction_id AND i.state_revision=NEW.state_revision_before+1
   AND l.world_id=NEW.world_id AND l.channel_id=NEW.channel_id
   AND l.spatial_position=NEW.spatial_position AND l.map_revision=NEW.map_revision
   AND l.content_revision=NEW.content_revision AND l.placement_context=NEW.placement_context
   AND l.definition_family=i.definition_family AND l.definition_production_key=i.definition_production_key
   AND l.definition_revision=i.definition_revision_ref
   AND ((NEW.tool_item_instance_id IS NULL
      AND i.definition_production_key IN('oteryn:item.tibia.i10181','oteryn:item.tibia.i10182'))
    OR (NEW.tool_item_instance_id IS NOT NULL
      AND i.definition_production_key IN('oteryn:item.tibia.i2130','oteryn:item.tibia.i3635','oteryn:item.tibia.i30224')
      AND game_field_machete_carried(NEW.character_id,NEW.world_id,NEW.tool_item_instance_id)))
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
 ) OR NOT EXISTS (
  SELECT 1 FROM game_durability_reconnect_sessions s
  JOIN game_durability_admission_character_guards g ON g.character_id=s.character_id
  WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
   AND s.session_state=2 AND s.current_generation=NEW.connection_generation
   AND s.character_lease_generation=NEW.character_lease_generation
   AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id
   AND s.runtime_scope_channel_id=NEW.channel_id AND s.scope_ownership_generation=NEW.ownership_generation
   AND g.eligible AND g.holder_game_session_id=s.game_session_id AND g.lease_generation=s.character_lease_generation
 ) THEN
  RAISE EXCEPTION 'field removal requires its exact current entry/tool and Item retirement' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_field_removal_proven AFTER INSERT ON game_field_removals
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_field_removal_proven();
CREATE TRIGGER game_field_removals_immutable BEFORE UPDATE OR DELETE ON game_field_removals
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_field_removals_no_truncate BEFORE TRUNCATE ON game_field_removals
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();

-- Add a narrow proof arm to the live guard bodies, preserving every other
-- mechanic's installed arms. The receipt's deferred trigger proves completion.
DO $$ DECLARE body TEXT; arm TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_instance_change_proven()'::regprocedure) INTO body;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_field_removals r WHERE r.item_instance_id=NEW.item_instance_id
   AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
   AND NEW.state_revision=OLD.state_revision+1 AND OLD.quantity=1 AND NEW.quantity=0 AND NEW.lifecycle=2
   AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
 $arm$;
 body:=regexp_replace(body,$regex$\mBEGIN\M$regex$,E'BEGIN\n'||arm,'i');
 IF position('game_field_removals' IN body)=0 THEN RAISE EXCEPTION 'field Item proof arm missing'; END IF;
 EXECUTE body;
 SELECT pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure) INTO body;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_field_removals r JOIN game_item_instances i USING(item_instance_id)
   WHERE r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id
   AND r.source_stack_ordinal=OLD.stack_ordinal AND r.spatial_position=OLD.spatial_position
   AND r.map_revision=OLD.map_revision AND r.content_revision=OLD.content_revision
   AND r.placement_context=OLD.native_room_placement_context AND i.lifecycle=2 AND i.quantity=0
   AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
   AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
 $arm$;
 body:=regexp_replace(body,$regex$\mBEGIN\M$regex$,E'BEGIN\n'||arm,'i');
 IF position('game_field_removals' IN body)=0 THEN RAISE EXCEPTION 'field Ground proof arm missing'; END IF;
 EXECUTE body;
END $$;
REVOKE ALL ON game_field_removals FROM PUBLIC;
REVOKE ALL ON FUNCTION game_field_removal_proven() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_field_machete_carried(UUID,UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_field_machete_carried(UUID,UUID,UUID) TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_field_removals TO oteryn_game_runtime;
GRANT SELECT ON game_field_removals TO oteryn_game_control;

-- WORLD-INTERACTION-0 §9: spell fields are volatile across a channel restart.
-- The new current owner retires old-generation fields; it never adopts their
-- player lineage or rewrites their old Ground fence to look current.
CREATE TABLE game_field_restart_retirements (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 source_transaction_id UUID NOT NULL REFERENCES game_spell_item_receipts(transaction_id),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 source_generation NUMERIC(20,0) NOT NULL CHECK(source_generation>0),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>source_generation),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 source_stack_ordinal BIGINT NOT NULL CHECK(source_stack_ordinal>0),
 spatial_position BYTEA NOT NULL,
 map_revision TEXT NOT NULL,
 content_revision TEXT NOT NULL,
 placement_context BYTEA NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_field_restart_retirement_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
  SELECT 1 FROM game_item_instances i JOIN game_spell_item_lines l USING(item_instance_id)
  WHERE i.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.source_transaction_id
   AND i.minted_transaction_id=l.transaction_id AND l.operation_kind=1
   AND (l.expires_at_unix_ms IS NOT NULL OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
    WHERE p.item_instance_id=i.item_instance_id AND p.source_transaction_id=l.transaction_id))
   AND i.world_id=NEW.world_id AND l.world_id=NEW.world_id AND l.channel_id=NEW.channel_id
   AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=NEW.state_revision_before+1
   AND i.last_transaction_id=NEW.transaction_id
   AND (l.spatial_position,l.map_revision,l.content_revision,l.placement_context)=
       (NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
 ) OR NOT EXISTS (
  SELECT 1 FROM game_runtime_scope_assignments s
   JOIN game_durability_admission_runtime_guards g USING(scope_key)
  WHERE s.world_id=NEW.world_id AND s.channel_id=NEW.channel_id AND s.state=1
   AND s.ownership_generation=NEW.ownership_generation AND g.ready
   AND g.ownership_generation=s.ownership_generation
 ) THEN RAISE EXCEPTION 'field restart retirement requires its old field and current scope owner' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_field_restart_retirement_proven AFTER INSERT ON game_field_restart_retirements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_field_restart_retirement_proven();
CREATE TRIGGER game_field_restart_retirements_immutable BEFORE UPDATE OR DELETE ON game_field_restart_retirements
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_field_restart_retirements_no_truncate BEFORE TRUNCATE ON game_field_restart_retirements
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
DO $$ DECLARE body TEXT; arm TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_instance_change_proven()'::regprocedure) INTO body;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_field_restart_retirements r WHERE r.item_instance_id=NEW.item_instance_id
   AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
   AND NEW.state_revision=OLD.state_revision+1 AND OLD.quantity=1 AND NEW.quantity=0 AND NEW.lifecycle=2
   AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
 $arm$;
 body:=regexp_replace(body,$regex$\mBEGIN\M$regex$,E'BEGIN\n'||arm,'i');
 IF position('game_field_restart_retirements' IN body)=0 THEN RAISE EXCEPTION 'restart Item proof arm missing'; END IF;
 EXECUTE body;
 SELECT pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure) INTO body;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_field_restart_retirements r JOIN game_item_instances i USING(item_instance_id)
   WHERE r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id
   AND r.source_generation=OLD.runtime_scope_ownership_generation
   AND r.ownership_generation>OLD.runtime_scope_ownership_generation
   AND r.source_stack_ordinal=OLD.stack_ordinal AND r.spatial_position=OLD.spatial_position
   AND r.map_revision=OLD.map_revision AND r.content_revision=OLD.content_revision
   AND r.placement_context=OLD.native_room_placement_context AND i.lifecycle=2 AND i.quantity=0
   AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
   AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
 $arm$;
 body:=regexp_replace(body,$regex$\mBEGIN\M$regex$,E'BEGIN\n'||arm,'i');
 IF position('game_field_restart_retirements' IN body)=0 THEN RAISE EXCEPTION 'restart Ground proof arm missing'; END IF;
 EXECUTE body;
END $$;
REVOKE ALL ON game_field_restart_retirements FROM PUBLIC;
REVOKE ALL ON FUNCTION game_field_restart_retirement_proven() FROM PUBLIC;
GRANT SELECT,INSERT ON game_field_restart_retirements TO oteryn_game_runtime;
GRANT SELECT ON game_field_restart_retirements TO oteryn_game_control;

-- A field cycle composes ordinary one-item charge writes with its HP successor.
-- Ordinary timed commands still admit one write per physical transaction. Each
-- composed write has its own (item, expected revision) receipt and must spend
-- exactly one charge for the same victim and cycle in this transaction.
ALTER TABLE game_player_field_cycles ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();
CREATE TABLE game_player_field_charge_writes (
 cycle BYTEA NOT NULL REFERENCES game_player_field_cycles(occurrence) DEFERRABLE INITIALLY DEFERRED,
 item_instance_id UUID NOT NULL,
 expected_revision NUMERIC(20,0) NOT NULL,
 victim UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(item_instance_id,expected_revision),
 FOREIGN KEY(item_instance_id,expected_revision) REFERENCES game_item_timed_state_writes(item_instance_id,expected_revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_player_field_charge_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
  SELECT 1 FROM game_player_field_cycles c JOIN game_item_timed_state_writes w
   ON w.item_instance_id=NEW.item_instance_id AND w.expected_revision=NEW.expected_revision
  WHERE c.occurrence=NEW.cycle AND c.victim=NEW.victim AND c.game_session_id=NEW.game_session_id
   AND c.created_xact_id=pg_current_xact_id() AND w.created_xact_id=pg_current_xact_id()
   AND w.holder_character_id=NEW.victim
   AND ((w.cause=1 AND w.charges_after=w.charges_before-1
         AND w.remaining_ms_after IS NOT DISTINCT FROM w.remaining_ms_before)
        OR (w.cause=3 AND w.charges_before=1))) THEN
  RAISE EXCEPTION 'a field charge requires its same-transaction victim cycle and timed write' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_player_field_charge_proven AFTER INSERT ON game_player_field_charge_writes
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_field_charge_proven();
CREATE TRIGGER game_player_field_charge_writes_immutable BEFORE UPDATE OR DELETE ON game_player_field_charge_writes
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_field_charge_writes_no_truncate BEFORE TRUNCATE ON game_player_field_charge_writes
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
REVOKE ALL ON game_player_field_charge_writes FROM PUBLIC;
REVOKE ALL ON FUNCTION game_player_field_charge_proven() FROM PUBLIC;
GRANT SELECT,INSERT ON game_player_field_charge_writes TO oteryn_game_runtime;
GRANT SELECT ON game_player_field_charge_writes TO oteryn_game_control;
DO $$
DECLARE body TEXT; needle TEXT;
BEGIN
 SELECT pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure) INTO body;
 needle:=E'AND (o.item_instance_id, o.expected_revision)\n                  <> (NEW.item_instance_id, NEW.expected_revision)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing timed single-write proof'; END IF;
 body:=replace(body,needle,needle||E'\n AND NOT EXISTS (SELECT 1 FROM game_player_field_charge_writes a JOIN game_player_field_charge_writes b ON a.cycle=b.cycle AND a.victim=b.victim AND a.game_session_id=b.game_session_id WHERE a.item_instance_id=NEW.item_instance_id AND a.expected_revision=NEW.expected_revision AND b.item_instance_id=o.item_instance_id AND b.expected_revision=o.expected_revision AND a.created_xact_id=pg_current_xact_id() AND b.created_xact_id=pg_current_xact_id())');
 EXECUTE body;
 EXECUTE format('ALTER FUNCTION game_player_field_charge_proven() SET search_path=%I,pg_temp',current_schema());
END;
$$;
