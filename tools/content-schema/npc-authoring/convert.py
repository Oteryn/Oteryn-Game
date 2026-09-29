"""Convert pinned Canary or Crystal NPC datapacks into candidate NPC authoring bundles (v1).

Evidence tooling only: every bundle is OTS_HYPOTHESIS_ONLY source evidence, never Game truth.
Keys use the provisional `canary:` / `crystal:` namespaces; they are source-scoped and are not
native Oteryn identities (see docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md).

Each NPC Lua file is evaluated by npc_sandbox.py. Placements come from the datapack's NPC spawn XML
files with the engine rule from src/creatures/npcs/spawns/spawn_npc.cpp:
position = (centerx + x, centery + y, centerz); the child `z` is ignored; direction defaults to north.

Dialogue and voice text is never written by default: each text becomes a reference carrying its
SHA-256, length, |PLACEHOLDERS| and {links}. `--include-text` adds the text for local review only;
such output must not be committed (LICENSE-ASSETS.md).

Usage:
  python convert.py --source canary --checkout <opentibiabr/canary at 47dfd51f> --out DIR
  python convert.py --source crystal --checkout <zimbadev/crystalserver at ff7ede59> --out DIR
"""
import argparse
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

from npc_sandbox import load_npc

SCHEMA = 'OTERYN_NPC_AUTHORING_CANDIDATE/v1'
SOURCES = {
    'canary': {
        'repository': 'opentibiabr/canary',
        'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
        'npc_dir': 'data-otservbr-global/npc',
        'world_dir': 'data-otservbr-global/world',
    },
    'crystal': {
        'repository': 'zimbadev/crystalserver',
        'revision': 'ff7ede593c69d4c658b382c97443e8155926924a',
        'npc_dir': 'data-global/npc',
        'world_dir': 'data-global/world',
    },
    # the Crystal `summer-update` supplement: NPCs added after ff7ede59, pinned to their own exact commit
    'crystal-summer': {
        'repository': 'zimbadev/crystalserver',
        'revision': '00ce02a57ca5a12e48f32a3476e37471167e4c3f',
        'namespace': 'crystal',  # the same Crystal NPC files, so the same `crystal:npc/<stem>` keys
        'npc_dir': 'data-global/npc',
        'world_dir': 'data-global/world',
    },
}
NPCLIB = 'data/npclib/npc_system'
STRING_LIB = 'data/libs/functions/string.lua'
SPAWN_INTERVAL_S = (1, 86400)  # spawn_npc.cpp MINSPAWN_INTERVAL / MAXSPAWN_INTERVAL; outside it the NPC never spawns
DIRECTIONS = {0: 'NORTH', 1: 'EAST', 2: 'SOUTH', 3: 'WEST'}
MESSAGES = {'MESSAGE_GREET': 'greet', 'MESSAGE_FAREWELL': 'farewell', 'MESSAGE_WALKAWAY': 'walkaway',
            'MESSAGE_SENDTRADE': 'send_trade'}
KINDS = {'StdModule.say': 'say', 'StdModule.travel': 'travel', 'StdModule.learnSpell': 'learn_spell',
         'StdModule.bless': 'bless', 'StdModule.kick': 'kick', 'StdModule.promotePlayer': 'promote',
         'StdModule.rookgaardHints': 'rookgaard_hints'}
SAY_FLAGS = {'onlyFocus': 'only_focus', 'onlyUnfocus': 'only_unfocus', 'reset': 'reset', 'ungreet': 'ungreet',
             'moveup': 'move_up'}
PLACEHOLDER = re.compile(r'\|[A-Z_]+\|')
LINK = re.compile(r'\{([^{}]+)\}')


def sha256(data):
    return hashlib.sha256(data).hexdigest()


