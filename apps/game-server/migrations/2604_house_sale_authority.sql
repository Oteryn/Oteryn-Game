-- Calendar conversion is a private business-clock seam. Production evaluates
-- the database timestamp unchanged. Session/Node freshness and Premium clocks
-- never consume this function; tests can advance a due date without weakening them.
CREATE FUNCTION game_house_calendar_milliseconds(t TIMESTAMPTZ) RETURNS BIGINT
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT floor(extract(epoch FROM t)*1000)::bigint $$;
-- T135 catalogue and actor authority. Only the possession-proved native port
-- grants a write; request JSON never supplies owner, rent, gold balance or custody.
CREATE FUNCTION game_house_sale_projection(w UUID,c UUID,cursor_key TEXT) RETURNS JSONB
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE rows JSONB;now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
BEGIN
 WITH eligible AS (
  SELECT p.*,s.state AS sale_state,s.recipient,s.due_at AS sale_due,s.price,
   s.rent AS sale_rent,d.state AS move_state,d.due_at AS move_due,owner.name AS owner_name,buyer.name AS recipient_name
  FROM game_house_properties p JOIN game_character_roots owner ON owner.character_id=p.owner_character_id
  LEFT JOIN game_house_sale_offers s ON s.world_id=p.world_id AND s.house_key=p.house_key AND s.state IN(1,2)
  LEFT JOIN game_character_roots buyer ON buyer.character_id=s.recipient
  LEFT JOIN LATERAL (SELECT * FROM game_house_dispositions active WHERE active.world_id=p.world_id
   AND active.house_key=p.house_key AND active.released_at IS NULL AND active.state IN(1,2,3)
   ORDER BY active.noticed_at DESC,active.operation_id DESC LIMIT 1) d ON true
  WHERE p.world_id=w AND p.owner_guild_id IS NULL AND (p.owner_character_id=c OR s.recipient=c)
   AND p.house_key>cursor_key ORDER BY p.house_key LIMIT 21
 ) SELECT coalesce(jsonb_agg(jsonb_build_object('house',house_key,'revision',revision,'owner',owner_name,
  'recipient',coalesce(recipient_name,''),'transfer_state',coalesce(sale_state,0),
  'due',coalesce(sale_due,move_due,0),'price',coalesce(price,0),'rent',coalesce(sale_rent,rent_gold),'move_out_state',coalesce(move_state,0),
  'allowed',CASE WHEN sale_state IS NOT NULL THEN
    CASE WHEN sale_due<=now_ms AND sale_state=2 THEN 0 WHEN owner_character_id=c THEN 16
     WHEN recipient=c THEN CASE WHEN sale_state=1 AND sale_due>now_ms THEN 96 ELSE 64 END ELSE 0 END
   WHEN owner_character_id<>c THEN 0 WHEN move_state IS NULL THEN 10
   WHEN move_state=1 AND move_due>now_ms THEN 4 ELSE 0 END) ORDER BY house_key),'[]'::jsonb) INTO rows FROM eligible;
 RETURN jsonb_build_object('now',now_ms,'has_more',jsonb_array_length(rows)>20,'rows',
  (SELECT coalesce(jsonb_agg(value ORDER BY ordinal),'[]'::jsonb) FROM jsonb_array_elements(rows) WITH ORDINALITY AS x(value,ordinal) WHERE ordinal<=20));
END $$;
CREATE FUNCTION game_house_sale_candidate(w UUID,h TEXT,c UUID,target UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_character_roots r JOIN game_character_build_state build USING(character_id)
  WHERE r.character_id=target AND r.character_id<>c AND r.world_id=w AND r.lifecycle=1 AND build.vocation<>'none'
   AND NOT EXISTS(SELECT 1 FROM game_account_house_bans WHERE account_id=r.account_id AND world_id=w AND banned_until>statement_timestamp())
   AND NOT EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=r.account_id AND world_id=w AND state=1)
   AND NOT EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=r.account_id AND world_id=w AND house_key<>h)
   AND NOT EXISTS(SELECT 1 FROM game_house_sale_offers WHERE recipient_account=r.account_id AND world_id=w AND state=2 AND house_key<>h))
