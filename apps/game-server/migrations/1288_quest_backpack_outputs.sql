-- Bounded additional backpack outputs reuse the existing MINT reservation,
-- complete-plan proof, one-item audit events and once-claim transaction.
ALTER TABLE game_reward_claim_mint_reservations
    ADD COLUMN backpack_outputs BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT game_reward_reserved_backpack_outputs_shape CHECK (
        NOT backpack_outputs OR (contents_capacity IS NOT NULL AND root_text IS NULL
            AND jsonb_array_length(contents_plan) BETWEEN 1 AND 19
            AND jsonb_array_length(contents_plan)+1<=contents_capacity));
ALTER TABLE game_reward_claim_mint_receipts
    ADD COLUMN backpack_outputs BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT game_reward_committed_backpack_outputs_shape CHECK (
        NOT backpack_outputs OR (contents_capacity IS NOT NULL AND root_text IS NULL
            AND jsonb_array_length(contents_plan) BETWEEN 1 AND 19
            AND jsonb_array_length(contents_plan)+1<=contents_capacity));

DO $migration$
DECLARE body TEXT;
BEGIN
    body:=pg_get_functiondef('game_reward_claim_container_complete(UUID)'::regprocedure);
    IF strpos(body,$old$    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id = r.game_session_id AND command_id = r.command_id;
    IF NOT FOUND OR v.contents_plan IS DISTINCT FROM r.contents_plan
       OR v.contents_capacity IS DISTINCT FROM r.contents_capacity
       OR v.root_text IS DISTINCT FROM r.root_text
$old$)=0 THEN RAISE EXCEPTION 'unexpected installed MINT contents proof'; END IF;
    body:=replace(body,$old$    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id = r.game_session_id AND command_id = r.command_id;
    IF NOT FOUND OR v.contents_plan IS DISTINCT FROM r.contents_plan
       OR v.contents_capacity IS DISTINCT FROM r.contents_capacity
       OR v.root_text IS DISTINCT FROM r.root_text
$old$,$new$    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id = r.game_session_id AND command_id = r.command_id;
    IF NOT FOUND OR v.backpack_outputs IS DISTINCT FROM r.backpack_outputs
       OR v.contents_plan IS DISTINCT FROM r.contents_plan
       OR v.contents_capacity IS DISTINCT FROM r.contents_capacity
       OR v.root_text IS DISTINCT FROM r.root_text
$new$);
    IF strpos(body,$old$       OR NOT game_reward_claim_contents_plan_valid(r.contents_plan,r.contents_capacity)
       OR (SELECT count(*) FROM game_item_nested_entries
            WHERE parent_item_instance_id = r.item_instance_id) <> jsonb_array_length(r.contents_plan)
    THEN RETURN FALSE; END IF;
    IF r.root_text IS NULL AND (
$old$)=0 THEN RAISE EXCEPTION 'unexpected installed MINT contents proof'; END IF;
    body:=replace(body,$old$       OR NOT game_reward_claim_contents_plan_valid(r.contents_plan,r.contents_capacity)
       OR (SELECT count(*) FROM game_item_nested_entries
            WHERE parent_item_instance_id = r.item_instance_id) <> jsonb_array_length(r.contents_plan)
    THEN RETURN FALSE; END IF;
    IF r.root_text IS NULL AND (
$old$,$new$       OR NOT game_reward_claim_contents_plan_valid(r.contents_plan,r.contents_capacity)
       OR (SELECT count(*) FROM game_item_nested_entries
            WHERE parent_item_instance_id = r.item_instance_id) <>
          (CASE WHEN r.backpack_outputs THEN 0 ELSE jsonb_array_length(r.contents_plan) END)
    THEN RETURN FALSE; END IF;
    IF r.root_text IS NULL AND (
$new$);
    IF strpos(body,$old$    IF r.contents_capacity IS NULL AND r.root_text IS NULL THEN RETURN TRUE; END IF;
    SELECT * INTO root_item FROM game_item_instances WHERE item_instance_id = r.item_instance_id;
    IF NOT FOUND OR root_item.quantity <> 1 OR r.quantity <> 1
       OR root_item.world_id <> v.world_id OR root_item.lifecycle <> 1
       OR root_item.state_revision <> 1 OR root_item.last_transaction_id IS NOT NULL
$old$)=0 THEN RAISE EXCEPTION 'unexpected installed MINT contents proof'; END IF;
    body:=replace(body,$old$    IF r.contents_capacity IS NULL AND r.root_text IS NULL THEN RETURN TRUE; END IF;
    SELECT * INTO root_item FROM game_item_instances WHERE item_instance_id = r.item_instance_id;
    IF NOT FOUND OR root_item.quantity <> 1 OR r.quantity <> 1
       OR root_item.world_id <> v.world_id OR root_item.lifecycle <> 1
       OR root_item.state_revision <> 1 OR root_item.last_transaction_id IS NOT NULL
$old$,$new$    IF r.contents_capacity IS NULL AND r.root_text IS NULL THEN RETURN TRUE; END IF;
    SELECT * INTO root_item FROM game_item_instances WHERE item_instance_id = r.item_instance_id;
    IF NOT FOUND OR root_item.quantity <> r.quantity
       OR (NOT r.backpack_outputs AND (root_item.quantity <> 1 OR r.quantity <> 1))
       OR root_item.world_id <> v.world_id OR root_item.lifecycle <> 1
       OR root_item.state_revision <> 1 OR root_item.last_transaction_id IS NOT NULL
$new$);
    IF strpos(body,$old$           OR NOT EXISTS (
               SELECT 1 FROM game_item_instances i
                 JOIN game_item_nested_entries e USING(item_instance_id)
                 JOIN game_item_audit_outbox a ON a.item_instance_id = i.item_instance_id
                WHERE i.item_instance_id = child AND i.minted_transaction_id = minted
$old$)=0 THEN RAISE EXCEPTION 'unexpected installed MINT contents proof'; END IF;
    body:=replace(body,$old$           OR NOT EXISTS (
               SELECT 1 FROM game_item_instances i
                 JOIN game_item_nested_entries e USING(item_instance_id)
                 JOIN game_item_audit_outbox a ON a.item_instance_id = i.item_instance_id
                WHERE i.item_instance_id = child AND i.minted_transaction_id = minted
$old$,$new$           OR NOT EXISTS (
               SELECT 1 FROM game_item_instances i
                 LEFT JOIN game_item_nested_entries e USING(item_instance_id)
                 LEFT JOIN game_item_container_entries f USING(item_instance_id)
                 JOIN game_item_audit_outbox a ON a.item_instance_id = i.item_instance_id
                WHERE i.item_instance_id = child AND i.minted_transaction_id = minted
$new$);
    IF strpos(body,$old$                      = (entry->>'family',entry->>'production_key',entry->>'revision_ref',
                         (entry->>'quantity')::BIGINT)
                  AND e.world_id = v.world_id AND e.parent_item_instance_id = r.item_instance_id
                  AND e.placement_ordinal = (entry->>'ordinal')::NUMERIC
                  AND e.placed_transaction_id = minted
                  AND a.event_id = event AND a.transaction_id = minted
                  AND a.transaction_ordinal = 1 AND a.transaction_count = 1
$old$)=0 THEN RAISE EXCEPTION 'unexpected installed MINT contents proof'; END IF;
    body:=replace(body,$old$                      = (entry->>'family',entry->>'production_key',entry->>'revision_ref',
                         (entry->>'quantity')::BIGINT)
                  AND e.world_id = v.world_id AND e.parent_item_instance_id = r.item_instance_id
                  AND e.placement_ordinal = (entry->>'ordinal')::NUMERIC
                  AND e.placed_transaction_id = minted
                  AND a.event_id = event AND a.transaction_id = minted
                  AND a.transaction_ordinal = 1 AND a.transaction_count = 1
$old$,$new$                      = (entry->>'family',entry->>'production_key',entry->>'revision_ref',
                         (entry->>'quantity')::BIGINT)
                  AND ((NOT r.backpack_outputs AND e.world_id=v.world_id
                        AND e.parent_item_instance_id=r.item_instance_id
                        AND e.placement_ordinal=(entry->>'ordinal')::NUMERIC
                        AND e.placed_transaction_id=minted)
                    OR (r.backpack_outputs AND f.world_id=v.world_id
                        AND f.character_id=r.character_id
                        AND f.parent_item_instance_id=r.destination_parent_item_instance_id
                        AND f.placement_ordinal=r.destination_ordinal+(entry->>'ordinal')::NUMERIC
                        AND f.placed_transaction_id=minted))
                  AND a.event_id = event AND a.transaction_id = minted
                  AND a.transaction_ordinal = 1 AND a.transaction_count = 1
$new$);
    EXECUTE body;
END
$migration$;
DO $migration$
DECLARE body TEXT; insert_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    insert_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
    IF insert_at=0 THEN RAISE EXCEPTION 'unexpected installed quest output proof'; END IF;
    EXECUTE overlay(body placing $arm$
    IF NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts m
        WHERE m.transaction_id=NEW.mint_transaction_id
          AND m.backpack_outputs=(jsonb_array_length(COALESCE(
                NEW.plan #> '{give,backpack_outputs}','[]'::jsonb))>0)
          AND (SELECT COALESCE(jsonb_agg(jsonb_build_object(
                'item',e->>'production_key','count',(e->>'quantity')::bigint)
                ORDER BY (e->>'ordinal')::integer),'[]'::jsonb)
               FROM jsonb_array_elements(m.contents_plan) e)
            = COALESCE(CASE WHEN m.backpack_outputs
                THEN NEW.plan #> '{give,backpack_outputs}'
                ELSE NEW.plan #> '{give,contents}' END,'[]'::jsonb)
    ) THEN
        RAISE EXCEPTION 'quest reward must mint every exact authored output'
            USING ERRCODE='23514';
    END IF;
$arm$ from insert_at for 0);
END
$migration$;

-- Preserve all current custody owners; only the complete reserved child plan
-- can authorize a newly minted additional direct backpack entry.
DO $migration$
DECLARE body TEXT; insert_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_item_placement_proven()'::regprocedure);
insert_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
    IF insert_at=0 THEN RAISE EXCEPTION 'unexpected backpack output proof'; END IF;
    EXECUTE overlay(body placing $arm$
    IF TG_OP='INSERT' AND TG_TABLE_NAME='game_item_container_entries'
       AND game_reward_claim_container_item_proven(NEW.item_instance_id,NEW.placed_transaction_id)
    THEN RETURN NULL; END IF;
$arm$ from insert_at for 0);
END
$migration$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_reward_claim_container_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
