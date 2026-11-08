-- Preserve Scavenge's exact original corpse provenance in the already-admitted
-- Instance death namespace. This grants no new skinning or Item MINT authority.
DO $$
DECLARE body TEXT; old TEXT;
BEGIN
    body:=pg_get_functiondef('game_corpse_source_race_admit()'::regprocedure);
    old:='g.channel_id=r.death_channel_id';
    IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
        RAISE EXCEPTION 'corpse race scope proof layout changed';
    END IF;
    EXECUTE replace(body,old,$scope$
        g.scope_kind=r.death_scope_kind AND (
            (g.scope_kind=1 AND g.channel_id=r.death_channel_id
                AND g.instance_id IS NULL AND r.death_instance_id IS NULL)
            OR (g.scope_kind=2 AND g.instance_id=r.death_instance_id
                AND g.channel_id IS NULL AND r.death_channel_id IS NULL))
    $scope$);
END $$;
