-- Six finite source-owned quantity-one NonStackable backpack outputs.
-- Mature RewardClaim MINT remains the only Item/custody/audit writer.
ALTER TABLE game_npc_source_context ADD COLUMN output_transaction_id UUID;
CREATE TABLE game_npc_source_item_outputs(
 parent_transaction_id UUID PRIMARY KEY REFERENCES game_npc_cosmetic_receipts(transaction_id),
 mint_transaction_id UUID NOT NULL UNIQUE REFERENCES game_reward_claim_mint_receipts(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 source_key TEXT NOT NULL,item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 source_item_id INTEGER NOT NULL CHECK(source_item_id IN(5957,895,894,39546)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_npc_source_item_outputs FROM PUBLIC;
GRANT SELECT ON game_npc_source_item_outputs TO oteryn_game_runtime;
CREATE INDEX game_npc_source_item_outputs_character_xact ON game_npc_source_item_outputs(character_id,created_xact_id);
CREATE TRIGGER game_npc_source_output_immutable BEFORE UPDATE OR DELETE ON game_npc_source_item_outputs FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_source_output_no_truncate BEFORE TRUNCATE ON game_npc_source_item_outputs FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_npc_source_output_id(p_source TEXT) RETURNS INTEGER LANGUAGE sql IMMUTABLE AS $$
 SELECT CASE p_source
 WHEN 'oteryn:npc-cosmetic/sandra/belt-ticket-283' THEN 5957
 WHEN 'oteryn:npc-cosmetic/sandra/belt-ticket-284' THEN 5957
 WHEN 'oteryn:npc-cosmetic/sandra/belt-ticket-285' THEN 5957
 WHEN 'oteryn:npc-cosmetic/bozo/jester-addon-1' THEN 895
 WHEN 'oteryn:npc-cosmetic/bozo/jester-addon-2' THEN 894
 WHEN 'oteryn:npc-cosmetic/gnomadness/ripptor-prize' THEN 39546 END
$$;
CREATE FUNCTION game_npc_source_bind_output(p_mint UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;reservation game_reward_claim_mint_reservations%ROWTYPE;expected_id INTEGER;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT * INTO reservation FROM game_reward_claim_mint_reservations WHERE transaction_id=p_mint;
 expected_id:=game_npc_source_output_id(c.source_key);
 IF c.npc_transaction_id IS NULL OR c.captured OR c.output_transaction_id IS NOT NULL OR expected_id IS NULL
  OR p_mint IS NULL OR p_mint=c.npc_transaction_id OR p_mint IS NOT DISTINCT FROM c.cost_transaction_id OR p_mint IS NOT DISTINCT FROM c.fee_transaction_id
  OR game_npc_source_effect_current(c.npc_transaction_id,c.source_key,jsonb_build_object('kind','mint_item','item',expected_id,'count',1)) IS DISTINCT FROM TRUE
  OR reservation.transaction_id IS NULL OR NOT EXISTS(SELECT 1 FROM game_reward_claim_mint_reservations fresh_reservation WHERE fresh_reservation.transaction_id=p_mint AND fresh_reservation.xmin=pg_current_xact_id()::xid)
  OR reservation.character_id IS DISTINCT FROM c.character_id OR reservation.game_session_id IS DISTINCT FROM c.game_session_id OR reservation.command_id IS DISTINCT FROM c.command_id
  OR reservation.world_id IS DISTINCT FROM c.world_id OR reservation.channel_id IS DISTINCT FROM c.channel_id
  OR reservation.claim_family IS DISTINCT FROM 'NpcCosmetic'
  OR reservation.claim_revision_ref IS DISTINCT FROM 'npc-source-program.'||encode(c.program_sha256,'hex')
  OR reservation.claim_production_key IS DISTINCT FROM 'oteryn:npc-source-output.'||c.npc_transaction_id::text
  OR reservation.work_units_used<>1 THEN
  RAISE EXCEPTION 'NPC output requires one current reserved native MINT' USING ERRCODE='23514';END IF;
 UPDATE game_npc_source_context SET output_transaction_id=p_mint WHERE backend_pid=pg_backend_pid();
END $$;
CREATE FUNCTION game_npc_source_output_namespace() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;expected_id INTEGER;actual_mint UUID;
BEGIN
 IF NEW.claim_production_key NOT LIKE 'oteryn:npc-source-output.%' THEN RETURN NEW;END IF;
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 expected_id:=game_npc_source_output_id(c.source_key);
 -- All three tables share the claim/Character fields. Access the differently
 -- named transaction field only inside its exact row-type branch.
 IF TG_TABLE_NAME='game_reward_claims' THEN actual_mint:=NEW.claimed_transaction_id;
 ELSE actual_mint:=NEW.transaction_id;END IF;
 IF c.npc_transaction_id IS NULL OR expected_id IS NULL
  OR NEW.claim_family IS DISTINCT FROM 'NpcCosmetic'
  OR NEW.claim_revision_ref IS DISTINCT FROM 'npc-source-program.'||encode(c.program_sha256,'hex') OR NEW.character_id IS DISTINCT FROM c.character_id
  OR NEW.claim_production_key IS DISTINCT FROM 'oteryn:npc-source-output.'||c.npc_transaction_id::text
  OR game_npc_source_effect_current(c.npc_transaction_id,c.source_key,jsonb_build_object('kind','mint_item','item',expected_id,'count',1)) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'NPC output namespace requires its private current source' USING ERRCODE='23514';END IF;
 IF TG_TABLE_NAME<>'game_reward_claim_mint_reservations' THEN
  IF actual_mint IS DISTINCT FROM c.output_transaction_id OR NOT c.captured
   OR game_npc_source_parent_current(c.npc_transaction_id,c.source_key) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'NPC output requires the exact immutable parent' USING ERRCODE='23514';END IF;
 END IF;
 RETURN NEW;
END $$;
DO $namespace$ DECLARE relation TEXT;BEGIN
 FOREACH relation IN ARRAY ARRAY['game_reward_claim_mint_reservations','game_reward_claim_mint_receipts','game_reward_claims'] LOOP
  EXECUTE format('CREATE TRIGGER game_npc_source_output_namespace BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_namespace()',relation);
 END LOOP;
END $namespace$;
CREATE FUNCTION game_npc_source_output_capture(p_parent UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;minted game_reward_claim_mint_receipts%ROWTYPE;item game_item_instances%ROWTYPE;expected_id INTEGER;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() AND npc_transaction_id=p_parent;
 SELECT * INTO minted FROM game_reward_claim_mint_receipts WHERE transaction_id=c.output_transaction_id;
 SELECT * INTO item FROM game_item_instances WHERE item_instance_id=minted.item_instance_id;
 expected_id:=game_npc_source_output_id(c.source_key);
 IF expected_id IS NULL OR game_npc_source_parent_current(p_parent,c.source_key) IS DISTINCT FROM TRUE
  OR minted.transaction_id IS NULL OR minted.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR minted.character_id IS DISTINCT FROM c.character_id OR minted.game_session_id IS DISTINCT FROM c.game_session_id OR minted.command_id IS DISTINCT FROM c.command_id
  OR minted.quantity<>1 OR item.quantity<>1 OR item.lifecycle<>1
  OR item.definition_family IS DISTINCT FROM 'Item' OR item.definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i'||expected_id
  OR item.definition_revision_ref IS DISTINCT FROM 'definition-r1' OR item.minted_transaction_id IS DISTINCT FROM minted.transaction_id
  OR NOT EXISTS(SELECT 1 FROM game_item_container_entries actual_entry WHERE actual_entry.item_instance_id=item.item_instance_id
   AND actual_entry.character_id=c.character_id AND actual_entry.world_id=c.world_id AND actual_entry.parent_item_instance_id=minted.destination_parent_item_instance_id
   AND actual_entry.placement_ordinal=minted.destination_ordinal AND actual_entry.placed_transaction_id=minted.transaction_id) THEN
  RAISE EXCEPTION 'NPC output lacks its exact fresh native item and custody' USING ERRCODE='23514';END IF;
 INSERT INTO game_npc_source_item_outputs VALUES(p_parent,minted.transaction_id,c.character_id,c.source_key,item.item_instance_id,expected_id,pg_current_xact_id());
END $$;
DO $paths$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY['game_npc_source_output_id(text)','game_npc_source_bind_output(uuid)','game_npc_source_output_namespace()','game_npc_source_output_capture(uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $paths$;
GRANT EXECUTE ON FUNCTION game_npc_source_bind_output(UUID),game_npc_source_output_capture(UUID) TO oteryn_game_runtime;

CREATE FUNCTION game_npc_source_output_complete(p_parent UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_context c
 JOIN game_npc_source_item_outputs output ON output.parent_transaction_id=c.npc_transaction_id
 JOIN game_reward_claim_mint_receipts mint ON mint.transaction_id=output.mint_transaction_id
 JOIN game_item_instances item ON item.item_instance_id=output.item_instance_id
 JOIN game_item_container_entries custody ON custody.item_instance_id=item.item_instance_id
 JOIN game_reward_claims claim ON claim.claimed_transaction_id=mint.transaction_id
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.npc_transaction_id=p_parent
 AND game_npc_source_parent_current(p_parent,c.source_key)
 AND game_npc_source_effect_current(p_parent,c.source_key,jsonb_build_object('kind','mint_item','item',output.source_item_id,'count',1))
 AND output.source_item_id=game_npc_source_output_id(c.source_key) AND output.source_key=c.source_key
 AND output.character_id=c.character_id AND output.created_xact_id=pg_current_xact_id()
 AND mint.transaction_id=c.output_transaction_id AND mint.created_xact_id=pg_current_xact_id()
 AND mint.game_session_id=c.game_session_id AND mint.command_id=c.command_id AND mint.character_id=c.character_id
 AND mint.item_instance_id=item.item_instance_id AND mint.quantity=1
 AND mint.claim_family='NpcCosmetic' AND mint.claim_production_key='oteryn:npc-source-output.'||p_parent::text
 AND mint.claim_revision_ref='npc-source-program.'||encode(c.program_sha256,'hex')
 AND claim.character_id=c.character_id AND claim.claim_family=mint.claim_family AND claim.claim_production_key=mint.claim_production_key
 AND claim.claim_revision_ref=mint.claim_revision_ref AND claim.next_allowed_at IS NULL
 AND item.world_id=c.world_id AND item.definition_family='Item'
 AND item.definition_production_key='oteryn:item.tibia.i'||output.source_item_id AND item.definition_revision_ref='definition-r1'
 AND item.quantity=1 AND item.lifecycle=1 AND item.state_revision=1 AND item.last_transaction_id IS NULL
 AND item.minted_transaction_id=mint.transaction_id
 AND custody.character_id=c.character_id AND custody.world_id=c.world_id
 AND custody.parent_item_instance_id=mint.destination_parent_item_instance_id AND custody.placement_ordinal=mint.destination_ordinal
 AND custody.placed_transaction_id=mint.transaction_id)
$$;
CREATE FUNCTION game_npc_source_output_required() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.program_sha256 IS NOT NULL AND EXISTS(SELECT 1 FROM game_npc_source_policies policy,
  LATERAL jsonb_array_elements(policy.effects) effect WHERE policy.program_sha256=NEW.program_sha256
  AND policy.source_key=NEW.source_key AND effect.value->>'kind'='mint_item') THEN
  IF game_npc_source_output_complete(NEW.transaction_id) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'NPC physical reward requires its same-transaction native MINT child' USING ERRCODE='23514';END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_output_required AFTER INSERT ON game_npc_cosmetic_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_required();
CREATE FUNCTION game_npc_source_output_successor() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE item_id UUID;parent_id UUID;
BEGIN
 IF TG_OP='DELETE' THEN item_id:=OLD.item_instance_id;ELSE item_id:=NEW.item_instance_id;END IF;
 FOR parent_id IN SELECT output.parent_transaction_id FROM game_npc_source_item_outputs output
  WHERE output.item_instance_id=item_id AND output.created_xact_id=pg_current_xact_id() LOOP
  IF game_npc_source_output_complete(parent_id) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'NPC physical reward lost its exact native successor' USING ERRCODE='23514';END IF;
 END LOOP;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_output_successor AFTER INSERT OR UPDATE OR DELETE ON game_item_instances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_successor();
CREATE CONSTRAINT TRIGGER game_npc_source_output_successor AFTER INSERT OR UPDATE OR DELETE ON game_item_container_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_successor();
CREATE CONSTRAINT TRIGGER game_npc_source_output_successor AFTER INSERT ON game_npc_source_item_outputs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_successor();
CREATE FUNCTION game_npc_source_output_reservation_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE parent_id UUID;
BEGIN
 IF NEW.claim_production_key NOT LIKE 'oteryn:npc-source-output.%' THEN RETURN NULL;END IF;
 SELECT c.npc_transaction_id INTO parent_id FROM game_npc_source_context c
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.output_transaction_id=NEW.transaction_id;
 IF parent_id IS NULL OR game_npc_source_output_complete(parent_id) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'NPC physical reward cannot leave an orphan reservation' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_output_reservation_complete AFTER INSERT ON game_reward_claim_mint_reservations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_output_reservation_complete();
DO $completion_paths$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY['game_npc_source_output_complete(uuid)','game_npc_source_output_required()',
 'game_npc_source_output_successor()','game_npc_source_output_reservation_complete()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $completion_paths$;
