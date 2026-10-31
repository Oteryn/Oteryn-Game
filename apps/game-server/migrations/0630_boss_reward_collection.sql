-- W03: the existing reward-claim ItemInstance owner consumes frozen personal entitlements.
-- One real CommandRef has one result; each selected Item keeps a one-item type-2 transaction.
CREATE TABLE game_boss_reward_collection_receipts (
    game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    intent BYTEA NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 32),
    status SMALLINT NOT NULL CHECK (status BETWEEN 0 AND 5),
    moved INTEGER NOT NULL CHECK (moved BETWEEN 0 AND 20),
    remaining BIGINT NOT NULL CHECK (remaining BETWEEN 0 AND 4294967295),
    source_placement TEXT NOT NULL CHECK (octet_length(source_placement) BETWEEN 1 AND 512),
    scope_kind SMALLINT NOT NULL CHECK (scope_kind IN (1,2)),
    scope_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation>0),
    lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation>0),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation>0),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest)=32),
    occurred_at BIGINT NOT NULL CHECK (occurred_at>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id),
    CHECK ((status=0 AND moved>0) OR (status<>0 AND moved=0))
);
CREATE TABLE game_boss_reward_item_collections (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    item_instance_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(item_instance_id)),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    item_ordinal INTEGER NOT NULL,
    parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    placement_ordinal NUMERIC(20,0) NOT NULL CHECK (placement_ordinal>0),
    occurred_at BIGINT NOT NULL CHECK (occurred_at>0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256)=32),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(character_id,world_id,death_id,item_ordinal),
    FOREIGN KEY(character_id,world_id,death_id,item_ordinal)
        REFERENCES game_boss_reward_items(character_id,world_id,death_id,item_ordinal),
    FOREIGN KEY(game_session_id,command_id)
        REFERENCES game_boss_reward_collection_receipts(game_session_id,command_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_boss_reward_item_collections_command ON game_boss_reward_item_collections(game_session_id,command_id);

CREATE FUNCTION game_boss_reward_collection_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='game_boss_reward_collection_receipts' THEN
        IF NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s
            JOIN game_character_roots r ON r.character_id=s.character_id AND r.world_id=s.world_id
            JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id
            JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id
            WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
              AND s.world_id=NEW.world_id AND s.runtime_scope_world_id=NEW.world_id
              AND s.runtime_scope_kind=NEW.scope_kind AND s.runtime_scope_house_key IS NULL
              AND ((NEW.scope_kind=1 AND s.runtime_scope_channel_id=NEW.scope_id AND s.runtime_scope_instance_id IS NULL)
                OR (NEW.scope_kind=2 AND s.runtime_scope_instance_id=NEW.scope_id AND s.runtime_scope_channel_id IS NULL))
              AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.lease_generation
              AND s.scope_ownership_generation=NEW.scope_generation AND s.session_state IN (1,2)
              AND c.account_id=s.account_id AND c.world_id=NEW.world_id AND c.eligible
              AND c.lease_generation=NEW.lease_generation AND c.holder_game_session_id=s.game_session_id
              AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id
              AND r.lifecycle=1) THEN
            RAISE EXCEPTION 'boss collection requires current Character/session/scope fence' USING ERRCODE='23514';
        END IF;
    ELSE
        IF NOT EXISTS (SELECT 1 FROM game_boss_reward_items i
            JOIN game_character_boss_reward_chests p USING(character_id,world_id,death_id)
            JOIN game_boss_reward_contributors c USING(character_id,world_id,death_id)
            WHERE (i.character_id,i.world_id,i.death_id,i.item_ordinal)=(NEW.character_id,NEW.world_id,NEW.death_id,NEW.item_ordinal)
              AND c.eligible AND NOT i.expired AND i.collected_transaction_id IS NULL
              AND NEW.occurred_at<p.expires_at AND p.expires_at>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
            OR EXISTS (SELECT 1 FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id)
            OR NOT EXISTS (SELECT 1 FROM game_item_container_slots s JOIN game_item_instances b USING(item_instance_id,world_id)
                WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id
                  AND s.item_instance_id=NEW.parent_item_instance_id AND b.lifecycle=1)
            OR NEW.placement_ordinal<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id) THEN
            RAISE EXCEPTION 'boss collection requires an unclaimed live entitlement and fresh backpack entry' USING ERRCODE='23514';
        END IF;
    END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    RETURN NEW;
END $$;

