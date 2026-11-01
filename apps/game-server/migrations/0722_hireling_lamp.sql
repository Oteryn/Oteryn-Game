-- E12: the existing native USE and Inbox owners remain the only item writers.
-- All prior item-use causes and shapes remain admitted unchanged.
ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_records_cause_kind_check,
 ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK(cause_kind IN(1,2,3,328,470,471,494,495,527)),
 DROP CONSTRAINT game_item_use_cause_shape,
 ADD CONSTRAINT game_item_use_cause_shape
 CHECK((cause_kind IN(1,328,470,471,494,495,527) AND shape=1) OR cause_kind=2 OR(cause_kind=3 AND shape IN(3,4)));

CREATE TABLE game_hireling_lamp_transitions (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 hireling_id UUID NOT NULL REFERENCES game_house_hirelings(hireling_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(octet_length(house_key) BETWEEN 1 AND 512),
 kind SMALLINT NOT NULL CHECK(kind IN(1,2,3)),
 lamp_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 x INTEGER NOT NULL CHECK(x BETWEEN 0 AND 65535),
 y INTEGER NOT NULL CHECK(y BETWEEN 0 AND 65535),
 floor SMALLINT NOT NULL CHECK(floor BETWEEN -15 AND 0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id,kind)
);
CREATE INDEX game_hireling_lamp_identity ON game_hireling_lamp_transitions(hireling_id);
CREATE INDEX game_hireling_lamp_character ON game_hireling_lamp_transitions(character_id);
CREATE INDEX game_hireling_lamp_item ON game_hireling_lamp_transitions(lamp_item_instance_id);
CREATE TRIGGER game_hireling_lamp_immutable BEFORE UPDATE OR DELETE ON game_hireling_lamp_transitions
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_lamp_no_truncate BEFORE TRUNCATE ON game_hireling_lamp_transitions
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_lamp_stamp BEFORE INSERT ON game_hireling_lamp_transitions
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE FUNCTION game_hireling_lamp_native_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE t game_hireling_lamp_transitions%ROWTYPE;
BEGIN
 SELECT * INTO t FROM game_hireling_lamp_transitions WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND OR t.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
   SELECT 1 FROM game_house_hirelings h WHERE h.hireling_id=t.hireling_id
    AND h.owner_character_id=t.character_id AND h.world_id=t.world_id
    AND h.lamp_item_instance_id=t.lamp_item_instance_id AND h.active=(t.kind=1)
    AND (t.kind<>1 OR (h.house_key=t.house_key AND h.x=t.x AND h.y=t.y AND h.floor=t.floor))
 ) THEN RAISE EXCEPTION 'Hireling lamp transition and identity differ' USING ERRCODE='23514'; END IF;
 IF t.kind=1 THEN
   IF NOT EXISTS(SELECT 1 FROM game_item_use_records u
     WHERE u.transaction_id=t.transaction_id AND u.game_session_id=t.game_session_id
      AND u.command_id=t.command_id AND u.cause_kind=470 AND u.shape=1
      AND u.character_id=t.character_id AND u.world_id=t.world_id
      AND u.source_item_instance_id=t.lamp_item_instance_id
      AND u.source_definition_family='Item' AND u.source_definition_production_key='oteryn:item.tibia.i29432'
      AND u.source_quantity_before=1 AND u.source_quantity_after=0
      AND u.created_xact_id=t.created_xact_id)
   THEN RAISE EXCEPTION 'Hireling placement requires the same native lamp burn' USING ERRCODE='23514'; END IF;
 ELSE
   IF NOT EXISTS(SELECT 1 FROM game_reward_claim_inbox_mint_receipts r
     JOIN game_reward_claim_inbox_mint_lines l USING(transaction_id)
     JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
     WHERE r.transaction_id=t.transaction_id AND r.character_id=t.character_id AND r.world_id=t.world_id
      AND l.item_instance_id=t.lamp_item_instance_id AND r.item_count=1
      AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i29432'
      AND i.quantity=1 AND i.lifecycle=1 AND r.created_xact_id=t.created_xact_id AND l.created_xact_id=t.created_xact_id)
   THEN RAISE EXCEPTION 'Hireling return requires the same native Inbox lamp mint' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_lamp_native_proven AFTER INSERT ON game_hireling_lamp_transitions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_lamp_native_proven();
