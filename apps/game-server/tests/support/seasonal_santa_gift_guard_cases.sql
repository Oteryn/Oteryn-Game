DO $case$ DECLARE v RECORD; actual INTEGER; BEGIN
 FOR v IN SELECT * FROM (VALUES
(1::numeric,210,6,5),
(1::numeric,211,5,4),
(2::numeric,210,6,0),
(2::numeric,211,5,3),
(9223372036854775807::numeric,210,6,2),
(9223372036854775807::numeric,211,5,3),
(9223372036854775808::numeric,210,6,4),
(9223372036854775808::numeric,211,5,0),
(18446744073709551615::numeric,210,6,2),
(18446744073709551615::numeric,211,5,4)) AS cases(command,domain,upper,expected) LOOP
  actual:=game_santa_gift_draw('00000000-0000-0000-0000-000000000001',v.command,'00000000-0000-0000-0000-000000000002',v.domain,v.upper);
  IF actual IS DISTINCT FROM v.expected THEN RAISE EXCEPTION 'Santa RNG vector failed command % domain %: % vs %',v.command,v.domain,actual,v.expected; END IF;
 END LOOP;
 RAISE NOTICE 'SRV7_SANTA_GIFT_RNG_CASES_PASSED cases=10';
END $case$;
DO $case$ DECLARE r game_item_consume_transform_records; BEGIN
 r.effects:='{"kind":"consume_effect","source_item_id":5080,"donor":"crystal-201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/dolls.lua:2,15-21,111-149","speech":"Hug me!","cues":[],"private_speech":false,"require_head":false,"require_player_target":false,"achievement_progress":[]}'::jsonb;
 IF game_santa_gift_effects_valid(r) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'Santa panda source speech rejected'; END IF;
 r.effects:=jsonb_set(r.effects,'{speech}','"Wrong speech"');
 IF game_santa_gift_effects_valid(r) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'Santa panda forged speech admitted'; END IF;
 r.effects:='{"kind":"consume_effect","source_item_id":6566,"donor":"crystal-201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/dolls.lua:2,15-21,111-149","speech":"You... will.... burn!!","damage":1,"cues":[{"binding":"appearance:effect/explosionhit","effect_id":6}],"private_speech":false,"require_head":false,"require_player_target":false,"achievement_progress":[]}'::jsonb;
 IF game_santa_gift_effects_valid(r) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'Santa dragon source branch rejected'; END IF;
 r.effects:=jsonb_set(r.effects,'{damage}','2');
 IF game_santa_gift_effects_valid(r) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'Santa dragon forged damage admitted'; END IF;
 RAISE NOTICE 'SRV7_SANTA_GIFT_EFFECT_CASES_PASSED cases=4';
END $case$;
