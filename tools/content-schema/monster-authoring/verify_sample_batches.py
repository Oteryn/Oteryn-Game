"""Validate committed sample structure and the explicitly retained source gaps."""
import json
from pathlib import Path
from validate_monster import read, validate

ROOT=Path(__file__).resolve().parent
EXPECTED_GAPS={'black_knight':{'events=BlackKnightDeath'},'paladin_familiar':{'familiar.owner_speed'},
    'the_enraged_thorn_knight':{'events=ForgottenKnowledgeBossDeath','events=HealthForgotten'},
    'warlock':{'attacks[3].name=warlock skill reducer'},'wyrm':{'attacks[3].name=wyrm wave'}}


def main():
    records=[]
    for name in ('canary-47dfd51f','canary-47dfd51f-batch-2'):
        root=ROOT/'samples'/name
        sources=read(root/'sources.json')
        expected_monsters={entry['monster'] for entry in sources['monsters']}
        actual_monsters={path.parent.name for path in root.glob('*/monster.json')}
        assert len(expected_monsters)==10 and actual_monsters==expected_monsters, 'sample inventory mismatch'
        for slug in sorted(expected_monsters):
            m,d,c,mf=[read(root/slug/file) for file in ('monster.json','dependencies.json','catalog.json','manifest.json')]
            errors=validate(m,d,c)
            assert not errors,(name,slug,errors)
            gaps={entry['source_field'] for entry in mf['entries'] if entry['status'] in
                  ('unsupported_source_field','unresolved_semantics','unresolved_dependency','partial_text')}
            expected=EXPECTED_GAPS.get(slug,set()) if name.endswith('batch-2') else set()
            assert gaps==expected,(name,slug,gaps,expected)
            readiness_errors=validate(m,d,c,mf)
            assert bool(readiness_errors)==bool(expected),(name,slug,readiness_errors)
            records.append({'batch':name,'monster':slug,'structure_valid':True,
                'declared_manifest_ready':not readiness_errors,'unresolved_source_fields':sorted(gaps),
                'readiness_errors':readiness_errors})
    report={'scope':'Offline authoring structure and declared manifest rows; no runtime or Global parity.',
            'bundles':len(records),'structure_valid':len(records),'declared_manifest_ready':sum(r['declared_manifest_ready'] for r in records),
            'runtime_qualified':False,'records':records}
    (ROOT/'samples'/'canary-47dfd51f-batch-2'/'validation-report.json').write_text(
        json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps({key:value for key,value in report.items() if key!='records'}))


if __name__=='__main__':main()
