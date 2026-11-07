DO $$ BEGIN
 IF game_lynda_wedding_current() THEN RAISE EXCEPTION 'unarmed pair context';END IF;RAISE NOTICE 'PASS unarmed pair current refused';
 IF game_lynda_wedding_grant_current('{}'::jsonb) IS TRUE THEN RAISE EXCEPTION 'unarmed pairedgrant';END IF;RAISE NOTICE 'PASS unarmed pair grant refused';
 IF game_lynda_wedding_grant_historical('01900000-0000-7000-8000-000000000040','oteryn:appearance-grant/npc/lynda/newly_wed/base','crystal-201189f-npc-cosmetics-v1','01900000-0000-7000-8000-000000000041',decode(repeat('00',32),'hex')) THEN RAISE EXCEPTION 'forged historical ancestry';END IF;RAISE NOTICE 'PASS missing reciprocal history refused';
 IF NOT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='game_lynda_wedding_receipts'::regclass AND attname='level_before' AND atttypid='bigint'::regtype) THEN RAISE EXCEPTION 'native BIGINT level omitted';END IF;RAISE NOTICE 'PASS exact native level stamp type';
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='game_lynda_wedding_character_chain' AND tgdeferrable AND tginitdeferred) THEN RAISE EXCEPTION 'one Root edge missing';END IF;RAISE NOTICE 'PASS immutable Wedding Root edge registered';
 IF has_table_privilege('oteryn_game_runtime','game_lynda_wedding_context','INSERT') THEN RAISE EXCEPTION 'runtime can forge private consent';END IF;RAISE NOTICE 'PASS pair context private';
END $$;