$$;
CREATE FUNCTION game_house_sale_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID;ch UUID;sk BYTEA;c UUID;sid UUID;cid NUMERIC;action INT;binding BYTEA;
 previous game_house_sale_commands;hp game_house_properties;o game_house_sale_offers;changed game_house_sale_offers;
 recipient_root game_character_roots;before_value JSONB;after_value JSONB;property_before JSONB;property_after JSONB;
 event UUID;now_ms BIGINT;due BIGINT;result JSONB;owner_root game_character_roots;account UUID;disposition_value JSONB;
BEGIN
 IF mode NOT IN('inspect','command') OR p IS NULL OR jsonb_typeof(p)<>'object' OR octet_length(p::text)>4096
  OR NOT p ?& ARRAY['world','channel','generation','session','character','command','connection','lease','action','house','revision','due','recipient','price','after']
  OR EXISTS(SELECT 1 FROM jsonb_object_keys(p) k WHERE k NOT IN('world','channel','generation','session','character','command','connection','lease','action','house','revision','due','recipient','price','after','premium_account','premium_allowed'))
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE
 THEN RAISE EXCEPTION 'house sale ingress refused' USING ERRCODE='23514'; END IF;
 w:=(p->>'world')::uuid;ch:=(p->>'channel')::uuid;sid:=(p->>'session')::uuid;
 c:=(p->>'character')::uuid;cid:=(p->>'command')::numeric;action:=(p->>'action')::int;
 sk:='\x01'::bytea||uuid_send(w)||uuid_send(ch);
 IF cid IS NULL OR cid NOT BETWEEN 1 AND 18446744073709551615 OR action IS NULL OR action NOT BETWEEN 0 AND 6
 THEN RETURN NULL; END IF;
 binding:=convert_to((p-ARRAY['premium_account','premium_allowed'])::text,'UTF8');
 IF action<>0 THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(sid::text||':'||cid::text,135));
  SELECT * INTO previous FROM game_house_sale_commands WHERE game_session_id=sid AND command_id=cid;
  IF FOUND THEN
   IF previous.binding<>binding THEN RETURN jsonb_build_object('conflict',true); END IF;
   RETURN jsonb_build_object('prior',previous.outcome);
  END IF;
 END IF;
 -- Serialize this World's ownership negotiations before taking actor/party locks.
 IF mode='command' THEN PERFORM pg_advisory_xact_lock(hashtextextended(w::text||':house-sales',135)); END IF;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN game_durability_admission_runtime_guards rg USING(scope_key)
  WHERE ra.scope_key=sk AND ra.scope_kind=1 AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
   AND ra.ownership_generation=(p->>'generation')::numeric AND ra.holder_node_id=n AND ra.holder_registration_revision=nr
   AND rg.ready AND rg.ownership_generation=ra.ownership_generation FOR SHARE OF ra,rg)
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id)
   JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id
   JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
   WHERE s.game_session_id=sid AND s.character_id=c AND s.account_id=r.account_id AND s.world_id=w AND r.world_id=w AND r.lifecycle=1
    AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=w AND s.runtime_scope_channel_id=ch
    AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
    AND s.current_generation=(p->>'connection')::numeric AND s.character_lease_generation=(p->>'lease')::numeric
    AND s.scope_ownership_generation=(p->>'generation')::numeric AND s.session_state IN(1,2)
    AND cg.eligible AND cg.account_id=r.account_id AND cg.world_id=w AND cg.lease_generation=s.character_lease_generation
    AND cg.holder_game_session_id=sid AND ag.presence_character_id=c AND ag.holder_game_session_id=sid
    FOR SHARE OF s,r,cg,ag) THEN RETURN NULL; END IF;
 IF action=0 THEN
  IF mode<>'inspect' THEN RETURN NULL; END IF;
  RETURN game_house_sale_projection(w,c,p->>'after');
 END IF;
 SELECT * INTO hp FROM game_house_properties WHERE world_id=w AND house_key=p->>'house' AND owner_guild_id IS NULL;
 IF NOT FOUND OR hp.owner_character_id IS NULL OR hp.revision<>(p->>'revision')::numeric THEN RETURN NULL; END IF;
 SELECT * INTO o FROM game_house_sale_offers WHERE world_id=w AND house_key=hp.house_key AND state IN(1,2);
 IF action=3 THEN
  IF hp.owner_character_id<>c OR o.offer_id IS NOT NULL THEN RETURN NULL; END IF;
  SELECT * INTO recipient_root FROM game_character_roots WHERE world_id=w AND name_key=game_character_name_key(p->>'recipient') AND lifecycle=1;
 ELSIF action=5 THEN
  IF o.state IS DISTINCT FROM 1 OR o.recipient<>c THEN RETURN NULL; END IF;
  SELECT * INTO recipient_root FROM game_character_roots WHERE character_id=o.recipient;
 END IF;
 IF action IN(3,5) AND (recipient_root.character_id IS NULL OR NOT game_house_sale_candidate(w,hp.house_key,hp.owner_character_id,recipient_root.character_id)) THEN RETURN NULL; END IF;
 IF mode='inspect' THEN RETURN jsonb_build_object('target',recipient_root.account_id); END IF;
 IF action IN(3,5) AND ((p->>'premium_allowed')::boolean IS DISTINCT FROM TRUE OR (p->>'premium_account')::uuid IS DISTINCT FROM recipient_root.account_id) THEN RETURN NULL; END IF;
 -- Character roots, then both Account/World bank rows, then property. This order
 -- is also used by the World-job prelock before contents disposition.
 PERFORM 1 FROM game_character_roots r WHERE r.character_id IN(c,hp.owner_character_id,o.recipient,recipient_root.character_id) ORDER BY r.character_id FOR UPDATE;
 SELECT * INTO STRICT owner_root FROM game_character_roots WHERE character_id=hp.owner_character_id;
 FOR account IN SELECT DISTINCT a FROM unnest(ARRAY[owner_root.account_id,coalesce(recipient_root.account_id,o.recipient_account)]) a WHERE a IS NOT NULL ORDER BY a LOOP
  INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(account,w,0) ON CONFLICT DO NOTHING;
  PERFORM 1 FROM game_account_bank_balances b WHERE b.account_id=account AND b.world_id=w FOR UPDATE;
 END LOOP;
 SELECT * INTO hp FROM game_house_properties WHERE world_id=w AND house_key=hp.house_key FOR UPDATE;
 IF hp.revision<>(p->>'revision')::numeric OR hp.owner_character_id<>owner_root.character_id OR hp.owner_account_id<>owner_root.account_id THEN RETURN NULL; END IF;
 PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=hp.house_key FOR UPDATE;
 IF o.offer_id IS NOT NULL THEN
  SELECT * INTO o FROM game_house_sale_offers WHERE offer_id=o.offer_id FOR UPDATE;
  IF o.state NOT IN(1,2) OR o.ownership_revision<>hp.ownership_generation OR o.seller<>hp.owner_character_id THEN RETURN NULL; END IF;
 END IF;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF action IN(1,2) THEN
  IF hp.owner_character_id<>c OR o.offer_id IS NOT NULL THEN RETURN NULL; END IF;
  disposition_value:=game_house_move_out_apply(w,hp.house_key,c,action=1,CASE WHEN action=1 THEN (p->>'due')::bigint END,now_ms);
  IF disposition_value IS NULL THEN RETURN NULL; END IF;
  -- Existing notice/cancel authority retains its disposition audit in this transaction.
 ELSIF action=3 THEN
  due:=(p->>'due')::bigint;
  IF due::numeric-now_ms NOT BETWEEN 86400000 AND 2592000000 OR (p->>'price')::numeric NOT BETWEEN 0 AND 999999999999-hp.rent_gold
   OR EXISTS(SELECT 1 FROM game_house_dispositions WHERE world_id=w AND house_key=hp.house_key AND state IN(1,2,3) AND released_at IS NULL)
  THEN RETURN NULL; END IF;
  event:=game_character_uuid_v7();
  changed.offer_id:=game_character_uuid_v7();changed.world_id:=w;changed.house_key:=hp.house_key;
  changed.seller:=c;changed.seller_account:=owner_root.account_id;changed.recipient:=recipient_root.character_id;
  changed.recipient_account:=recipient_root.account_id;changed.ownership_revision:=hp.ownership_generation;
  changed.price:=(p->>'price')::bigint;changed.rent:=hp.rent_gold;changed.due_at:=due;changed.created_at:=now_ms;
  changed.state:=1;changed.last_event:=event;
 ELSE
  IF o.offer_id IS NULL OR (now_ms>=o.due_at AND (action=5 OR o.state=2)) OR (action=4 AND o.seller<>c)
   OR (action IN(5,6) AND o.recipient<>c) THEN RETURN NULL; END IF;
  IF action=5 AND (o.state<>1 OR NOT game_house_sale_candidate(w,hp.house_key,o.seller,o.recipient)) THEN RETURN NULL; END IF;
  event:=game_character_uuid_v7();changed:=o;changed.last_event:=event;
  IF action=5 THEN
   IF recipient_root.account_id<>o.recipient_account OR owner_root.account_id<>o.seller_account
    OR (SELECT balance FROM game_account_bank_balances WHERE account_id=o.recipient_account AND world_id=w)<o.price+o.rent
    OR (o.seller_account<>o.recipient_account AND game_account_bank_credit_headroom(o.seller_account,w)<o.price) THEN RETURN NULL; END IF;
   IF EXISTS(SELECT 1 FROM game_house_dispositions WHERE world_id=w AND house_key=hp.house_key AND state IN(1,2,3) AND released_at IS NULL) THEN RETURN NULL; END IF;
   INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at)
    VALUES(w,hp.house_key,o.seller,o.ownership_revision,1,o.created_at,o.due_at) RETURNING operation_id INTO changed.disposition;
   disposition_value:=jsonb_build_object('operation',changed.disposition);changed.state:=2;
  ELSE
   changed.state:=CASE action WHEN 4 THEN 3 ELSE 4 END;
   IF o.state=2 THEN
    UPDATE game_house_dispositions SET state=4,cancelled_at=now_ms WHERE operation_id=o.disposition AND state=1 AND due_at>now_ms;
    IF NOT FOUND THEN RETURN NULL; END IF;
    disposition_value:=jsonb_build_object('operation',o.disposition);
   END IF;
  END IF;
 END IF;
 IF event IS NOT NULL THEN
  property_before:=to_jsonb(hp);property_after:=property_before||jsonb_build_object('revision',hp.revision+1);
  INSERT INTO game_house_sale_events VALUES(event,changed.offer_id,action,CASE WHEN o.offer_id IS NOT NULL THEN to_jsonb(o) END,
   to_jsonb(changed),property_before,property_after,sid,cid,c,n,nr,pg_current_xact_id(),DEFAULT);
  IF action=3 THEN INSERT INTO game_house_sale_offers SELECT (jsonb_populate_record(NULL::game_house_sale_offers,to_jsonb(changed))).*;
  ELSE UPDATE game_house_sale_offers SET state=changed.state,last_event=event,disposition=changed.disposition WHERE offer_id=o.offer_id; END IF;
  IF action=5 THEN
   PERFORM game_house_sale_bank_write(event,'RESERVE',changed.recipient_account,w,changed.recipient,changed.price+changed.rent);
   INSERT INTO game_house_sale_holds VALUES(changed.offer_id,w,changed.recipient_account,changed.seller_account,changed.price+changed.rent,changed.price);
  ELSIF action IN(4,6) AND o.state=2 THEN
   DELETE FROM game_house_sale_holds WHERE offer_id=o.offer_id;
   PERFORM game_house_sale_bank_write(event,'REFUND',o.recipient_account,w,o.recipient,o.price+o.rent);
  END IF;
  UPDATE game_house_properties SET revision=revision+1 WHERE world_id=w AND house_key=hp.house_key;
 END IF;
 result:=game_house_sale_projection(w,c,'');
 INSERT INTO game_house_sale_commands VALUES(sid,cid,binding,result,pg_current_xact_id(),(disposition_value->>'operation')::uuid);
 RETURN jsonb_build_object('outcome',result,'event',event,'disposition',disposition_value->>'operation','new',true);
