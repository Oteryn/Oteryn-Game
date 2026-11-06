"""Local NPCFIX qualification; never stages or publishes. Resume with --from <check>."""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
CARGO = '/home/mole/oteryn-cache/codex-bin/cargo'
def cargo(*args):
    return [CARGO, *args]
packages = ['oteryn-gameplay-content', 'oteryn-gameplay-npc', 'oteryn-gameplay-dispatch', 'oteryn-gameplay-durability', 'oteryn-game-server']
pkg_args = [arg for package in packages for arg in ['-p', package]]
checks = [
    ('regeneration', ['python3', 'tools/content-migration/regenerate_content.py', '--resolve', '--no-stage']),
    ('fmt', cargo('fmt', '--all')),
    ('fmt-mounted', ['rustfmt', '--edition', '2024', 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_postage.rs']),
    ('check', cargo('check', '--offline', '--locked', *pkg_args, '--all-targets', '--all-features', '--quiet')),
    ('clippy', cargo('clippy', '--offline', '--locked', *pkg_args, '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings')),
    ('content-unit-npc', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-content', '--lib', 'npc', '--quiet')),
    ('native-content-unit', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-content', '--lib', 'native_gameplay', '--quiet')),
    ('npc-item-provider', ['python3', '-m', 'unittest', 'discover', '-s', 'tools/content-migration', '-p', 'test_npc_trade_item_profiles.py']),
    ('npc-unit', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-npc', '--lib', '--quiet')),
    ('durability-npc-unit', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-durability', '--lib', 'npc_trade', '--quiet')),
    ('npc-materializer', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--example', 'materialize_content_world_project_v2', 'npcfix', '--quiet')),
    ('content-npc-catalogue', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--test', 'content_npc_catalogue', '--quiet')),
    ('content-npc-service-model', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--test', 'content_npc_service_model', '--', '--nocapture')),
    ('content-world-repository', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--test', 'content_world_project_repository', '--quiet')),
    ('content-world-v2', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--test', 'content_world_project_v2', '--quiet')),
    ('integration-content', cargo('test', '--offline', '--locked', '-p', 'oteryn-game-server', '--test', 'integration_rest', 'content_', '--quiet')),
    ('dispatch-service-unit', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib', 'content_npc_service_tests', '--quiet')),
    ('dispatch-npc-unit', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib', 'gameplay_transport::npc_command', '--quiet')),
    ('dispatch-guardian-chat', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib', 'npcfix_guardian_chat', '--quiet')),
    ('postgres-npcfix-services', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib', 'npcfix_postgres_dispatch_services_and_replay', '--', '--nocapture')),
    ('postgres-paid-travel', cargo('test', '--offline', '--locked', '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib', 'npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals', '--', '--nocapture')),
    ('placements', ['python3', '-m', 'unittest', 'discover', '-s', 'tools/content-schema/npc-authoring', '-p', 'test_convert_placements.py']),
    # Built by the preceding wrapper cargo run; run separately so its cargo metadata child
    # can acquire the wrapper lane lock instead of deadlocking against its parent.
    ('architecture', ['/home/mole/oteryn-cache/target-codex/lanes/npcfix/debug/oteryn-architecture-check', 'workspace', '.']),
    ('pin-check', cargo('run', '--offline', '--locked', '--release', '-p', 'oteryn-world-bundle-compiler', '--quiet', '--', 'pin-check', '.', '.npcfix/pin-check')),
    ('world-npc-admission', cargo('run', '--offline', '--locked', '--release', '-p', 'oteryn-world-bundle-compiler', '--quiet', '--', 'parity', '.')),
    ('governance', ['python3', 'tools/agents/validate_governance.py']),
    ('governance-unit', ['python3', '-m', 'unittest', 'discover', '-s', 'tools/agents/tests']),
    ('fmt-check', cargo('fmt', '--all', '--', '--check')),
    ('fmt-mounted-check', ['rustfmt', '--edition', '2024', '--check', 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_postage.rs']),
    ('diff-check', ['git', 'diff', '--check', 'ed788c4bb1e2b420c8bfbd7a9bd68e36ea8ebfeb']),
]
parser=argparse.ArgumentParser();parser.add_argument('--from', dest='start');parser.add_argument('--only');parser.add_argument('--until');args=parser.parse_args()
env=os.environ.copy();env['PATH']=str(Path(CARGO).parent)+os.pathsep+env['PATH'];env['CARGO_NET_OFFLINE']='true';env['TMPDIR']=str(ROOT/'.npcfix/tmp')
active=args.start is None
for name, command in checks:
    if name == args.start: active=True
    if not active or (args.only and args.only!=name): continue
    log=ROOT/'.npcfix/logs'/f'qualified-{name}.log';started=time.monotonic()
    print(f'START {name}', flush=True)
    with log.open('w') as output:
        status=subprocess.run(command, cwd=ROOT, env=env, stdout=output, stderr=subprocess.STDOUT).returncode
    text=log.read_text()
    if name.startswith('postgres-') and status==0 and ('POSTGRES_STARTED' not in text or 'not configured' in text): status=99
    record={'check':name,'command':shlex.join(command),'status':'PASS' if status==0 else 'FAIL','exit_code':status,'seconds':round(time.monotonic()-started,1),'log':str(log.relative_to(ROOT))}
    with (ROOT/'.npcfix/checks.jsonl').open('a') as output:output.write(json.dumps(record)+'\n')
    print(f'{record["status"]} {name} ({record["seconds"]}s)', flush=True)
    if status:
        print('\n'.join(text.splitlines()[-24:]), flush=True)
        raise SystemExit(status)
    if args.until == name: break
