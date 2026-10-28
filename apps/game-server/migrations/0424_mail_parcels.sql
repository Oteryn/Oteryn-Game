-- MAIL2-1: admit a qualified parcel's attached nested tree and account every
-- descendant against the Inbox ceiling.  Nested rows retain their parent
-- links; only the parcel root changes from backpack to Inbox custody.
CREATE OR REPLACE FUNCTION game_mail_posting_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actual_count INTEGER; i game_item_instances%ROWTYPE; e game_item_container_entries%ROWTYPE;
BEGIN
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.root_item_instance_id FOR UPDATE;
    SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id=NEW.root_item_instance_id FOR UPDATE;
    WITH RECURSIVE tree(item_instance_id) AS (
        SELECT NEW.root_item_instance_id
        UNION
        SELECT e.item_instance_id FROM game_item_nested_entries e
          JOIN tree t ON e.parent_item_instance_id=t.item_instance_id
    ) SELECT count(*)::INTEGER INTO actual_count FROM tree;
    IF i.item_instance_id IS NULL OR e.item_instance_id IS NULL
       OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR to_jsonb(e) IS DISTINCT FROM NEW.source_entry
       OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item'
       OR i.world_id<>NEW.world_id OR e.world_id<>NEW.world_id OR e.character_id<>NEW.sender_character_id
       OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.recipient_character_id AND world_id=NEW.world_id AND lifecycle=1)
       OR actual_count<>NEW.item_count
       OR (NEW.stamped_production_key='oteryn:item.tibia.i3506' AND actual_count<>1)
       OR (NEW.stamped_production_key='oteryn:item.tibia.i3504' AND actual_count NOT BETWEEN 2 AND 499)
       OR NEW.stamped_production_key NOT IN ('oteryn:item.tibia.i3504','oteryn:item.tibia.i3506')
    THEN RAISE EXCEPTION 'mail tree no longer matches qualified post' USING ERRCODE='23514'; END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    NEW.occurred_at:=clock_timestamp();
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_mail_posting_apply(
    p_mail UUID, p_session UUID, p_command NUMERIC, p_world UUID, p_channel UUID,
    p_sender UUID, p_recipient UUID, p_item UUID, p_stamp TEXT, p_mailbox BYTEA,
    p_hash BYTEA, p_transaction UUID, p_event BYTEA, p_before JSONB, p_source JSONB
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_character_inbox_counters%ROWTYPE; affected BIGINT; tree_count INTEGER; max_depth INTEGER; direct_count INTEGER;
BEGIN
    PERFORM 1 FROM game_item_instances WHERE item_instance_id=p_item FOR UPDATE;
    PERFORM 1 FROM game_item_container_entries WHERE item_instance_id=p_item FOR UPDATE;
    WITH RECURSIVE tree(item_instance_id,depth) AS (
        SELECT p_item,0
        UNION
        SELECT e.item_instance_id,t.depth+1 FROM game_item_nested_entries e JOIN tree t ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8
    ) SELECT count(*)::INTEGER,max(depth)::INTEGER INTO tree_count,max_depth FROM tree;
    SELECT count(*)::INTEGER INTO direct_count FROM game_item_nested_entries WHERE parent_item_instance_id=p_item;
    IF tree_count>499 OR max_depth>7 OR direct_count>10
       OR (p_stamp='oteryn:item.tibia.i3506' AND tree_count<>1)
       OR (p_stamp='oteryn:item.tibia.i3504' AND tree_count<2)
       OR p_stamp NOT IN ('oteryn:item.tibia.i3504','oteryn:item.tibia.i3506')
    THEN RAISE EXCEPTION 'unqualified mail tree' USING ERRCODE='23514'; END IF;
    INSERT INTO game_character_inbox_counters(character_id) VALUES(p_recipient) ON CONFLICT DO NOTHING;
    SELECT * INTO STRICT c FROM game_character_inbox_counters WHERE character_id=p_recipient FOR UPDATE;
    IF c.committed + COALESCE((SELECT sum(o.item_count-1) FROM game_mail_operations o JOIN game_item_character_inbox_locations l ON l.item_instance_id=o.root_item_instance_id AND l.character_id=p_recipient WHERE o.recipient_character_id=p_recipient),0) + tree_count > 50000 THEN RETURN FALSE; END IF;
    INSERT INTO game_mail_operations(mail_id,game_session_id,command_id,world_id,channel_id,
        sender_character_id,recipient_character_id,root_item_instance_id,mailbox_placement_key,
        item_count,request_sha256,transaction_id,before_item,source_entry,stamped_production_key)
    VALUES(p_mail,p_session,p_command,p_world,p_channel,p_sender,p_recipient,p_item,p_mailbox,
        tree_count,p_hash,p_transaction,p_before,p_source,p_stamp);
    UPDATE game_item_instances SET definition_production_key=p_stamp WHERE item_instance_id=p_item AND world_id=p_world;
    GET DIAGNOSTICS affected=ROW_COUNT;
    IF affected<>1 THEN RAISE EXCEPTION 'mail stamp did not update exactly one item' USING ERRCODE='OTM02'; END IF;
    DELETE FROM game_item_container_entries WHERE item_instance_id=p_item AND character_id=p_sender;
    GET DIAGNOSTICS affected=ROW_COUNT;
    IF affected<>1 THEN RAISE EXCEPTION 'mail source removal did not remove exactly one entry' USING ERRCODE='OTM02'; END IF;
    INSERT INTO game_character_inbox_deliveries(item_instance_id,cause_kind,cause_ref,world_id,character_id,ordinal)
        VALUES(p_item,'MAIL_POST',replace(p_mail::text,'-',''),p_world,p_recipient,c.next_ordinal);
    INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal)
        VALUES(p_item,p_world,p_recipient,c.next_ordinal);
    UPDATE game_character_inbox_counters SET next_ordinal=next_ordinal+1 WHERE character_id=p_recipient;
    INSERT INTO game_mail_outbox(mail_id,event_type,event_payload) VALUES(p_mail,'MAIL_POSTED',p_event);
    RETURN TRUE;
END $$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_mail_posting_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_mail_posting_apply(UUID,UUID,NUMERIC,UUID,UUID,UUID,UUID,UUID,TEXT,BYTEA,BYTEA,UUID,BYTEA,JSONB,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