END $$;

-- Ordinary notices cannot cancel an accepted sale behind the escrow owner.
ALTER FUNCTION game_house_move_out_apply(UUID,TEXT,UUID,BOOLEAN,BIGINT,BIGINT) RENAME TO game_house_move_out_apply_before_t135;
CREATE FUNCTION game_house_move_out_apply(w UUID,h TEXT,c UUID,notice BOOLEAN,due BIGINT,now_ms BIGINT) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE result JSONB;
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_sale_offers WHERE world_id=w AND house_key=h AND state IN(1,2)) THEN RETURN NULL; END IF;
 result:=game_house_move_out_apply_before_t135(w,h,c,notice,due,now_ms);
 IF result IS NOT NULL THEN UPDATE game_house_properties SET revision=revision+1 WHERE world_id=w AND house_key=h; END IF;
 RETURN result;
END $$;

CREATE FUNCTION game_house_sale_character_participant() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF (TG_OP='DELETE' OR (NEW.account_id,NEW.lifecycle) IS DISTINCT FROM (OLD.account_id,OLD.lifecycle))
  AND EXISTS(SELECT 1 FROM game_house_sale_offers WHERE state=2 AND (seller=OLD.character_id OR recipient=OLD.character_id))
 THEN RAISE EXCEPTION 'accepted house transfer retains its Character and Account participants' USING ERRCODE='23514'; END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;RETURN NEW;
