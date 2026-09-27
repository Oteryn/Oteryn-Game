"""Transcribe quest scripts of Canary and CrystalServer into candidate interaction definitions (D36).

Usage: python ots_interactions.py --canary <opentibiabr/canary at 47dfd51f> --crystal <zimbadev/crystalserver at ff7ede59>
                                  [--scripts scripts/quests] [--questlog samples/questlog]
                                  [--out samples/interactions]

An interaction definition is what GAME-INTERACTION-01 plans run: a source edge on a target (a tile entered, an
item used, a creature killed), read-only conditions, and children that each go to the domain owning the effect.
- player storage writes -> Quest (D35), named by the mission transition when one matches;
- `Game.setStorageValue` -> Quest world state (state shared by all players, as encounters read it, D29);
- `Game.createMonster` -> Ability summon effect (GAME-ABILITY-01);
- `player:addItem` -> Item hand-out (DUR-03 item transaction); `addAchievement` -> Achievement grant;
- `sendMagicEffect` and player messages -> presentation only (a message keeps its source line, never its text);
- teleports -> Movement, blocked until a movement owner contract exists (GAME-INTERACTION-01 §19.3);
- map item create/remove/transform -> world object state, blocked until an owner contract exists.
Storage aliases (`local X = Storage.…`) are expanded; a track resolves to the catalogue's track whichever server
declared it (D33). A function literal passed to a call (`addEvent(function() … end)`) runs later and stays one
unresolved entry. Conditions read quest stages, world state, whether the actor is a player, its level, and the item type, unique id, action id
or subtype of the edge source (the registered target), the object in contact with it or the use target. Any other
line or condition stays unresolved with its source line. Source positions become named anchors with the
coordinates kept as evidence. Every output is OTS_HYPOTHESIS_ONLY.
"""
import argparse
import glob
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

import lua_blocks
from lua_writers import REGISTRATION, expand_aliases, storage_aliases, strip_code
from ots_chests import CONFLICT_DECISIONS, REVISION, ROOT, SOURCES, check_checkout, decided, git_blob, ref, slug, unused_decisions
from ots_questlog import norm, script_of, track_of
from validate_quest_content import BLOCKED

CALLBACK = re.compile(r'^\s*function\s+(\w+)[.:](onStepIn|onStepOut|onAddItem|onUse|onDeath|onKill|onPrepareDeath)\s*\(')
OBJECT_LINE = re.compile(r'^\s*local\s+\w+\s*=\s*(MoveEvent|Action|CreatureEvent)\s*\(')
EDGES = {'onStepIn': 'ON_ENTER', 'onStepOut': 'ON_LEAVE', 'onAddItem': 'ON_CONTACT', 'onUse': 'USE',
         'onDeath': 'ON_DEATH', 'onKill': 'ON_KILL', 'onPrepareDeath': 'ON_DEATH'}
POSITION = re.compile(r'\{\s*x\s*=\s*(\d+)\s*,\s*y\s*=\s*(\d+)\s*,\s*z\s*=\s*(\d+)\s*\}|Position\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)')
NOOP = re.compile(r'^(local\s+\w+(\s*,\s*\w+)*\s*=\s*[\w.:]+\([^()]*\)|local\s+\w+\s*=\s*[\w.\[\]#]+|local\s+\w+\s*=\s*\{.*\}|'
                  r'return\b.*|end|)$')
# callback parameter order: which argument is the actor, the edge source (the registered target), the object in
# contact with it and the target of a use
ROLES = {'onUse': ('actor', 'source', None, 'use_target'), 'onStepIn': ('actor', 'source'),
         'onStepOut': ('actor', 'source'), 'onAddItem': ('contact', 'source')}
FIELDS = {'itemid': 'item_type', 'uid': 'unique_id', 'actionid': 'action_id', 'type': 'subtype'}
# calls that only read state: a local assignment made of these is not an effect
READ_ONLY = re.compile(r'^(get\w*|is\w*|has\w*|can\w*|Tile|Position|Player|Creature|Monster|Item|Npc|Container|lower|upper|'
                       r'random|max|min|floor|ceil|abs|contains|find|match|sub|gsub|format|len|kv|scoped|time|pairs|ipairs|'
                       r'type|tostring|tonumber|unpack|getn)$')
TABLE_LINE = re.compile(r'^(\{.*\}?,?|\}\)?,?|\[?[\w"\']+\]?\s*=\s*[^()]*,?)$')
MESSAGE = re.compile(r':(sendTextMessage|say|sendCancelMessage|sendChannelMessage|popupFYI)\(')
CONTROL = re.compile(r'^(if|elseif|else|end|return|break)\b')
BLOCKED_MOVEMENT, BLOCKED_WORLD_OBJECT = BLOCKED['Movement'], BLOCKED['WorldObject']


