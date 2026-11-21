"""Real-PG qualification of the exact M42 receipt-key function from migration2080."""
import os
from pathlib import Path
import re
import subprocess
import time
import uuid


def main():
    root = Path(__file__).resolve().parents[2]
    migration = (root / 'apps/game-server/migrations/2080_quest_utc_cooking_receipts.sql').read_text()
    function = re.search(r'CREATE FUNCTION game_quest_npc_reward_claim_key\(.*?END \$\$;', migration, re.S)
    if function is None:
        raise SystemExit('M42 receipt-key function missing')
    configured = os.environ.get('OTERYN_TEST_POSTGRES_ADMIN_URL')
    if not configured:
        raise SystemExit('M42 real-PG case requires OTERYN_TEST_POSTGRES_ADMIN_URL; no skip')
    schema = 'm42_receipt_' + uuid.uuid4().hex
    sql = f'BEGIN; CREATE SCHEMA {schema}; SET LOCAL search_path={schema},pg_catalog;\n' + function[0]
    sql += r'''
SELECT 'M42_RECEIPT_SQL POSTGRES_STARTED version='||current_setting('server_version');
DO $$
DECLARE total BIGINT; plan JSONB; key TEXT := 'oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-6';
BEGIN
 IF current_setting('server_version') NOT LIKE '18.6%' THEN RAISE EXCEPTION 'PostgreSQL18.6 required'; END IF;
 IF game_quest_npc_reward_claim_key('{"quest":"oteryn:quest.other"}', 'oteryn:test/once')
    <> 'oteryn:reward-claim.npc-program.'||encode(sha256(convert_to('oteryn:test/once','UTF8')),'hex')
 THEN RAISE EXCEPTION 'Existing once-only reward identity changed'; END IF;
 SELECT count(DISTINCT game_quest_npc_reward_claim_key(jsonb_build_object(
    'quest','oteryn:quest.hot_cuisine_quest','tracks',jsonb_build_array(jsonb_build_object(
    'key','oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-7',
    'before',n-1,'after',n))),key)) INTO total FROM generate_series(1,10000) n;
 IF total<>10000 THEN RAISE EXCEPTION 'Permitted cooking occurrences share a reward identity'; END IF;
 plan:=jsonb_build_object('quest','oteryn:quest.hot_cuisine_quest','tracks',jsonb_build_array(jsonb_build_object(
    'key','oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-7','before',0,'after',0)));
 BEGIN
    PERFORM game_quest_npc_reward_claim_key(plan,key);
    RAISE EXCEPTION 'Non-advancing cook counter admitted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
    PERFORM game_quest_npc_reward_claim_key(plan,'oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-16');
    RAISE EXCEPTION 'Unknown cooking recipe admitted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 plan:=jsonb_set(plan,'{tracks,0,after}','1');
 plan:=jsonb_set(plan,'{tracks}',(plan->'tracks')||(plan->'tracks'));
 BEGIN
    PERFORM game_quest_npc_reward_claim_key(plan,key);
    RAISE EXCEPTION 'Ambiguous cooking counter admitted';
 EXCEPTION WHEN too_many_rows THEN NULL; END;
END $$;
SELECT 'M42_RECEIPT_SQL native_unique_occurrences=10000 legacy_once=preserved malformed=refused';
ROLLBACK;
'''
    env = os.environ.copy()
    started = time.monotonic()
    result = subprocess.run(['psql', '--dbname', configured, '-X', '-v', 'ON_ERROR_STOP=1', '-q', '-A', '-t'],
                            input=sql, text=True, capture_output=True, env=env)
    seconds = time.monotonic() - started
    print(result.stdout.strip())
    if result.returncode:
        print(result.stderr.strip())
        raise SystemExit(result.returncode)
    if seconds <= 0.01:
        raise SystemExit('M42 PG duration proof was unexpectedly below0.01s')
    print(f'M42_RECEIPT_SQL POSTGRES_CASE_COMPLETED duration={seconds:.9f}s success=true')


if __name__ == '__main__':
    main()