CREATE FUNCTION game_boss_reward_item_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE c game_boss_reward_item_collections%ROWTYPE;
BEGIN
    SELECT * INTO c FROM game_boss_reward_item_collections WHERE transaction_id=p_transaction;
    IF c.transaction_id IS NULL OR c.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS (SELECT 1 FROM game_boss_reward_collection_receipts r
          WHERE (r.game_session_id,r.command_id,r.character_id,r.world_id)=(c.game_session_id,c.command_id,c.character_id,c.world_id)
            AND r.status=0 AND r.created_xact_id=pg_current_xact_id() AND r.occurred_at=c.occurred_at)
      OR NOT EXISTS (SELECT 1 FROM game_boss_reward_items e JOIN game_item_instances i
            ON i.item_instance_id=c.item_instance_id
          JOIN game_item_container_entries p ON p.item_instance_id=i.item_instance_id
          WHERE (e.character_id,e.world_id,e.death_id,e.item_ordinal)=(c.character_id,c.world_id,c.death_id,c.item_ordinal)
            AND e.collected_transaction_id=c.transaction_id AND NOT e.expired
            AND i.world_id=c.world_id AND i.definition_family=e.definition_family
            AND i.definition_production_key=e.definition_production_key AND i.definition_revision_ref=e.definition_revision_ref
            AND i.quantity=e.quantity AND i.lifecycle=1 AND i.minted_transaction_id=c.transaction_id AND i.last_transaction_id IS NULL
            AND p.character_id=c.character_id AND p.world_id=c.world_id AND p.parent_item_instance_id=c.parent_item_instance_id
            AND p.placement_ordinal=c.placement_ordinal AND p.placed_transaction_id=c.transaction_id)
      OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=c.transaction_id)<>1
      OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=c.parent_item_instance_id)>20
      OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=c.transaction_id)<>1
      OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=c.event_id AND a.transaction_id=c.transaction_id
          AND a.item_instance_id=c.item_instance_id AND a.occurred_at=c.occurred_at AND a.envelope_sha256=c.envelope_sha256
          AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.event_type_id=2
          AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'boss reward mint requires receipt, consumed entitlement, exact custody and type-2 audit together' USING ERRCODE='23514';
    END IF;
END $$;
CREATE FUNCTION game_boss_reward_collection_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='game_boss_reward_item_collections' THEN
        PERFORM game_boss_reward_item_complete(NEW.transaction_id);
    ELSE
        IF (SELECT count(*) FROM game_boss_reward_item_collections c
            WHERE c.game_session_id=NEW.game_session_id AND c.command_id=NEW.command_id
              AND c.created_xact_id=pg_current_xact_id())<>NEW.moved THEN
            RAISE EXCEPTION 'boss collection receipt count differs from its item transitions' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_boss_reward_entitlement_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'boss entitlement provenance is immutable' USING ERRCODE='23514'; END IF;
    IF (to_jsonb(NEW)-ARRAY['expired','collected_transaction_id']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['expired','collected_transaction_id'])
        OR OLD.expired OR OLD.collected_transaction_id IS NOT NULL THEN
        RAISE EXCEPTION 'boss reward transition must preserve its frozen entitlement' USING ERRCODE='23514';
    END IF;
    IF NEW.expired AND NEW.collected_transaction_id IS NULL THEN
        IF NOT EXISTS (SELECT 1 FROM game_character_boss_reward_chests p
            WHERE (p.character_id,p.world_id,p.death_id)=(NEW.character_id,NEW.world_id,NEW.death_id)
              AND p.expires_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint) THEN
            RAISE EXCEPTION 'boss reward cannot expire before seven-day deadline' USING ERRCODE='23514';
        END IF;
    ELSIF NOT NEW.expired AND NEW.collected_transaction_id IS NOT NULL THEN
        IF NOT EXISTS (SELECT 1 FROM game_boss_reward_item_collections c
            WHERE (c.character_id,c.world_id,c.death_id,c.item_ordinal)=(NEW.character_id,NEW.world_id,NEW.death_id,NEW.item_ordinal)
              AND c.transaction_id=NEW.collected_transaction_id AND c.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'boss reward take requires same-transaction custody proof' USING ERRCODE='23514';
        END IF;
    ELSE RAISE EXCEPTION 'boss reward transition must collect or expire' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION game_boss_reward_projection_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'boss reward group provenance is immutable' USING ERRCODE='23514'; END IF;
    IF TG_OP='UPDATE' THEN
        IF (to_jsonb(NEW)-'item_count') IS DISTINCT FROM (to_jsonb(OLD)-'item_count') THEN
            RAISE EXCEPTION 'boss group metadata is frozen at death' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM game_boss_reward_contributors c JOIN game_boss_reward_deaths d USING(world_id,death_id)
        WHERE (c.character_id,c.world_id,c.death_id)=(NEW.character_id,NEW.world_id,NEW.death_id)
          AND c.eligible AND d.boss_key=NEW.boss_key AND d.minted_at=NEW.minted_at AND d.expires_at=NEW.expires_at) THEN
        RAISE EXCEPTION 'boss group requires its frozen eligible contributor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION game_boss_reward_projection_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actual BIGINT; projected INTEGER;
BEGIN
    SELECT count(*) INTO actual FROM game_boss_reward_items i
        WHERE (i.character_id,i.world_id,i.death_id)=(NEW.character_id,NEW.world_id,NEW.death_id)
          AND NOT i.expired AND i.collected_transaction_id IS NULL;
    SELECT item_count INTO projected FROM game_character_boss_reward_chests p
        WHERE (p.character_id,p.world_id,p.death_id)=(NEW.character_id,NEW.world_id,NEW.death_id);
    IF projected IS NULL OR projected<>actual THEN
        RAISE EXCEPTION 'boss reward visible group must match retained live items' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