CREATE CONSTRAINT TRIGGER game_hireling_burn_has_placement AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=470)
 EXECUTE FUNCTION game_hireling_lamp_native_proven();

CREATE FUNCTION game_hireling_identity_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF ROW(NEW.hireling_id,NEW.owner_character_id,NEW.world_id,NEW.legacy_grant)
    IS DISTINCT FROM ROW(OLD.hireling_id,OLD.owner_character_id,OLD.world_id,OLD.legacy_grant) THEN
   RAISE EXCEPTION 'immutable Hireling identity and owner' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_hireling_identity_immutable BEFORE UPDATE ON game_house_hirelings
 FOR EACH ROW EXECUTE FUNCTION game_hireling_identity_immutable();
CREATE TRIGGER game_hireling_identity_no_delete BEFORE DELETE ON game_house_hirelings
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_identity_no_truncate BEFORE TRUNCATE ON game_house_hirelings
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_hireling_identity_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='INSERT' THEN
   IF NEW.legacy_grant OR NOT EXISTS(SELECT 1 FROM game_store_hireling_effects e
     WHERE e.kind=1 AND e.hireling_id=NEW.hireling_id AND e.character_id=NEW.owner_character_id
       AND e.created_xact_id=pg_current_xact_id()) THEN
     RAISE EXCEPTION 'new Hireling requires Store grant' USING ERRCODE='23514';
   END IF;
 ELSE
   IF ROW(NEW.active,NEW.house_key,NEW.x,NEW.y,NEW.floor,NEW.lamp_item_instance_id,
          NEW.last_lamp_session,NEW.last_lamp_command,NEW.last_lamp_transaction,
          NEW.last_lamp_connection_generation,NEW.last_lamp_lease_generation,NEW.last_lamp_channel)
      IS DISTINCT FROM ROW(OLD.active,OLD.house_key,OLD.x,OLD.y,OLD.floor,OLD.lamp_item_instance_id,
          OLD.last_lamp_session,OLD.last_lamp_command,OLD.last_lamp_transaction,
          OLD.last_lamp_connection_generation,OLD.last_lamp_lease_generation,OLD.last_lamp_channel)
      AND NOT EXISTS(SELECT 1 FROM game_hireling_lamp_transitions t
        WHERE t.hireling_id=NEW.hireling_id AND t.character_id=NEW.owner_character_id
         AND t.created_xact_id=pg_current_xact_id() AND (t.kind=1)=NEW.active) THEN
     RAISE EXCEPTION 'Hireling custody requires native transition' USING ERRCODE='23514';
   END IF;
   IF ROW(NEW.look_type,NEW.colors) IS DISTINCT FROM ROW(OLD.look_type,OLD.colors) THEN
     IF NOT EXISTS(SELECT 1 FROM game_store_hireling_effects e
          WHERE e.kind=4 AND e.hireling_id=NEW.hireling_id AND e.character_id=NEW.owner_character_id
            AND e.created_xact_id=pg_current_xact_id())
        AND NOT EXISTS(SELECT 1 FROM game_house_hireling_receipts r
          WHERE r.hireling_id=NEW.hireling_id AND r.character_id=NEW.owner_character_id
            AND r.created_xact_id=pg_current_xact_id() AND r.response='\x01'::bytea
            AND r.request_binding IS NOT NULL) THEN
       RAISE EXCEPTION 'Hireling appearance requires the same committed owner command' USING ERRCODE='23514';
     END IF;
     -- Crystal hireling.lua:30-41,205-227: sex-specific default/profession looks.
     -- Initial uncustomized look128/136 may only be preserved/flipped by paid sex change.
     IF NEW.look_type NOT IN(CASE WHEN NEW.sex=2 THEN 1107 ELSE 1108 END,
                            CASE WHEN NEW.sex=2 THEN 136 ELSE 128 END)
        AND NOT EXISTS(SELECT 1 FROM (VALUES
          (2001,1109,1110),(2002,1113,1114),(2003,1115,1116),(2004,1111,1112),
          (2005,1117,1118),(2006,1129,1130),(2007,1131,1132),(2008,1123,1124),(2009,1125,1126)
        ) AS appearance(entitlement,female,male)
        JOIN game_hireling_entitlements e ON e.entitlement_id=appearance.entitlement
        WHERE e.character_id=NEW.owner_character_id
          AND NEW.look_type=CASE WHEN NEW.sex=2 THEN appearance.female ELSE appearance.male END) THEN
       RAISE EXCEPTION 'Hireling profession appearance requires its owner entitlement' USING ERRCODE='23514';
     END IF;
   END IF;
   IF ROW(NEW.name,NEW.sex) IS DISTINCT FROM ROW(OLD.name,OLD.sex)
      AND NOT EXISTS(SELECT 1 FROM game_store_hireling_effects e
        WHERE e.hireling_id=NEW.hireling_id AND e.character_id=NEW.owner_character_id
         AND e.kind IN(3,4) AND e.created_xact_id=pg_current_xact_id()) THEN
     RAISE EXCEPTION 'Hireling name or sex requires Store delivery' USING ERRCODE='23514';
   END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_identity_proven AFTER INSERT OR UPDATE ON game_house_hirelings
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_identity_proven();

