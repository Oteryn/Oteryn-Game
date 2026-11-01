-- E12, Crystal 201189f hireling.lua. Extend the existing owner, preserving legacy bankers.
-- Legacy conversion and character-wide grants: PR #2783 ANSWER 6547428201 (Q1/Q5).
ALTER TABLE game_house_hirelings
 ADD COLUMN hireling_id UUID,
 ADD COLUMN name TEXT,
 ADD COLUMN sex SMALLINT NOT NULL DEFAULT 1 CHECK(sex IN(1,2)),
 ADD COLUMN look_type INTEGER NOT NULL DEFAULT 128 CHECK(look_type BETWEEN 1 AND 65535),
 ADD COLUMN colors BYTEA NOT NULL DEFAULT '\x00000000' CHECK(octet_length(colors)=4),
 ADD COLUMN active BOOLEAN NOT NULL DEFAULT TRUE,
 ADD COLUMN lamp_item_instance_id UUID UNIQUE REFERENCES game_item_instances(item_instance_id),
 ADD COLUMN last_lamp_session UUID,
 ADD COLUMN last_lamp_command NUMERIC(20,0),
 ADD COLUMN last_lamp_transaction UUID,
 ADD COLUMN last_lamp_connection_generation NUMERIC(20,0),
 ADD COLUMN last_lamp_lease_generation NUMERIC(20,0),
 ADD COLUMN last_lamp_channel UUID,
 ADD COLUMN legacy_grant BOOLEAN NOT NULL DEFAULT TRUE;

WITH named AS (
 SELECT world_id,house_key,row_number() OVER(ORDER BY world_id,house_key) AS ordinal
 FROM game_house_hirelings
)
UPDATE game_house_hirelings h
 SET hireling_id=uuidv7(),
     name='Hireling '||translate(named.ordinal::text,'0123456789','abcdefghij')
 FROM named WHERE (h.world_id,h.house_key)=(named.world_id,named.house_key);

-- Preserve the genuine command provenance of legacy free placements. A historic row
-- without a matching creation receipt is not granted synthetic session authority.
WITH provenance AS (
 SELECT DISTINCT ON(h.hireling_id) h.hireling_id,r.game_session_id,r.command_id,
   s.current_generation,s.character_lease_generation,s.origin_channel_id
 FROM game_house_hirelings h JOIN game_house_hireling_receipts r
   ON r.active AND r.x=h.x AND r.y=h.y AND r.floor=h.floor
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=r.game_session_id
   AND s.world_id=h.world_id AND s.character_id=h.owner_character_id
 ORDER BY h.hireling_id,r.command_id DESC,r.game_session_id
)
UPDATE game_house_hirelings h SET last_lamp_session=p.game_session_id,
 last_lamp_command=p.command_id,last_lamp_transaction=uuidv7(),
 last_lamp_connection_generation=p.current_generation,
 last_lamp_lease_generation=p.character_lease_generation,last_lamp_channel=p.origin_channel_id
 FROM provenance p WHERE p.hireling_id=h.hireling_id AND p.origin_channel_id IS NOT NULL;

ALTER TABLE game_house_hirelings
 DROP CONSTRAINT game_house_hirelings_pkey,
 ALTER COLUMN hireling_id SET NOT NULL,
 ALTER COLUMN name SET NOT NULL,
 ALTER COLUMN house_key DROP NOT NULL,
 ALTER COLUMN active SET DEFAULT FALSE,
 ALTER COLUMN legacy_grant SET DEFAULT FALSE,
 ADD PRIMARY KEY(hireling_id),
 ADD CONSTRAINT game_hireling_identity_uuid CHECK(game_character_is_uuid_v7(hireling_id)),
 ADD CONSTRAINT game_hireling_source_uuid CHECK(last_lamp_session IS NULL OR (game_character_is_uuid_v7(last_lamp_session)
   AND game_character_is_uuid_v7(last_lamp_transaction) AND game_character_is_uuid_v7(last_lamp_channel))),
 ADD CONSTRAINT game_hireling_name_size CHECK(octet_length(name) BETWEEN 3 AND 18),
 ADD CONSTRAINT game_hireling_active_house CHECK(NOT active OR house_key IS NOT NULL),
 ADD CONSTRAINT game_hireling_lamp_provenance CHECK(
    (last_lamp_session IS NULL AND last_lamp_command IS NULL AND last_lamp_transaction IS NULL
       AND last_lamp_connection_generation IS NULL AND last_lamp_lease_generation IS NULL AND last_lamp_channel IS NULL)
    OR (last_lamp_session IS NOT NULL AND last_lamp_command IS NOT NULL
        AND last_lamp_connection_generation IS NOT NULL AND last_lamp_lease_generation IS NOT NULL
        AND last_lamp_command BETWEEN 1 AND 18446744073709551615
        AND last_lamp_transaction IS NOT NULL AND last_lamp_connection_generation BETWEEN 1 AND 18446744073709551615
        AND last_lamp_lease_generation BETWEEN 1 AND 18446744073709551615 AND last_lamp_channel IS NOT NULL));
