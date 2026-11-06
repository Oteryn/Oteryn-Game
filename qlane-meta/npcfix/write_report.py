"""Summarize every audited DIFF/MISSING row using current local evidence."""
import collections
import json
import re
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[1]
BASE='ed788c4bb1e2b420c8bfbd7a9bd68e36ea8ebfeb'
rows=json.loads((ROOT/'.npcfix/audit-scope.json').read_text())
packet=json.loads((ROOT/'tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json').read_text())
repairs={r['after']['identity']['key']:r for r in packet['repairs']}
placements={}
for p in (ROOT/'content/world/npc-placements').glob('placements-*.json'):
    for i,r in enumerate(json.loads(p.read_text())['records']):placements.setdefault(r['declaration']['npc'],[]).append(f'{p.relative_to(ROOT)}:1#/records/{i}')
held={r['npc']:r['reason'] for r in json.loads((ROOT/'content/world/npc-placements/held.json').read_text())['held']}
declarations={}
for folder in ['npcs/definitions','dialogues/definitions','services/trade','services/travel']:
    for p in (ROOT/'content'/folder).glob('*.json'):
        for i,r in enumerate(json.loads(p.read_text()).get('records',[])):
            d=r.get('declaration',{})
            if d.get('identity'):declarations[d['identity']['key']]=(d,f'{p.relative_to(ROOT)}:1#/records/{i}')
remaining={
'a_sweaty_cyclops':'Placement is repaired. The audit leaves exact retail smithing eligibility/effects unresolved; verified quest exchanges must be routed to the excluded quest owner.',
'ahmet':'Oil-filled Vial20 is not executable: fluid mint provenance and subtype trade support need the existing fluid owner contract; this package adds placement only.',
'rascalio':'Nimmersatt event-conditioned Banor\'s Eye transport needs an admitted event selector and return route.',
'captain_jack':'Tibia return depends on Costello permission and monastery fines; no qualified native predicate. Quest-lane repair required.',
'altar':'Crimson Offering/Bakragore eligibility and reward transaction require the excluded quest lane.',
'petros':'Eight ordinary destinations repaired. Travora remains held: daily ticket and Captain Tiberius last-port return owner are absent.',
'a_fading_memory':'Ghost\'s Tear reward is a Blood Brothers quest transaction; excluded quest-lane repair required.',
'a_strange_chalice':'Forgotten Knowledge quest topic/actions need the excluded quest lane.',
'a_wandering_soul':'Blood Crystal enchantment is a Blood Brothers quest transaction; excluded quest-lane repair required.',
'cranky_lizard_crone':'Affirmative cape/hat exchanges require Wayfarer quest transitions; excluded quest-lane repair required.',
'alyxo':'Kilmaresh quest dialogue/progression needs the excluded quest lane.',
'gnomission':'Rank4 Warzone access needs Bigfoot quest predicates and action owner; excluded quest-lane repair required.',
'al_dee':'Pick Quest affirmative exchange requires the excluded quest lane.',
'a_restless_soul':'Spirit-language greeting and story progression require the Ice Islands quest owner; do not grant speech unconditionally.',
'anna':'Retail Targuna continuation conflicts with the existing owner-selected landing and needs tutorial/quest transition and destination admission; coordinator required.',
'a_behemoth':'Spike quest-conditioned greetings require the excluded quest lane.',
'a_strange_fellow':'Postman progression dialogue/actions require the excluded quest lane.',
'a_weakened_forest_fury':'Forgotten Knowledge guarded interaction requires the excluded quest lane.',
'emberwing':'Book World scene shares its exact cell with Grovebeast; no admitted mutually exclusive scene selector.',
'grovebeast_npc':'Book World scene shares its exact cell with Emberwing; no admitted mutually exclusive scene selector.',
'ghost_captain':'Conditional Ghost Ship scene has no admitted event selector; unconditional placement would invent availability.',
'hoaxette':'April event availability has no admitted selector; unconditional placement would invent availability.',
'minzy':'Bewitched event NPC; no admitted availability selector.',
'doctor_marrow':'Identity added; D17 map-owner conflict remains between Wiki and reviewed map coordinates. Placement is held, not guessed.',
'reference_deseus':'Audit explicitly leaves current availability unresolved; no exact current-playable identity/placement evidence.',
}
alias={'reference_ellipsis':'ellipsis','yielothax_npc':'yielothax'}
admission_path = ROOT/'.npcfix/logs/qualified-world-npc-admission.log'
admission = None
if admission_path.exists():
    raw = admission_path.read_text()
    start = re.search(r'(?m)^\{', raw)
    if start:
        admission, _ = json.JSONDecoder().raw_decode(raw[start.start():])
        for placement in admission.get('npcs', {}).get('held', []):
            native_slug = placement['npc'].removeprefix('oteryn:npc.')
            slug = next((row['slug'] for row in rows if alias.get(row['slug'], row['slug']) == native_slug), native_slug)
            reason = f"Compiled placement held at {placement['cell']}: {placement['reason']}; native map/admission dependency remains."
            remaining[slug] = (remaining.get(slug, '') + ' ' + reason).strip()
        for route in admission.get('npcs', {}).get('routes_held', []):
            slug = route['service'].removeprefix('oteryn:service.travel.')
            if slug in {'petros', 'captain_max', 'captain_seahorse', 'tarak_inner', 'tarak_sunken'}:
                reason = f"Compiled travel destination held for {route['route']}: {route['reason']}; native map/admission dependency remains."
                remaining[slug] = (remaining.get(slug, '') + ' ' + reason).strip()
