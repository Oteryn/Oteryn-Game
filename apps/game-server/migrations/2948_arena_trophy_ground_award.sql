-- Crystal 201189f8 movements_arena_trophy.lua: the trophy starts on Ground.
CREATE TABLE game_arena_trophy_mints (
    transaction_id uuid PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id uuid NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    item_instance_id uuid NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(item_instance_id))
        REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
    character_id uuid NOT NULL REFERENCES game_character_roots,
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    scope_generation numeric(20,0) NOT NULL CHECK (scope_generation > 0),
    rank smallint NOT NULL CHECK (rank BETWEEN 1 AND 3),
    claim_revision text NOT NULL,
    item_revision text NOT NULL,
    ground_position bytea NOT NULL CHECK (octet_length(ground_position) = 10),
    map_revision text NOT NULL,
    content_revision text NOT NULL,
    placement_context bytea NOT NULL CHECK (octet_length(placement_context) = 32),
    inscription text NOT NULL CHECK (length(inscription) > 0),
    occurred_at bigint NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 bytea NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, rank)
);
REVOKE ALL ON game_arena_trophy_mints FROM PUBLIC;
GRANT SELECT ON game_arena_trophy_mints TO oteryn_game_control, oteryn_game_runtime;
GRANT INSERT(transaction_id,event_id,item_instance_id,character_id,world_id,channel_id,
    scope_generation,rank,claim_revision,item_revision,ground_position,map_revision,
    content_revision,placement_context,inscription,occurred_at,envelope_sha256)
    ON game_arena_trophy_mints TO oteryn_game_runtime;

CREATE FUNCTION game_arena_trophy_mint_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_item_ground_locations g USING (item_instance_id, world_id)
        JOIN game_item_audit_outbox a
          ON a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id
        JOIN game_character_roots c ON c.character_id=NEW.character_id AND c.world_id=i.world_id
        JOIN game_reward_claims r ON r.character_id=c.character_id
          AND r.claimed_transaction_id=NEW.transaction_id
        WHERE i.item_instance_id=NEW.item_instance_id AND i.world_id=NEW.world_id
          AND i.lifecycle=1 AND i.quantity=1 AND i.last_transaction_id IS NULL
          AND i.minted_transaction_id=NEW.transaction_id AND i.definition_family='Item'
          AND i.definition_production_key='oteryn:item.tibia.i' || (5808-NEW.rank)::text
          AND i.definition_revision_ref=NEW.item_revision
          AND g.channel_id=NEW.channel_id AND g.scope_kind=1
          AND g.runtime_scope_ownership_generation=NEW.scope_generation
          AND g.spatial_position=NEW.ground_position AND g.map_revision=NEW.map_revision
          AND g.content_revision=NEW.content_revision AND g.native_room_placement_context=NEW.placement_context
          AND g.corpse_ref=convert_to('arena-trophy-v1','UTF8')
          AND a.item_instance_id=i.item_instance_id AND a.occurred_at=NEW.occurred_at
          AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1
          AND a.event_type_id=2 AND a.envelope_sha256=NEW.envelope_sha256
          AND r.claim_family='RewardClaim' AND r.claim_revision_ref=NEW.claim_revision
          AND r.claim_production_key='oteryn:reward.svargrond-arena.' || NEW.rank::text || '.4'
          AND r.next_allowed_at IS NULL
    ) THEN
        RAISE EXCEPTION 'arena trophy requires a Ground item, audit and entitlement in one transaction'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_arena_trophy_mint_proven
    AFTER INSERT ON game_arena_trophy_mints DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_arena_trophy_mint_proven();
CREATE TRIGGER game_arena_trophy_mint_immutable BEFORE UPDATE OR DELETE ON game_arena_trophy_mints
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_arena_trophy_mint_no_truncate BEFORE TRUNCATE ON game_arena_trophy_mints
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();


DO $migration$
DECLARE
    name text;
    definition text;
    begin_at integer;
    proof text;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_ground_insertion_guard',
                                'game_reward_claim_proven'] LOOP
        definition := pg_get_functiondef((name||'()')::regprocedure);
        begin_at := regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN
            RAISE EXCEPTION 'unexpected arena ground award guard: %',name;
        END IF;
        IF name='game_reward_claim_proven' THEN
            proof := $arm$
              IF EXISTS(SELECT 1 FROM game_arena_trophy_mints t
                 WHERE t.transaction_id=NEW.claimed_transaction_id
                   AND t.character_id=NEW.character_id AND t.created_xact_id=pg_current_xact_id()
                   AND NEW.claim_family='RewardClaim' AND NEW.claim_revision_ref=t.claim_revision
                   AND NEW.claim_production_key='oteryn:reward.svargrond-arena.'||t.rank::text||'.4')
              THEN RETURN NULL; END IF;
$arm$;
        ELSE
            proof := $arm$
              IF EXISTS(SELECT 1 FROM game_arena_trophy_mints t
                 WHERE t.item_instance_id=NEW.item_instance_id
                   AND t.world_id=NEW.world_id AND t.created_xact_id=pg_current_xact_id())
              THEN RETURN NULL; END IF;
$arm$;
        END IF;
        EXECUTE overlay(definition placing E'BEGIN\n'||proof from begin_at for 5);
    END LOOP;

    -- The loop's ordinary Ground pickup path recognizes a proven trophy mint.
    SELECT prosrc INTO definition FROM pg_proc WHERE oid='game_item_loop_map_mint_source(uuid)'::regprocedure;
    IF definition IS NULL OR position('game_map_item_mint_receipts' IN definition)=0 THEN
        RAISE EXCEPTION 'missing map-mint pickup source proof';
    END IF;
    definition := rtrim(definition,E' \n\r\t;') ||
        ' OR EXISTS(SELECT 1 FROM game_arena_trophy_mints WHERE item_instance_id=p_item)';
    EXECUTE format('CREATE OR REPLACE FUNCTION game_item_loop_map_mint_source(p_item uuid)
        RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp AS %L',
        current_schema(),definition);
    EXECUTE format('ALTER FUNCTION game_arena_trophy_mint_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_arena_trophy_mint_proven() FROM PUBLIC;
