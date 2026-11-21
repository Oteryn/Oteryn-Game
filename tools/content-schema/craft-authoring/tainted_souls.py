"""M22: source-qualified whole-antler-stack transformation for The Tainted Souls."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = Path('tools/content-schema/craft-authoring/samples/tainted-souls.json')
KEY = 'oteryn:item-conversion.tainted-souls-antler-talisman'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
SCRIPT = 'data-global/scripts/quests/the_tainted_soul/actions_star_herb.lua'


def capture(script, wiki):
    raw = script.read_bytes()
    text = raw.decode()
    for witness in ('antlers = 10297', 'antler_talisman = 22008',
                    'target:transform(config.antler_talisman)', 'item:remove(1)', 'starHerb:id(3736)'):
        if witness not in text:
            raise ValueError('TAINTED_SOULS_SOURCE_CHANGED:' + witness)
    pages = {p['title']: p for p in json.loads(wiki.read_bytes())['query']['pages'].values()}
    items = []
    for title, item, stackable, weight in [('Star Herb', 3736, True, '0.30'),
                                           ('Antlers', 10297, True, '0.90'),
                                           ('Antler Talisman', 22008, False, '2.40')]:
        page = pages[title]
        revision = page['revisions'][0]
        body = revision['slots']['main']['*']
        fields = dict(line.lstrip('|').strip().split('=', 1) for line in body.splitlines()
                      if line.startswith('|') and '=' in line)
        fields = {key.strip(): value.strip() for key, value in fields.items()}
        if fields['stackable'] != ('sim' if stackable else 'não') or fields['weight'] != weight:
            raise ValueError('TAINTED_SOULS_ITEM_CONFLICT:' + title)
        items.append({'item': item, 'stackable': stackable, 'weight_centi_oz': int(float(weight) * 100),
                      'url': f"https://www.tibiawiki.com.br/index.php?curid={page['pageid']}&oldid={revision['revid']}",
                      'revision': revision['revid'], 'body_sha256': hashlib.sha256(body.encode()).hexdigest()})
    return {'schema': 'OTERYN_CAPTURED_STACK_TRANSFORM/v1', 'recipe': KEY,
            'used': 3736, 'target': 10297, 'output': 22008, 'used_count': 1,
            'target_quantity': 'CAPTURED_WHOLE_STACK', 'output_quantity': 1,
            'source': {'repository': 'zimbadev/crystalserver', 'revision': REVISION,
                       'path': SCRIPT, 'lines': len(text.splitlines()), 'sha256': hashlib.sha256(raw).hexdigest(),
                       'url': f'https://github.com/zimbadev/crystalserver/blob/{REVISION}/{SCRIPT}'},
            'items': items}


def row(item, operand, fixed, messages):
    packet = json.loads((ROOT / PACKET).read_bytes())
    if (packet['recipe'], packet['used'], packet['target'], packet['output'],
        packet['used_count'], packet['target_quantity'], packet['output_quantity']) != (
            KEY, 3736, 10297, 22008, 1, 'CAPTURED_WHOLE_STACK', 1):
        raise ValueError('TAINTED_SOULS_QUANTITY_POLICY')
    return {'kind': 'item_conversion', 'identity': {'key': KEY, 'revision': 'craft-r1'},
            'recipe': {'input_item': item(3736), 'target': {'kind': 'carried'},
                       'operands': [operand('USED', 3736, 'CONSUME'),
                                    operand('TARGET', 10297, 'TRANSFORM_TO', output=22008) | {'capture_target_stack': True}],
                       'outputs': fixed(), 'tool': 'none', 'capacity_check': True,
                       'result_message': messages,
                       'source': {key: packet['source'][key] for key in ('repository', 'revision', 'path', 'sha256', 'lines')}
                                 | {'line_start': 1, 'line_end': packet['source']['lines'], 'evidence': 'OtsHypothesisOnly',
                                    'differences': []}}, 'fields': []}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--script', type=Path, required=True)
    parser.add_argument('--wiki', type=Path, required=True)
    args = parser.parse_args()
    (ROOT / PACKET).write_text(json.dumps(capture(args.script, args.wiki), indent=2, ensure_ascii=False) + '\n')
