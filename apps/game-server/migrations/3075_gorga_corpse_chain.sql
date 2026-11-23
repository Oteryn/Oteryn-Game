-- Crystal 201189f8 data/items/items.xml:25914-25933.
-- The existing corpse scheduler supplies the exact 10s/300s/300s/60s durations.
DO $$ DECLARE body TEXT; anchor TEXT := ' SELECT CASE p_key'; BEGIN
 body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
  RAISE EXCEPTION 'Crystal corpse registry anchor changed';
 END IF;
 EXECUTE replace(body,anchor,anchor || $branches$
  WHEN 'oteryn:item.tibia.i9607' THEN ARRAY[9607,9608,9609,9610]$branches$);
END $$;