fixes={
'alesar':'29 retail offers, including a full-state Ice Rapier purchase; existing Efreet mission03=3 gate enforced in window and durable trade.',
'nah_bob':'32 retail offers including default-state Ice Rapier buyback; existing Marid mission03=3 gate enforced in window and durable sale.',
'rashid':'155 retail offers including seven default-state amulet buybacks; mission07 gate and seven weekday placements matching the existing runtime selector.',
'liane':'Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation.',
'benjamin':'Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation.',
'ray':'Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation.',
'robson':'Parcel15 and Label1 service; parcel/label keyword aliases; no Postman discount.',
'kasmir':'Heal to40 and cure fire/poison/energy through the canonical vitals/condition owner.',
'muzir':'Connect existing native bank service and source dialogue.',
'murim':'Connect native bank service alongside postal shop; conditional postage.',
'znozel':'Connect existing native bank service and source dialogue.',
'petros':'Wiki fares and destinations for eight ordinary routes; carried scarab-cheese guard. Travora source row retained held.',
'captain_max':'Yalahar400 route with started-Yalahar quest gate; correct scene identities; no Postman discount.',
'captain_seahorse':'Enable Venore40 with carried scarab-cheese guard.',
'tarak_inner':'Monument Tower50 route; passage/trip keyword aliases; displayed name Tarak.',
'tarak_sunken':'Free return passage; displayed name Tarak.',
'dixi':'Buy Chain Helmet from player for17 instead of12.',
'lubo':'Sell Torch for2.',
'flint':'Add three exercise-wrap offers and buy Sword25; qualify full default charge purchases.',
'romella':'Add three exercise-wrap offers; qualify full default charge purchases.',
'shiantis':'Add sixteen Wiki-confirmed pillow/pick offers.',
'pompan':'Add ten Wiki-confirmed ammunition, weapon and flag offers.',
'pugwah':'Sell empty Mug4 using the existing default empty state.',
'albinius':'Buy Heavy Old Tome30; compose its existing canonical policy into the active provider, preserving the gated blank-scroll purchase.',
'a_ghostly_guardian':'Retail greeting/name substitution, farewell and eight keyword aliases; preserve all multipart lines through command and ordinary chat.',
'zirkon':'Add the recorded sacred-place greeting.',
'toffee':'Add seven candy buyback offers, including Pastry Dragon95000.',
}
for postal in ['chemar', 'chrystal', 'fyodor', 'lokur', 'murim', 'nelly', 'olrik', 'wally']:
    fixes[postal] = fixes.get(postal, '') + ' Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer.'
fixes['lokur'] += ' Connect the native postal service (Letter, Parcel and Label).'
checks=[]
cp=ROOT/'.npcfix/checks.jsonl'
if cp.exists():
    latest={}
    for line in cp.read_text().splitlines():
        r=json.loads(line);latest[r['check']]=r
    checks=list(latest.values())
