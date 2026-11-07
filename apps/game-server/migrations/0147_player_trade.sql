-- TRADE-PLAYER-1 / PLAYER-TRADE0 §§5-6: one atomic two-leaf exchange.
-- No pre-swap reservation, Inbox write, MINT, merge or CharacterRevision update.
-- Runtime activation remains gated on SESSION-INGRESS-1 and physical exclusion.
CREATE TABLE game_player_trades (
 occurrence_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 pair_game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(pair_game_session_id)),
 pair_command_id NUMERIC(20,0) NOT NULL CHECK(pair_command_id BETWEEN 1 AND 18446744073709551615),
 UNIQUE(pair_game_session_id,pair_command_id),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 32768),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_player_trade_participants (
 occurrence_id UUID NOT NULL REFERENCES game_player_trades,
 side SMALLINT NOT NULL CHECK(side IN (0,1)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(occurrence_id,side),
 UNIQUE(occurrence_id,character_id),
 UNIQUE(game_session_id,command_id)
);
CREATE TABLE game_player_trade_receipts (
 occurrence_id UUID NOT NULL REFERENCES game_player_trades,
 side SMALLINT NOT NULL CHECK(side IN (0,1)),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 source_character_id UUID NOT NULL REFERENCES game_character_roots,
 destination_character_id UUID NOT NULL REFERENCES game_character_roots,
 source_parent UUID NOT NULL REFERENCES game_item_instances,
 source_ordinal NUMERIC(20,0) NOT NULL CHECK(source_ordinal BETWEEN 1 AND 18446744073709551615),
 destination_parent UUID NOT NULL REFERENCES game_item_instances,
 destination_ordinal NUMERIC(20,0) NOT NULL CHECK(destination_ordinal BETWEEN 1 AND 18446744073709551615),
 destination_capacity INT NOT NULL CHECK(destination_capacity BETWEEN 1 AND 20),
 before_item JSONB NOT NULL,
 before_key JSONB,
 before_timed JSONB,
 before_text JSONB,
 before_text_revision JSONB,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(occurrence_id,side),
 UNIQUE(occurrence_id,item_instance_id),
 CHECK(source_character_id<>destination_character_id AND source_parent<>destination_parent)
);

-- These transaction predicates otherwise scan global item/evidence history on
-- every deferred check. The proof observes only one fixed two-item occurrence.
CREATE INDEX game_item_instances_last_transaction ON game_item_instances(last_transaction_id);
CREATE INDEX game_item_quantity_evidence_transaction ON game_item_transfer_quantity_evidence(transaction_id);
CREATE INDEX game_player_trade_receipt_item ON game_player_trade_receipts(item_instance_id);

-- Lock before observing or changing either source. Slot locks require a narrow
-- SECURITY DEFINER seam: runtime has no slot UPDATE grant. The writer already
-- holds both Character roots and items in deterministic order.
CREATE FUNCTION game_player_trade_lock_slots(a UUID,b UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF a IS NULL OR b IS NULL OR a=b THEN
  RAISE EXCEPTION 'two distinct trade participants required' USING ERRCODE='23514';
 END IF;
 PERFORM 1 FROM game_item_container_slots WHERE character_id IN(a,b)
  ORDER BY character_id FOR UPDATE;
END $$;

-- Every line binds real OLD state before any move. Neither a fabricated snapshot,
-- an already removed source, a full destination nor a historical cause can prove it.
CREATE FUNCTION game_player_trade_line_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_player_trades%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_player_trades WHERE occurrence_id=NEW.occurrence_id;
 IF c.created_xact_id IS DISTINCT FROM pg_current_xact_id() THEN
  RAISE EXCEPTION 'trade line requires fresh cause' USING ERRCODE='23514';
 END IF;
 PERFORM 1 FROM game_character_roots
  WHERE character_id IN(NEW.source_character_id,NEW.destination_character_id)
  ORDER BY character_id FOR UPDATE;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 PERFORM game_player_trade_lock_slots(NEW.source_character_id,NEW.destination_character_id);
 IF i.item_instance_id IS NULL OR i.world_id<>c.world_id OR i.lifecycle<>1
    OR to_jsonb(i) IS DISTINCT FROM NEW.before_item
    OR i.minted_transaction_id=c.transaction_id
    OR (SELECT to_jsonb(k) FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
       IS DISTINCT FROM NEW.before_key
    OR (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
       IS DISTINCT FROM NEW.before_timed
    OR (SELECT jsonb_build_object('sha256',encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex')) FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
       IS DISTINCT FROM NEW.before_text
    OR (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=i.item_instance_id)
       IS DISTINCT FROM NEW.before_text_revision
    OR NOT EXISTS(SELECT 1 FROM game_player_trade_participants p
       WHERE p.occurrence_id=c.occurrence_id AND p.side=NEW.side
         AND p.character_id=NEW.source_character_id AND p.created_xact_id=pg_current_xact_id())
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
          ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id
       WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.source_character_id
         AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal
         AND e.world_id=c.world_id AND s.world_id=c.world_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s JOIN game_item_instances b USING(item_instance_id)
       WHERE s.character_id=NEW.destination_character_id AND s.item_instance_id=NEW.destination_parent
         AND s.world_id=c.world_id AND b.lifecycle=1)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.destination_parent)
       >=NEW.destination_capacity
    OR NEW.destination_ordinal<>(SELECT coalesce(max(placement_ordinal),0)+1
          FROM game_item_container_entries WHERE parent_item_instance_id=NEW.destination_parent) THEN
  RAISE EXCEPTION 'trade requires unchanged backpack leaves and room before removal' USING ERRCODE='23514';
 END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_player_trade_complete(p_occurrence UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_player_trades%ROWTYPE; l game_player_trade_receipts%ROWTYPE;
        i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_player_trades WHERE occurrence_id=p_occurrence;
 IF c.occurrence_id IS NULL OR c.created_xact_id<>pg_current_xact_id()
    OR (SELECT count(*) FROM game_player_trade_participants WHERE occurrence_id=p_occurrence)<>2
    OR (SELECT count(*) FROM game_player_trade_receipts WHERE occurrence_id=p_occurrence)<>2
    OR (SELECT count(DISTINCT r.account_id) FROM game_player_trade_participants p
         JOIN game_character_roots r USING(character_id)
        WHERE p.occurrence_id=p_occurrence AND r.world_id=c.world_id AND r.lifecycle=1)<>2
    OR EXISTS(SELECT 1 FROM game_player_trade_participants WHERE occurrence_id=p_occurrence
               AND created_xact_id<>pg_current_xact_id())
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=c.transaction_id)<>2
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=c.transaction_id)
    OR (SELECT count(*) FROM game_item_transfer_quantity_evidence WHERE transaction_id=c.transaction_id)<>2
    OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=c.transaction_id)<>1
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a JOIN game_player_trade_receipts r
           ON r.occurrence_id=p_occurrence AND r.side=0 AND r.item_instance_id=a.item_instance_id
        WHERE a.event_id=c.event_id AND a.transaction_id=c.transaction_id
          AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.event_type_id=2
          AND a.occurred_at=c.occurred_at AND a.envelope_sha256=c.envelope_sha256
          AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL) THEN
  RAISE EXCEPTION 'trade must commit exactly two participants, two items and one fresh event' USING ERRCODE='23514';
 END IF;
 FOR l IN SELECT * FROM game_player_trade_receipts WHERE occurrence_id=p_occurrence ORDER BY side LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=l.item_instance_id;
  IF l.created_xact_id<>pg_current_xact_id()
     OR (to_jsonb(i)-'last_transaction_id') IS DISTINCT FROM (l.before_item-'last_transaction_id')
     OR i.last_transaction_id IS DISTINCT FROM c.transaction_id
     OR (SELECT to_jsonb(k) FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
        IS DISTINCT FROM l.before_key
     OR (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
        IS DISTINCT FROM l.before_timed
     OR (SELECT jsonb_build_object('sha256',encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex')) FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
        IS DISTINCT FROM l.before_text
     OR (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=i.item_instance_id)
        IS DISTINCT FROM l.before_text_revision
     OR EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=i.item_instance_id)
     OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence e WHERE e.item_instance_id=i.item_instance_id
          AND e.transaction_id=c.transaction_id AND e.quantity_before=i.quantity)
     OR NOT EXISTS(SELECT 1 FROM game_player_trade_participants p WHERE p.occurrence_id=p_occurrence
          AND p.side=l.side AND p.character_id=l.source_character_id)
     OR NOT EXISTS(SELECT 1 FROM game_player_trade_participants p WHERE p.occurrence_id=p_occurrence
          AND p.side=1-l.side AND p.character_id=l.destination_character_id)
     OR NOT EXISTS(SELECT 1 FROM game_player_trade_receipts other WHERE other.occurrence_id=p_occurrence
          AND other.side=1-l.side AND other.source_parent=l.destination_parent
          AND other.destination_parent=l.source_parent)
     OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
          AND e.character_id=l.destination_character_id AND e.parent_item_instance_id=l.destination_parent
          AND e.world_id=c.world_id AND e.placement_ordinal=l.destination_ordinal
          AND e.placed_transaction_id=c.transaction_id)
     OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=l.destination_parent)
        >l.destination_capacity THEN
   RAISE EXCEPTION 'trade leaf state and reciprocal custody must be conserved' USING ERRCODE='23514';
  END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_player_trade_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_player_trade_complete(NEW.occurrence_id); RETURN NULL; END $$;

-- Preserve the complete latest bodies (HOUSE, nested use, NPC, equipment, etc.)
-- verbatim. Add only a fresh TRADE proof arm to each existing proof function.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
                         'game_item_container_entry_removal_proven','game_item_placement_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position(E'BEGIN\n' IN body)=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(
       SELECT 1 FROM game_player_trade_receipts l JOIN game_player_trades c USING(occurrence_id)
        WHERE l.item_instance_id=OLD.item_instance_id AND c.transaction_id=NEW.last_transaction_id
          AND l.before_item=to_jsonb(OLD) AND c.created_xact_id=pg_current_xact_id()
          AND l.created_xact_id=pg_current_xact_id()) THEN
      IF OLD.lifecycle<>1 OR NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id
         OR (to_jsonb(NEW)-'last_transaction_id') IS DISTINCT FROM (to_jsonb(OLD)-'last_transaction_id') THEN
        RAISE EXCEPTION 'trade preserves complete item state and revision' USING ERRCODE='23514';
      END IF;
      INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before)
       VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
      RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_player_trade_receipts l JOIN game_player_trades c USING(occurrence_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND c.transaction_id=NEW.last_transaction_id
         AND c.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
      PERFORM game_player_trade_complete((SELECT occurrence_id FROM game_player_trades
                                         WHERE transaction_id=NEW.last_transaction_id));
      RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_player_trade_receipts l JOIN game_player_trades c USING(occurrence_id)
       WHERE l.item_instance_id=OLD.item_instance_id AND l.source_character_id=OLD.character_id
         AND l.source_parent=OLD.parent_item_instance_id AND l.source_ordinal=OLD.placement_ordinal
         AND c.world_id=OLD.world_id AND l.created_xact_id=pg_current_xact_id()
         AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_player_trade_receipts l JOIN game_player_trades c USING(occurrence_id)
        WHERE l.item_instance_id=NEW.item_instance_id AND l.destination_character_id=NEW.character_id
          AND l.destination_parent=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
          AND c.transaction_id=NEW.placed_transaction_id AND c.world_id=NEW.world_id
          AND l.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN
      RETURN NULL;
    END IF;
    END IF;
$arm$;
  END IF;
  -- Replace only the first BEGIN: earlier proof bodies retain their exact bytes.
  body:=overlay(body placing E'BEGIN\n'||arm from position(E'BEGIN\n' IN body) for 6);
  EXECUTE body;
 END LOOP;
END $migration$;

CREATE TRIGGER game_player_trade_line_admit BEFORE INSERT ON game_player_trade_receipts
 FOR EACH ROW EXECUTE FUNCTION game_player_trade_line_admit();
DO $$
DECLARE t TEXT; f TEXT;
BEGIN
 FOREACH t IN ARRAY ARRAY['game_player_trades','game_player_trade_participants','game_player_trade_receipts'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id()',t||'_stamp',t);
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t||'_immutable',t);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
  EXECUTE format('CREATE CONSTRAINT TRIGGER %I AFTER INSERT ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_trade_consistent()',t||'_consistent',t);
 END LOOP;
 FOREACH f IN ARRAY ARRAY['game_player_trade_lock_slots(uuid,uuid)','game_player_trade_line_admit()',
                         'game_player_trade_complete(uuid)','game_player_trade_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, %I, pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
REVOKE ALL ON game_player_trades,game_player_trade_participants,game_player_trade_receipts
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_player_trades,game_player_trade_participants,game_player_trade_receipts
 TO oteryn_game_runtime;
GRANT SELECT ON game_player_trades,game_player_trade_participants,game_player_trade_receipts TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_player_trade_lock_slots(UUID,UUID) TO oteryn_game_runtime;
