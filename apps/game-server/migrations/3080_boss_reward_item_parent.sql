-- Personal rewards retain a one-level tree within each character's frozen chest.
ALTER TABLE game_boss_reward_items
    ADD COLUMN parent_item_ordinal INTEGER,
    ADD CONSTRAINT game_boss_reward_item_parent_precedes CHECK (parent_item_ordinal < item_ordinal),
    ADD CONSTRAINT game_boss_reward_item_parent_fk
        FOREIGN KEY (character_id,world_id,death_id,parent_item_ordinal)
        REFERENCES game_boss_reward_items (character_id,world_id,death_id,item_ordinal);
CREATE INDEX game_boss_reward_items_parent
    ON game_boss_reward_items (character_id,world_id,death_id,parent_item_ordinal,item_ordinal)
    WHERE parent_item_ordinal IS NOT NULL;

-- A receipt still counts physical transitions, including children of each selected root.
-- The separate collection prefix remains bounded by the 20 direct backpack slots.
ALTER TABLE game_boss_reward_collection_receipts
    DROP CONSTRAINT game_boss_reward_collection_receipts_moved_check,
    ADD CONSTRAINT game_boss_reward_collection_receipts_moved_check CHECK (moved BETWEEN 0 AND 500);

CREATE FUNCTION game_boss_reward_item_parent_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM 1 FROM game_boss_reward_items p
    WHERE (p.character_id,p.world_id,p.death_id,p.item_ordinal)
         =(NEW.character_id,NEW.world_id,NEW.death_id,NEW.parent_item_ordinal)
      AND p.parent_item_ordinal IS NULL AND p.quantity=1 AND p.expired=NEW.expired
      AND p.collected_transaction_id IS NULL
    FOR NO KEY UPDATE;
    IF NOT FOUND OR (SELECT count(*) FROM game_boss_reward_items i
          WHERE (i.character_id,i.world_id,i.death_id,i.parent_item_ordinal)
               =(NEW.character_id,NEW.world_id,NEW.death_id,NEW.parent_item_ordinal))>=20 THEN
        RAISE EXCEPTION 'boss reward child requires a single root parent with an available slot' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_boss_reward_item_parent_guard
    BEFORE INSERT ON game_boss_reward_items FOR EACH ROW
    WHEN (NEW.parent_item_ordinal IS NOT NULL)
    EXECUTE FUNCTION game_boss_reward_item_parent_guard();

CREATE OR REPLACE FUNCTION game_boss_reward_collection_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE parent_ordinal INTEGER;
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
        SELECT i.parent_item_ordinal INTO parent_ordinal FROM game_boss_reward_items i
            JOIN game_character_boss_reward_chests p USING(character_id,world_id,death_id)
            JOIN game_boss_reward_contributors c USING(character_id,world_id,death_id)
            WHERE (i.character_id,i.world_id,i.death_id,i.item_ordinal)=(NEW.character_id,NEW.world_id,NEW.death_id,NEW.item_ordinal)
              AND c.eligible AND NOT i.expired AND i.collected_transaction_id IS NULL
              AND NEW.occurred_at<p.expires_at AND p.expires_at>floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
        IF NOT FOUND OR EXISTS (SELECT 1 FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id) THEN
            RAISE EXCEPTION 'boss collection requires an unclaimed live entitlement' USING ERRCODE='23514';
        END IF;
        IF parent_ordinal IS NULL THEN
            IF NOT EXISTS (SELECT 1 FROM game_item_container_slots s JOIN game_item_instances b USING(item_instance_id,world_id)
                  WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id
                    AND s.item_instance_id=NEW.parent_item_instance_id AND b.lifecycle=1)
                OR NEW.placement_ordinal<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries
                                            WHERE parent_item_instance_id=NEW.parent_item_instance_id) THEN
                RAISE EXCEPTION 'boss collection requires a fresh backpack entry' USING ERRCODE='23514';
            END IF;
        ELSIF NOT EXISTS (
            SELECT 1 FROM game_boss_reward_item_collections parent
            WHERE (parent.character_id,parent.world_id,parent.death_id,parent.item_ordinal)
                 =(NEW.character_id,NEW.world_id,NEW.death_id,parent_ordinal)
              AND parent.item_instance_id=NEW.parent_item_instance_id
              AND (parent.game_session_id,parent.command_id,parent.occurred_at)
                 =(NEW.game_session_id,NEW.command_id,NEW.occurred_at)
              AND parent.created_xact_id=pg_current_xact_id()
        ) OR NEW.placement_ordinal<>(
            SELECT count(*) FROM game_boss_reward_items sibling
            WHERE (sibling.character_id,sibling.world_id,sibling.death_id,sibling.parent_item_ordinal)
                 =(NEW.character_id,NEW.world_id,NEW.death_id,parent_ordinal)
              AND sibling.item_ordinal<=NEW.item_ordinal
        ) THEN
            RAISE EXCEPTION 'boss reward child requires its own parent and sibling slot in the same command' USING ERRCODE='23514';
        END IF;
    END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_boss_reward_item_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE c game_boss_reward_item_collections%ROWTYPE;