CREATE FUNCTION game_hireling_entitlement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_store_hireling_effects e WHERE e.kind=2
   AND e.character_id=NEW.character_id AND e.entitlement_id=NEW.entitlement_id
   AND e.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Hireling unlock requires Store grant' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_entitlement_proven AFTER INSERT ON game_hireling_entitlements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_entitlement_proven();
CREATE TRIGGER game_hireling_entitlement_immutable BEFORE UPDATE OR DELETE ON game_hireling_entitlements
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_entitlement_no_truncate BEFORE TRUNCATE ON game_hireling_entitlements
 EXECUTE FUNCTION game_item_immutable();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_lamp_native_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_identity_immutable() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_identity_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_entitlement_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_hireling_lamp_transitions FROM PUBLIC;
GRANT SELECT,INSERT ON game_hireling_lamp_transitions TO oteryn_game_runtime;
REVOKE DELETE,TRUNCATE ON game_house_hirelings FROM oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_hireling_lamp_native_proven(),game_hireling_identity_immutable(),game_hireling_identity_proven(),game_hireling_entitlement_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_hireling_lamp_native_proven(),game_hireling_identity_immutable(),game_hireling_identity_proven(),game_hireling_entitlement_proven() TO oteryn_game_runtime;

-- The existing native USE owner gains one
-- exact Inbox source; ordinary uses and depot-search removal keep their guards.
ALTER TABLE game_item_use_records
 ADD COLUMN source_kind SMALLINT NOT NULL DEFAULT 1 CHECK(source_kind IN(1,2)),
 ALTER COLUMN source_parent_item_instance_id DROP NOT NULL,
 ADD CONSTRAINT game_item_use_source_shape CHECK(
   (source_kind=1 AND source_parent_item_instance_id IS NOT NULL)
   OR (source_kind=2 AND source_parent_item_instance_id IS NULL AND cause_kind=470
       AND shape=1 AND source_quantity_before=1 AND source_quantity_after=0
       AND source_definition_family='Item'
       AND source_definition_production_key='oteryn:item.tibia.i29432'));

-- Prefix the current guards instead of replacing another owner's accepted arm.
DO $hireling$ DECLARE original TEXT; prefix TEXT; BEGIN
 original:=pg_get_functiondef('game_item_use_consistency_guard()'::regprocedure);
 IF strpos(original,E'\nBEGIN\n')=0 OR strpos(original,'source_kind = 2')>0 THEN
   RAISE EXCEPTION 'unexpected item-use guard shape for Hireling extension';
 END IF;
 prefix:=$guard$
    IF NEW.source_kind = 2 THEN
        SELECT * INTO src FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id;
        IF NEW.cause_kind<>470 OR NEW.shape<>1 OR NEW.source_parent_item_instance_id IS NOT NULL
           OR NEW.source_quantity_before<>1 OR NEW.source_quantity_after<>0
           OR NEW.created_xact_id<>pg_current_xact_id()
           OR src.item_instance_id IS NULL OR src.world_id<>NEW.world_id
           OR src.lifecycle<>2 OR src.quantity<>0 OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
           OR (src.definition_family,src.definition_production_key,src.definition_revision_ref)
              IS DISTINCT FROM (NEW.source_definition_family,NEW.source_definition_production_key,NEW.source_definition_revision_ref)
           OR NEW.source_definition_family<>'Item' OR NEW.source_definition_production_key<>'oteryn:item.tibia.i29432'
           OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence e
               WHERE e.item_instance_id=NEW.source_item_instance_id AND e.transaction_id=NEW.transaction_id AND e.quantity_before=1)
           OR NOT EXISTS(SELECT 1 FROM game_character_inbox_deliveries d
               WHERE d.item_instance_id=NEW.source_item_instance_id AND d.character_id=NEW.character_id
                 AND d.world_id=NEW.world_id AND d.ordinal=NEW.source_placement_ordinal)
           OR EXISTS(SELECT 1 FROM game_item_character_inbox_locations l WHERE l.item_instance_id=NEW.source_item_instance_id)
           OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id)
           OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a
               WHERE a.event_id=NEW.event_id AND a.transaction_id=NEW.transaction_id
                 AND a.item_instance_id=NEW.source_item_instance_id AND a.event_type_id=2
                 AND a.occurred_at=NEW.occurred_at AND a.envelope_sha256=NEW.envelope_sha256
                 AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
        THEN RAISE EXCEPTION 'Hireling Inbox use requires its exact native burn and audit' USING ERRCODE='23514'; END IF;
        PERFORM game_item_location_exclusive(NEW.source_item_instance_id);
        RETURN NULL;
    END IF;