-- Extend only the existing custody proof dispatch arms; every prior mechanic stays intact.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
        body:=pg_get_functiondef((n||'()')::regprocedure);
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'unexpected Item custody proof body: %',n; END IF;
        IF n='game_item_placement_proven' THEN
            arm:=$arm$
            IF TG_TABLE_NAME='game_item_container_entries' THEN
                IF EXISTS (SELECT 1 FROM game_boss_reward_item_collections c
                    WHERE c.item_instance_id=NEW.item_instance_id AND c.world_id=NEW.world_id
                      AND c.character_id=NEW.character_id AND c.parent_item_instance_id=NEW.parent_item_instance_id
                      AND c.placement_ordinal=NEW.placement_ordinal AND c.transaction_id=NEW.placed_transaction_id
                      AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
            END IF;
            $arm$;
        ELSE
            arm:=$arm$
            IF EXISTS (SELECT 1 FROM game_boss_reward_item_collections c
                WHERE c.item_instance_id=NEW.item_instance_id AND c.transaction_id=NEW.minted_transaction_id
                  AND c.created_xact_id=pg_current_xact_id()) THEN
                PERFORM game_boss_reward_item_complete(NEW.minted_transaction_id); RETURN NULL;
            END IF;
            $arm$;
        END IF;
        EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    END LOOP;
END $migration$;

CREATE TRIGGER game_boss_reward_collection_receipts_admit BEFORE INSERT ON game_boss_reward_collection_receipts FOR EACH ROW EXECUTE FUNCTION game_boss_reward_collection_admit();
CREATE TRIGGER game_boss_reward_item_collections_admit BEFORE INSERT ON game_boss_reward_item_collections FOR EACH ROW EXECUTE FUNCTION game_boss_reward_collection_admit();
CREATE CONSTRAINT TRIGGER game_boss_reward_collection_receipts_complete AFTER INSERT ON game_boss_reward_collection_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_reward_collection_complete();
CREATE CONSTRAINT TRIGGER game_boss_reward_item_collections_complete AFTER INSERT ON game_boss_reward_item_collections DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_reward_collection_complete();
CREATE TRIGGER game_boss_reward_collection_receipts_immutable BEFORE UPDATE OR DELETE ON game_boss_reward_collection_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_reward_item_collections_immutable BEFORE UPDATE OR DELETE ON game_boss_reward_item_collections FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_reward_collection_receipts_no_truncate BEFORE TRUNCATE ON game_boss_reward_collection_receipts EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_boss_reward_item_collections_no_truncate BEFORE TRUNCATE ON game_boss_reward_item_collections EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_boss_reward_entitlement_guard BEFORE UPDATE OR DELETE ON game_boss_reward_items FOR EACH ROW EXECUTE FUNCTION game_boss_reward_entitlement_guard();
CREATE CONSTRAINT TRIGGER game_boss_reward_entitlement_complete AFTER UPDATE ON game_boss_reward_items DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_reward_projection_complete();
CREATE TRIGGER game_boss_reward_projection_guard BEFORE INSERT OR UPDATE OR DELETE ON game_character_boss_reward_chests FOR EACH ROW EXECUTE FUNCTION game_boss_reward_projection_guard();
CREATE CONSTRAINT TRIGGER game_boss_reward_projection_complete AFTER INSERT OR UPDATE ON game_character_boss_reward_chests DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_reward_projection_complete();
CREATE TRIGGER game_boss_reward_projection_no_truncate BEFORE TRUNCATE ON game_character_boss_reward_chests EXECUTE FUNCTION game_item_reject_truncate();

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_boss_reward_collection_admit','game_boss_reward_collection_complete',
        'game_boss_reward_entitlement_guard','game_boss_reward_projection_guard','game_boss_reward_projection_complete'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_boss_reward_item_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_boss_reward_collection_receipts,game_boss_reward_item_collections FROM PUBLIC;
REVOKE ALL ON FUNCTION game_boss_reward_collection_admit(),game_boss_reward_collection_complete(),
    game_boss_reward_item_complete(UUID),game_boss_reward_entitlement_guard(),game_boss_reward_projection_guard(),game_boss_reward_projection_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_boss_reward_collection_receipts,game_boss_reward_item_collections TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_boss_reward_item_complete(UUID) TO oteryn_game_runtime;
REVOKE DELETE,UPDATE ON game_character_boss_reward_chests FROM oteryn_game_runtime;
GRANT UPDATE(item_count) ON game_character_boss_reward_chests TO oteryn_game_runtime;

-- The native private boss owner reuses PARTY-XP-1. Channel receipt encodings remain
-- unchanged; only a tagged Instance key can use the additional byte.
ALTER TABLE game_party_kill_xp_receipts
    DROP CONSTRAINT game_party_kill_xp_receipts_death_key_check;
ALTER TABLE game_party_kill_xp_receipts
    ADD CONSTRAINT game_party_kill_xp_death_scope_key CHECK (
        octet_length(death_key)=52 OR
        (octet_length(death_key)=53 AND get_byte(death_key,0)=2)
    );

-- W02 prerequisite, coordinator ANSWER 6543264466: extend the existing physical
-- Ground owner with its exact Instance namespace. Existing Channel data and
-- audit encodings are retained; no origin Channel substitutes for an Instance.
ALTER TABLE game_character_death_receipts
 ALTER COLUMN death_channel_id DROP NOT NULL,
 DROP CONSTRAINT game_character_death_receipts_death_channel_id_check,
 ADD COLUMN death_scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN death_instance_id UUID,
 ADD COLUMN death_scope_generation NUMERIC(20,0)
  CHECK(death_scope_generation BETWEEN 1 AND 18446744073709551615),
 ADD CONSTRAINT game_character_death_receipts_scope CHECK(
  (death_scope_kind=1 AND death_channel_id IS NOT NULL AND game_character_is_uuid_v7(death_channel_id) AND death_instance_id IS NULL)
  OR (death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id IS NOT NULL AND game_character_is_uuid_v7(death_instance_id) AND death_scope_generation IS NOT NULL));

ALTER TABLE game_item_ground_locations
 ALTER COLUMN channel_id DROP NOT NULL,
 DROP CONSTRAINT game_item_ground_locations_channel_id_check,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_item_ground_locations_scope CHECK(
  (scope_kind=1 AND channel_id IS NOT NULL AND game_character_is_uuid_v7(channel_id) AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));
CREATE INDEX game_item_ground_instance_cell_lookup
 ON game_item_ground_locations(world_id,instance_id,spatial_position) WHERE scope_kind=2;
CREATE INDEX game_item_ground_instance_loop_drops
 ON game_item_ground_locations(world_id,instance_id) WHERE scope_kind=2 AND game_item_loop_drop_ref(corpse_ref);

ALTER TABLE game_player_death_item_records
 ALTER COLUMN channel_id DROP NOT NULL,
 DROP CONSTRAINT game_player_death_item_records_channel_id_check,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_player_death_item_records_scope CHECK(
  (scope_kind=1 AND channel_id IS NOT NULL AND game_character_is_uuid_v7(channel_id) AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));

ALTER TABLE game_item_ground_removal_evidence
 ALTER COLUMN channel_id DROP NOT NULL,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_item_ground_removal_evidence_scope CHECK(
  (scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));
ALTER TABLE game_item_loop_move_receipts
 ALTER COLUMN channel_id DROP NOT NULL,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_item_loop_move_receipts_scope CHECK(
  (scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));

-- The existing House reservation shape has source_kind>0 and no Channel.
-- Keep its stored/default discriminator and existing proof unchanged; source_kind=0
-- is the physical Ground shape and must name exactly one Channel or Instance.
ALTER TABLE game_item_transfer_reservations
 ALTER COLUMN channel_id DROP NOT NULL,
 DROP CONSTRAINT game_item_transfer_reservations_channel_id_check,
 DROP CONSTRAINT game_item_transfer_house_scope,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_item_transfer_reservations_scope CHECK(
  (source_kind=0 AND (
   (scope_kind=1 AND channel_id IS NOT NULL AND game_character_is_uuid_v7(channel_id) AND instance_id IS NULL)
   OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id))))
  OR (source_kind>0 AND scope_kind=1 AND channel_id IS NULL AND instance_id IS NULL)),
 ADD CONSTRAINT game_item_transfer_house_scope CHECK(
  (source_kind=0 AND house_key IS NULL AND house_scope_generation IS NULL AND house_intent IS NULL AND destination_kind IN(1,2))
  OR (source_kind>0 AND channel_id IS NULL AND house_key IS NOT NULL AND house_scope_generation IS NOT NULL AND game_house_scope_key_valid(house_key)
   AND house_intent IS NOT NULL AND jsonb_typeof(house_intent)='object' AND octet_length(house_intent::text)<=16384
   AND house_scope_generation BETWEEN 1 AND 18446744073709551615 AND destination_kind IN(3,4,5)));

