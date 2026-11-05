-- Crystal 201189f data/scripts/actions/items/party_trumpet.lua:6-7;
-- data/items/items.xml item 6573: duration=4, decayTo=6572 (captured novelty catalogue).
-- Reuse the absolute Item timer and its existing custody/lease/drain owner.
-- Both immutable source owners are proved by the same deferred admission function.
-- Migration 0883 already admits event receipts through this deferred proof owner.
DO $$ DECLARE body TEXT; at INTEGER; BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'unexpected Item timer proof body'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF EXISTS(SELECT 1 FROM game_item_use_records WHERE transaction_id=NEW.source_transaction_id AND cause_kind=970) THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_use_records u JOIN game_usable_action_receipts r ON r.transaction_id=u.transaction_id
      WHERE u.transaction_id=NEW.source_transaction_id AND u.cause_kind=970 AND u.shape=4
       AND u.source_definition_production_key='oteryn:item.tibia.i6572'
       AND u.flask_item_instance_id=NEW.item_instance_id AND NEW.source_ordinal=1
       AND (u.flask_definition_family,u.flask_definition_production_key,u.flask_definition_revision_ref)
           =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
       AND NEW.definition_family='Item' AND NEW.definition_production_key='oteryn:item.tibia.i6573'
       AND NEW.target_family='Item' AND NEW.target_production_key='oteryn:item.tibia.i6572'
       AND NEW.target_revision=u.source_definition_revision_ref
       AND NEW.duration_millis=4000 AND NEW.expires_at_unix_ms=u.occurred_at+4000
       AND NEW.created_xact_id=pg_current_xact_id() AND u.created_xact_id=NEW.created_xact_id
       AND r.created_xact_id=NEW.created_xact_id)
  THEN RAISE EXCEPTION 'trumpet timer differs from atomic source transform' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
$arm$ from at for 5);
END $$;