$guard$;
 EXECUTE replace(original,E'\nBEGIN\n',E'\nBEGIN\n'||prefix);

 original:=pg_get_functiondef('game_character_inbox_remove()'::regprocedure);
 IF strpos(original,E'\nBEGIN\n')=0 OR strpos(original,'u.source_kind=2')>0 THEN
   RAISE EXCEPTION 'unexpected Inbox removal guard shape for Hireling extension';
 END IF;
 prefix:=$guard$
    IF EXISTS(SELECT 1 FROM game_item_use_records u JOIN game_item_instances i ON i.item_instance_id=u.source_item_instance_id
        WHERE u.source_kind=2 AND u.cause_kind=470 AND u.shape=1
          AND u.created_xact_id=pg_current_xact_id() AND u.source_item_instance_id=OLD.item_instance_id
          AND u.character_id=OLD.character_id AND u.world_id=OLD.world_id
          AND u.source_placement_ordinal=OLD.ordinal AND u.source_quantity_before=1 AND u.source_quantity_after=0
          AND u.source_parent_item_instance_id IS NULL
          AND u.source_definition_family='Item' AND u.source_definition_production_key='oteryn:item.tibia.i29432'
          AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=u.transaction_id) THEN
        PERFORM 1 FROM game_character_roots WHERE character_id=OLD.character_id FOR UPDATE;
        RETURN OLD;
    END IF;