-- Preserve the existing Channel ON CONFLICT target and scope-local resource
-- bound. A second unique key admits one counter row for each actual Instance.
ALTER TABLE game_item_loop_drop_channels
 DROP CONSTRAINT game_item_loop_drop_channels_pkey,
 ALTER COLUMN channel_id DROP NOT NULL,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_item_loop_drop_channels_channel UNIQUE(world_id,channel_id),
 ADD CONSTRAINT game_item_loop_drop_channels_instance UNIQUE(world_id,instance_id),
 ADD CONSTRAINT game_item_loop_drop_channels_scope CHECK(
  (scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));

-- Reissue only the scope comparisons in installed functions. This preserves
-- all later proof arms (depot, quests, map, reset, source removal and audits).
DO $migration$
DECLARE body TEXT; amended TEXT; name TEXT;
BEGIN
 body:=pg_get_functiondef('game_player_death_item_admit()'::regprocedure);
 amended:=replace(body,'d.death_channel_id=NEW.channel_id',
  '(d.death_scope_kind,d.death_channel_id,d.death_instance_id) IS NOT DISTINCT FROM (NEW.scope_kind,NEW.channel_id,NEW.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing death exact-scope receipt proof'; END IF;
 EXECUTE amended;

 body:=pg_get_functiondef('game_player_death_item_complete(uuid)'::regprocedure);
 amended:=replace(body,'g.channel_id=r.channel_id',
  '(g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing death Ground completion scope proof'; END IF;
 EXECUTE amended;

 FOREACH name IN ARRAY ARRAY['game_item_loop_line_admit()','game_item_loop_line_proven()'] LOOP
  body:=pg_get_functiondef(name::regprocedure);
  amended:=replace(body,'g.channel_id <> r.channel_id',
   '(g.scope_kind,g.channel_id,g.instance_id) IS DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id)');
  IF amended=body THEN RAISE EXCEPTION 'missing loop source/destination scope comparison: %',name; END IF;
  amended:=replace(amended,'x.channel_id = g.channel_id',
   '(x.scope_kind,x.channel_id,x.instance_id) IS NOT DISTINCT FROM (g.scope_kind,g.channel_id,g.instance_id)');
  EXECUTE amended;
 END LOOP;

 body:=pg_get_functiondef('game_item_transfer_consistency_guard()'::regprocedure);
 amended:=replace(body,'gre.channel_id = v.channel_id',
  '(gre.scope_kind,gre.channel_id,gre.instance_id) IS NOT DISTINCT FROM (v.scope_kind,v.channel_id,v.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing TRANSFER actual Ground removal scope proof'; END IF;
 EXECUTE amended;

 -- Only the existing death primitive has these discriminator columns. Do not
 -- rewrite other Channel-only proof arms that use the same local alias.
 -- Install the narrow accepted Instance arm ahead of the unmodified fallback.
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF NEW.scope_kind=2 THEN
  IF EXISTS(SELECT 1 FROM game_player_death_item_records r
    WHERE r.item_instance_id=NEW.item_instance_id AND r.operation=1
     AND r.world_id=NEW.world_id AND r.scope_kind=2 AND r.instance_id=NEW.instance_id
     AND r.created_xact_id=pg_current_xact_id()) THEN
   PERFORM game_player_death_item_complete((SELECT transaction_id FROM game_player_death_item_records
    WHERE item_instance_id=NEW.item_instance_id AND created_xact_id=pg_current_xact_id()));
   RETURN NULL;
  END IF;
  RAISE EXCEPTION 'Instance Ground requires its exact accepted placement cause' USING ERRCODE='23514';
 END IF;
$arm$ from regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i') for 5);
END $migration$;

-- Match spell_item_transaction::lock_runtime_items: Channel retains its
-- original key; Instance uses assignment_scope_key = 0x03||world||instance.
-- Assignment kind 3 and custody/session kind 2 are distinct tag spaces.
DO $migration$
DECLARE body TEXT; amended TEXT; side TEXT; before_key TEXT; after_key TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_ground_owner_lock()'::regprocedure);
 FOREACH side IN ARRAY ARRAY['NEW','OLD'] LOOP
  before_key:=format('encode(uuid_send(%1$s.world_id),''hex'') || encode(uuid_send(%1$s.channel_id),''hex'')',side);
  after_key:=format('(CASE WHEN %1$s.scope_kind=1 THEN encode(uuid_send(%1$s.world_id),''hex'') || encode(uuid_send(%1$s.channel_id),''hex'') ELSE encode(decode(''03'',''hex'') || uuid_send(%1$s.world_id) || uuid_send(%1$s.instance_id),''hex'') END)',side);
  amended:=replace(body,before_key,after_key);
  IF amended=body THEN RAISE EXCEPTION 'missing Ground key33 lock side %',side; END IF;
  body:=amended;
 END LOOP;
 EXECUTE body;
END $migration$;

CREATE OR REPLACE FUNCTION game_item_ground_removal_evidence_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_item_ground_removal_evidence(item_instance_id,world_id,channel_id,scope_kind,instance_id)
 VALUES(OLD.item_instance_id,OLD.world_id,OLD.channel_id,OLD.scope_kind,OLD.instance_id)
 ON CONFLICT(item_instance_id) DO UPDATE
  SET world_id=EXCLUDED.world_id,channel_id=EXCLUDED.channel_id,
      scope_kind=EXCLUDED.scope_kind,instance_id=EXCLUDED.instance_id;
 RETURN OLD;
END $$;

DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_ground_removal_evidence_guard()'::regprocedure);
 amended:=replace(body,'g.channel_id = NEW.channel_id',
  '(g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (NEW.scope_kind,NEW.channel_id,NEW.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing Ground removal evidence scope proof'; END IF;
 EXECUTE amended;
END $migration$;

DO $$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY[
  'game_player_death_item_admit()','game_player_death_item_complete(uuid)',
  'game_item_loop_line_admit()','game_item_loop_line_proven()',
  'game_item_transfer_consistency_guard()',
  'game_item_ground_insertion_guard()','game_item_ground_owner_lock()',
  'game_item_ground_removal_evidence_capture()','game_item_ground_removal_evidence_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $$;

-- W02 prerequisite, append to reserved 0630 after the Instance Ground custody
-- extension. This remains the existing one-item retirement owner and tables.
-- No runtime assignment/grant writer and no Channel alias is introduced.
DO $migration$
DECLARE t TEXT;
BEGIN
 FOREACH t IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
  EXECUTE format('ALTER TABLE %I ALTER COLUMN channel_id DROP NOT NULL,
   ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
   ADD COLUMN instance_id UUID,
   ADD COLUMN activity_handoff_id UUID REFERENCES game_activity_handoff_groups(handoff_id),
   ADD COLUMN activity_ownership_generation NUMERIC(20,0),
   ADD CONSTRAINT game_retire_exact_scope CHECK (
    (scope_kind=1 AND channel_id IS NOT NULL AND game_character_is_uuid_v7(channel_id) AND instance_id IS NULL)
    OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)))',t);
  EXECUTE format('ALTER TABLE %I DROP CONSTRAINT game_reset_retire_shape',t);
  EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_reset_retire_shape CHECK (
   occurred_at>0 AND (
    (cause_kind=''CORPSE_DECAY'' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND deadline>=60000 AND occurred_at>=deadline
     AND (item_instance_id=corpse_item_instance_id)=(placement_ordinal IS NULL)
     AND reset_epoch IS NULL AND retirement_binding IS NULL AND target_digest IS NULL AND ground_root_item_instance_id IS NULL AND activity_handoff_id IS NULL AND activity_ownership_generation IS NULL)
    OR (cause_kind=''WORLD_RESET'' AND scope_kind=1 AND corpse_item_instance_id IS NULL AND deadline IS NULL
     AND reset_epoch BETWEEN 0 AND 18446744073709551614 AND reset_epoch IS NOT NULL
     AND octet_length(retirement_binding)=32 AND retirement_binding IS NOT NULL
     AND octet_length(target_digest)=32 AND target_digest IS NOT NULL
     AND ground_root_item_instance_id IS NOT NULL AND activity_handoff_id IS NULL AND activity_ownership_generation IS NULL
     AND (item_instance_id=ground_root_item_instance_id)=(placement_ordinal IS NULL)
     AND retirement_binding=sha256(convert_to(''oteryn:world-reset-retirement:v1'',''UTF8'') || target_digest))
    OR (cause_kind=''ACTIVITY_RETIREMENT'' AND scope_kind=2 AND corpse_item_instance_id IS NULL AND deadline IS NULL
     AND reset_epoch IS NULL AND retirement_binding IS NULL AND target_digest IS NULL
     AND activity_handoff_id IS NOT NULL AND ground_root_item_instance_id IS NOT NULL
     AND activity_ownership_generation IS NOT NULL AND activity_ownership_generation BETWEEN 1 AND fence_scope_ownership_generation
     AND (item_instance_id=ground_root_item_instance_id)=(placement_ordinal IS NULL))))',t);
 END LOOP;
END $migration$;

ALTER TABLE game_item_corpse_entry_removal_evidence
 ALTER COLUMN channel_id DROP NOT NULL,
 ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_corpse_entry_removal_exact_scope CHECK (
  (scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
  OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id)));

-- Retaining a handoff id is not authority. All members must have their real
-- immutable return receipts; no current Instance session may remain. A current
-- assignment must still be this generation and node, or its exact holder
-- registration must have ended and the recovery node must hold a current ready
-- origin Channel. A revoked assignment may be cleaned, but a replacement active
-- generation is never touched. This matches the existing Rust retirement proof.
CREATE FUNCTION game_activity_retirement_binding_valid(
 p_handoff UUID,p_world UUID,p_instance UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC)
 RETURNS BOOLEAN LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
 SELECT game_node_lock_current_registration(p_node,p_registration)
  AND game_runtime_scope_assignment_history_valid()
  AND EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
   JOIN game_runtime_scope_assignments a ON a.scope_key='\x03'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id)
   WHERE g.handoff_id=p_handoff AND g.world_id=p_world AND g.instance_id=p_instance
    AND g.ownership_generation=p_generation AND g.state=2 AND a.source_revision>=e.assignment_source_revision
    AND g.participant_count=(SELECT count(*) FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id)
    AND NOT EXISTS(SELECT 1 FROM game_activity_handoff_participants p
     LEFT JOIN game_activity_runtime_returns r ON (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)
     LEFT JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.destination_game_session_id
     WHERE p.handoff_id=g.handoff_id AND (r.return_id IS NULL OR r.source_game_session_id<>p.destination_game_session_id
      OR s.session_state IS DISTINCT FROM 3 OR NOT COALESCE(game_activity_session_origin_valid(r.destination_game_session_id,p.character_id),false)))
    AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_kind=2
     AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_instance_id=g.instance_id AND s.session_state IN(1,2))
    AND (a.state<>1 OR (a.ownership_generation=g.ownership_generation
     AND ((a.holder_node_id=p_node AND a.holder_registration_revision=p_registration)
      OR (NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision)
       AND EXISTS(SELECT 1
        FROM game_activity_handoff_participants origin_p
        JOIN game_runtime_scope_assignments origin_a
         ON origin_a.scope_key='\x01'::bytea||uuid_send(origin_p.world_id)||uuid_send(origin_p.origin_channel_id)
        JOIN game_durability_admission_runtime_guards origin_guard ON origin_guard.scope_key=origin_a.scope_key
        WHERE origin_p.handoff_id=g.handoff_id AND origin_p.world_id=g.world_id
         AND origin_a.world_id=origin_p.world_id
         AND origin_a.channel_id IS NOT DISTINCT FROM origin_p.origin_channel_id
         AND origin_a.activity_instance_id IS NULL AND origin_a.house_key IS NULL
         AND origin_a.scope_kind=1 AND origin_a.state=1
         AND origin_a.holder_node_id=p_node AND origin_a.holder_registration_revision=p_registration
         AND origin_guard.ready AND origin_guard.ownership_generation=origin_a.ownership_generation
        FOR SHARE OF origin_a,origin_guard))))))
