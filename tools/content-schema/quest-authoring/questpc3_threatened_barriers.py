#!/usr/bin/env python3
"""Author fifteen source barrier repairs as bounded exact-state alternatives."""
import argparse
import json
from pathlib import Path
from questpc3_threatened_light import BASE, CASES, NIGHTMARE

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-transition/questpc3/threatened-barrier/'
TARGETS = {
    'moon': [(33386, 32215, -7), (33511, 32320, -6), (33518, 32193, -7),
             (33549, 32219, -7), (33597, 32182, -7)],
    'star': [(33442, 32201, -7), (33458, 32299, -7), (33562, 32256, -7),
             (33541, 32244, -7), (33529, 32187, -7)],
    'sun': [(33419, 32242, -7), (33484, 32192, -7), (33546, 32155, -7),
            (33547, 32206, -7), (33568, 32243, -7)],
}
FLAG = {'moon': 'moon_mirror_pos', 'star': 'starlight_pos', 'sun': 'sun_catcher_pos'}
FAILURES = {
    'moon': 'The moon has to be shining. Wait for the night. Wait for the night.',
    'star': 'The stars has to be glittering in order to strengthen the barrier. Wait for the night. ',
    'sun': 'The sun has to be shining in order to strengthen the barrier. Wait for the day.',
}
MESSAGES = {
    'moon': ["As soon as you're touching the moon sculpture with the mirror the sculpture is infused with moonlight. The barrier strengthens.",
             "As soon as you're touching the moon sculpture with the mirror the sculpture is infused with moonlight. This was the last sculpture."],
    'star': ["As soon as you're pouring out the vial over the dreambird tree the plant is infused with starlight. The barrier strengthens.",
             "As soon as you're pouring out the vial over the dreambird tree the plant is infused with starlight. This was the last tree."],
    'sun': ["As soon as you're placing the sun catcher on the stone the pattern the mosaic is infused with sunlight. The barrier strengthens.",
            "As soon as you're placing the sun catcher on the stone the pattern the mosaic is infused with sunlight. This was the last mosaic."],
}


def position(x, y, floor):
    return dict(x=x, y=y, floor=floor)


def rows():
    result = []
    for name, empty, charged, _cosmetic, _minimum, _maximum, day, counter, blob, digest, _failure, _success in CASES:
        script = 'action_' + {'moon': 'moon_mirror', 'star': 'starlight_vial', 'sun': 'sun_catcher'}[name] + '.lua'
        for number, (x, y, floor) in enumerate(TARGETS[name], 1):
            for remaining in range(1, 6):
                result.append(dict(
                    key=f'{PREFIX}{name}-pos{number:02d}-charges{remaining}',
                    quest='oteryn:quest.threatened_dreams_quest', trigger='use_carried_on_target',
                    selector=dict(position=position(x, y, floor), unique=None, action=None, appearance=None),
                    lever=None, reversible=False, passages=[], teleport=None, offerings=[],
                    storage=[dict(track=NIGHTMARE, minimum=7, maximum=7, set=7, preserve=True),
                             dict(track=BASE+FLAG[name]+f'{number:02d}', minimum=-1, maximum=0, set=1, preserve=False),
                             dict(track=BASE+counter, minimum=remaining, maximum=remaining, set=remaining-1, preserve=False)],
                    item_on_target=dict(item=f'oteryn:item.tibia.i{charged}', count=1,
                                        replacement=f'oteryn:item.tibia.i{empty if remaining == 1 else charged}', target_replacement=None),
                    return_on_failure=False, failure_message=FAILURES[name],
                    success_message=MESSAGES[name][int(remaining == 1)],
                    carried_use=dict(actor_region=dict(minimum=position(x-1, y-1, floor), maximum=position(x+1, y+1, floor)),
                                     time_window=dict(from_minute=360 if day else 1080, to_minute=1080 if day else 360)),
                    sources=[f'crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/threatened_dreams_quest/{script} canonical quest-state declaration',
                             f'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json data-global/scripts/quests/threatened_dreams_quest/{script}:64-93 git-blob={blob} sha256={digest}']))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    path = ROOT/'content/quests/world-callbacks.json'
    document = json.loads(path.read_text())
    document['callbacks'] = [row for row in document['callbacks'] if not row['key'].startswith(PREFIX)] + rows()
    document['callbacks'].sort(key=lambda row: row['key'])
    text = json.dumps(document, indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if path.read_text() != text:
            raise SystemExit('Threatened barrier source drift')
    else:
        path.write_text(text)


if __name__ == '__main__':
    main()
