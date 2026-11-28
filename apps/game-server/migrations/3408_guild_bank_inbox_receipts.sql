-- Each committed player attempt delivers one paper to its acting Character.
-- The operation insert is already covered by the native writer's generation,
-- replay and value proofs. There is no independent runtime delivery entrypoint.
INSERT INTO game_character_inbox_cause_kinds VALUES('GUILD_BANK_RECEIPT');

ALTER TABLE game_guild_bank_operations ADD COLUMN transfer_recipient_name TEXT
 CHECK(transfer_recipient_name IS NULL OR
  (kind='GUILD_TRANSFER' AND char_length(transfer_recipient_name) BETWEEN 1 AND 29));

CREATE TABLE game_guild_bank_inbox_receipts (
 operation_id UUID PRIMARY KEY REFERENCES game_guild_bank_operations,
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_guild_bank_inbox_receipts FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_guild_bank_inbox_receipts_immutable
 BEFORE UPDATE OR DELETE ON game_guild_bank_inbox_receipts FOR EACH ROW
 EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_bank_inbox_receipts_no_truncate
 BEFORE TRUNCATE ON game_guild_bank_inbox_receipts FOR EACH STATEMENT
 EXECUTE FUNCTION game_guild_rows_immutable();

-- Match Market's counter -> Account balance lock order. Only an admitted
-- Character can acquire this counter through the runtime role.
CREATE FUNCTION game_guild_bank_lock_receipt_inbox(c UUID,w UUID,s UUID,g NUMERIC) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions d
    JOIN game_character_roots r USING(character_id,world_id)
    WHERE d.game_session_id=s AND d.character_id=c AND d.world_id=w
     AND d.session_state IN(1,2) AND d.current_generation=g AND d.account_id=r.account_id AND r.lifecycle=1) THEN
  RAISE EXCEPTION 'Guild Bank receipt Inbox requires an admitted Character' USING ERRCODE='23514';
 END IF;
 INSERT INTO game_character_inbox_counters(character_id) VALUES(c) ON CONFLICT DO NOTHING;
 PERFORM 1 FROM game_character_inbox_counters WHERE character_id=c FOR UPDATE;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_guild_bank_lock_receipt_inbox(UUID,UUID,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_guild_bank_lock_receipt_inbox(UUID,UUID,UUID,NUMERIC) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_guild_bank_lock_receipt_inbox(UUID,UUID,UUID,NUMERIC) TO oteryn_game_runtime;

CREATE FUNCTION game_guild_bank_deliver_receipt() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE item UUID:=game_character_uuid_v7();
 actor_name TEXT; guild_name TEXT; recipient TEXT; operation_label TEXT; outcome TEXT;
 at_time TIMESTAMPTZ:=to_timestamp(NEW.occurred_at::double precision/1000.0);
BEGIN
 SELECT r.name INTO STRICT actor_name FROM game_character_roots r
 JOIN game_durability_reconnect_sessions d USING(character_id,account_id,world_id)
 WHERE r.character_id=NEW.actor AND r.account_id=NEW.account_id AND r.world_id=NEW.world_id
  AND r.lifecycle=1 AND d.game_session_id=NEW.game_session_id AND d.session_state IN(1,2);
 SELECT name INTO STRICT guild_name FROM game_guilds WHERE guild_id=NEW.guild_id;
 CASE NEW.kind
  WHEN 'GUILD_DEPOSIT' THEN operation_label:='Guild deposit'; recipient:=guild_name;
  WHEN 'GUILD_WITHDRAW' THEN operation_label:='Guild withdrawal'; recipient:=actor_name;
  WHEN 'GUILD_TRANSFER' THEN
   operation_label:='Guild transfer';
   SELECT name INTO recipient FROM game_guilds WHERE guild_id=NEW.transfer_target;
   recipient:=coalesce(recipient,NEW.transfer_recipient_name);
  ELSE RAISE EXCEPTION 'Unsupported Guild Bank receipt operation' USING ERRCODE='23514';
 END CASE;
 outcome:=CASE NEW.result
  WHEN 1 THEN 'Completed'
  WHEN 2 THEN 'Transfer not permitted'
  WHEN 3 THEN 'Guild account unavailable'
  WHEN 4 THEN 'Insufficient gold'
  WHEN 5 THEN 'Recipient balance limit'
  WHEN 6 THEN 'Bank service unavailable to this character'
  ELSE 'Declined' END;
 INSERT INTO game_guild_bank_inbox_receipts(operation_id,item_instance_id) VALUES(NEW.operation_id,item);
 INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,
  quantity,lifecycle,minted_transaction_id)
 VALUES(item,NEW.world_id,'Item',CASE WHEN NEW.result=1 THEN 'oteryn:item.tibia.i21932' ELSE 'oteryn:item.tibia.i21933' END,
  'definition-r1',1,1,NEW.operation_id);
 INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(item,1);
 INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
 VALUES(item,concat_ws(E'\n',
  to_char(at_time AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS')||' UTC',
  operation_label,
  'Gold: '||NEW.amount,
  'Requested by: '||actor_name||' ('||guild_name||')',
  'Recipient: '||recipient,
  'Result: '||outcome),NULL,at_time,1);
 PERFORM game_character_inbox_deliver(item,NEW.actor,NEW.world_id,'GUILD_BANK_RECEIPT',NEW.operation_id::text);
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_guild_bank_deliver_receipt() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_guild_bank_deliver_receipt() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_guild_bank_deliver_receipt AFTER INSERT ON game_guild_bank_operations
 FOR EACH ROW WHEN(NEW.kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_TRANSFER'))
 EXECUTE FUNCTION game_guild_bank_deliver_receipt();

-- Extend the installed mint proof without resetting the other Item owners'
-- proof branches. Only the migration-owned delivery trigger can write this
-- immutable receipt row, and it must prove the same transaction and Inbox.
CREATE FUNCTION game_guild_bank_receipt_mint_proven(i game_item_instances) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(
  SELECT 1 FROM game_guild_bank_inbox_receipts r
  JOIN game_guild_bank_operations o USING(operation_id)
  JOIN game_item_character_inbox_locations l ON l.item_instance_id=r.item_instance_id
  JOIN game_character_inbox_deliveries d ON d.item_instance_id=r.item_instance_id
  JOIN game_item_texts t ON t.item_instance_id=r.item_instance_id
  WHERE r.item_instance_id=i.item_instance_id AND r.created_xact_id=pg_current_xact_id()
   AND o.created_xact_id=r.created_xact_id AND o.operation_id=i.minted_transaction_id
   AND (l.character_id,l.world_id)=(o.actor,o.world_id) AND l.world_id=i.world_id
   AND (d.character_id,d.world_id,d.ordinal)=(l.character_id,l.world_id,l.ordinal)
   AND d.cause_kind='GUILD_BANK_RECEIPT' AND d.cause_ref=o.operation_id::text
   AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
   AND i.definition_production_key=CASE WHEN o.result=1 THEN 'oteryn:item.tibia.i21932' ELSE 'oteryn:item.tibia.i21933' END
   AND i.quantity=1 AND i.lifecycle=1 AND t.writer_character_id IS NULL AND t.text_revision=1
 )
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_guild_bank_receipt_mint_proven(game_item_instances) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_guild_bank_receipt_mint_proven(game_item_instances) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_guild_bank_receipt_mint_proven(game_item_instances) TO oteryn_game_runtime,oteryn_game_control;

DO $$ DECLARE body TEXT; updated TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_mint_consistency_guard()'::regprocedure) INTO STRICT body;
 updated:=regexp_replace(body,'BEGIN',$proof$BEGIN
  IF game_guild_bank_receipt_mint_proven(NEW) THEN RETURN NEW; END IF;$proof$);
 IF updated=body THEN RAISE EXCEPTION 'missing Item mint proof'; END IF;
 EXECUTE updated;
END $$;
