-- HOUSE-OWN-0 §5. Grace is attached to the unpaid period and ownership
-- generation, independently of the configured paid rental period.
ALTER TABLE game_house_rent_state
 ADD COLUMN IF NOT EXISTS grace_until TIMESTAMPTZ,
 ADD COLUMN IF NOT EXISTS ownership_generation NUMERIC(20,0),
 ADD COLUMN IF NOT EXISTS notice_due_at TIMESTAMPTZ,
 ADD COLUMN IF NOT EXISTS notice_letter_id UUID REFERENCES game_house_rent_inbox_letters(letter_id);
-- Only the fenced tick can record a readable notice for a live tenure.
REVOKE INSERT,UPDATE ON game_house_rent_state FROM oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_house_rent_warning_body(
 w UUID,h TEXT,rent BIGINT,rent_period TEXT,deadline TIMESTAMPTZ
) RETURNS TEXT LANGUAGE sql STABLE AS $$
 SELECT format(
  E'Warning! \nThe %srent of %s gold for your house "%s" is payable. Pay by %s or you will lose this house.',
  CASE rent_period WHEN 'yearly' THEN 'annual ' WHEN 'never' THEN '' ELSE rent_period||' ' END,
  rent,coalesce((SELECT c.name FROM game_public_house_catalogue c
                 WHERE c.world_id=w AND c.house_key=h),h),
  to_char(deadline AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS "UTC"'))
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_rent_warning_body(UUID,TEXT,BIGINT,TEXT,TIMESTAMPTZ) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_rent_warning_body(UUID,TEXT,BIGINT,TEXT,TIMESTAMPTZ)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- The caller audits both mutations when a pending notice is superseded.
DROP FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA);

CREATE FUNCTION game_house_rent_tick(w UUID,n UUID,nr NUMERIC,secret BYTEA,OUT changed BOOLEAN,OUT operation TEXT,OUT cancelled_operation TEXT) RETURNS RECORD
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 p game_house_properties;
 s game_house_rent_state;
 b game_account_bank_balances;
 competing game_house_dispositions;
 entry UUID;
 payment UUID;
 letter UUID;
 letter_tx UUID;
 deadline TIMESTAMPTZ;
 has_notice BOOLEAN;
 now_ms BIGINT;
 period INTERVAL;
 rent_label TEXT;
 warning_number SMALLINT;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1 AND state=1
   AND holder_node_id=n AND holder_registration_revision=nr) THEN
  RAISE EXCEPTION 'house rent ingress refused' USING ERRCODE='23514';
 END IF;
 SELECT rent_period,CASE rent_period WHEN 'daily' THEN interval '1 day' WHEN 'weekly' THEN interval '7 days'
  WHEN 'monthly' THEN interval '30 days' WHEN 'yearly' THEN interval '365 days' END
  INTO rent_label,period FROM game_house_rent_config;
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
        AND d.house_key=hp.house_key AND d.state<>4 AND d.released_at IS NULL
        AND d.ownership_revision=hp.ownership_generation AND d.owner_character_id=hp.owner_character_id)
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
        AND d.house_key=hp.house_key AND d.cause='EVICTION' AND d.state<>4 AND d.released_at IS NULL
        AND d.ownership_revision=hp.ownership_generation AND d.owner_character_id=hp.owner_character_id)
  ORDER BY rs.next_rent_at,hp.house_key LIMIT 1 FOR UPDATE OF hp,rs SKIP LOCKED;
 IF NOT FOUND THEN changed:=FALSE; RETURN; END IF;
 SELECT * INTO s FROM game_house_rent_state WHERE world_id=w AND house_key=p.house_key FOR UPDATE;
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
  UPDATE game_house_rent_state SET warnings=0,grace_until=NULL,notice_due_at=NULL,notice_letter_id=NULL,
   next_rent_at=p.paid_until+period,
   ownership_generation=p.ownership_generation WHERE world_id=w AND house_key=p.house_key;
  changed:=TRUE; RETURN;
 END IF;
 -- A recorded notice survives the owner claiming, moving or destroying the
 -- letter. The current tenure and original unpaid date must still match.
 has_notice:=s.ownership_generation IS NOT DISTINCT FROM p.ownership_generation
  AND s.notice_due_at IS NOT DISTINCT FROM p.paid_until AND s.grace_until IS NOT NULL
  AND EXISTS(SELECT 1 FROM game_house_rent_inbox_letters l WHERE l.letter_id=s.notice_letter_id
       AND (l.world_id,l.house_key,l.owner_character_id)=(w,p.house_key,p.owner_character_id));
 IF NOT has_notice THEN
  deadline:=p.paid_until+interval '168 hours';
  -- An outage can consume the whole original grace window before delivery.
  IF deadline<=statement_timestamp() THEN deadline:=statement_timestamp()+interval '168 hours'; END IF;
  -- A historical empty or retired letter is not a readable notice. Keep its
  -- receipt and use another available ordinal if it recorded this exact date.
  SELECT slot.n INTO STRICT warning_number FROM generate_series(1,7) AS slot(n)
   WHERE NOT EXISTS(SELECT 1 FROM game_house_rent_inbox_letters l
    WHERE (l.world_id,l.house_key,l.rent_due_at,l.warning)=(w,p.house_key,p.paid_until,slot.n))
   ORDER BY slot.n LIMIT 1;
  letter:=game_character_uuid_v7();letter_tx:=game_character_uuid_v7();
  INSERT INTO game_house_rent_inbox_letters(letter_id,world_id,house_key,owner_character_id,warning,rent_gold,rent_due_at)
   VALUES(letter,w,p.house_key,p.owner_character_id,warning_number,p.rent_gold,p.paid_until);
  INSERT INTO game_house_rent_letter_mints VALUES(letter,letter_tx,w,p.owner_character_id);
  INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
   VALUES(letter,w,'Item','oteryn:item.tibia.i3506','definition-r1',1,1,letter_tx);
  INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(letter,1);
  INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
   VALUES(letter,game_house_rent_warning_body(w,p.house_key,p.rent_gold,rent_label,deadline),NULL,statement_timestamp(),1);
  PERFORM game_character_inbox_deliver(letter,p.owner_character_id,w,'HOUSE_RENT_WARNING',letter::text);
  UPDATE game_house_rent_state SET warnings=1,grace_until=deadline,
   notice_due_at=p.paid_until,notice_letter_id=letter,
   next_rent_at=least(statement_timestamp()+interval '24 hours',deadline),
   ownership_generation=p.ownership_generation WHERE world_id=w AND house_key=p.house_key;
  changed:=TRUE; RETURN;
 END IF;

 deadline:=s.grace_until;
 IF statement_timestamp()<deadline THEN
  UPDATE game_house_rent_state SET next_rent_at=least(statement_timestamp()+interval '24 hours',deadline)
   WHERE world_id=w AND house_key=p.house_key;
  changed:=TRUE; RETURN;
 END IF;

 -- The bank check is the final collection attempt. The property is locked.
 -- The disposition worker and the move-out producer can acquire the
 -- ownership row before the property. Avoid waiting in reverse lock order.
 PERFORM 1 FROM game_house_ownership o
  WHERE (o.world_id,o.house_key,o.owner_character_id,o.ownership_revision)=
   (w,p.house_key,p.owner_character_id,p.ownership_generation)
  FOR UPDATE NOWAIT;
 IF NOT FOUND THEN RAISE EXCEPTION 'house rent lost its ownership anchor' USING ERRCODE='23514'; END IF;
 SELECT * INTO competing FROM game_house_dispositions d WHERE d.world_id=w
  AND d.house_key=p.house_key AND d.state<>4 AND d.released_at IS NULL FOR UPDATE NOWAIT;
 UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()+interval '24 hours'
  WHERE world_id=w AND house_key=p.house_key;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF competing.operation_id IS NOT NULL THEN
  -- An accepted sale uses a MOVE_OUT disposition but has its own escrow owner.
  IF competing.state<>1 OR competing.cause<>'MOVE_OUT'
   OR (competing.owner_character_id,competing.ownership_revision) IS DISTINCT FROM
      (p.owner_character_id,p.ownership_generation)
   OR deadline>=to_timestamp(competing.due_at::numeric/1000)
   OR EXISTS(SELECT 1 FROM game_house_sale_offers o WHERE o.disposition=competing.operation_id AND o.state=2)
  THEN changed:=TRUE; RETURN; END IF;
  UPDATE game_house_dispositions SET state=4,cancelled_at=now_ms
   WHERE operation_id=competing.operation_id RETURNING operation_id::text INTO cancelled_operation;
 END IF;
 INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause)
  SELECT w,p.house_key,o.owner_character_id,o.ownership_revision,1,now_ms,now_ms,'EVICTION'
  FROM game_house_ownership o WHERE (o.world_id,o.house_key,o.owner_character_id,o.ownership_revision)=
   (w,p.house_key,p.owner_character_id,p.ownership_generation)
  RETURNING operation_id::text INTO STRICT operation;
 UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=w AND runtime_scope_kind=2
  AND runtime_scope_house_key=p.house_key AND session_state IN(1,2);
 changed:=TRUE; RETURN;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;

