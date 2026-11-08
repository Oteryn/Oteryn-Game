BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT 'QN1_ELEMENTAL_PG_READY',current_setting('server_version');
DO $case$
DECLARE fixture JSONB:='[{"altar":7738,"corpse":8139,"charged":2126,"creature":"oteryn:creature.energy_overlord","summon":[33095,32194,-13],"positions":[[33094,32189,-13],[33097,32189,-13],[33099,32191,-13],[33099,32194,-13],[33097,32196,-13],[33094,32196,-13],[33092,32194,-13],[33092,32191,-13]]},{"altar":7739,"corpse":8141,"charged":7745,"creature":"oteryn:creature.fire_overlord","summon":[33199,32103,-13],"positions":[[33198,32102,-13],[33201,32102,-13],[33203,32104,-13],[33203,32107,-13],[33201,32109,-13],[33198,32109,-13],[33196,32107,-13],[33196,32104,-13]]},{"altar":7740,"corpse":8140,"charged":7744,"creature":"oteryn:creature.ice_overlord","summon":[33286,32102,-13],"positions":[[33285,32097,-13],[33288,32097,-13],[33290,32099,-13],[33290,32102,-13],[33288,32104,-13],[33285,32104,-13],[33283,32102,-13],[33283,32099,-13]]},{"altar":7748,"corpse":8106,"charged":7746,"creature":"oteryn:creature.earth_overlord","summon":[33347,32208,-13],"positions":[[33346,32203,-13],[33349,32203,-13],[33351,32205,-13],[33351,32208,-13],[33349,32210,-13],[33346,32210,-13],[33344,32208,-13],[33344,32205,-13]]}]';
 r JSONB; cells JSONB; bad JSONB; i INTEGER; checks INTEGER:=0; expected_count INTEGER;
BEGIN
 SELECT count(*) INTO expected_count FROM game_elemental_arrival_rules;
 IF expected_count<>4 THEN RAISE EXCEPTION 'expected exactly four source recipes'; END IF;
 FOR r IN SELECT value FROM jsonb_array_elements(fixture) LOOP
   IF NOT EXISTS(SELECT 1 FROM game_elemental_arrival_rules WHERE altar_id=(r->>'altar')::integer
     AND corpse_key='oteryn:item.tibia.i'||(r->>'corpse') AND charged_appearance=(r->>'charged')::integer
     AND creature_key=r->>'creature' AND summon_position=r->'summon' AND positions=r->'positions')
   THEN RAISE EXCEPTION 'source recipe mismatch %',r->>'altar'; END IF;
   SELECT jsonb_agg(jsonb_build_object('key',n,'revision',0,'position',position,'appearance',r->'charged') ORDER BY n)
     INTO cells FROM jsonb_array_elements(r->'positions') WITH ORDINALITY AS p(position,n);
   IF NOT game_elemental_arrival_readset_valid((r->>'altar')::integer,999,cells)
   THEN RAISE EXCEPTION 'complete source ring refused %',r->>'altar'; END IF;
   checks:=checks+1;
   FOR i IN 0..7 LOOP
     bad:=jsonb_set(cells,ARRAY[i::text,'appearance'],to_jsonb((r->>'charged')::integer+1));
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'wrong charged cell % accepted',i; END IF;
     bad:=jsonb_set(cells,ARRAY[i::text,'position','0'],to_jsonb((cells->i->'position'->>0)::integer+1));
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'wrong position % accepted',i; END IF;
     bad:=jsonb_set(cells,ARRAY[i::text,'key'],to_jsonb(((i+1)%8)+1));
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'duplicate identity % accepted',i; END IF;
     bad:=jsonb_set(cells,ARRAY[i::text,'revision'],'-1');
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'negative revision accepted'; END IF;
     bad:=jsonb_set(cells,ARRAY[i::text,'revision'],'"0"');
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'string revision accepted'; END IF;
     bad:=jsonb_set(cells,ARRAY[i::text,'revision'],'0.5');
     IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'fractional revision accepted'; END IF;
     checks:=checks+6;
   END LOOP;
   IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,cells-0)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,999,cells||jsonb_build_array(cells->0))
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,1,cells)
     OR game_elemental_arrival_readset_valid(1,999,cells)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,NULL,cells)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,18446744073709551616,cells)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,0.5,cells)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,999,NULL)
     OR game_elemental_arrival_readset_valid((r->>'altar')::integer,999,'{}')
   THEN RAISE EXCEPTION 'incomplete, substituted or malformed witness accepted'; END IF;
   checks:=checks+9;
   bad:=jsonb_set(cells,ARRAY['0','revision'],'18446744073709551615');
   IF NOT game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'u64 maximum revision lost precision'; END IF;
   bad:=jsonb_set(cells,ARRAY['0','revision'],'18446744073709551616');
   IF game_elemental_arrival_readset_valid((r->>'altar')::integer,999,bad) THEN RAISE EXCEPTION 'overflowing revision accepted'; END IF;
   checks:=checks+2;
 END LOOP;
 IF has_table_privilege(current_user,'game_elemental_arrival_rules','INSERT,UPDATE,DELETE,TRUNCATE') THEN RAISE EXCEPTION 'runtime can alter recipe authority'; END IF;
 RAISE NOTICE 'QN1_ELEMENTAL_CASES_PASSED checks=%',checks;
END $case$;
RESET ROLE;
DO $case$
BEGIN
 BEGIN UPDATE game_elemental_arrival_rules SET charged_appearance=1 WHERE altar_id=7738;
   RAISE EXCEPTION 'recipe mutation was admitted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN DELETE FROM game_elemental_arrival_rules WHERE altar_id=7738;
   RAISE EXCEPTION 'recipe deletion was admitted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN TRUNCATE game_elemental_arrival_rules;
   RAISE EXCEPTION 'recipe truncation was admitted'; EXCEPTION WHEN check_violation THEN NULL; END;
END $case$;
COMMIT;
