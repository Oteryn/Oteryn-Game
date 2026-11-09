-- M41: only a successful, self-performed skinning during Hartford's mission counts.
-- Inventory possession is deliberately not a proof; the donor says to keep the beaks.
CREATE TABLE game_stag_beak_proofs (
 craft_transaction_id UUID PRIMARY KEY REFERENCES game_corpse_skinning_records(transaction_id),
 corpse_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_corpse_skinning_records(corpse_item_instance_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 mission_revision NUMERIC(20,0) NOT NULL CHECK(mission_revision BETWEEN 1 AND 18446744073709551615),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_stag_beak_proofs_character ON game_stag_beak_proofs(character_id,mission_revision);
CREATE FUNCTION game_stag_beak_proof_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 IF NOT EXISTS (
  SELECT 1 FROM game_item_craft_records c
  JOIN game_corpse_skinning_records s ON s.transaction_id=c.transaction_id
  JOIN game_corpse_source_races r ON r.item_instance_id=s.corpse_item_instance_id
  JOIN game_character_quest_tracks q ON q.character_id=c.character_id
  JOIN game_item_instances i ON i.item_instance_id=(c.outputs->0->>'item')::uuid
  JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id
  WHERE c.transaction_id=NEW.craft_transaction_id AND c.character_id=NEW.character_id
   AND c.created_xact_id=pg_current_xact_id() AND s.created_xact_id=pg_current_xact_id()
   AND s.corpse_item_instance_id=NEW.corpse_item_instance_id
   AND s.before_item->>'minted_transaction_id'=r.mint_transaction_id::text
   AND r.race_key IN ('oteryn:creature.night_harpy','oteryn:creature.day_night_harpy')
   AND s.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i52678','oteryn:item.tibia.i52679','oteryn:item.tibia.i52680')
   AND s.spent_key='oteryn:item.tibia.i52677'
   AND c.recipe='oteryn:skinning.crystal' AND NOT c.consume_source
   AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i5908'
   AND jsonb_array_length(c.outputs)=1 AND c.outputs->0->>'key'='oteryn:item.tibia.i52632'
   AND (c.outputs->0->>'count')::int=1
   AND i.minted_transaction_id=c.transaction_id AND i.world_id=c.world_id
   AND i.definition_production_key='oteryn:item.tibia.i52632'
   AND i.definition_revision_ref=c.outputs->0->>'revision' AND i.quantity=1 AND i.lifecycle=1
   AND e.character_id=c.character_id AND e.parent_item_instance_id=c.parent_item_instance_id
   AND q.quest_key='oteryn:quest.the_order_of_the_stag_quest'
   AND q.track_key='oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline'
   AND q.value=6 AND q.committed_character_revision=NEW.mission_revision
 ) OR (SELECT count(*) FROM game_stag_beak_proofs
       WHERE character_id=NEW.character_id AND mission_revision=NEW.mission_revision)>=20
 THEN RAISE EXCEPTION 'Stag requires a fresh self-skinned Night Harpy beak during Hartford mission' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_stag_beak_proof_admit BEFORE INSERT ON game_stag_beak_proofs
 FOR EACH ROW EXECUTE FUNCTION game_stag_beak_proof_admit();
CREATE TRIGGER game_stag_beak_proof_immutable BEFORE UPDATE OR DELETE ON game_stag_beak_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_stag_beak_proof_no_truncate BEFORE TRUNCATE ON game_stag_beak_proofs
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_stag_beak_proofs FROM PUBLIC;
REVOKE ALL ON FUNCTION game_stag_beak_proof_admit() FROM PUBLIC;
GRANT SELECT,INSERT ON game_stag_beak_proofs TO oteryn_game_runtime;
GRANT SELECT ON game_stag_beak_proofs TO oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_stag_beak_proof_admit() SET search_path = %I, pg_temp',current_schema());
END $$;
