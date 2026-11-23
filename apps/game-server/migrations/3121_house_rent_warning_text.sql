-- HOUSE-OWN-0 §5. Grace is attached to the unpaid period and ownership
-- generation, independently of the configured paid rental period.
ALTER TABLE game_house_rent_state
 ADD COLUMN IF NOT EXISTS grace_until TIMESTAMPTZ,
 ADD COLUMN IF NOT EXISTS ownership_generation NUMERIC(20,0);

CREATE OR REPLACE FUNCTION game_house_rent_tick(w UUID,n UUID,nr NUMERIC,secret BYTEA,OUT changed BOOLEAN,OUT operation TEXT) RETURNS RECORD
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_properties; s game_house_rent_state; b game_account_bank_balances; entry UUID; payment UUID; letter UUID; letter_tx UUID; deadline TIMESTAMPTZ; opens_grace BOOLEAN; now_ms BIGINT; period INTERVAL;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1 AND state=1
   AND holder_node_id=n AND holder_registration_revision=nr) THEN
  RAISE EXCEPTION 'house rent ingress refused' USING ERRCODE='23514';
 END IF;
 SELECT CASE rent_period WHEN 'daily' THEN interval '1 day' WHEN 'weekly' THEN interval '7 days'
  WHEN 'monthly' THEN interval '30 days' WHEN 'yearly' THEN interval '365 days' END INTO period FROM game_house_rent_config;
 SELECT hp.* INTO p FROM game_house_properties hp
  WHERE hp.world_id=w AND hp.owner_character_id IS NOT NULL
   AND (SELECT lose_after_inactivity_days FROM game_house_rent_config)>0
   AND (SELECT max(ds.training_created_at_us) FROM game_durability_reconnect_sessions ds
        WHERE ds.character_id=hp.owner_character_id) IS NOT NULL
   AND (SELECT max(ds.training_created_at_us) FROM game_durability_reconnect_sessions ds
        WHERE ds.character_id=hp.owner_character_id)
       <=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint
         -(SELECT lose_after_inactivity_days FROM game_house_rent_config)*86400000000::bigint
   AND NOT EXISTS(SELECT 1 FROM game_house_dispositions d WHERE d.world_id=w
        AND d.house_key=hp.house_key AND d.state<>4)
  ORDER BY hp.house_key LIMIT 1 FOR UPDATE OF hp SKIP LOCKED;
 IF FOUND THEN
  now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  BEGIN
   INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause)
    SELECT w,p.house_key,o.owner_character_id,o.ownership_revision,1,now_ms,now_ms,'EVICTION' FROM game_house_ownership o
     WHERE o.world_id=w AND o.house_key=p.house_key RETURNING operation_id::text INTO operation;
   UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=w AND runtime_scope_kind=2
    AND runtime_scope_house_key=p.house_key AND session_state IN(1,2);
  EXCEPTION WHEN unique_violation THEN
   operation:=NULL;
  END;
  changed:=TRUE; RETURN;
 END IF;
 IF period IS NULL THEN changed:=FALSE; RETURN; END IF;
 INSERT INTO game_house_rent_state(world_id,house_key,next_rent_at,ownership_generation)
  SELECT world_id,house_key,paid_until,ownership_generation FROM game_house_properties
   WHERE world_id=w AND owner_character_id IS NOT NULL ON CONFLICT DO NOTHING;
 SELECT hp.* INTO p FROM game_house_properties hp JOIN game_house_rent_state rs USING(world_id,house_key)
  WHERE hp.world_id=w AND hp.owner_character_id IS NOT NULL AND hp.paid_until<=statement_timestamp()
   AND (rs.ownership_generation IS DISTINCT FROM hp.ownership_generation OR rs.next_rent_at<=statement_timestamp())
   AND NOT EXISTS(SELECT 1 FROM game_house_dispositions d WHERE d.world_id=w
        AND d.house_key=hp.house_key AND d.cause='EVICTION' AND d.state<>4)
  ORDER BY rs.next_rent_at,hp.house_key LIMIT 1 FOR UPDATE OF hp,rs SKIP LOCKED;
 IF NOT FOUND THEN changed:=FALSE; RETURN; END IF;
 SELECT * INTO s FROM game_house_rent_state WHERE world_id=w AND house_key=p.house_key FOR UPDATE;
 opens_grace:=(s.ownership_generation,s.grace_until) IS DISTINCT FROM
  (p.ownership_generation,p.paid_until+interval '168 hours');
 deadline:=CASE WHEN opens_grace THEN p.paid_until+interval '168 hours' ELSE s.grace_until END;
 SELECT * INTO b FROM game_account_bank_balances WHERE account_id=p.owner_account_id AND world_id=w FOR UPDATE;
 IF p.rent_gold=0 OR (FOUND AND b.balance>=p.rent_gold) THEN
  IF p.rent_gold>0 THEN
   payment:=game_character_uuid_v7();entry:=game_character_uuid_v7();
   INSERT INTO game_house_rent_payments(payment_id,world_id,house_key,account_id,owner_character_id,amount,bank_entry_id)
    VALUES(payment,w,p.house_key,p.owner_account_id,p.owner_character_id,p.rent_gold,entry);
   INSERT INTO game_account_bank_entries(entry_id,house_rent_payment_id,house_rent_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
    VALUES(entry,payment,'HOUSE_RENT',p.owner_account_id,w,NULL,p.rent_gold,b.balance,b.balance-p.rent_gold,b.last_entry_id,p.owner_character_id);
   UPDATE game_account_bank_balances SET balance=b.balance-p.rent_gold,last_entry_id=entry WHERE account_id=p.owner_account_id AND world_id=w;
  END IF;
  UPDATE game_house_properties SET paid_until=p.paid_until+period,revision=revision+1 WHERE world_id=w AND house_key=p.house_key;
  UPDATE game_house_rent_state SET warnings=0,grace_until=NULL,next_rent_at=p.paid_until+period,
   ownership_generation=p.ownership_generation WHERE world_id=w AND house_key=p.house_key;
  changed:=TRUE; RETURN;
 END IF;
 IF statement_timestamp()<deadline THEN
  UPDATE game_house_rent_state SET warnings=1,grace_until=deadline,
   next_rent_at=least(statement_timestamp()+interval '24 hours',deadline),
   ownership_generation=p.ownership_generation WHERE world_id=w AND house_key=p.house_key;
  IF opens_grace THEN
   -- An old rent warning may already have a receipt for this exact due date.
   INSERT INTO game_house_rent_inbox_letters(letter_id,world_id,house_key,owner_character_id,warning,rent_gold,rent_due_at)
    VALUES(game_character_uuid_v7(),w,p.house_key,p.owner_character_id,1,p.rent_gold,p.paid_until)
    ON CONFLICT (world_id,house_key,warning,rent_due_at) DO NOTHING RETURNING letter_id INTO letter;
   IF letter IS NOT NULL THEN
    letter_tx:=game_character_uuid_v7();
    INSERT INTO game_house_rent_letter_mints VALUES(letter,letter_tx,w,p.owner_character_id);
    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
     VALUES(letter,w,'Item','oteryn:item.tibia.i3506','definition-r1',1,1,letter_tx);
    INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(letter,1);
    INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
     VALUES(letter,format(
      E'Warning! \nThe %s rent of %s gold for your house "%s" is payable. Pay by %s or you will lose this house.',
      CASE period WHEN interval '1 day' THEN 'daily' WHEN interval '7 days' THEN 'weekly'
       WHEN interval '30 days' THEN 'monthly' WHEN interval '365 days' THEN 'annual' END,
      p.rent_gold,
      COALESCE((SELECT c.name FROM game_public_house_catalogue c
                WHERE c.world_id=w AND c.house_key=p.house_key),p.house_key),
      to_char(deadline AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS "UTC"')),NULL,statement_timestamp(),1);
    PERFORM game_character_inbox_deliver(letter,p.owner_character_id,w,'HOUSE_RENT_WARNING',letter::text);
   END IF;
  END IF;
  changed:=TRUE; RETURN;
 END IF;

 -- The bank check above is the final collection attempt. A competing
 -- disposition stays local to this house and can be retried on the next day.
 UPDATE game_house_rent_state SET grace_until=deadline,next_rent_at=statement_timestamp()+interval '24 hours',
  ownership_generation=p.ownership_generation WHERE world_id=w AND house_key=p.house_key;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 BEGIN
  INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause)
   SELECT w,p.house_key,o.owner_character_id,o.ownership_revision,1,now_ms,now_ms,'EVICTION' FROM game_house_ownership o
    WHERE o.world_id=w AND o.house_key=p.house_key RETURNING operation_id::text INTO operation;
  UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=w AND runtime_scope_kind=2
   AND runtime_scope_house_key=p.house_key AND session_state IN(1,2);
 EXCEPTION WHEN unique_violation THEN
  operation:=NULL;
 END;
 changed:=TRUE; RETURN;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;

-- 0941's live Inbox letters have the exact mint and delivery receipts but an
-- obsolete definition revision. The table lock from ALTER TABLE keeps the two
-- item-change guards unavailable only inside this migration transaction.
ALTER TABLE game_item_instances DISABLE TRIGGER game_item_instance_guard;
ALTER TABLE game_item_instances DISABLE TRIGGER game_item_instance_change_proven;
SET CONSTRAINTS ALL IMMEDIATE;
UPDATE game_item_instances i SET definition_revision_ref='definition-r1'
FROM game_house_rent_letter_mints m
JOIN game_house_rent_inbox_letters r
 ON (r.letter_id,r.world_id,r.owner_character_id)=(m.letter_id,m.world_id,m.owner_character_id)
JOIN game_character_inbox_deliveries d
 ON (d.item_instance_id,d.world_id,d.character_id)=(m.letter_id,m.world_id,m.owner_character_id)
 AND d.cause_kind='HOUSE_RENT_WARNING' AND d.cause_ref=m.letter_id::text
 AND d.created_xact_id=m.created_xact_id
JOIN game_item_character_inbox_locations l
 ON (l.item_instance_id,l.world_id)=(d.item_instance_id,d.world_id)
WHERE (i.item_instance_id,i.world_id,i.minted_transaction_id)=(m.letter_id,m.world_id,m.transaction_id)
 AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=('Item','oteryn:item.tibia.i3506','house-rent-warning')
 AND i.lifecycle=1 AND i.quantity=1;
ALTER TABLE game_item_instances ENABLE TRIGGER game_item_instance_change_proven;
ALTER TABLE game_item_instances ENABLE TRIGGER game_item_instance_guard;