$guard$;
 EXECUTE replace(original,E'\nBEGIN\n',E'\nBEGIN\n'||prefix);
 EXECUTE format('ALTER FUNCTION game_item_use_consistency_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_character_inbox_remove() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $hireling$;

CREATE FUNCTION game_hireling_inbox_lamp_unlink(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE u game_item_use_records%ROWTYPE;
BEGIN
 SELECT * INTO u FROM game_item_use_records WHERE transaction_id=p_transaction
   AND created_xact_id=pg_current_xact_id() AND cause_kind=470 AND source_kind=2
   AND shape=1 AND source_quantity_before=1 AND source_quantity_after=0;
 IF NOT FOUND THEN RAISE EXCEPTION 'Hireling Inbox removal lacks native use' USING ERRCODE='23514'; END IF;
 DELETE FROM game_item_character_inbox_locations WHERE item_instance_id=u.source_item_instance_id
   AND character_id=u.character_id AND world_id=u.world_id AND ordinal=u.source_placement_ordinal;
 IF NOT FOUND THEN RAISE EXCEPTION 'Hireling Inbox lamp location changed' USING ERRCODE='23514'; END IF;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_inbox_lamp_unlink(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hireling_inbox_lamp_unlink(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_hireling_inbox_lamp_unlink(UUID) TO oteryn_game_runtime;

-- An origin Channel is
-- provenance only; the exact world + named House is bound into cause470's audit.
ALTER TABLE game_item_use_records
 ADD COLUMN source_house_key TEXT,
 ADD CONSTRAINT game_item_use_house_source CHECK(
   (cause_kind=470 AND source_house_key IS NOT NULL AND game_house_scope_key_valid(source_house_key))
   OR (cause_kind<>470 AND source_house_key IS NULL));

DO $hireling_house$ DECLARE original TEXT; prefix TEXT; BEGIN
 original:=pg_get_functiondef('game_item_use_consistency_guard()'::regprocedure);
 IF strpos(original,E'\nBEGIN\n')=0 OR strpos(original,'Hireling use requires exact named House')>0 THEN
   RAISE EXCEPTION 'unexpected item-use guard shape for Hireling House audit';
 END IF;
 prefix:=$guard$
    IF NEW.cause_kind=470 AND NOT EXISTS(
        SELECT 1 FROM game_hireling_lamp_transitions t
        WHERE t.transaction_id=NEW.transaction_id AND t.game_session_id=NEW.game_session_id
          AND t.command_id=NEW.command_id AND t.character_id=NEW.character_id
          AND t.world_id=NEW.world_id AND t.house_key=NEW.source_house_key
          AND t.lamp_item_instance_id=NEW.source_item_instance_id AND t.kind=1
          AND t.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'Hireling use requires exact named House' USING ERRCODE='23514';
    END IF;
$guard$;
 EXECUTE replace(original,E'\nBEGIN\n',E'\nBEGIN\n'||prefix);
 EXECUTE format('ALTER FUNCTION game_item_use_consistency_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $hireling_house$;

-- E12: additive current House authority evidence for the existing native Inbox
-- mint. The old source reservation, CommandRef and receipt remain unchanged.
CREATE TABLE game_hireling_inbox_scope_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_reward_claim_inbox_mint_receipts(transaction_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 scope_ownership_generation NUMERIC(20,0) NOT NULL
   CHECK(scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
 authority_kind SMALLINT NOT NULL CHECK(authority_kind IN(1,2)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
COMMENT ON COLUMN game_hireling_inbox_scope_receipts.authority_kind IS
 '1 live House session; 2 current House recall. Native mint CommandRef and session generations are historical placement provenance for2.';
CREATE TRIGGER game_hireling_inbox_scope_stamp BEFORE INSERT ON game_hireling_inbox_scope_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_hireling_inbox_scope_immutable BEFORE UPDATE OR DELETE ON game_hireling_inbox_scope_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_inbox_scope_no_truncate BEFORE TRUNCATE ON game_hireling_inbox_scope_receipts
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_hireling_inbox_scope_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_reward_claim_inbox_mint_receipts%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_reward_claim_inbox_mint_receipts WHERE transaction_id=NEW.transaction_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR NEW.created_xact_id<>pg_current_xact_id() OR r.world_id<>NEW.world_id
    OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
       WHERE a.scope_key=game_house_scope_key(NEW.world_id,NEW.house_key)
         AND a.scope_kind=2 AND a.world_id=NEW.world_id AND a.house_key=NEW.house_key
         AND a.state=1 AND a.ownership_generation=NEW.scope_ownership_generation)
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a
       WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
         AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Hireling Inbox audit lacks current House and same native mint' USING ERRCODE='23514';
 END IF;
 IF NEW.authority_kind=1 THEN
   IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
       WHERE s.game_session_id=r.game_session_id AND s.character_id=r.character_id
         AND s.world_id=r.world_id AND s.runtime_scope_world_id=s.world_id
         AND s.runtime_scope_kind=2 AND s.runtime_scope_channel_id IS NULL
         AND s.runtime_scope_house_key=NEW.house_key
         AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.house_key)
         AND s.origin_channel_id=r.channel_id AND s.session_state IN(1,2)
         AND s.current_generation=r.expected_session_generation
         AND s.character_lease_generation=r.expected_character_lease_generation
         AND s.scope_ownership_generation=NEW.scope_ownership_generation) THEN
     RAISE EXCEPTION 'Hireling Inbox live House audit lacks exact session' USING ERRCODE='23514';
   END IF;
 ELSE
   IF NOT EXISTS(SELECT 1 FROM game_hireling_lamp_transitions t
       WHERE t.transaction_id=r.transaction_id AND t.game_session_id=r.game_session_id
         AND t.command_id=r.command_id AND t.character_id=r.character_id
         AND t.world_id=NEW.world_id AND t.house_key=NEW.house_key AND t.kind=3
         AND t.created_xact_id=pg_current_xact_id()) THEN
     RAISE EXCEPTION 'Hireling Inbox recall audit lacks same native return' USING ERRCODE='23514';
   END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_inbox_scope_proven AFTER INSERT ON game_hireling_inbox_scope_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_inbox_scope_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_inbox_scope_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hireling_inbox_scope_proven() FROM PUBLIC;
REVOKE ALL ON game_hireling_inbox_scope_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_hireling_inbox_scope_receipts TO oteryn_game_runtime;

-- Delta after the470 House audit fragment: gourmet471 can be consumed in a
-- genuine Channel or named House. Every other cause retains its old shape.
ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_house_source,
 ADD CONSTRAINT game_item_use_house_source CHECK(
   (cause_kind=470 AND source_house_key IS NOT NULL AND game_house_scope_key_valid(source_house_key))
   OR (cause_kind=471 AND (source_house_key IS NULL OR game_house_scope_key_valid(source_house_key)))
   OR (cause_kind NOT IN(470,471) AND source_house_key IS NULL));

DO $gourmet_scope$ DECLARE original TEXT; prefix TEXT; BEGIN
 original:=pg_get_functiondef('game_item_use_consistency_guard()'::regprocedure);
 IF strpos(original,E'\nBEGIN\n')=0 OR strpos(original,'Gourmet use scope does not match its real source')>0 THEN
   RAISE EXCEPTION 'unexpected item-use guard shape for gourmet scope';
 END IF;
 prefix:=$guard$
    IF NEW.cause_kind=471 AND NOT EXISTS(
        SELECT 1 FROM game_durability_reconnect_sessions s
        WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
          AND s.world_id=NEW.world_id AND s.current_generation=NEW.expected_session_generation
          AND s.scope_ownership_generation=NEW.runtime_scope_ownership_generation
          AND s.session_state IN(1,2)
          AND ((NEW.source_house_key IS NULL AND s.runtime_scope_kind=1
                AND s.runtime_scope_channel_id=NEW.channel_id AND s.runtime_scope_instance_id IS NULL)
            OR (NEW.source_house_key IS NOT NULL AND s.runtime_scope_kind=2
                AND s.runtime_scope_house_key=NEW.source_house_key AND s.runtime_scope_channel_id IS NULL
                AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.source_house_key)
                AND s.origin_channel_id=NEW.channel_id))
    ) THEN
        RAISE EXCEPTION 'Gourmet use scope does not match its real source' USING ERRCODE='23514';
    END IF;
$guard$;
 EXECUTE replace(original,E'\nBEGIN\n',E'\nBEGIN\n'||prefix);
 EXECUTE format('ALTER FUNCTION game_item_use_consistency_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $gourmet_scope$;

-- Current named-House service access uses the existing native property/ACL
-- lock owner order from 0595:78-92. The runtime receives no table-write grant.
-- This read composition writes no Item, property, ACL, placement or session.
CREATE FUNCTION game_hireling_service_access_lock(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE guild UUID; access RECORD;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,
   p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 IF NOT FOUND THEN RETURN false; END IF;
 SELECT owner_guild_id INTO guild FROM game_house_properties
  WHERE world_id=p_world AND house_key=p_house;
 IF guild IS NOT NULL THEN
  PERFORM 1 FROM game_guilds WHERE guild_id=guild AND world_id=p_world FOR SHARE;
 END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 IF NOT FOUND THEN RETURN false; END IF;
 IF (SELECT owner_guild_id FROM game_house_properties
   WHERE world_id=p_world AND house_key=p_house) IS DISTINCT FROM guild THEN RETURN false; END IF;
 SELECT q.*,count(*) OVER() AS grant_count INTO access
  FROM game_house_access(p_world,p_house,p_character) q;
 RETURN COALESCE(FOUND AND access.grant_count=1 AND access.content_fenced IS NOT DISTINCT FROM false
  AND access.role IN('OWNER','SUBOWNER','GUEST'),false);
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_service_access_lock(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hireling_service_access_lock(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_hireling_service_access_lock(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 TO oteryn_game_runtime;

-- BEGIN HIRELING HOUSE POSITION (Q16)
-- Q16 / PR2783 ANSWER6549584730: current House cell/vitals extend the same
-- ordered Character LastPosition projection. Channel rows keep null metadata;
-- origin coordinates are separate routing history, never the current House cell.
ALTER TABLE game_character_last_positions
    ADD COLUMN house_key TEXT,
    ADD COLUMN origin_channel_id UUID,
    ADD COLUMN origin_tile_x INTEGER,
    ADD COLUMN origin_tile_y INTEGER,
    ADD COLUMN origin_tile_floor SMALLINT,
    ADD CONSTRAINT game_character_last_positions_house_scope CHECK (
        (house_key IS NULL AND origin_channel_id IS NULL AND origin_tile_x IS NULL
            AND origin_tile_y IS NULL AND origin_tile_floor IS NULL)
        OR (house_key IS NOT NULL AND game_house_scope_key_valid(house_key)
            AND origin_channel_id IS NOT NULL AND game_node_is_uuid_v7(origin_channel_id)
            AND origin_tile_x IS NOT NULL AND origin_tile_y IS NOT NULL AND origin_tile_floor IS NOT NULL)
    );
-- Existing world/order/no-truncate guards and runtime SELECT/INSERT/UPDATE grants
-- remain the only LastPosition authority. No new row key, ledger or receipt.

-- A fresh authenticated restore uses the existing native entry receipt and its
-- own immutable entry acknowledgement. command_id=0 is explicitly server
-- admission provenance; it never represents a client command or replay ID.
ALTER TABLE game_house_transfer_receipts
    ADD COLUMN admission_restore BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN restored_position_lease NUMERIC(20,0),
    ADD COLUMN restored_position_sequence NUMERIC(20,0),
    DROP CONSTRAINT game_house_transfer_receipts_command_id_check,
    ADD CONSTRAINT game_house_transfer_command_source CHECK (
        (NOT admission_restore AND command_id BETWEEN 1 AND 18446744073709551615
            AND restored_position_lease IS NULL AND restored_position_sequence IS NULL)
        OR (admission_restore AND direction=1 AND command_id=0
            AND restored_position_lease IS NOT NULL AND restored_position_sequence IS NOT NULL
            AND restored_position_lease BETWEEN 1 AND 18446744073709551615
            AND restored_position_sequence BETWEEN 1 AND 18446744073709551615)
    );

CREATE FUNCTION game_house_admission_restore_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT NEW.admission_restore THEN RETURN NEW; END IF;
    -- This is additional to every original 0215 source, assignment and claim
    -- proof. Only a newly committed fresh grant can be this server source.
    PERFORM 1 FROM game_durability_reconnect_sessions s
      JOIN game_durability_fresh_admission_receipts f
        ON f.replay_key=s.fresh_replay_key AND f.game_session_id=s.game_session_id
      JOIN game_character_last_positions p ON p.character_id=s.character_id
      WHERE s.game_session_id=NEW.source_game_session_id AND s.session_state=2
        AND s.current_generation=1 AND s.prepared_attempt_ref IS NULL
        AND s.current_transport_ref=NEW.source_transport
        AND f.transport_ref=NEW.source_transport AND f.connection_generation=1
        AND f.character_id=NEW.character_id AND f.account_id=NEW.account_id
        AND f.world_id=NEW.world_id AND f.channel_id=NEW.origin_channel_id
        AND f.character_lease_generation=NEW.source_character_lease_generation
        AND f.scope_ownership_generation=NEW.source_scope_generation
        AND p.world_id=NEW.world_id AND p.house_key=NEW.house_key
        AND p.origin_channel_id=NEW.origin_channel_id
        AND p.character_lease_generation=NEW.restored_position_lease
        AND p.write_sequence=NEW.restored_position_sequence
        AND (p.character_lease_generation<NEW.source_character_lease_generation
            OR (p.character_lease_generation=NEW.source_character_lease_generation
                AND p.write_sequence=1 AND EXISTS (
                    SELECT 1 FROM game_bed_regen_receipts b
                    WHERE b.game_session_id=s.game_session_id AND b.character_id=s.character_id
                      AND (b.health,b.mana,b.rest) IS NOT DISTINCT FROM (p.health,p.mana,p.rest))))
        AND (p.tile_x,p.tile_y,p.tile_floor)=(NEW.tile_x,NEW.tile_y,NEW.tile_floor)
      FOR SHARE OF s,f,p;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'House restore requires its current fresh admission and exact saved predecessor'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_admission_restore_guard BEFORE INSERT ON game_house_transfer_receipts
    FOR EACH ROW EXECUTE FUNCTION game_house_admission_restore_guard();
REVOKE ALL ON FUNCTION game_house_admission_restore_guard() FROM PUBLIC;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_admission_restore_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
-- END HIRELING HOUSE POSITION (Q16)