-- Crystal201189f schema.sql:697-713 identifies hirelings by id, not by a
-- globally unique name. Different identities may have the same valid name.
CREATE INDEX game_hireling_owner ON game_house_hirelings(owner_character_id,hireling_id);
CREATE INDEX game_hireling_house ON game_house_hirelings(world_id,house_key,hireling_id) WHERE active;
CREATE UNIQUE INDEX game_hireling_house_tile ON game_house_hirelings(world_id,house_key,x,y,floor) WHERE active;

CREATE TABLE game_hireling_entitlements (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 entitlement_id SMALLINT NOT NULL CHECK(entitlement_id BETWEEN 1001 AND 1004 OR entitlement_id BETWEEN 2001 AND 2009),
 PRIMARY KEY(character_id,entitlement_id)
);
INSERT INTO game_hireling_entitlements(character_id,entitlement_id)
 SELECT DISTINCT owner_character_id,1001 FROM game_house_hirelings;

ALTER TABLE game_house_hireling_receipts
 ADD COLUMN character_id UUID REFERENCES game_character_roots(character_id),
 ADD COLUMN hireling_id UUID REFERENCES game_house_hirelings(hireling_id),
 ADD COLUMN request_binding BYTEA CHECK(request_binding IS NULL OR octet_length(request_binding)=32),
 ADD COLUMN response BYTEA,
 ADD COLUMN created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE INDEX game_hireling_receipt_character ON game_house_hireling_receipts(character_id);
CREATE INDEX game_hireling_receipt_identity ON game_house_hireling_receipts(hireling_id);
-- Insert after the 0700 receipt columns exist and before receipt immutability.
-- The source CommandRef was matched to its authentic session/owner/world above.
-- Preserve that identity link in the immutable receipt so pruning or moving the
-- original reconnect session cannot revoke an otherwise valid legacy recall.
UPDATE game_house_hireling_receipts r
 SET character_id=h.owner_character_id,hireling_id=h.hireling_id
 FROM game_house_hirelings h
 WHERE h.legacy_grant
   AND r.game_session_id=h.last_lamp_session AND r.command_id=h.last_lamp_command
   AND r.active AND (r.x,r.y,r.floor)=(h.x,h.y,h.floor)
   AND EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
      WHERE s.game_session_id=r.game_session_id
        AND s.character_id=h.owner_character_id AND s.world_id=h.world_id);

CREATE TRIGGER game_hireling_receipt_immutable BEFORE UPDATE OR DELETE ON game_house_hireling_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_receipt_no_truncate BEFORE TRUNCATE ON game_house_hireling_receipts
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_receipt_stamp BEFORE INSERT ON game_house_hireling_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

REVOKE ALL ON game_hireling_entitlements FROM PUBLIC;
GRANT SELECT,INSERT ON game_hireling_entitlements TO oteryn_game_runtime;
-- Existing Hireling tables retain their explicit runtime grants from migration0405.
-- No control-plane consumer requires access to the new entitlement table.

-- PR2783 ANSWER6547953322 Q9: malformed0405 rows are preserved and isolated.
-- The original writer created placement+receipt atomically; missing provenance
-- is not authority to invent a session or a player command for native item minting.
CREATE VIEW game_hireling_recovery_quarantine AS
 SELECT hireling_id,owner_character_id,world_id,house_key,
        'missing authentic legacy creation receipt'::TEXT AS reason
 FROM game_house_hirelings
 WHERE legacy_grant AND active AND last_lamp_session IS NULL;
REVOKE ALL ON game_hireling_recovery_quarantine FROM PUBLIC;
GRANT SELECT ON game_hireling_recovery_quarantine TO oteryn_game_runtime;
COMMENT ON VIEW game_hireling_recovery_quarantine IS
 'Preserved malformed0405 identities; manual repair requires authentic historical evidence. Never a mint authority.';
DO $$ DECLARE blocked BIGINT; BEGIN
 SELECT count(*) INTO blocked FROM game_hireling_recovery_quarantine;
 IF blocked>0 THEN
   RAISE WARNING 'Hireling recovery quarantined % legacy identities without authentic creation receipts; inspect game_hireling_recovery_quarantine and repair only from historical evidence',blocked;
 END IF;
END $$;
