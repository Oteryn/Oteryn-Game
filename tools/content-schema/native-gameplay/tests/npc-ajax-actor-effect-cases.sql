DO $$ BEGIN
 IF (SELECT count(*) FROM game_npc_source_policies WHERE source_key='oteryn:npc-cosmetic/ajax/barbarian-fire' AND enabled)<>1 THEN RAISE EXCEPTION 'exact Ajax policy not enabled'; END IF;
 IF EXISTS(SELECT 1 FROM game_npc_source_policies WHERE enabled AND effects @> '[{"kind":"condition_damage"}]'::jsonb AND source_key<>'oteryn:npc-cosmetic/ajax/barbarian-fire') THEN RAISE EXCEPTION 'foreign condition policy enabled'; END IF;
 RAISE NOTICE 'PASS exact Ajax FIRE policy only';
END $$;
DO $$ BEGIN
 IF has_table_privilege('oteryn_game_runtime','game_npc_source_actor_snapshot_proofs','INSERT') OR has_table_privilege('oteryn_game_runtime','game_npc_source_context','INSERT') THEN RAISE EXCEPTION 'runtime private observation write'; END IF;
 IF game_npc_source_effect_current('00000000-0000-0000-0000-000000000001','oteryn:npc-cosmetic/ajax/barbarian-fire','{"kind":"condition_damage","condition":"FIRE","delayed":true,"ticks":10,"interval_ms":2000,"damage_per_tick":10}'::jsonb) THEN RAISE EXCEPTION 'unarmed source effect accepted'; END IF;
 RAISE NOTICE 'PASS private context and unarmed actor effect refusal';
END $$;
DO $$ BEGIN
 BEGIN
 INSERT INTO game_npc_source_actor_effects VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003',1,'oteryn:npc-cosmetic/ajax/barbarian-fire',decode(repeat('00',32),'hex'),2,'{}',pg_current_xact_id());
 RAISE EXCEPTION 'unarmed child unexpectedly accepted';
 EXCEPTION WHEN raise_exception THEN IF SQLERRM<>'NPC actor effect lacks exact source/Node/actor/parent proof' THEN RAISE; END IF; END;
 RAISE NOTICE 'PASS actual unarmed child insertion rejected before foreign key';
END $$;
DO $$ BEGIN
 IF has_function_privilege('oteryn_game_runtime','game_npc_source_actor_effect_admit()','EXECUTE') THEN RAISE EXCEPTION 'runtime can call private actor trigger'; END IF;
 RAISE NOTICE 'PASS private actor admission helper privilege';
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='game_npc_source_actor_snapshot_complete' AND tgdeferrable AND tginitdeferred AND (tgtype::int & 28)=28) THEN RAISE EXCEPTION 'snapshot successor mutation completeness missing'; END IF;
 RAISE NOTICE 'PASS snapshot insert update delete deferred completeness';
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='game_npc_source_ajax_child_required' AND tgdeferrable AND tginitdeferred) THEN RAISE EXCEPTION 'reciprocal parent completeness missing'; END IF;
 RAISE NOTICE 'PASS parent reciprocal child completeness';
END $$;
