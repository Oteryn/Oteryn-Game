\set ON_ERROR_STOP on
SET search_path=spell_house_privacy_audit,pg_catalog;
-- The runner installs the exact Rust receipt predicate as test_due_paid_match.
-- This fixture tests only committed-source SQL evidence, never session/timer authority.
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,caster_lease_generation,caster_placement_digest,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms)
SELECT '019a0000-0000-7000-8000-000000000090','019a0000-0000-7000-8000-000000000091','019a0000-0000-7000-8000-000000000030',9000,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000002','019a0000-0000-7000-8000-000000000004',1,3,decode(repeat('22',16),'hex'),'Spell','oteryn:spell.chain-test','r21',decode(repeat('11',32),'hex'),sha256(convert_to('{}','UTF8')),convert_to('{}','UTF8'),cost,sha256(cost),0,1
FROM (SELECT convert_to(jsonb_build_object('version',1,'before_revision',1,'after_revision',2,'mana_before',100,'mana_after',90,'soul_before',100,'soul_after',100,'cooldowns_before','[]'::jsonb,'cooldowns_after','[["spell:test",1000]]'::jsonb,'caster_before',to_jsonb(array_fill(1,ARRAY[32])),'caster_after',to_jsonb(array_fill(2,ARRAY[32])))::text,'UTF8') AS cost) AS fixture;
SET LOCAL session_replication_role=origin;
CREATE FUNCTION test_due_expected(command TEXT DEFAULT '9000',target_character BYTEA DEFAULT decode('019a0000000070008000000000000011','hex'),world BYTEA DEFAULT decode('019a0000000070008000000000000002','hex'),channel BYTEA DEFAULT decode('019a0000000070008000000000000004','hex'),generation TEXT DEFAULT '1',lease TEXT DEFAULT '3',placement BYTEA DEFAULT decode(repeat('22',16),'hex'),content BYTEA DEFAULT decode(repeat('11',32),'hex'),spell TEXT DEFAULT 'oteryn:spell.chain-test',revision TEXT DEFAULT 'r21') RETURNS BOOLEAN LANGUAGE SQL AS $$
 SELECT test_due_paid_match(decode('019a0000000070008000000000000030','hex'),command,target_character,world,channel,generation,lease,placement,content,spell,revision)
$$;
DO $$ BEGIN
 IF test_due_expected() THEN RAISE EXCEPTION 'uncommitted current transaction authorized due callback'; END IF;
END $$;
COMMIT;
DO $$ BEGIN
 IF NOT test_due_expected() THEN RAISE EXCEPTION 'genuine committed source receipt rejected'; END IF;
 IF test_due_expected(command=>'9001') THEN RAISE EXCEPTION 'missing source receipt accepted'; END IF;
 IF test_due_expected(target_character=>decode('019a0000000070008000000000000012','hex')) THEN RAISE EXCEPTION 'source Character substitution accepted'; END IF;
 IF test_due_expected(world=>decode('019a0000000070008000000000000003','hex')) THEN RAISE EXCEPTION 'source World substitution accepted'; END IF;
 IF test_due_expected(channel=>decode('019a0000000070008000000000000005','hex')) THEN RAISE EXCEPTION 'source Channel substitution accepted'; END IF;
 IF test_due_expected(generation=>'2') THEN RAISE EXCEPTION 'stale owner generation accepted'; END IF;
 IF test_due_expected(lease=>'4') THEN RAISE EXCEPTION 'stale Character lease accepted'; END IF;
 IF test_due_expected(placement=>decode(repeat('23',16),'hex')) THEN RAISE EXCEPTION 'different physical actor accepted'; END IF;
 IF test_due_expected(content=>decode(repeat('12',32),'hex')) THEN RAISE EXCEPTION 'different active artifact accepted'; END IF;
 IF test_due_expected(spell=>'oteryn:spell.other') THEN RAISE EXCEPTION 'different source spell accepted'; END IF;
 IF test_due_expected(revision=>'r22') THEN RAISE EXCEPTION 'different source revision accepted'; END IF;
END $$;
SELECT 'due read exact Rust SQL predicate: committed-source/current-XID/missing/Character/World/Channel/generation/lease/placement/artifact/spell/revision: PASS' AS result;