$$;

-- Capture the exact old Ground namespace. Existing Channel rows and their
-- recorded identifiers remain unchanged; evidence is never caller-authored.
CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_evidence_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_item_corpse_entry_removal_evidence
  (item_instance_id,parent_item_instance_id,placement_ordinal,world_id,channel_id,scope_kind,instance_id)
 SELECT OLD.item_instance_id,OLD.parent_item_instance_id,OLD.placement_ordinal,
  g.world_id,g.channel_id,g.scope_kind,g.instance_id
 FROM game_item_ground_locations g JOIN game_item_instances ci USING(item_instance_id)
 WHERE g.item_instance_id=OLD.parent_item_instance_id AND g.world_id=OLD.world_id AND ci.lifecycle=1;
 IF NOT FOUND THEN RAISE EXCEPTION 'corpse container entry removal requires its corpse live on Ground' USING ERRCODE='23514'; END IF;
 RETURN OLD;
END $$;

-- The same existing direct death-container capture supports activity retirement.
-- Its later deferred proof still requires the actual shared retirement receipt.
CREATE OR REPLACE FUNCTION game_player_death_reset_entry_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_item_ground_locations g USING(item_instance_id)
  WHERE r.item_instance_id=OLD.parent_item_instance_id AND r.operation=1 AND r.source_slot=9
   AND g.world_id=OLD.world_id
   AND (g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id)
   AND ((g.scope_kind=1 AND EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=g.world_id AND j.phase='RETIRING'))
    OR (g.scope_kind=2 AND EXISTS(SELECT 1 FROM game_item_decay_retire_reservations v
     WHERE v.item_instance_id=OLD.parent_item_instance_id AND v.ground_root_item_instance_id=v.item_instance_id
      AND v.cause_kind='ACTIVITY_RETIREMENT' AND v.world_id=g.world_id AND v.instance_id=g.instance_id
      AND game_activity_retirement_binding_valid(v.activity_handoff_id,v.world_id,v.instance_id,
       v.activity_ownership_generation,v.fence_holder_node_id,v.fence_holder_registration_revision)
      AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.scope_key='\x03'::bytea||uuid_send(v.world_id)||uuid_send(v.instance_id) AND a.ownership_generation=v.fence_scope_ownership_generation))))
   AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id)) THEN
  INSERT INTO game_item_corpse_entry_removal_evidence
   (item_instance_id,parent_item_instance_id,placement_ordinal,world_id,channel_id,scope_kind,instance_id)
  SELECT OLD.item_instance_id,OLD.parent_item_instance_id,OLD.placement_ordinal,OLD.world_id,g.channel_id,g.scope_kind,g.instance_id
   FROM game_item_ground_locations g WHERE g.item_instance_id=OLD.parent_item_instance_id;
 END IF;
 RETURN OLD;
