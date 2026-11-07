-- Actual production BEFORE INSERT trigger, with a historical worn legion helmet.
-- Deferred completion intentionally remains uncommitted: the complete real NPC
-- payment/replay is separately exercised by the framed Norma scenario.
BEGIN;
SET LOCAL session_replication_role=replica;
UPDATE game_durability_reconnect_sessions SET current_transport_ref=decode(repeat('ab',16),'hex')
 WHERE game_session_id='00000000-0000-7000-8000-000000000040';
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES('00000000-0000-7000-8000-000000000081','00000000-0000-7000-8000-000000000042','Item','oteryn:item.tibia.i3374','rev-1',1,1,'00000000-0000-7000-8000-000000000082');
INSERT INTO game_character_equipment_state(character_id,revision) VALUES('00000000-0000-7000-8000-000000000041',7);
INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id)
 VALUES('00000000-0000-7000-8000-000000000041',1,'00000000-0000-7000-8000-000000000081','00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000083');
SET LOCAL session_replication_role=origin;
SET LOCAL ROLE oteryn_game_runtime;
INSERT INTO game_srv7_item_costs(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,connection_generation,lease_generation,scope_generation,cause_kind,source_key,requirements,inputs,occurred_at,proof_sha256)
 SELECT '00000000-0000-7000-8000-000000000084','00000000-0000-7000-8000-000000000085','00000000-0000-7000-8000-000000000040',700,
 '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,1,1,'npc_cosmetic','oteryn:npc.norma',
 jsonb_build_array(jsonb_build_object('family','Item','key',i.definition_production_key,'revision',i.definition_revision_ref,'count',1)),
 jsonb_build_array(jsonb_build_object('item',i.item_instance_id,'before',to_jsonb(i),'location',c.location,'kind',c.kind,'count',1)),1,decode(repeat('00',32),'hex')
 FROM game_item_instances i JOIN game_quest_callback_carried('00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000042') c ON c.item=i.item_instance_id
 WHERE i.item_instance_id='00000000-0000-7000-8000-000000000081';
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_srv7_item_costs r WHERE r.command_id=700 AND r.equipment_revision_before=7
  AND r.created_xact_id=pg_current_xact_id() AND r.proof_sha256=sha256(convert_to(game_srv7_cost_basis(r)::text,'UTF8')))
 THEN RAISE EXCEPTION 'actual equipment-cost admission did not stamp exact predecessor'; END IF;
 RAISE NOTICE 'EQUIPMENT_COST_ADMISSION_PASSED actual_trigger=true revision_before=7';
END $$;
ROLLBACK;
