"""Generate the closed native StepIn binding for the Cursed Crystal stairs."""
import argparse
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE / 'batch3_a'))
from capture_world_the_cursed_crystal import document

OUTPUT = ROOT / 'content/quests/action-marker-bindings.json'


def build():
    callbacks = document()['callbacks']
    for callback in callbacks:
        # loadLuaMapAction stamps ground and top item; select the stair once.
        callback['selector']['ordinal'] = 1
        callback['effects'] = {'teleport_effect': 11}
    return {'schema': 1, 'bindings': [{'action': 25018,
            'handler': 'cursed_crystal_stairs', 'callbacks': callbacks}]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    encoded = json.dumps(build(), indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != encoded:
            raise SystemExit('stale native action-marker bindings')
    else:
        OUTPUT.write_text(encoded)


if __name__ == '__main__':
    main()