END $$;

-- One physical transaction must conserve the whole item, remove its real
-- location, retain the exact immutable event, and prove the completed activity.
CREATE FUNCTION game_activity_retirement_item_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_decay_retire_receipts%ROWTYPE; v game_item_decay_retire_reservations%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_decay_retire_receipts WHERE item_instance_id=p_item
  AND cause_kind='ACTIVITY_RETIREMENT' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO v FROM game_item_decay_retire_reservations WHERE item_instance_id=p_item
  AND fence_scope_ownership_generation=r.fence_scope_ownership_generation;
 IF NOT FOUND OR NOT game_activity_retirement_binding_valid(r.activity_handoff_id,r.world_id,r.instance_id,
  r.activity_ownership_generation,v.fence_holder_node_id,v.fence_holder_registration_revision)
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.scope_key='\x03'::bytea||uuid_send(r.world_id)||uuid_send(r.instance_id) AND a.ownership_generation=r.fence_scope_ownership_generation)
  OR (v.item_instance_id,v.fence_scope_ownership_generation,v.world_id,v.scope_kind,v.instance_id,
      v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.cause_kind,v.activity_handoff_id,v.activity_ownership_generation,v.ground_root_item_instance_id)
   IS DISTINCT FROM (r.item_instance_id,r.fence_scope_ownership_generation,r.world_id,r.scope_kind,r.instance_id,
      r.transaction_id,r.event_id,r.quantity_before,r.occurred_at,r.cause_kind,r.activity_handoff_id,r.activity_ownership_generation,r.ground_root_item_instance_id)
  OR v.placement_ordinal IS DISTINCT FROM r.placement_ordinal OR sha256(v.envelope)<>r.envelope_sha256
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.world_id=r.world_id
   AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item)
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev WHERE ev.item_instance_id=p_item
   AND ev.transaction_id=r.transaction_id AND ev.quantity_before=r.quantity_before)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.item_instance_id=p_item AND o.event_id=r.event_id
   AND o.transaction_id=r.transaction_id AND o.occurred_at=r.occurred_at AND o.envelope_sha256=r.envelope_sha256
   AND o.created_xact_id=pg_current_xact_id() AND o.publication_state=1 AND o.published_at IS NULL)
  OR (r.placement_ordinal IS NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence e WHERE e.item_instance_id=p_item
    AND e.world_id=r.world_id AND e.scope_kind=2 AND e.instance_id=r.instance_id AND e.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p_item)))
  OR (r.placement_ordinal IS NOT NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence e WHERE e.item_instance_id=p_item
    AND e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal=r.placement_ordinal
    AND e.world_id=r.world_id AND e.scope_kind=2 AND e.instance_id=r.instance_id AND e.created_xact_id=pg_current_xact_id())
   OR NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
    WHERE g.item_instance_id=r.ground_root_item_instance_id AND g.world_id=r.world_id AND g.scope_kind=2
     AND g.instance_id=r.instance_id AND g.runtime_scope_ownership_generation=r.activity_ownership_generation AND i.lifecycle=1
     AND NOT EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.item_instance_id=g.item_instance_id AND d.operation=1 AND d.source_slot=9
      AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(d.transaction_id)))
    OR EXISTS(SELECT 1 FROM game_item_decay_retire_receipts parent
     JOIN game_player_death_item_records d ON d.item_instance_id=parent.item_instance_id
     JOIN game_item_ground_removal_evidence g ON g.item_instance_id=parent.item_instance_id
     WHERE parent.item_instance_id=r.ground_root_item_instance_id AND parent.ground_root_item_instance_id=parent.item_instance_id
      AND parent.cause_kind='ACTIVITY_RETIREMENT' AND parent.world_id=r.world_id AND parent.scope_kind=2 AND parent.instance_id=r.instance_id
      AND parent.activity_handoff_id=r.activity_handoff_id AND parent.activity_ownership_generation=r.activity_ownership_generation AND parent.fence_scope_ownership_generation=r.fence_scope_ownership_generation
      AND parent.created_xact_id=pg_current_xact_id() AND g.created_xact_id=pg_current_xact_id()
      AND g.world_id=r.world_id AND g.scope_kind=2 AND g.instance_id=r.instance_id
      AND d.operation=1 AND d.source_slot=9 AND d.world_id=r.world_id AND d.scope_kind=2 AND d.instance_id=r.instance_id))
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal<r.placement_ordinal))) THEN
  RAISE EXCEPTION 'activity retirement requires exact completed Instance, conserved item, removal, reservation and audit' USING ERRCODE='23514';
 END IF;
 RETURN TRUE;