END $$;
CREATE TRIGGER game_house_sale_character_participant BEFORE UPDATE OR DELETE ON game_character_roots
 FOR EACH ROW EXECUTE FUNCTION game_house_sale_character_participant();

CREATE FUNCTION game_house_sale_audit_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE e game_house_sale_events;o game_house_sale_offers;w UUID;ch UUID;
BEGIN
 IF mode NOT IN('snapshot','insert') OR octet_length(p::text)>40000 OR jsonb_typeof(p)<>'object'
  OR NOT p ?& ARRAY['world','channel','generation','event']
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'house sale audit ingress refused' USING ERRCODE='23514'; END IF;
 w:=(p->>'world')::uuid;ch:=(p->>'channel')::uuid;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN game_durability_admission_runtime_guards rg USING(scope_key)
  WHERE ra.scope_key='\x01'::bytea||uuid_send(w)||uuid_send(ch) AND ra.world_id=w AND ra.channel_id=ch AND ra.scope_kind=1 AND ra.state=1
   AND ra.ownership_generation=(p->>'generation')::numeric AND ra.holder_node_id=n AND ra.holder_registration_revision=nr
   AND rg.ready AND rg.ownership_generation=ra.ownership_generation FOR SHARE OF ra,rg)
 THEN RETURN NULL; END IF;
 SELECT * INTO e FROM game_house_sale_events WHERE event_id=(p->>'event')::uuid;
 SELECT * INTO o FROM game_house_sale_offers WHERE offer_id=e.offer_id;
 IF e.event_id IS NULL OR e.created_xact_id<>pg_current_xact_id() OR o.world_id<>w OR e.node_id<>n OR e.node_revision<>nr
 THEN RAISE EXCEPTION 'house sale audit has no own physical transition' USING ERRCODE='23514'; END IF;
 IF mode='snapshot' THEN RETURN jsonb_build_object('event',e.event_id,'world',w,'house',o.house_key,'offer',o.offer_id,
  'seller',o.seller,'recipient',o.recipient,'seller_account',o.seller_account,'recipient_account',o.recipient_account,
  'price',o.price,'rent',o.rent,'due',o.due_at,'state',o.state,'action',e.action,'before_state',coalesce((e.before_offer->>'state')::int,0),
  'disposition',o.disposition,'now',e.occurred_at); END IF;
 IF (p->>'now')::bigint IS DISTINCT FROM e.occurred_at THEN RAISE EXCEPTION 'house sale event time mismatch' USING ERRCODE='23514'; END IF;
 INSERT INTO game_house_sale_audit_outbox(event_id,envelope,occurred_at,expires_at,envelope_sha256)
 VALUES(e.event_id,decode(p->>'envelope','hex'),e.occurred_at,e.occurred_at+2592000000,sha256(decode(p->>'envelope','hex')));
 RETURN 'true'::jsonb;
