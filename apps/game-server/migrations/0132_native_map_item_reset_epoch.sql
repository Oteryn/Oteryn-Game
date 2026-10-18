-- D1451: a world reset retires the source-map Items it finds on Ground. The
-- source identity is therefore scoped by the active reset epoch, so the first
-- initialization after activation mints that epoch's source set once while the
-- retired epoch's receipts stay immutable history.
ALTER TABLE game_native_map_item_receipts
 ADD COLUMN reset_epoch NUMERIC(20,0) NOT NULL DEFAULT 0
 CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615);
DO $$ DECLARE c TEXT; BEGIN
 SELECT conname INTO STRICT c FROM pg_constraint
  WHERE conrelid='game_native_map_item_receipts'::regclass AND contype='u'
   AND conkey=ARRAY(SELECT attnum FROM unnest(ARRAY['world_id','channel_id','content_digest','placement_key']) WITH ORDINALITY n(a,o)
    JOIN pg_attribute ON attrelid='game_native_map_item_receipts'::regclass AND attname=n.a ORDER BY o)::int2[];
 EXECUTE format('ALTER TABLE game_native_map_item_receipts DROP CONSTRAINT %I',c);
END $$;
ALTER TABLE game_native_map_item_receipts
 ADD CONSTRAINT game_native_map_item_receipts_epoch_placement_key
 UNIQUE(world_id,channel_id,reset_epoch,content_digest,placement_key);
CREATE OR REPLACE FUNCTION game_native_map_item_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE j JSONB; e JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF TG_TABLE_NAME='game_native_map_item_receipts' THEN
  -- A whole-row writer predating this column leaves it NULL; that is epoch 0,
  -- which the active-epoch check below still has to accept.
  NEW.reset_epoch:=COALESCE(NEW.reset_epoch,0);
  j:=convert_from(NEW.source_intent,'UTF8')::jsonb;
  -- Built in an assignment: a CASE inside the IF condition would end it at its THEN.
  e:=jsonb_build_object('schema','OTERYN_NATIVE_MAP_ITEM_INIT/v1',
    'world',encode(uuid_send(NEW.world_id),'hex'),'channel',encode(uuid_send(NEW.channel_id),'hex'),
    'scope_generation',NEW.scope_generation::text,'content_digest',encode(NEW.content_digest,'hex'),
    'map_digest',encode(NEW.map_digest,'hex'),'frame_digest',encode(NEW.frame_digest,'hex'),
    'placement_key',NEW.placement_key,'definition_family',NEW.definition_family,'definition_key',NEW.definition_key,
    'definition_revision',NEW.definition_revision,'quantity',NEW.quantity,
    'spatial_position',encode(NEW.spatial_position,'hex'),'source_binding',encode(NEW.source_binding,'hex'),
    'source_attributes',NEW.source_attributes,'blocks_movement',NEW.blocks_movement,
    'blocks_projectile',NEW.blocks_projectile,'immovable_block_solid',NEW.immovable_block_solid,'owner_kind',NEW.owner_kind)
    || CASE WHEN NEW.reset_epoch>0 THEN jsonb_build_object('reset_epoch',NEW.reset_epoch::text) ELSE '{}'::jsonb END;
  IF j IS DISTINCT FROM e THEN
   RAISE EXCEPTION 'map source intent does not bind exact receipt' USING ERRCODE='23514';
  END IF;
  -- Reset activation holds the admission relations EXCLUSIVE; the initializer
  -- holds them too, so this read sees the epoch that admitted the mint.
  IF NEW.reset_epoch IS DISTINCT FROM COALESCE((SELECT a.reset_epoch FROM game_world_reset_active_epochs a WHERE a.world_id=NEW.world_id),0) THEN
   RAISE EXCEPTION 'map source receipt is not in the active reset epoch' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key)
    WHERE s.world_id=NEW.world_id AND s.channel_id=NEW.channel_id AND s.state=1
     AND s.ownership_generation=NEW.scope_generation AND s.holder_node_id=NEW.holder_node_id
     AND s.holder_registration_revision=NEW.holder_registration_revision AND g.ready AND g.ownership_generation=s.ownership_generation) THEN
   RAISE EXCEPTION 'map source has no current ready assignment' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NEW;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_native_map_item_stamp() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_native_map_item_stamp() FROM PUBLIC;