class Converter:
    def __init__(self, source, checkout, include_text=False):
        self.source = source
        self.meta = SOURCES[source]
        self.checkout = Path(checkout)
        self.include_text = include_text
        head = subprocess.run(['git', '-C', str(self.checkout), 'rev-parse', 'HEAD'], capture_output=True, text=True,
                              check=True).stdout.strip()
        if head != self.meta['revision']:
            raise SystemExit(f'{source} checkout is at {head}, expected {self.meta["revision"]}')

    # -- text -------------------------------------------------------------------------------------
    def text_ref(self, value, unresolved, path):
        if value is None:
            return None
        if isinstance(value, list) and value and all(isinstance(part, str) for part in value):
            return {'parts': [self.text_ref(part, unresolved, f'{path}[{i}]') for i, part in enumerate(value)]}
        if not isinstance(value, str):  # the detail names the shape only, never the text
            unresolved.append({'path': path, 'reason': 'NON_STATIC_TEXT', 'detail': type(value).__name__})
            return None
        ref = {'sha256': sha256(value.encode('utf-8')), 'length': len(value),
               'placeholders': sorted(set(PLACEHOLDER.findall(value))),
               'links': sorted({link.lower() for link in LINK.findall(value)})}
        if self.include_text:
            ref['text'] = value
        return ref

    # -- definition ---------------------------------------------------------------------------------
    @staticmethod
    def symbol(value, prefix=''):
        if isinstance(value, dict) and 'symbol' in value:
            name = value['symbol']
            return name[len(prefix):] if prefix and name.startswith(prefix) else name
        return value

    def outfit(self, outfit, unresolved):
        if not isinstance(outfit, dict):
            unresolved.append({'path': 'presentation.outfit', 'reason': 'MISSING', 'detail': None})
            return None
        if 'lookTypeEx' in outfit and not outfit.get('lookType'):
            return {'item_look': outfit['lookTypeEx']}
        return {'look_type': outfit.get('lookType'), 'head': outfit.get('lookHead', 0), 'body': outfit.get('lookBody', 0),
                'legs': outfit.get('lookLegs', 0), 'feet': outfit.get('lookFeet', 0),
                'addons': outfit.get('lookAddons', 0), 'mount': outfit.get('lookMount')}

    def voices(self, voices, unresolved):
        if not isinstance(voices, dict):
            return None
        lines = []
        for key in sorted((k for k in voices if k.isdigit()), key=int):
            line = voices[key]
            lines.append({'text': self.text_ref(line.get('text'), unresolved, f'voices.lines[{len(lines)}]'),
                          'yell': bool(line.get('yell', False))})
        return {'interval_ms': voices.get('interval'), 'chance_percent': voices.get('chance'), 'lines': lines}

    def trade(self, config, events, unresolved):
        shop = config.get('shop')
        if not shop:
            return None
        if isinstance(shop, dict):  # a table with holes or string keys; keep numeric order
            unresolved.append({'path': 'services.trade', 'reason': 'NON_SEQUENTIAL_SHOP_TABLE', 'detail': str(len(shop))})
            shop = [shop[k] for k in sorted(shop, key=lambda k: (not k.isdigit(), int(k) if k.isdigit() else 0, k))]
        offers = []
        for index, row in enumerate(shop):
            if not isinstance(row, dict):
                unresolved.append({'path': f'services.trade.rows[{index}]', 'reason': 'NON_STATIC_SHOP_ROW',
                                   'detail': json.dumps(row, sort_keys=True)[:120]})
                continue
            offer = {'item_name': row.get('itemName', row.get('itemname', row.get('name'))),
                     'client_id': row.get('clientId', row.get('clientid')), 'server_item_id': row.get('itemid'),
                     'buy_price': row.get('buy'), 'sell_price': row.get('sell'), 'count': row.get('count'),
                     'sub_type': row.get('subType'), 'stock_gate': None}
            if 'storageKey' in row:
                offer['stock_gate'] = {'storage_key': self.symbol(row['storageKey']),
                                       'storage_value': self.symbol(row.get('storageValue'))}
            offers.append(offer)
        # Shops filled from a `pairs` loop have no stable source order, so offers are kept in a
        # canonical order instead of declaration order.
        offers.sort(key=lambda o: json.dumps(o, sort_keys=True))
        for index, offer in enumerate(offers):
            path = f'services.trade.offers[{index}]'
            if offer['stock_gate']:
                unresolved.append({'path': path + '.stock_gate', 'reason': 'PLAYER_STORAGE_GATE',
                                   'detail': json.dumps(offer['stock_gate'], sort_keys=True)[:160]})
            if offer['client_id'] is None and offer['server_item_id'] is None:
                unresolved.append({'path': path, 'reason': 'NO_ITEM_REFERENCE', 'detail': offer['item_name']})
        currency = config.get('currency')
        return {'currency': {'client_id': currency} if currency is not None else 'GOLD', 'offers': offers,
                'engine_trade_events': sorted(e for e in events if e in ('onBuyItem', 'onSellItem', 'onCheckItem'))}

    # -- keywords -----------------------------------------------------------------------------------
    def keyword_nodes(self, nodes, unresolved, path, services, trail, inherited_gate=False):
        result = []
        for index, node in enumerate(nodes):
            node_path = f'{path}[{index}]'
            callback = node['callback']
            params = node['parameters'] if isinstance(node['parameters'], dict) else {}
            keywords = node['keywords'] if isinstance(node['keywords'], list) else []
            if isinstance(callback, dict) and 'symbol' in callback:
                kind = KINDS.get(callback['symbol'], 'unknown_module')
                if kind == 'unknown_module':
                    unresolved.append({'path': node_path, 'reason': 'UNKNOWN_MODULE', 'detail': callback['symbol']})
            elif isinstance(callback, dict) and callback.get('lua_function'):
                kind = 'script'
                unresolved.append({'path': node_path, 'reason': 'LUA_CALLBACK', 'detail': ','.join(map(str, keywords))})
            else:
                kind = 'none'
            entry = {'keywords': [str(k) if not isinstance(k, dict) else self.symbol(k) for k in keywords], 'kind': kind,
                     'text': self.text_ref(params.get('text'), unresolved, node_path + '.text'),
                     'flags': {SAY_FLAGS[k]: params[k] for k in sorted(params) if k in SAY_FLAGS},
                     'gate': 'LUA_PREDICATE' if node['condition'] else 'NONE',
                     'effect': 'LUA_ACTION' if node['action'] else 'NONE'}
            if node['condition']:
                unresolved.append({'path': node_path + '.gate', 'reason': 'LUA_PREDICATE', 'detail': ','.join(entry['keywords'])})
            if node['action']:
                unresolved.append({'path': node_path + '.effect', 'reason': 'LUA_ACTION', 'detail': ','.join(entry['keywords'])})
            # The engine answers with the first sibling whose keywords match and whose condition
            # passes, so an earlier gated sibling with the same keyword makes this node conditional.
            words = {w.lower() for w in entry['keywords']}
            shadowed = any(n['condition'] and words & {str(w).lower() for w in (n['keywords'] or [])}
                           for n in nodes[:index] if isinstance(n['keywords'], list))
            gated = bool(node['condition']) or shadowed or inherited_gate
            here = trail + [entry['keywords']]
            self.service_row(kind, params, entry, gated, here, node_path, services, unresolved)
            entry['children'] = self.keyword_nodes(node['children'], unresolved, node_path + '.children', services, here,
                                                   gated)
            result.append(entry)
        return result

    def service_row(self, kind, params, entry, gated, trail, path, services, unresolved):
        if kind not in ('travel', 'learn_spell', 'bless', 'promote', 'kick'):
            return
        topic = next((k[0] for k in reversed(trail) if k and k[0] not in ('yes', 'no')), None)
        gate = 'LUA_PREDICATE' if gated else 'NONE'
        if gated and entry['gate'] == 'NONE' and kind != 'kick':
            unresolved.append({'path': path + '.gate', 'reason': 'LUA_PREDICATE', 'detail': 'inherited from an ancestor or earlier sibling'})
        if kind == 'travel':
            destination = params.get('destination')
            static = isinstance(destination, dict) and set(destination) == {'x', 'y', 'z'}
            services['travel'].append({'keyword': topic, 'destination': destination if static else None,
                                       'price': params.get('cost'), 'premium': params.get('premium'),
                                       'min_level': params.get('level'), 'discount': params.get('discount'),
                                       'gate': gate, 'effect': entry['effect'], 'dialogue_path': path})
            if not static:
                unresolved.append({'path': path, 'reason': 'NON_STATIC_DESTINATION', 'detail': json.dumps(destination, sort_keys=True)[:120]})
        elif kind == 'learn_spell':
            vocation = params.get('vocation')
            services['spells'].append({'keyword': topic, 'spell_name': params.get('spellName'), 'price': params.get('price'),
                                       'min_level': params.get('level'), 'premium': params.get('premium'),
                                       'vocations': self.symbol(vocation) if not isinstance(vocation, list)
                                       else [self.symbol(v) for v in vocation], 'gate': gate, 'dialogue_path': path})
        elif kind == 'bless':
            services['blessings'].append({'keyword': topic, 'blessing': self.symbol(params.get('bless')),
                                          'price': self.symbol(params.get('cost')), 'gate': gate, 'dialogue_path': path})
        elif kind == 'promote':
            services['promotion'].append({'keyword': topic, 'price': params.get('cost'), 'min_level': params.get('level'),
                                          'promotion': params.get('promotion'), 'gate': gate, 'dialogue_path': path})
        elif kind == 'kick':
            destination = params.get('destination')
            services['kick'].append({'destinations': destination if isinstance(destination, list) else [destination],
                                     'dialogue_path': path})

    # -- placements ---------------------------------------------------------------------------------
    def placements(self):
        world = self.checkout / self.meta['world_dir']
        rows = []
        for xml_path in sorted(world.rglob('*npc*.xml')):
            relative = str(xml_path.relative_to(self.checkout))
            for spawn in ET.parse(xml_path).getroot().findall('npc'):
                center = (int(spawn.get('centerx')), int(spawn.get('centery')), int(spawn.get('centerz')))
                radius = int(spawn.get('radius')) if spawn.get('radius') is not None else -1
                for child in spawn.findall('npc'):
                    if child.get('name') is None:
                        continue
                    direction = int(child.get('direction')) if child.get('direction') is not None else 0
                    interval = int(child.get('spawntime', 0))
                    rows.append({'name': child.get('name'),
                                 'position': {'x': center[0] + int(child.get('x', 0)), 'y': center[1] + int(child.get('y', 0)),
                                              'z': center[2]},
                                 'direction': DIRECTIONS.get(direction, direction), 'spawn_interval_s': interval,
                                 'engine_spawns': SPAWN_INTERVAL_S[0] <= interval <= SPAWN_INTERVAL_S[1],
                                 'spawn_radius': radius, 'placement_file': relative})
        return rows

    # -- bundle -------------------------------------------------------------------------------------
    def bundle(self, lua_path, placements_by_name):
        relative = str(lua_path.relative_to(self.checkout))
        stem = lua_path.stem
        base = {'schema': SCHEMA, 'key': f'{self.meta.get("namespace", self.source)}:npc/{stem}', 'evidence': 'OTS_HYPOTHESIS_ONLY',
                'source': {'repository': self.meta['repository'], 'revision': self.meta['revision'], 'path': relative,
                           'sha256': sha256(lua_path.read_bytes())}}
        try:
            capture = load_npc(lua_path, self.checkout / NPCLIB, self.checkout / STRING_LIB)
        except Exception as exc:
            return {**base, 'status': 'LOAD_ERROR', 'error': str(exc).splitlines()[0][:200]}
        if not capture['configs']:
            return {**base, 'status': 'NOT_AN_NPC', 'error': 'file registers no NpcType'}
        unresolved = []
        config = capture['configs'][0]
        if len(capture['configs']) > 1:
            unresolved.append({'path': 'definition', 'reason': 'MULTIPLE_REGISTRATIONS', 'detail': str(len(capture['configs']))})
        if capture['error_after_register']:
            unresolved.append({'path': 'definition', 'reason': 'ERROR_AFTER_REGISTER', 'detail': capture['error_after_register']})
        type_name = (capture['npc_type_names'] or [config.get('name')])[0]
        flags = config.get('flags') or {}
        definition = {
            'name': type_name if isinstance(type_name, str) else None,
            'display_name': config.get('name'),
            'description': self.text_ref(config.get('description'), unresolved, 'definition.description'),
            'profession': flags.get('profession'),
            'presentation': {'outfit': self.outfit(config.get('outfit'), unresolved),
                             'speech_bubble': self.symbol(config.get('speechBubble'), 'SPEECHBUBBLE_'),
                             'light': config.get('light')},
            'movement': {'walk_interval_ms': config.get('walkInterval'), 'walk_radius': config.get('walkRadius'),
                         'floor_change': flags.get('floorchange')},
            'vitals': {'health': config.get('health'), 'max_health': config.get('maxHealth')},
            'respawn': self.symbol(config.get('respawnType')),
        }
        for key in ('maxLevel', 'amountMoney', 'amountLevel', 'amountTibiaCoin', 'moneyToNeedDonation', 'spells'):
            if key in config:
                unresolved.append({'path': f'definition.{key}', 'reason': 'UNMAPPED_CONFIG_FIELD', 'detail': json.dumps(config[key], sort_keys=True)[:120]})
        services = {'trade': self.trade(config, capture['events'], unresolved), 'travel': [], 'spells': [], 'blessings': [],
                    'promotion': [], 'kick': []}
        messages = {}
        for message in capture['messages']:
            name = self.symbol(message.get('id'))
            label = MESSAGES.get(name, name)
            messages[label] = self.text_ref(message.get('text'), unresolved, f'dialogue.messages.{label}')
        keywords = []
        for tree_index, tree in enumerate(capture['keywords']):
            keywords.extend(self.keyword_nodes(tree, unresolved, f'dialogue.keywords', services, []))
            if tree_index:
                unresolved.append({'path': 'dialogue.keywords', 'reason': 'MULTIPLE_KEYWORD_HANDLERS', 'detail': str(tree_index + 1)})
        handlers = sorted({self.symbol(c.get('id')) for c in capture['callbacks'] if isinstance(c, dict)})
        for handler in handlers:
            unresolved.append({'path': 'dialogue.scripted_handlers', 'reason': 'LUA_CALLBACK', 'detail': handler})
        dialog_options = [c for c in capture['type_calls'] if c.get('name') == 'addDialogOptions']
        if dialog_options:
            unresolved.append({'path': 'dialogue.dialog_options', 'reason': 'UNMAPPED_ENGINE_CALL',
                               'detail': f'addDialogOptions x{len(dialog_options)}'})
        placements = [{k: v for k, v in row.items() if k != 'name'}
                      for row in placements_by_name.get(str(type_name).lower(), [])]
        if not placements:
            unresolved.append({'path': 'placements', 'reason': 'NO_PLACEMENT', 'detail': None})
        for index, row in enumerate(placements):
            if not row['engine_spawns']:
                unresolved.append({'path': f'placements[{index}]', 'reason': 'SPAWN_INTERVAL_OUT_OF_RANGE',
                                   'detail': str(row['spawn_interval_s'])})
        bundle = {**base, 'status': 'RESOLVED' if not unresolved else 'PARTIAL', 'definition': definition,
                  'voices': self.voices(config.get('voices'), unresolved),
                  'dialogue': {'messages': messages, 'keywords': keywords, 'scripted_handlers': handlers,
                               'dialog_option_calls': len(dialog_options)},
                  'services': services, 'placements': placements, 'unresolved': unresolved}
        bundle['status'] = 'RESOLVED' if not unresolved else 'PARTIAL'
        return bundle

    def run(self):
        placements = self.placements()
        by_name = {}
        for row in placements:
            by_name.setdefault(row['name'].lower(), []).append(row)
        bundles = [self.bundle(path, by_name) for path in sorted((self.checkout / self.meta['npc_dir']).rglob('*.lua'))]
        registered = {str(b['definition']['name']).lower() for b in bundles if 'definition' in b}
        orphans = sorted({row['name'] for row in placements if row['name'].lower() not in registered})
        return bundles, placements, orphans


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--source', choices=sorted(SOURCES), required=True)
    parser.add_argument('--checkout', required=True)
    parser.add_argument('--out', required=True)
    parser.add_argument('--include-text', action='store_true', help='local review only; never commit the output')
    args = parser.parse_args()
    converter = Converter(args.source, args.checkout, args.include_text)
    bundles, placements, orphans = converter.run()
    out = Path(args.out)
    (out / 'bundles').mkdir(parents=True, exist_ok=True)
    for bundle in bundles:
        name = bundle['key'].split('/', 1)[1]
        (out / 'bundles' / f'{name}.json').write_text(json.dumps(bundle, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    index = {'schema': 'OTERYN_NPC_AUTHORING_CANDIDATE_INDEX/v1', 'source': {k: v for k, v in SOURCES[args.source].items()},
             'bundles': len(bundles), 'placements': len(placements), 'orphan_placement_names': orphans,
             'include_text': args.include_text}
    (out / 'index.json').write_text(json.dumps(index, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in index.items() if k != 'orphan_placement_names'} | {'orphans': len(orphans)}))


if __name__ == '__main__':
    main()