BEGIN
    SELECT * INTO c FROM game_boss_reward_item_collections WHERE transaction_id=p_transaction;
    IF c.transaction_id IS NULL OR c.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS (SELECT 1 FROM game_boss_reward_collection_receipts r
          WHERE (r.game_session_id,r.command_id,r.character_id,r.world_id)=(c.game_session_id,c.command_id,c.character_id,c.world_id)
            AND r.status=0 AND r.created_xact_id=pg_current_xact_id() AND r.occurred_at=c.occurred_at)
      OR NOT EXISTS (SELECT 1 FROM game_boss_reward_items e JOIN game_item_instances i
            ON i.item_instance_id=c.item_instance_id
          WHERE (e.character_id,e.world_id,e.death_id,e.item_ordinal)=(c.character_id,c.world_id,c.death_id,c.item_ordinal)
            AND e.collected_transaction_id=c.transaction_id AND NOT e.expired
            AND i.world_id=c.world_id AND i.definition_family=e.definition_family
            AND i.definition_production_key=e.definition_production_key AND i.definition_revision_ref=e.definition_revision_ref
            AND i.quantity=e.quantity AND i.lifecycle=1 AND i.minted_transaction_id=c.transaction_id AND i.last_transaction_id IS NULL
            AND ((e.parent_item_ordinal IS NULL AND EXISTS (
                  SELECT 1 FROM game_item_container_entries p
                  WHERE (p.item_instance_id,p.character_id,p.world_id,p.parent_item_instance_id,p.placement_ordinal,p.placed_transaction_id)
                       =(c.item_instance_id,c.character_id,c.world_id,c.parent_item_instance_id,c.placement_ordinal,c.transaction_id)))
              OR (e.parent_item_ordinal IS NOT NULL AND EXISTS (
                  SELECT 1 FROM game_item_nested_entries p
                  WHERE (p.item_instance_id,p.world_id,p.parent_item_instance_id,p.placement_ordinal,p.placed_transaction_id)
                       =(c.item_instance_id,c.world_id,c.parent_item_instance_id,c.placement_ordinal,c.transaction_id)))))
      OR EXISTS (
          SELECT 1 FROM game_boss_reward_items child
          WHERE (child.character_id,child.world_id,child.death_id,child.parent_item_ordinal)
               =(c.character_id,c.world_id,c.death_id,c.item_ordinal)
            AND NOT EXISTS (
                SELECT 1 FROM game_boss_reward_item_collections collected
                WHERE (collected.character_id,collected.world_id,collected.death_id,collected.item_ordinal)
                     =(child.character_id,child.world_id,child.death_id,child.item_ordinal)
                  AND collected.transaction_id=child.collected_transaction_id
                  AND collected.parent_item_instance_id=c.item_instance_id
                  AND (collected.game_session_id,collected.command_id,collected.occurred_at)
                     =(c.game_session_id,c.command_id,c.occurred_at)
                  AND collected.created_xact_id=pg_current_xact_id()))
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
-- Preserve the existing movement and other mint owners on nested custody.
DO $migration$
DECLARE body TEXT; arm TEXT; insert_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
    IF insert_at=0 THEN RAISE EXCEPTION 'unexpected nested Item custody proof body'; END IF;
    arm:=$arm$
    IF TG_OP='INSERT' AND EXISTS (
        SELECT 1 FROM game_boss_reward_item_collections c
        JOIN game_boss_reward_items i USING(character_id,world_id,death_id,item_ordinal)
        WHERE i.parent_item_ordinal IS NOT NULL
          AND (c.item_instance_id,c.world_id,c.parent_item_instance_id,c.placement_ordinal,c.transaction_id)
             =(NEW.item_instance_id,NEW.world_id,NEW.parent_item_instance_id,NEW.placement_ordinal,NEW.placed_transaction_id)
          AND c.created_xact_id=pg_current_xact_id()
    ) THEN
        PERFORM game_boss_reward_item_complete(NEW.placed_transaction_id);
        RETURN NULL;
    END IF;
    $arm$;
    EXECUTE overlay(body placing arm from insert_at for 0);
    EXECUTE format('ALTER FUNCTION game_boss_reward_item_parent_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_boss_reward_collection_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_boss_reward_item_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_boss_reward_item_parent_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_boss_reward_item_parent_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_boss_reward_item_parent_guard() TO oteryn_game_control;