-- Arbitrate under the disposition worker's ownership and operation locks.
-- The rent worker performs the final bank collection and audits both changes
-- if it replaces the move-out. A paid house can proceed on the next pass.
DO $patch$
DECLARE
 body TEXT;
 freeze_seam CONSTANT TEXT:='PERFORM game_house_disposition_fence_check(o.world_id,o.house_key,NULL,FALSE);';
 freeze_guard CONSTANT TEXT:=$guard$
  IF o.cause='MOVE_OUT' AND o.owner_guild_id IS NULL
   AND EXISTS(SELECT 1 FROM game_house_rent_config WHERE rent_period<>'never')
   AND NOT EXISTS(SELECT 1 FROM game_house_sale_offers sale WHERE sale.disposition=op AND sale.state=2) THEN
   -- Rent and acquisition pin this row before changing the unpaid period.
   -- Avoid waiting in reverse order when a property writer holds it.
   PERFORM 1 FROM game_house_properties p WHERE (p.world_id,p.house_key)=(o.world_id,o.house_key)
    FOR SHARE NOWAIT;
   IF FOUND AND EXISTS(
    SELECT 1 FROM game_house_properties p JOIN game_house_rent_state s USING(world_id,house_key)
    JOIN game_house_rent_inbox_letters l ON l.letter_id=s.notice_letter_id
    WHERE (p.world_id,p.house_key,p.owner_character_id,p.ownership_generation)=
          (o.world_id,o.house_key,o.owner_character_id,o.ownership_revision)
     AND s.ownership_generation=p.ownership_generation AND s.notice_due_at=p.paid_until
     AND (l.world_id,l.house_key,l.owner_character_id)=(p.world_id,p.house_key,p.owner_character_id)
     AND s.grace_until<to_timestamp(o.due_at::numeric/1000)
   ) THEN RETURN 0; END IF;
  END IF;
  $guard$;
 release_seam CONSTANT TEXT:='DELETE FROM game_account_housing_slots WHERE account_id=p.owner_account_id AND world_id=d.world_id;';
 eviction_ban CONSTANT TEXT:=$ban$
 IF d.cause='EVICTION' THEN
  INSERT INTO game_account_house_bans AS b(account_id,world_id,banned_until)
   VALUES(p.owner_account_id,d.world_id,clock_timestamp()+interval '30 days')
   ON CONFLICT(account_id,world_id) DO UPDATE
   SET banned_until=greatest(b.banned_until,EXCLUDED.banned_until);
 END IF;
 $ban$;
