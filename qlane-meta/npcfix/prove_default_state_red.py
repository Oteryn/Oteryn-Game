from pathlib import Path
import subprocess
import json
p=Path('crates/gameplay/content/src/content/npc_catalogue/service.rs')
green=p.read_text();start=green.index('    if offer.direction',green.index('fn classify('));end=green.index('    if let Some(charges)',start)
red=green[:start]+green[end:]
assert red!=green and 'default_state_purchase(service, key' not in red
command=['/home/mole/oteryn-cache/codex-bin/cargo','test','--offline','--locked','-p','oteryn-game-server','--test','content_npc_catalogue','npcfix_retail_services_and_greeting_match_the_audited_repairs','--','--nocapture']
try:
    p.write_text(red)
    with Path('.npcfix/logs/default-state-red.log').open('w') as log:status=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT).returncode
finally:p.write_text(green)
text=Path('.npcfix/logs/default-state-red.log').read_text()
proved=status!=0 and all(f'oteryn:npc.{npc}: repaired offer must be executable' in text for npc in ['alesar', 'flint', 'romella', 'pugwah']) and 'could not compile' not in text
Path('.npcfix/default-state-red.json').write_text(json.dumps({'command':command,'exit_code':status,'expected_red':proved,'restored_exact_source':p.read_text()==green})+'\n')
print('\n'.join(text.splitlines()[-22:]));raise SystemExit(0 if proved else 1)