class Script:
    def __init__(self, server, repo, path, transitions, name):
        self.server, self.path, self.name = server, path, name
        self.namespace = 'canary' if server == 'canary' else 'crystalserver'
        self.lines = (Path(repo) / path).read_text(errors='replace').split('\n')
        self.transitions = transitions
        self.anchors, self.unresolved = [], []
        self.roles, self.players, self.declared = {}, set(), {}
        self.aliases = storage_aliases(self.lines)

    def bind(self, number, callback):
        """Name the callback parameters by their role and note the locals that hold the acting player."""
        params = re.search(r'\(([^)]*)\)', self.lines[number - 1]).group(1)
        names = [n.strip() for n in params.split(',')]
        self.roles = {n: r for n, r in zip(names, ROLES.get(callback, ())) if r}
        actor = next((n for n, r in self.roles.items() if r == 'actor'), None)
        self.players = {actor} if callback == 'onUse' else set()
        for line in self.lines[number:]:
            if actor and (m := re.match(rf'\s*local\s+(\w+)\s*=\s*{actor}:getPlayer\(\)', line)):
                self.players.add(m.group(1))

    def anchor(self, match):
        x, y, z = [int(v) for v in (match[:3] if match[0] else match[3:])]
        for a in self.anchors:
            if (a['source_position']['x'], a['source_position']['y'], a['source_position']['z']) == (x, y, z):
                return a['key']
        key = f'p{len(self.anchors) + 1}'
        self.anchors.append({'key': key, 'source_position': {'x': x, 'y': y, 'z': z}})
        return key

    def track(self, storage):
        """The catalogue's progress track for a storage, whichever server declared it (D33), else this server's."""
        path = track_of(storage)
        return self.declared.get(norm(path), f'{self.namespace}:quest-progress/{path}')

    def raw(self, number):
        return expand_aliases(re.sub(r'--.*$', '', self.lines[number - 1]).strip(), self.aliases)

    def read_only(self, code):
        """A local assignment or table-constructor line whose calls only read state."""
        if TABLE_LINE.match(code):
            return True
        if not re.match(r'local\s+\w+(\s*,\s*\w+)*\s*=', code):
            return False
        return all(READ_ONLY.match(name) for name in re.findall(r'(\w+)\s*\(', code))

    def condition(self, text, number):
        parts = re.split(r'\s+(and|or)\s+', text)
        terms, joins = parts[0::2], set(parts[1::2])
        if len(joins) > 1:
            return {'unresolved': {'line': number}}
        out = []
        for term in terms:
            negate = term.startswith('not ')
            body = term[4:] if negate else term
            if (m := re.fullmatch(r'\w+:getStorageValue\(\s*(Storage\.[\w.\[\]]+)\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)', body)):
                out.append({'quest_stage': {'progress': self.track(m.group(1)),
                                            'op': m.group(2), 'value': int(m.group(3))}, 'negate': negate})
            elif (m := re.fullmatch(r'Game\.getStorageValue\(\s*"?([\w.]+)"?\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)', body)):
                out.append({'world_state': {'key': f'{self.namespace}:world-state/{slug(m.group(1))}',
                                            'op': m.group(2), 'value': int(m.group(3))}, 'negate': negate})
            elif (m := re.fullmatch(r'(\w+)\.(itemid|uid|actionid|type)\s*(==|~=)\s*(\d+)', body)) and m.group(1) in self.roles:
                field = FIELDS[m.group(2)]
                value = ({'item': ref('Item', f'{self.namespace}:item/{m.group(4)}')} if field == 'item_type'
                         else {'value': int(m.group(4))})
                out.append({'object': {'role': self.roles[m.group(1)], 'field': field, 'op': m.group(3), **value},
                            'negate': negate})
            elif body in self.players or ((m := re.fullmatch(r'(\w+):isPlayer\(\)', body))
                                          and self.roles.get(m.group(1)) == 'actor'):
                out.append({'actor_is_player': True, 'negate': negate})
            elif (m := re.fullmatch(r'(\w+):getLevel\(\)\s*(==|~=|<=|>=|<|>)\s*(\d+)', body)) and m.group(1) in self.players:
                out.append({'actor_level': {'op': m.group(2), 'value': int(m.group(3))}, 'negate': negate})
            else:
                return {'unresolved': {'line': number}}
        if len(out) == 1:
            return out[0]
        return {('any' if 'or' in joins else 'all'): out}

    def children(self, number, in_loop=False):
        code, raw = strip_code(self.lines[number - 1]).strip(), self.raw(number)
        found = []
        for m in re.finditer(r'(\w+):setStorageValue\(\s*(Storage\.[\w.\[\]]+|\d+)\s*,', raw):
            target = int(m.group(2)) if m.group(2).isdigit() else m.group(2)
            track = self.track(target)
            value = raw[m.end():].split(')')[0].strip()
            child = {'owner': 'Quest', 'request': 'set_progress', 'progress': track,
                     **({'to': int(value)} if re.fullmatch(r'-?\d+', value) else {'value_source_line': number})}
            key = self.transitions.get((norm(track_of(target)), script_of(self.path), number))
            if key:
                child['transition'] = key
            found.append(child)
        for m in re.finditer(r'Game\.setStorageValue\(\s*"?([\w.]+)"?\s*,\s*([^)]*)\)', raw):
            value = m.group(2).strip()
            found.append({'owner': 'Quest', 'request': 'set_world_state', 'key': f'{self.namespace}:world-state/{slug(m.group(1))}',
                          **({'to': int(value)} if re.fullmatch(r'-?\d+', value) else {'value_source_line': number})})
        if (m := re.search(r'Game\.createMonster\(\s*"([^"]+)"\s*,\s*(.*)', raw)):
            pos = POSITION.search(m.group(2))
            found.append({'owner': 'Ability', 'effect': 'summon',
                          'creature': ref('Creature', f'{self.namespace}:creature/{slug(m.group(1))}'),
                          **({'anchor': self.anchor(pos.groups())} if pos else {'anchor_source_line': number})})
        if 'teleportTo(' in code:
            pos = POSITION.search(raw)
            found.append({'owner': 'Movement', 'status': 'blocked', 'reason': BLOCKED_MOVEMENT,
                          **({'to_anchor': self.anchor(pos.groups())} if pos else
                             {'to': 'previous_position'} if 'fromPosition' in raw else {'to_source_line': number})})
        if re.search(r'[:.](transform|transformItem|createItem|removeItem|revertItem|remove|setActionId|decay)\(|\b(add|stop)Event\(\s*Position\.revertItem|\bPosition\.revertItem\(', raw):
            found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_WORLD_OBJECT, 'source_line': number})
        if (m := re.search(r'(\w+):addItem\(\s*(\d+)?\s*(?:,\s*(\d+)\s*)?', raw)) and m.group(1) in self.players | {'player'}:
            found.append({'owner': 'Item', 'request': 'hand_out',
                          **({'item': ref('Item', f'{self.namespace}:item/{m.group(2)}'), 'count': int(m.group(3) or 1)}
                             if m.group(2) and raw[m.end():m.end() + 1] == ')' else {'value_source_line': number})})
        if (m := re.search(r':addAchievement\(\s*"([^"]+)"\s*\)', raw)):
            found.append({'owner': 'Achievement', 'request': 'grant',
                          'achievement': ref('Achievement', f'{self.namespace}:achievement/{slug(m.group(1))}')})
        if 'sendMagicEffect(' in code:
            found.append({'owner': 'Presentation', 'effect': 'magic_effect', 'authoritative': False})
        if MESSAGE.search(code):
            # the text itself is reserved (LICENSE-ASSETS.md); only its source line is kept
            found.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False, 'source_line': number})
        if not found and re.search(r'\baddEvent\(', code):
            self.unresolved.append({'line': number, 'reason': 'delayed callback (addEvent) without a scheduler owner'})
        elif not found and not NOOP.match(code) and not self.read_only(code):
            self.unresolved.append({'line': number, 'reason': 'statement outside the transcribed vocabulary'})
        for child in found:
            if in_loop:
                child['repeated'] = True
        return found

    def convert(self, nodes):
        out = []
        for node in nodes:
            if node[0] == 'stmt':
                out.extend(self.children(node[1]))
            elif node[0] == 'block':
                inner = node[1][1:-1]
                control = [n for n in inner if CONTROL.match(strip_code(self.lines[n - 1]).strip())]
                for number in inner:
                    if number not in control:
                        out.extend(self.children(number, in_loop=True))
                if control:
                    self.unresolved.append({'line': node[1][0], 'reason': 'loop with its own control flow'})
            elif node[0] == 'deferred':
                reason = ('delayed callback (addEvent) without a scheduler owner'
                          if re.search(r'\baddEvent\(', self.raw(node[1][0])) else 'function literal outside the transcribed vocabulary')
                self.unresolved.append({'line': node[1][0], 'reason': reason})
            elif node[0] == 'if':
                branches = [{'when': self.condition(re.sub(r'^(else)?if\s+|\s+then.*$', '', self.raw(line)), line),
                             'then': self.convert(body)} for _, line, body in node[1]]
                otherwise = self.convert(node[2])
                out.append({'branch': branches, **({'otherwise': otherwise} if otherwise else {})})
        return out

    def interactions(self):
        """One definition per callback; a file may redefine the same script object several times, so each
        callback takes the registrations that follow it up to the next object definition."""
        starts = [n for n, line in enumerate(self.lines, 1) if CALLBACK.match(line)]
        objects = [n for n, line in enumerate(self.lines, 1) if OBJECT_LINE.match(line)]
        out, stem = [], self.name
        for index, number in enumerate(starts):
            m = CALLBACK.match(self.lines[number - 1])
            end = next((o for o in objects if o > number), len(self.lines) + 1)
            registrations = sorted({f'{r.group(2)}({r.group(3).strip()[:60]})' for line in self.lines[number:end - 1]
                                    if (r := REGISTRATION.match(line)) and r.group(1) == m.group(1)})
            self.anchors, self.unresolved = [], []
            self.bind(number, m.group(2))
            nodes = lua_blocks.parse(self.lines, lua_blocks.function_body(self.lines, number))
            rules = self.convert(nodes)
            # several callbacks in one file are told apart by their script object, which both servers share even when
            # one of them adds or reorders callbacks; a repeated object name takes its occurrence number
            objects_before = [CALLBACK.match(self.lines[n - 1]).group(1) for n in starts[:index]]
            key = stem if len(starts) == 1 else f'{stem}_{slug(m.group(1))}'
            if m.group(1) in objects_before:
                key += f'_{objects_before.count(m.group(1)) + 1}'
            out.append({'identity': {'key': f'{self.namespace}:interaction/{key}', 'revision': REVISION},
                        'source': {'edge': EDGES[m.group(2)], 'callback': m.group(2), 'target_registrations': registrations},
                        'rules': rules, 'anchors': self.anchors, 'unresolved': self.unresolved,
                        'script': script_of(self.path), 'callback_line': number})
        return out