END $$;

CREATE FUNCTION game_house_sale_prelock(op UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_sale_offers;party UUID;account UUID;
BEGIN
 SELECT * INTO o FROM game_house_sale_offers WHERE disposition=op AND state=2;
 IF NOT FOUND THEN RETURN; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(o.world_id::text||':house-sales',135));
 SELECT * INTO o FROM game_house_sale_offers WHERE disposition=op AND state=2;
 IF NOT FOUND THEN RETURN; END IF;
 LOCK TABLE game_house_scope_handoffs IN SHARE ROW EXCLUSIVE MODE;
 FOR party IN SELECT id FROM (SELECT o.seller AS id UNION SELECT o.recipient UNION
  SELECT reclaim_subject_character_id FROM game_house_disposition_container_sources(o.world_id,o.house_key)) parties ORDER BY id LOOP
  PERFORM 1 FROM game_character_roots WHERE character_id=party FOR UPDATE;
 END LOOP;
 FOR account IN SELECT DISTINCT a FROM unnest(ARRAY[o.seller_account,o.recipient_account]) a ORDER BY a LOOP
  PERFORM 1 FROM game_account_bank_balances b WHERE b.account_id=account AND b.world_id=o.world_id FOR UPDATE;
 END LOOP;
END $$;
CREATE FUNCTION game_house_sale_finish(op UUID,n UUID,nr NUMERIC) RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_sale_offers;changed game_house_sale_offers;p game_house_properties;vacant game_house_properties;
 event UUID;property_after JSONB;
