-- Explorer Society butterflies use their source Ground remains without loot containers.
DO $migration$
DECLARE
 body TEXT := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
 predecessor TEXT := '''oteryn:item.tibia.i33984'')';
BEGIN
 IF (length(body)-length(replace(body,predecessor,'')))/length(predecessor)<>1 THEN
  RAISE EXCEPTION 'unexpected installed plain corpse Ground admission';
 END IF;
 EXECUTE replace(body,predecessor,
  '''oteryn:item.tibia.i33984'',''oteryn:item.tibia.i4991'',''oteryn:item.tibia.i4992'',''oteryn:item.tibia.i4993'')');
END $migration$;