def comparable(interaction):
    """Two servers' transcriptions agree when their rules, anchors and edges agree (source lines may differ)."""
    def strip(value):
        if isinstance(value, dict):
            return {k: strip(v) for k, v in value.items()
                    if k not in ('line', 'source_line', 'value_source_line', 'anchor_source_line', 'to_source_line')}
        if isinstance(value, list):
            return [strip(v) for v in value]
        return value.replace('crystalserver:', 'canary:') if isinstance(value, str) else value
    return strip({k: interaction[k] for k in ('source', 'rules', 'anchors')})


def transition_keys(questlog_dir):
    """(track path letters, script, source line) -> '<quest>#<mission>:<transition key>' from the D35 transcription."""
    quests = json.loads((questlog_dir / 'quests.json').read_text())['quests']
    progress = {t['key']: t for t in json.loads((questlog_dir / 'progress.json').read_text())['progress']}
    keys = {}
    for quest in quests:
        for mission in quest.get('missions', []):
            track = progress.get(mission['progress'], {})
            for evidence in track.get('transitions', []):
                for source in evidence['sources'].values():
                    keys[(norm(mission['progress'].split('/', 1)[1]), script_of(source['path']), source['line'])] = \
                        f'{quest["identity"]["key"]}#{mission["key"]}:{evidence["key"]}'
    return keys