BEGIN
 body:=pg_get_functiondef('game_house_disposition_advance(UUID,INT,INT,INT)'::regprocedure);
 IF strpos(body,freeze_guard)=0 THEN
  IF (length(body)-length(replace(body,freeze_seam,'')))/length(freeze_seam)<>1 THEN
   RAISE EXCEPTION 'house disposition freeze seam changed';
  END IF;
  EXECUTE replace(body,freeze_seam,freeze_guard||freeze_seam);
 END IF;
 -- The same custody and audit proof that releases the slot admits the ban.
 body:=pg_get_functiondef('game_house_move_out_release(UUID)'::regprocedure);
 IF strpos(body,eviction_ban)=0 THEN
  IF (length(body)-length(replace(body,release_seam,'')))/length(release_seam)<>1 THEN
   RAISE EXCEPTION 'house disposition release seam changed';
  END IF;
  EXECUTE replace(body,release_seam,eviction_ban||release_seam);
 END IF;
END $patch$;

-- 0941's live Inbox letters have the exact mint and delivery receipts but an
-- obsolete definition revision. The table lock from ALTER TABLE keeps the two
-- item-change guards unavailable only inside this migration transaction.
ALTER TABLE game_item_instances DISABLE TRIGGER game_item_instance_guard;
ALTER TABLE game_item_instances DISABLE TRIGGER game_item_instance_change_proven;
SET CONSTRAINTS ALL IMMEDIATE;
-- Normalize every proven live Inbox letter. Render a deadline for each
-- active legacy warning still in its owner's Inbox and bind one of those
-- receipts to the original paid_until. A retained retry date can be on
-- either side of paid_until, so match the actual delivery to the unpaid
-- period. Open a seven-day window only if eviction has not already started.
WITH normalized AS (
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
 AND i.lifecycle=1 AND i.quantity=1
 RETURNING r.letter_id,r.world_id,r.house_key,r.owner_character_id,
           r.delivered_at,r.rent_gold,l.character_id AS inbox_owner
), active AS (
 SELECT l.*,p.paid_until,p.ownership_generation,c.rent_period,
        statement_timestamp()+interval '168 hours' AS deadline
 FROM normalized l JOIN game_house_properties p USING(world_id,house_key)
 JOIN game_house_rent_state s USING(world_id,house_key)
 CROSS JOIN game_house_rent_config c
 WHERE l.inbox_owner=l.owner_character_id AND p.owner_character_id=l.owner_character_id
  AND p.paid_until<=statement_timestamp() AND s.warnings>0
  AND l.delivered_at>=p.paid_until
  AND NOT EXISTS(SELECT 1 FROM game_house_dispositions d
   WHERE (d.world_id,d.house_key,d.owner_character_id,d.ownership_revision)=
         (p.world_id,p.house_key,p.owner_character_id,p.ownership_generation)
    AND d.cause='EVICTION' AND d.state<>4 AND d.released_at IS NULL)
), revisions AS (
 INSERT INTO game_item_text_revisions(item_instance_id,last_revision)
  SELECT letter_id,1 FROM active RETURNING item_instance_id
), authored AS (
 INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
  SELECT a.letter_id,game_house_rent_warning_body(a.world_id,a.house_key,a.rent_gold,a.rent_period,a.deadline),
         NULL,statement_timestamp(),1 FROM active a JOIN revisions r ON r.item_instance_id=a.letter_id
  RETURNING item_instance_id
), chosen AS (
 SELECT DISTINCT ON (a.world_id,a.house_key) a.* FROM active a
 JOIN authored t ON t.item_instance_id=a.letter_id
 ORDER BY a.world_id,a.house_key,a.delivered_at DESC,a.letter_id DESC
)
UPDATE game_house_rent_state s SET warnings=1,grace_until=c.deadline,
 notice_due_at=c.paid_until,notice_letter_id=c.letter_id,next_rent_at=statement_timestamp(),
 ownership_generation=c.ownership_generation
 FROM chosen c WHERE (s.world_id,s.house_key)=(c.world_id,c.house_key);
ALTER TABLE game_item_instances ENABLE TRIGGER game_item_instance_change_proven;
ALTER TABLE game_item_instances ENABLE TRIGGER game_item_instance_guard;
