"""Closed source facts for Corym key forging; existing craft owners execute the rows."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CAPSULE = 'tools/content-schema/craft-authoring/samples/hidden-threats-forge.json'
SHA256 = '67facca7e961f3272045bc6cc7640e0ae9df9713237bb443098efd6e53bb7280'
PREFIX = 'oteryn:item-conversion.hidden-threats-'


def facts(root=ROOT):
    raw = (root / CAPSULE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('HIDDEN_THREATS_CRAFT_SOURCE_PIN')
    return json.loads(raw)


def native_item_ids(root=ROOT):
    return set(facts(root)['native_item_ids'])


def append_rows(rows, root=ROOT):
    data = facts(root)
    ref = lambda value: {'family': 'Item', 'key': f'oteryn:item.tibia.i{value}', 'revision': 'definition-r1'}

    def operand(role, item, effect, output=None):
        value = dict(role=role, item_id=ref(item), count=1, effect=effect)
        if output is not None:
            value.update(output_item_id=ref(output), lifecycle='REPLACE_INSTANCE')
        return value

    def row(name, source_name, lines, used, target, operands):
        witness = next(s for s in data['sources'] if s['path'].endswith(source_name))
        source = {key: witness[key] for key in ['repository', 'revision', 'path', 'sha256', 'lines']}
        source.update(line_start=lines[0], line_end=lines[1], evidence='OtsHypothesisOnly',
                      differences=['Plain transformations use the existing atomic retire-and-mint owner; binding and payload state is never copied.'])
        rows.append(dict(kind='item_conversion', identity=dict(key=PREFIX + name, revision='craft-r1'),
                         recipe=dict(input_item=ref(used), target=target, operands=operands,
                                     outputs=dict(kind='fixed', entries=[]), tool='none', capacity_check=True,
                                     result_message=dict(success='', no_room='not enough room', too_heavy='too heavy',
                                                         no_fish='no fish', not_owner='You are not the owner.', empty='There was just rubbish in it.'),
                                     source=source), fields=[]))

    for fragment, polished in data['filing']['targets'].items():
        fragment = int(fragment)
        row(f'file-{fragment}', 'metal_file.lua', (1, 21), data['filing']['tool'], dict(kind='carried'),
            [operand('USED', data['filing']['tool'], 'KEEP'), operand('TARGET', fragment, 'TRANSFORM_TO', polished)])
    for station in data['heating']['stations']:
        for fragment, hot in data['heating']['conversions'].items():
            fragment = int(fragment)
            row(f'heat-{fragment}-{station}', 'action-key_forge.lua', (3, 13), fragment,
                dict(kind='map_item', definition=ref(station)), [operand('USED', fragment, 'TRANSFORM_TO', hot)])
    for used in data['joining']['used']:
        for target in data['joining']['target']:
            row(f'join-{used}-{target}', 'action-key_forge.lua', (14, 23), used, dict(kind='carried'),
                [operand('USED', used, 'CONSUME'), operand('TARGET', target, 'TRANSFORM_TO', data['joining']['target_output'])])
    for operation in ['hammering', 'finishing']:
        step = data[operation]
        row(operation, 'action-key_forge.lua', (24, 35), step['input'],
            dict(kind='map_item', definition=ref(step['station'])), [operand('USED', step['input'], 'TRANSFORM_TO', step['output'])])