def walk(rules):
    for rule in rules:
        if 'branch' in rule:
            for branch in rule['branch']:
                yield from walk(branch['then'])
            yield from walk(rule.get('otherwise', []))
        else:
            yield rule


def conditions(rules):
    """Every leaf condition of a rule tree."""
    def leaves(condition):
        for term in condition.get('all', condition.get('any', [])):
            yield from leaves(term)
        if 'all' not in condition and 'any' not in condition:
            yield condition
    for rule in rules:
        if 'branch' in rule:
            for branch in rule['branch']:
                yield from leaves(branch['when'])
                yield from conditions(branch['then'])
            yield from conditions(rule.get('otherwise', []))


def unresolved_conditions(interaction):
    return sum(1 for c in conditions(interaction['rules']) if 'unresolved' in c)


def build(repos, scripts, questlog_dir):
    keys = transition_keys(questlog_dir)
    declared_by_path = {norm(t['key'].split('/', 1)[1]): t['key']
                        for t in json.loads((questlog_dir / 'progress.json').read_text())['progress']}
    by_script = {}
    for name, repo in repos.items():
        pack = SOURCES[name]['datapack']
        root = Path(repo) / pack / scripts
        for path in sorted(glob.glob(str(root / '**/*.lua'), recursive=True)):
            rel = str(Path(path).relative_to(repo))
            # the key keeps the quest directory, e.g. the_queen_of_the_banshees/action_1_first_seal_lever
            local = Path(path).relative_to(root).with_suffix('')
            script = Script(name, repo, rel, keys, '/'.join(slug(part) for part in local.parts))
            script.declared = declared_by_path
            for interaction in script.interactions():
                by_script.setdefault(interaction['identity']['key'].split(':', 1)[1], {})[name] = interaction
    interactions, manifest_entries, used = [], [], set()
    for _, pair in sorted(by_script.items()):
        primary = pair.get('canary') or pair['crystalserver']
        agree = len(pair) == 2 and comparable(pair['canary']) == comparable(pair['crystalserver'])
        status = 'unresolved_semantics' if primary['unresolved'] or unresolved_conditions(primary) else 'mapped'
        resolution = ('identical in both servers' if agree else 'present only in ' + next(iter(pair)) if len(pair) == 1
                      else 'the servers differ; the Canary transcription is kept')
        if len(pair) == 2 and not agree:
            status = 'conflict'
            decision = CONFLICT_DECISIONS['interactions'].get(primary['identity']['key'])
            if decision:
                used.add(primary['identity']['key'])
                if decision['decision'] == 'crystalserver':
                    # the CrystalServer transcription under the quest's Canary identity (D33)
                    primary = dict(pair['crystalserver'], identity=primary['identity'])
                status = 'unresolved_semantics' if primary['unresolved'] or unresolved_conditions(primary) else 'mapped'
                resolution = 'the servers differ; ' + decided(decision)
        interactions.append({k: v for k, v in primary.items() if k not in ('script', 'callback_line')})
        manifest_entries.append({'destination': primary['identity']['key'], 'status': status, 'resolution': resolution,
                                 'sources': [{'source': n, 'path': SOURCES[n]['datapack'] + '/' + i['script'],
                                              'callback_line': i['callback_line'],
                                              'blob_sha1': git_blob(repos[n], SOURCES[n]['datapack'] + '/' + i['script'])}
                                             for n, i in pair.items()]})
    unused_decisions('interactions', used)
    children = [c for i in interactions for c in walk(i['rules'])]
    declared = {t['key'] for t in json.loads((questlog_dir / 'progress.json').read_text())['progress']}
    manifest = {
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'join': 'script path without its datapack',
        'counts': {
            'interactions': len(interactions),
            'edges': dict(sorted(Counter(i['source']['edge'] for i in interactions).items())),
            'children_by_owner': dict(sorted(Counter(c['owner'] for c in children).items())),
            'blocked_children': dict(sorted(Counter(c['owner'] for c in children if c.get('status') == 'blocked').items())),
            'quest_children_naming_a_transition': sum(1 for c in children if c.get('transition')),
            'unresolved_lines': sum(len(i['unresolved']) for i in interactions),
            'unresolved_conditions': sum(unresolved_conditions(i) for i in interactions),
            'by_status': dict(sorted(Counter(e['status'] for e in manifest_entries).items())),
        },
        # D35: the quest domain has to declare these tracks before an interaction may request them
        'undeclared_progress_tracks': sorted({c['progress'] for c in children if c.get('request') == 'set_progress'} - declared),
        'entries': manifest_entries,
    }
    return {'interactions.json': {'interactions': interactions}, 'manifest.json': manifest}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--scripts', default='scripts/quests')
    parser.add_argument('--questlog', type=Path, default=ROOT / 'samples/questlog')
    parser.add_argument('--out', type=Path, default=ROOT / 'samples/interactions')
    args = parser.parse_args()
    repos = {'canary': args.canary, 'crystalserver': args.crystal}
    for name, repo in repos.items():
        check_checkout(name, repo)
    outputs = build(repos, args.scripts, args.questlog)
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in outputs.items():
        (args.out / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(outputs['manifest.json']['counts'], indent=2))
    print('output sha256 ' + hashlib.sha256(json.dumps(outputs, sort_keys=True).encode()).hexdigest())


if __name__ == '__main__':
    main()