BEGIN
 SELECT * INTO o FROM game_house_sale_offers WHERE disposition=op AND state=2 FOR UPDATE;
 IF NOT FOUND THEN
  RETURN jsonb_build_object('changed',CASE WHEN game_house_disposition_is_personal_move_out(op) THEN game_house_move_out_release(op) ELSE FALSE END);
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=o.world_id AND house_key=o.house_key FOR UPDATE;
 IF p.owner_character_id<>o.seller OR p.owner_account_id<>o.seller_account OR p.ownership_generation<>o.ownership_revision
  OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=o.recipient AND account_id=o.recipient_account AND world_id=o.world_id AND lifecycle=1)
  OR NOT EXISTS(SELECT 1 FROM game_house_sale_holds WHERE offer_id=o.offer_id AND refund=o.price+o.rent AND incoming=o.price)
 THEN RAISE EXCEPTION 'house sale handover lost its exact participants or escrow' USING ERRCODE='23514'; END IF;
 IF NOT game_house_move_out_release(op) THEN RETURN jsonb_build_object('changed',false); END IF;
 SELECT * INTO STRICT vacant FROM game_house_properties WHERE world_id=o.world_id AND house_key=o.house_key;
 event:=game_character_uuid_v7();changed:=o;changed.state:=5;changed.last_event:=event;
 property_after:=to_jsonb(vacant)||jsonb_build_object('state',3,'owner_character_id',o.recipient,
  'owner_account_id',o.recipient_account,'ownership_generation',vacant.ownership_generation+1,
  'revision',vacant.revision+1,'acquisition_receipt_ref',event,'paid_until',to_timestamp(game_house_calendar_milliseconds(statement_timestamp())::numeric/1000)+interval '30 days');
 INSERT INTO game_house_sale_events VALUES(event,o.offer_id,7,to_jsonb(o),to_jsonb(changed),to_jsonb(p),property_after,
  NULL,NULL,NULL,n,nr,pg_current_xact_id(),DEFAULT);
 UPDATE game_house_sale_offers SET state=5,last_event=event WHERE offer_id=o.offer_id;
 DELETE FROM game_house_sale_holds WHERE offer_id=o.offer_id;
 PERFORM game_house_sale_bank_write(event,'PAYMENT',o.seller_account,o.world_id,o.seller,o.price);
 UPDATE game_house_properties SET state=3,owner_character_id=o.recipient,owner_account_id=o.recipient_account,
  ownership_generation=ownership_generation+1,revision=revision+1,acquisition_receipt_ref=event,
  paid_until=to_timestamp(game_house_calendar_milliseconds(statement_timestamp())::numeric/1000)+interval '30 days' WHERE world_id=o.world_id AND house_key=o.house_key;
 UPDATE game_house_ownership SET owner_character_id=o.recipient,ownership_revision=ownership_revision+1,
  acquisition_receipt_ref=event WHERE world_id=o.world_id AND house_key=o.house_key;
 INSERT INTO game_account_housing_slots VALUES(o.recipient_account,o.world_id,o.house_key);
 RETURN jsonb_build_object('changed',true,'event',event);