END $$;

-- Keep every installed quest, death, reset, map and custody proof arm. Add the
-- accepted cause ahead of the latest bodies; never copy an obsolete guard.
DO $migration$
DECLARE f TEXT; body TEXT; item_ref TEXT; begin_at INT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_item_decay_retire_consistency_guard','game_item_instance_change_proven',
  'game_item_ground_removal_proven','game_item_corpse_entry_removal_proven','game_item_container_entry_removal_proven'] LOOP
  body:=pg_get_functiondef(to_regprocedure(f||'()'));
  item_ref:=CASE WHEN f IN('game_item_ground_removal_proven','game_item_corpse_entry_removal_proven','game_item_container_entry_removal_proven')
   THEN 'OLD.item_instance_id' ELSE 'NEW.item_instance_id' END;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing retirement guard %',f; END IF;
  EXECUTE overlay(body placing 'BEGIN IF game_activity_retirement_item_proven('||item_ref||') THEN RETURN NULL; END IF;' from begin_at for 5);
  EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 END LOOP;
END $migration$;

-- Ordinary CorpseDecay uses the same real Instance Ground without changing
-- any deadline or loss rule. Amend only scope comparisons in its latest guard.
DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_decay_retire_consistency_guard()'::regprocedure);
 amended:=replace(body,'v.world_id, v.channel_id, v.transaction_id', 'v.world_id, v.scope_kind, COALESCE(v.channel_id,v.instance_id), v.transaction_id');
 IF amended=body THEN RAISE EXCEPTION 'missing frozen decay scope tuple'; END IF;
 body:=amended;
 amended:=replace(body,'NEW.world_id, NEW.channel_id,', 'NEW.world_id, NEW.scope_kind, COALESCE(NEW.channel_id,NEW.instance_id),');
 IF amended=body THEN RAISE EXCEPTION 'missing receipt decay scope tuple'; END IF;
 body:=amended;
 amended:=replace(body,'gre.channel_id = NEW.channel_id', '(gre.scope_kind,gre.channel_id,gre.instance_id) IS NOT DISTINCT FROM (NEW.scope_kind,NEW.channel_id,NEW.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing decay Ground evidence comparison'; END IF;
 body:=amended;
 amended:=replace(body,'cre.channel_id = NEW.channel_id', '(cre.scope_kind,cre.channel_id,cre.instance_id) IS NOT DISTINCT FROM (NEW.scope_kind,NEW.channel_id,NEW.instance_id)');
 IF amended=body THEN RAISE EXCEPTION 'missing decay corpse evidence comparison'; END IF;
 EXECUTE amended;
END $migration$;

DO $migration$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_activity_retirement_binding_valid(uuid,uuid,uuid,numeric,uuid,numeric)',
  'game_activity_retirement_item_proven(uuid)','game_item_corpse_entry_removal_evidence_capture()',
  'game_player_death_reset_entry_capture()','game_item_decay_retire_consistency_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $migration$;

-- game_world_reset_activate retains its existing all-Ground absence check.
-- It cannot activate a replacement map while any Instance Ground remains.