required=['regeneration','fmt','fmt-mounted','check','clippy','content-unit-npc','native-content-unit','npc-item-provider','npc-unit','durability-npc-unit','npc-materializer','content-npc-catalogue','content-npc-service-model','content-world-repository','content-world-v2','integration-content','dispatch-service-unit','dispatch-npc-unit','dispatch-guardian-chat','postgres-npcfix-services','postgres-paid-travel','placements','architecture','pin-check','world-npc-admission','governance','governance-unit','fmt-check','fmt-mounted-check','diff-check']
qualified=all(any(c['check']==name and c['status']=='PASS' for c in checks) for name in required)
results=[]
for row in rows:
    slug=row['slug'];key='oteryn:npc.'+alias.get(slug,slug); parts=[];files=[];tests=[]
    excluded=slug == 'reference_deseus' or any(x in ' '.join(row['differences']) for x in ['historical/deprecated','test-server-only'])
    if excluded:
        disposition='EXCLUDED';parts=['Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs.']
    elif slug in ['razan','shalmar']:
        disposition='SUPERSEDED';parts=['Paid spell teaching removed in retail15.22 (2026-01-27); preserve existing automatic spell acquisition.']
        files=['crates/gameplay/dispatch/src/gameplay_transport/automatic_spell_progression.rs:1'];tests=['dispatch_planner_gates_level_vocation_and_explicit_exclusions']
    else:
        if key in placements:parts.append('Retail placement authored.');files.extend(placements[key][:1]);tests.append('test_npcfix_retail_population_has_each_audited_cell')
        if row['status']=='MISSING' and key in declarations:
            parts.append('Add exact displayed identity.');files.append(declarations[key][1]);tests.append('npcfix_retail_services_and_greeting_match_the_audited_repairs')
        if slug in fixes:parts.append(fixes[slug]);tests.append('npcfix_retail_services_and_greeting_match_the_audited_repairs')
        if slug == 'kasmir':
            files.append('crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs:760')
            tests.append('npcfix_postgres_dispatch_services_and_replay')
        if slug in ['alesar', 'nah_bob', 'rashid', 'flint', 'romella', 'pugwah', 'albinius', 'a_ghostly_guardian', 'benjamin', 'liane', 'ray', 'robson']:
            tests.append('npcfix_postgres_dispatch_services_and_replay')
        if slug == 'a_ghostly_guardian':
            tests.append('npcfix_guardian_chat_preserves_name_and_multipart_source_order')
        if slug in ['petros', 'tarak_inner', 'tarak_sunken', 'captain_max', 'captain_seahorse']:
            tests.append('npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals')
            route_path = 'crates/gameplay/content/src/content/npc_catalogue/travel_routes.json'
            route_line = next((i for i, line in enumerate((ROOT/route_path).read_text().splitlines(), 1) if f'oteryn:service.travel.{slug}/' in line), 1)
            files.append(f'{route_path}:{route_line}')
        if key in declarations:
            npc=declarations[key][0]
            for ref in [npc.get('dialogue'),*npc.get('services',[])]:
                if ref and ref['key'] in repairs and ref['key'] in declarations:files.append(declarations[ref['key']][1])
        if slug in remaining:
            disposition='REMAINING';parts.append(remaining[slug])
        else:disposition='FIXED' if qualified else 'AUTHORED; QUALIFICATION PENDING'
        if not parts:parts=['Existing authored placement retained; see audit evidence.']
    results.append((row,disposition,' '.join(parts),'; '.join(dict.fromkeys(files)) or row['research_file']+':1', '; '.join(dict.fromkeys(tests)) or 'Source review'))