END $$;
DO $patch$ DECLARE body TEXT;old TEXT;arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_move_out_port(TEXT,JSONB,UUID,NUMERIC,BYTEA)'::regprocedure);
 old:='''audit_insert'')';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing T135 World release mode seam'; END IF;
 body:=replace(body,old,'''audit_insert'',''release'')');
 old:='IF mode=''advance'' THEN';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing T135 World prelock seam'; END IF;
 body:=replace(body,old,E'IF mode=''release'' THEN RETURN game_house_sale_finish(o.operation_id,n,nr); END IF;\n IF mode=''advance'' THEN\n  PERFORM game_house_sale_prelock(o.operation_id);');
 EXECUTE body;
 body:=pg_get_functiondef('game_house_management_acl_proven()'::regprocedure);
 arm:=$arm$
 IF TG_OP='DELETE' THEN
  FOR sale_event IN SELECT e.event_id FROM game_house_sale_events e
   JOIN game_house_sale_offers sale ON sale.offer_id=e.offer_id
   JOIN game_house_properties property ON property.world_id=sale.world_id AND property.house_key=sale.house_key
   WHERE e.action=7 AND e.created_xact_id=pg_current_xact_id() AND sale.state=5
    AND sale.world_id=OLD.world_id AND sale.house_key=OLD.house_key AND e.property_after=to_jsonb(property) LOOP
   PERFORM game_house_sale_event_proven(sale_event);RETURN NULL;
  END LOOP;
 END IF;
$arm$;
 body:=replace(body,'DECLARE w UUID; h TEXT; k TEXT;','DECLARE w UUID; h TEXT; k TEXT; sale_event UUID;');
 IF strpos(body,'sale_event UUID')=0 THEN RAISE EXCEPTION 'missing T135 ACL declaration seam'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM strpos(body,'BEGIN') FOR 5);
END $patch$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT oid::regprocedure FROM pg_proc WHERE pronamespace=current_schema()::regnamespace
  AND (proname LIKE 'game_house_sale_%' OR proname IN('game_house_move_out_apply','game_house_move_out_apply_before_t135')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_sale_port(TEXT,JSONB,UUID,NUMERIC,BYTEA),
 game_house_sale_audit_port(TEXT,JSONB,UUID,NUMERIC,BYTEA),game_house_sale_reserves_slot(UUID,UUID),game_house_sale_event_proven(UUID) TO oteryn_game_runtime;

DO $$ DECLARE f REGPROCEDURE;body TEXT;BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_sale_port(TEXT,JSONB,UUID,NUMERIC,BYTEA)'::regprocedure,
  'game_house_sale_projection(UUID,UUID,TEXT)'::regprocedure,
  'game_house_disposition_advance(UUID,INT,INT,INT)'::regprocedure,
  'game_house_move_out_port(TEXT,JSONB,UUID,NUMERIC,BYTEA)'::regprocedure,
  'game_house_move_out_release(UUID)'::regprocedure] LOOP
  body:=pg_get_functiondef(f);
  body:=replace(body,'floor(extract(epoch FROM clock_timestamp())*1000)::bigint','game_house_calendar_milliseconds(clock_timestamp())');
  body:=replace(body,'floor(extract(epoch FROM statement_timestamp())*1000)::bigint','game_house_calendar_milliseconds(statement_timestamp())');
  EXECUTE body;
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_calendar_milliseconds(TIMESTAMPTZ) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_calendar_milliseconds(TIMESTAMPTZ) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
