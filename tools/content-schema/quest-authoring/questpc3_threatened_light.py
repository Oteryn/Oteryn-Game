#!/usr/bin/env python3
"""Author six source light gathers through generic carried callback predicates."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-light/'
BASE='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/'
NIGHTMARE=BASE+'nightmare_intruders'
CASES=[('moon',25729,25730,25975,(33518,32209,-7),(33527,32215,-7),False,'charged_moon_mirror',
        '2fc2948aa828ec28639b56d2cca937cd6ff42dc6','4540401bfc7184a84983e687627e2a8576a9ddef02901d20932d461f684af11a',
        'The moon is not shining. Wait for the night.','The mirror is shining with the moonlight now.'),
       ('star',25731,25732,25976,(33526,32298,-4),(33532,32303,-4),False,'charged_starlight_vial',
        'f92134285a8f9d8a852ee57eb513eed16aa335e7','dfe4a91db453b7ef4e3a82510876ec62eb478943ef448f08e429f6f96f51fd96',
        'You are rubbing the opaque glass of the vial but nothing happens.','The vial is glittering with starlight now.'),
       ('sun',25733,25734,25977,(33508,32163,-7),(33522,32173,-7),True,'charged_sun_catcher',
        '775604d784190930c0102184baccaf901cff4053','385386841c340af0924721e645f64dc56929e5663f12131bda9e8b642768f3b4',
        "You are rubbing the gem in the sun catcher's centre but nothing happens.",'The sun catcher is shining with sunlight now.')]


def documents():
    rows=[]
    for name,source,charged,cosmetic,minimum,maximum,day,counter,blob,digest,failure,success in CASES:
        script='action_'+{'moon':'moon_mirror','star':'starlight_vial','sun':'sun_catcher'}[name]+'.lua'
        position=lambda p:dict(x=p[0],y=p[1],floor=p[2])
        for suffix,stage,output in [('gather',6,charged),('cosmetic',8,cosmetic)]:
            storage=[dict(track=NIGHTMARE,minimum=stage,maximum=stage,set=stage,preserve=stage==6)]
            if stage==6:storage.append(dict(track=BASE+counter,minimum=-1,maximum=5,set=5,preserve=False))
            rows.append(dict(key=PREFIX+name+'-'+suffix,quest='oteryn:quest.threatened_dreams_quest',trigger='use_carried',
                selector=dict(position=None,unique=None,action=None,appearance=None),lever=None,reversible=False,passages=[],teleport=None,
                storage=storage,offerings=[],item_on_target=dict(item=f'oteryn:item.tibia.i{source}',count=1,replacement=f'oteryn:item.tibia.i{output}',target_replacement=None),
                return_on_failure=False,failure_message=failure,success_message=success,
                carried_use=dict(actor_region=dict(minimum=position(minimum),maximum=position(maximum)),time_window=dict(from_minute=360 if day else 1080,to_minute=1080 if day else 360)),
                sources=[f'crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/threatened_dreams_quest/{script} canonical quest-state declaration',
                         f'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json data-global/scripts/quests/threatened_dreams_quest/{script}:1-73 git-blob={blob} sha256={digest}']))
    path='content/quests/world-callbacks.json';d=json.loads((ROOT/path).read_text());d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows
    d['callbacks'].sort(key=lambda r:r['key']);return {path:d}


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();drift=[]
    for relative,d in documents().items():
        p=ROOT/relative;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
        if p.read_text()!=text:
            if args.check:drift.append(relative)
            else:p.write_text(text)
    if drift:raise SystemExit('Threatened light source drift: '+', '.join(drift))


if __name__=='__main__':main()