counts=collections.Counter(r[1] for r in results)
lines=['# NPCFIX report','',f'Branch: `{subprocess.check_output(["git","branch","--show-current"],cwd=ROOT,text=True).strip()}`; base `{BASE}`. AUTHORING, local changes only. No staging, commits, pushes, resets, checkout, stash, credential access, dependency downloads or external publication performed.','',f'Audit inventory: {len(rows)} rows (288 DIFF, 32 MISSING). Historical/test-server references excluded only where the audit says so; Dawnport/W14/P23/X01-X06 are outside the selected audit rows. Current result counts: '+', '.join(f'{v} {k}' for k,v in counts.items())+'.','', f'External WIP snapshots were observed during this continuation; current HEAD is `{subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip()}`. This worker invoked no Git mutation. All package diffs/file inventories use the original task base above.','', 'This is a partial package while any eligible row below remains unresolved. Authored source presence is distinguished from a running production actor; no live deployment was performed.','', '## Per-NPC results','', '| npc | town | audit status | fix | file:line | test | result |','| --- | --- | --- | --- | --- | --- | --- |']
for row,status,fix,files,tests in results:
    values=[row['name'],row['town'],row['status'],fix,files,tests,status]
    lines.append('| '+' | '.join(v.replace('|','\\|').replace('\n',' ') for v in values)+' |')
if admission:
    npc_report = admission['npcs']
    lines += ['', '## Bundle NPC admission', '', f"Compiler inspected {npc_report.get('placements')} authored placement records; held {len(npc_report.get('held', []))}. Exact compiler evidence: `.npcfix/logs/qualified-world-npc-admission.log`. Source-only holds are recorded separately in `content/world/npc-placements/held.json`."]
lines+=['','## Unfinished','']
for row,status,fix,files,tests in results:
    if status=='REMAINING':lines.append(f'- **{row["name"]} ({row["town"]})**: {remaining[row["slug"]]}')
lines+=['','No quest-lane module or `content/quests/**` was edited. The coordinator must route the explicit quest dependencies above; event, map and ticket ownership gaps are not treated as fixed by adding unconditional actors.','', '## Checks','', '| check | result | command | log |','| --- | --- | --- | --- |']
byname={c['check']:c for c in checks}
for name in required:
    c=byname.get(name)
    lines.append('| '+name+' | '+(c['status'] if c else 'NOT RUN')+' | '+('`'+c['command']+'`' if c else 'See `.npcfix/run_checks.py`')+' | '+(c['log'] if c else '')+' |')
lines+=['','Regression RED evidence: `.npcfix/logs/placement-red.log`, `.npcfix/logs/services-red.log`, `.npcfix/logs/albinius-red.log`; intermediate failed build/regeneration logs are retained and are not final qualification. PostgreSQL qualifications require the absolute wrapper plus a `POSTGRES_STARTED` marker; early-return cases do not count.','', '## Source reconciliation','','Validation scope correction: the earlier broad `npc_command` unit filter also ran `gameplay_transport::monk_save::postgres_tests::classic_quests::classic_quests_npc_commands_commit_exact_start_branches_and_fence_topics` (passed). The final unit selector is `gameplay_transport::npc_command`; no quest module was changed and this unrelated PG case is not repeated.','', '- Razan/Shalmar: current retail notes remove paid spell teaching in15.22: [Razan](https://www.tibiawiki.com.br/wiki/Razan), [Shalmar](https://www.tibiawiki.com.br/wiki/Shalmar). Older audit pages describe superseded behavior.','- Every placement retains the exact Wiki revision; all49 initial and3 follow-up native repairs retain audit research references and an exact before-state. Canary/Crystal hints do not override retail prices.','- Default-state charged buybacks retain the existing durable refusal for instances with mutable timed state. Ahmet filled-fluid support remains explicitly unfinished.','- Original source holds/provenance survive. Generated prey valuations change because newly admitted buyback offers change loot resale values; the repository generator owns those reports and pins.','', '## Self-review','', 'Checklist: `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Field/constructor/consumer search: `.npcfix/constructor-sites.txt`. Publication and any independently required review belong to the coordinator; this worker did not trigger owner-funded review. Whole-diff review evidence is recorded in `.npcfix/SELF-REVIEW.md`.','', '## Changed files','']
files=subprocess.check_output(['git','diff',BASE,'--name-only'],cwd=ROOT,text=True).splitlines()
files+=subprocess.check_output(['git','ls-files','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
lines += ['- `'+f+'`' for f in sorted(set(files))]
if qualified and not counts['REMAINING']:lines+=['','ALL PASS']
(ROOT/'.npcfix/REPORT.md').write_text('\n'.join(lines)+'\n')
print('report:',dict(counts),'checks',sum(c['status']=='PASS' for c in checks),'/',len(required))
