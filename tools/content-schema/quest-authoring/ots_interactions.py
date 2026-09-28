"""Transcribe quest scripts of Canary and CrystalServer into candidate interaction definitions (D36).

Usage: python ots_interactions.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
                                  [--scripts scripts/quests] [--questlog samples/questlog]
                                  [--out samples/interactions]

An interaction definition is what GAME-INTERACTION-01 plans run: a source edge on a target (a tile entered, an
item used, a creature killed), read-only conditions, and children that each go to the domain owning the effect.
- player storage writes -> Quest (D35), named by the mission transition when one matches;
- `Game.setStorageValue` -> Quest world state (state shared by all players, as encounters read it, D29);
- `Game.createMonster` -> Ability summon effect (GAME-ABILITY-01);
- `player:addItem` -> Item hand-out (DUR-03 item transaction); `addAchievement` -> Achievement grant;
- `sendMagicEffect` (method or procedural `doSendMagicEffect`) and player messages -> presentation only (a message
  keeps its source line, never its text); debug/telemetry logging (`logger.*`) is dropped, never an effect;
- teleports -> Movement, a D37 relocation to a named anchor or the previous tile (in-scope only; a
  computed target that cannot yet name an anchor stays blocked, GAME-INTERACTION-01 §19.3);
- map item transform/create/remove/action-id change -> a D38 world-object overlay operation
  (TRANSFORM/CREATE/REMOVE/RETAG) at the interaction's own target, with `revert_after_ms` when the
  source schedules a decay or timed revert of the same object; a call the converter cannot classify
  by kind stays blocked.
Storage aliases (`local X = Storage.…`) are expanded; a track resolves to the catalogue's track whichever server
declared it (D33). A function literal passed to a call (`addEvent(function() … end)`) runs later and stays one
unresolved entry. Conditions read quest stages, world state, whether the actor is a player, its level, its item
count of a literal item, and the item type, unique id, action id or subtype of the edge source (the registered
target), the object in contact with it or the use target (by field access or, for action id/unique id/subtype,
the equivalent getter method). Any other line or condition stays unresolved with its source line. Source
positions become named anchors with the coordinates kept as evidence. Every output is OTS_HYPOTHESIS_ONLY.
"""
import argparse
import glob
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

import lua_blocks
import lua_tables
from lua_writers import REGISTRATION, argument, expand_aliases, storage_aliases, strip_code
from ots_chests import CONFLICT_DECISIONS, REVISION, ROOT, SOURCES, check_checkout, decided, git_blob, ref, slug, unused_decisions
from ots_questlog import norm, script_of, track_of
from validate_quest_content import BLOCKED, BLOCKED_DUPLICATE_SCHEDULED_REVERT, BLOCKED_INCOMPLETE_CALL, BLOCKED_SCHEDULED_REVERT_DELAY

# Curated per-interaction, per-line replacements for a condition ots_interactions.py cannot read statically (a
# sibling lib/quests/*.lua table indexed by a role field or a world state): interaction key -> source line -> the
# resolved condition plus a basis citing the table/registration evidence. Applied by `apply_overrides`; a stale
# entry (its line no longer unresolved) fails the run via `unused_overrides`.
OVERRIDES = json.loads((ROOT / 'interaction_overrides.json').read_text())

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
# method-call equivalents of the dot-field checks above, restricted to methods that only exist on an Item (never a
# Creature), so the role need not be proven to hold an item first; `getId()` is left out because it is ambiguous
# between an item type id and a creature id and stays unresolved.
METHOD_FIELDS = {'getActionId': 'action_id', 'getUniqueId': 'unique_id', 'getSubType': 'subtype'}
# calls that only read state: a local assignment made of these is not an effect
READ_ONLY = re.compile(r'^(get\w*|is\w*|has\w*|can\w*|Tile|Position|Player|Creature|Monster|Item|Npc|Container|lower|upper|'
                       r'random|max|min|floor|ceil|abs|contains|find|match|sub|gsub|format|len|kv|scoped|time|pairs|ipairs|'
                       r'type|tostring|tonumber|unpack|getn)$')
TABLE_LINE = re.compile(r'^(\{.*\}?,?|\}\)?,?|\[?[\w"\']+\]?\s*=\s*[^()]*,?)$')
MESSAGE = re.compile(r':(sendTextMessage|say|sendCancelMessage|sendChannelMessage|popupFYI)\(')
# a nested loop inside a loop body is itself control structure, not an effect statement: its header/footer must not
# leak into the flat per-line statement pass that `convert()` runs over a loop's body (`block` nodes are not
# reparsed by lua_blocks, so a nested `for`/`while`/`repeat` is only ever told apart from its body here).
CONTROL = re.compile(r'^(if|elseif|else|end|return|break|for|while|repeat|until)\b')
# debug/telemetry logging: never a gameplay effect, so it is dropped rather than left unresolved.
LOGGER = re.compile(r'^logger\.\w+\(.*\)$')
# effects with no accepted owner yet: named precisely so the readiness map can count what is missing.
BOSS_COOLDOWN = re.compile(r':setBossCooldown\(')
CONDITION_CALL = re.compile(r':(addCondition|removeCondition)\(')
KV_WRITE = re.compile(r':kv\(\)|(?:^|[^.\w])kv:(?:set|remove)\(')
# a reward container built in this same callback (`local X = player:addItem(id, n)`), and a plain item created
# for it (`local X = Game.createItem(id)`, never one later customized with its own text, which stays unresolved).
CONTAINER_FILL = re.compile(r'^(\w+):addItem\(\s*(\d+)\s*(?:,\s*(\d+)\s*)?\)$')
CONTAINER_FILL_EX = re.compile(r'^(\w+):addItemEx\(\s*(\w+)\s*\)$')
CREATE_ITEM = re.compile(r'^local\s+(\w+)\s*=\s*Game\.createItem\(')
TEXTED_ITEM = re.compile(r'(\w+):setAttribute\(\s*ITEM_ATTRIBUTE_TEXT\b')
BLOCKED_MOVEMENT = BLOCKED['Movement']
BLOCKED_WORLD_OBJECT = BLOCKED['WorldObject']
# D37: `teleportTo`'s own target argument (the first one, split from the rest with the same
# bracket-aware `split_args` the D38 operations use) decides the relocation: a complete literal
# `Position(x,y,z)` is a named anchor, and the bare `fromPosition` variable itself (trailing arguments
# such as a `pushMove` flag are its own, separate arguments and do not disqualify it) is the previous
# tile. Anything else -- an offset, a lookup, or merely referencing `fromPosition` inside a larger
# expression such as `Position(fromPosition.x + 1, ...)` or `toPosition or Position(1,2,7)` -- is not a
# fully-delimited literal match of that one argument, so it stays a computed, blocked target.
TELEPORT_CALL = re.compile(r'teleportTo\(')
# D38 world-object overlay operations: a call matched by one regex names its operation kind and, via its
# own capture group, the receiver it acts on (the identity a later revert must match to attach, never by
# list position alone). `Game.createItem(...)` has no such receiver; a `local X = Game.createItem(...)`
# constructor's identity is `X`, the name a later revert would itself have to name to attach to it.
TRANSFORM_CALL = re.compile(r'(\w+)[:.](transform|transformItem)\(')
CREATE_CALL = re.compile(r'(\w+)[:.]createItem\(')
RETAG_CALL = re.compile(r'(\w+)[:.]setActionId\(')
# code-gate for a removal call, checked against string-stripped `code` before the receiver-capturing
# form below is ever searched in `raw` (never a look-alike inside a string literal).
REMOVE_METHOD = re.compile(r':(remove|removeItem)\(')
# a transform/createItem/setActionId whose receiver is a call chain (`Tile(pos):getItemById(id):transform(to)`):
# its object cannot be proven, so it stays a blocked WorldObject child rather than typed or lost to unresolved
CHAINED_OBJECT_CALL = re.compile(r'[\w.]+\([^()]*\)(?::\w+\([^()]*\))*[:.](transform|transformItem|createItem|setActionId)\(')
ASSIGNED_LOCAL = re.compile(r'^local\s+(\w+)\s*=')
# a bare `X:revertItem(...)`/`X:decay()` method call names its own receiver; `addEvent`/`stopEvent`
# scheduling `Position.revertItem` and a direct `Position.revertItem(...)` call have no receiver at all
# (revertItem is not a method call there), so only a literal position in their own argument list --
# never a variable's name -- can prove they target the same object as a candidate operation. Checked
# before REVERT_METHOD: `Position.revertItem(` itself syntactically matches `(\w+)[:.]revertItem\(`
# with receiver "Position", which would consume it before its own literal position ever gets parsed.
POSITION_REVERT_ITEM = re.compile(r'\bPosition\.revertItem\(')
REVERT_METHOD = re.compile(r'(\w+)[:.](revertItem|decay)\(')
ADD_STOP_EVENT = re.compile(r'\b(?:add|stop)Event\(')


def parse_revert(code, raw):
    """Whether `raw` is a revert call, and what it provably reverts: (is_revert, receiver, literal
    position groups, literal delay in ms, scheduled, incomplete). Each form is gated on a match in
    `code` (comments/string contents stripped, so a look-alike inside a string literal such as
    `player:say("item:decay()")` never matches) before `raw` is ever searched, and `raw` is searched
    only to parse the call `code` already confirmed. Each call's own argument list is split with
    `split_args` (bracket-nesting aware) and only the specific position argument itself -- never the
    argument list searched as a whole, which would also match a look-alike literal buried in an
    unrelated expression such as `toPosition + Position(1,2,7)` -- is checked for a complete literal
    match. `scheduled` is true only for the addEvent/stopEvent form, the one form whose revert is timed
    by a delay argument rather than being immediate: a non-literal delay there must fail closed (D38
    §4's `revert_after_ms` is never recorded without a literal duration), never merge silently without
    one the way an inherently undelayed `:decay()`/`:revertItem(...)` call is allowed to. `incomplete`
    is true only when a form that parses an argument list has one that never closes on this line (a
    multi-line call): its own fields are never read from that partial text."""
    if POSITION_REVERT_ITEM.search(code) and (m := POSITION_REVERT_ITEM.search(raw)):
        args_text = call_complete(raw, m.end())
        if args_text is None:
            return True, None, None, None, False, True
        args = split_args(args_text)
        target = args[0] if args else None
        pos = POSITION.fullmatch(target) if target else None
        return True, None, (pos.groups() if pos else None), None, False, False
    if REVERT_METHOD.search(code) and (m := REVERT_METHOD.search(raw)):
        return True, m.group(1), None, None, False, False
    if ADD_STOP_EVENT.search(code) and (m := ADD_STOP_EVENT.search(raw)):
        args_text = call_complete(raw, m.end())
        if args_text is None:
            return True, None, None, None, True, True
        args = split_args(args_text)
        if not args or args[0] != 'Position.revertItem':
            return False, None, None, None, False, False
        # addEvent(Position.revertItem, delay, position, ...): the scheduler's own two fixed arguments,
        # then whatever Position.revertItem itself is invoked with; a position is conventionally its
        # first argument (the third argument to addEvent overall). A non-positive literal delay (schema
        # `revert_after_ms` requires >= 1) is treated exactly like a non-literal one -- unresolved, never
        # a recordable duration -- so both fail closed the same way in `revert()`.
        literal_delay = int(args[1]) if len(args) > 1 and re.fullmatch(r'\d+', args[1]) else None
        delay = literal_delay if literal_delay and literal_delay >= 1 else None
        target = args[2] if len(args) > 2 else None
        pos = POSITION.fullmatch(target) if target else None
        return True, None, (pos.groups() if pos else None), delay, True, False
    return False, None, None, None, False, False


def call_complete(text, start):
    """The rest of a call's argument list from `start`, up to its own closing parenthesis -- or `None`
    when that closing parenthesis is not on this line at all (a multi-line call): `lua_writers.argument`
    is read one physical line at a time, so a call whose argument list is still open at end of line would
    otherwise be handed to `split_args` as if it were complete, silently dropping or misreading whatever
    the next line(s) actually contribute. A D38 operation is typed only from a call this confirms closed;
    otherwise its child stays blocked with `BLOCKED_INCOMPLETE_CALL`, never guessed from partial text."""
    depth, i = 0, start
    while i < len(text):
        c = text[i]
        if c in '({[':
            depth += 1
        elif c in ')}]':
            if depth == 0:
                return text[start:i]
            depth -= 1
        i += 1
    return None


def split_args(text):
    """The top-level, comma-separated arguments of a call's argument-list text (bracket-nesting aware,
    so a literal `Position(x, y, z)` argument is not itself split on its own commas)."""
    if not text.strip():
        return []
    parts, depth, current = [], 0, ''
    for c in text:
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        if c == ',' and depth == 0:
            parts.append(current)
            current = ''
        else:
            current += c
    parts.append(current)
    return [p.strip() for p in parts]


def strip_internal(rules):
    """Remove every converter-internal `_`-prefixed key (identity bookkeeping for revert association)
    from a built rule tree, once the whole interaction is final; never reaches interactions.json."""
    for rule in rules:
        if 'branch' in rule:
            for branch in rule['branch']:
                strip_internal(branch['then'])
            strip_internal(rule.get('otherwise', []))
        else:
            for key in [k for k in rule if k.startswith('_')]:
                del rule[key]
# a `Name = {` anywhere in the file, used to discover which names are worth asking lua_tables to parse.
NAME_TABLE = re.compile(r'(\w+)\s*=\s*\{')
# a literal `Storage.…` path, an array step kept only when its index is a literal digit (never a variable).
LITERAL_STORAGE = re.compile(r'Storage(?:\.\w+(?:\[\d+\])?)+$')
# a dotted/bracket path rooted at a table name, split into its `.field`, `[digit]` or `[name]` steps; a `[name]`
# step is a runtime-selected entry (a loop counter or another variable), never resolved statically.
CHAIN = re.compile(r'([A-Za-z_]\w*)((?:\.[A-Za-z_]\w*|\[\d+\]|\[[A-Za-z_]\w*\])*)$')
CHAIN_STEP = re.compile(r'\.([A-Za-z_]\w*)|\[(\d+)\]|\[([A-Za-z_]\w*)\]')
# `local X = player:getStorageValue(...)` / `local X = Game.getStorageValue(...)`: a value alias, scoped to the
# callback it is read in (unlike the file-wide Storage.… path aliases), so a later bare `X <op> N` reads the same
# storage/world-state comparison the direct call already resolves (D36 condition vocabulary, never guessed).
VALUE_ALIAS = re.compile(r'^local\s+(\w+)\s*=\s*((?:\w+:getStorageValue|Game\.getStorageValue)\([^()]*\))$')


class Script:
    def __init__(self, server, repo, path, transitions, name):
        self.server, self.path, self.name = server, path, name
        self.namespace = 'canary' if server == 'canary' else 'crystalserver'
        self.lines = (Path(repo) / path).read_text(errors='replace').split('\n')
        self.transitions = transitions
        self.anchors, self.unresolved = [], []
        self.roles, self.players, self.declared, self.containers = {}, set(), {}, {}
        self.aliases = storage_aliases(self.lines)
        self.tables = self.discover_tables()
        # a plain item created for a reward container, by the local that holds it; excluded once it is customized
        # with its own text (LICENSE-ASSETS.md), so that text is never even indirectly implied by a content list.
        # Any assigned, positionless `Game.createItem(...)` constructor is a reward item, of any arity (a bare
        # id, or the engine's `itemId, count/subtype` form): never a world CREATE, even when its item id is not
        # itself a literal. A literal Position among its arguments makes it a world placement instead (D38), and
        # an incomplete (multi-line) call is never guessed either way.
        texted = {m.group(1) for line in self.lines if (m := TEXTED_ITEM.search(line))}
        self.created_items = {}
        for line in self.lines:
            code = strip_code(line).strip()
            m = CREATE_ITEM.match(code)
            if not m or m.group(1) in texted:
                continue
            args_text = call_complete(code, m.end())
            if args_text is None:
                continue
            args = split_args(args_text)
            if any(POSITION.fullmatch(a) for a in args):
                continue
            self.created_items[m.group(1)] = args[0] if args and re.fullmatch(r'\d+', args[0]) else None

    def bind(self, number, callback):
        """Name the callback parameters by their role and note the locals that hold the acting player."""
        params = re.search(r'\(([^)]*)\)', self.lines[number - 1]).group(1)
        names = [n.strip() for n in params.split(',')]
        self.roles = {n: r for n, r in zip(names, ROLES.get(callback, ())) if r}
        self.callback = callback
        actor = next((n for n, r in self.roles.items() if r == 'actor'), None)
        self.players = {actor} if callback == 'onUse' else set()
        self.containers = {}
        body = [self.raw(n) for n in lua_blocks.function_body(self.lines, number)]
        declared = [m for text in body if (m := VALUE_ALIAS.match(text))]
        # an alias counts only when declared once and never assigned again in the callback, so every later
        # read sees the storage value it was declared with
        self.value_aliases = {m.group(1): m.group(2) for m in declared
                              if sum(1 for d in declared if d.group(1) == m.group(1)) == 1
                              and not any(re.match(rf'^(local\s+)?{re.escape(m.group(1))}\s*=(?!=)', text)
                                          for text in body if not VALUE_ALIAS.match(text))}
        for line in self.lines[number:]:
            if actor and (m := re.match(rf'\s*local\s+(\w+)\s*=\s*{actor}:getPlayer\(\)', line)):
                self.players.add(m.group(1))

    @staticmethod
    def xyz(match):
        """A `POSITION` regex match's groups (from either its literal alternative) as (x, y, z) ints."""
        return tuple(int(v) for v in (match[:3] if match[0] else match[3:]))

    def anchor(self, match):
        x, y, z = self.xyz(match)
        for a in self.anchors:
            if (a['source_position']['x'], a['source_position']['y'], a['source_position']['z']) == (x, y, z):
                return a['key']
        key = f'p{len(self.anchors) + 1}'
        self.anchors.append({'key': key, 'source_position': {'x': x, 'y': y, 'z': z}})
        return key

    def same_target(self, prior, receiver, pos):
        """Whether a revert's own receiver or literal position provably names the same object as
        `prior` (an already-emitted typed WorldObject operation): the same identity (a receiver name,
        or a `local X = ...` constructor's own name), or the same literal position as `prior`'s already
        pre-authored anchor. List position alone (being merely the nearest preceding operation) is
        never sufficient by itself."""
        if receiver and prior.get('_identity') == receiver:
            return True
        if pos and prior.get('anchor'):
            registered = next((a['source_position'] for a in self.anchors if a['key'] == prior['anchor']), None)
            return registered is not None and tuple(registered.values()) == self.xyz(pos)
        return False

    def track(self, storage):
        """The catalogue's progress track for a storage, whichever server declared it (D33), else this server's."""
        path = track_of(storage)
        return self.declared.get(norm(path), f'{self.namespace}:quest-progress/{path}')

    def discover_tables(self):
        """Every `Name = { … }` table literal in this file, read once with lua_tables so a storage key that is a
        path into one of them (`config.storage`, `rewards[3148].storage`) can be resolved without evaluating Lua.
        Any parse trouble (a table holding something lua_tables cannot read) just leaves this file's tables empty:
        the keys stay unresolved rather than guessed."""
        text = '\n'.join(self.lines)
        names = {m.group(1) for m in NAME_TABLE.finditer(text)}
        try:
            parsed = lua_tables.assignments(text, names)
            return {name: lua_tables.as_python(table) for name, table in parsed.items()}
        except Exception:
            return {}

    def resolve_chain(self, expr):
        """A dotted/bracket-indexed path rooted at one of this file's table literals, resolved to its leaf value
        (never a runtime-selected entry: a `[name]` step, indexing by a variable or loop counter, stops it)."""
        m = CHAIN.fullmatch(expr)
        if not m or m.group(1) not in self.tables:
            return None
        value = self.tables[m.group(1)]
        for field, index, runtime in CHAIN_STEP.findall(m.group(2)):
            if runtime:
                return None
            if field:
                if not isinstance(value, dict) or field not in value:
                    return None
                value = value[field]
            else:
                index = int(index)
                if isinstance(value, list) and 1 <= index <= len(value):
                    value = value[index - 1]
                elif isinstance(value, dict) and index in value:
                    value = value[index]
                else:
                    return None
        return value

    def resolve_storage(self, expr):
        """A storage-key expression as a literal `Storage.…` path or integer: already literal, or a path into a
        table literal declared in this same file (D33); anything else (a runtime-selected entry, an expression
        lua_tables cannot read) stays unresolved rather than guessed."""
        if LITERAL_STORAGE.fullmatch(expr):
            return expr
        if re.fullmatch(r'-?\d+', expr):
            return int(expr)
        value = self.resolve_chain(expr)
        if isinstance(value, dict) and isinstance(value.get('expr'), str) and LITERAL_STORAGE.fullmatch(value['expr']):
            return value['expr']
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return int(value)
        return None

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
        # never inside a string literal (e.g. the `"switchNum"` key of the very call an alias stands for)
        for alias, expr in self.value_aliases.items():
            text = re.sub(rf'(?<![\w.:"\']){re.escape(alias)}(?![\w"\'])', expr, text)
        parts = re.split(r'\s+(and|or)\s+', text)
        terms, joins = parts[0::2], set(parts[1::2])
        if len(joins) > 1:
            return {'unresolved': {'line': number}}
        out = []
        for term in terms:
            negate = term.startswith('not ')
            body = term[4:] if negate else term
            if (m := re.fullmatch(r'\w+:getStorageValue\(\s*([\w.\[\]]+)\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)', body)) \
                    and (target := self.resolve_storage(m.group(1))) is not None:
                out.append({'quest_stage': {'progress': self.track(target),
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
            elif (m := re.fullmatch(r'(\w+):(getActionId|getUniqueId|getSubType)\(\)\s*(==|~=)\s*(\d+)', body)) and m.group(1) in self.roles:
                out.append({'object': {'role': self.roles[m.group(1)], 'field': METHOD_FIELDS[m.group(2)],
                                       'op': m.group(3), 'value': int(m.group(4))}, 'negate': negate})
            elif body in self.players or ((m := re.fullmatch(r'(\w+):isPlayer\(\)', body))
                                          and self.roles.get(m.group(1)) == 'actor'):
                out.append({'actor_is_player': True, 'negate': negate})
            elif (m := re.fullmatch(r'(\w+):getLevel\(\)\s*(==|~=|<=|>=|<|>)\s*(\d+)', body)) and m.group(1) in self.players:
                out.append({'actor_level': {'op': m.group(2), 'value': int(m.group(3))}, 'negate': negate})
            elif (m := re.fullmatch(r'(\w+):getItemCount\(\s*(\d+)\s*\)\s*(==|~=|<=|>=|<|>)\s*(\d+)', body)) \
                    and m.group(1) in self.players:
                out.append({'actor_item_count': {'item': ref('Item', f'{self.namespace}:item/{m.group(2)}'),
                                                 'op': m.group(3), 'value': int(m.group(4))}, 'negate': negate})
            else:
                return {'unresolved': {'line': number}}
        if len(out) == 1:
            return out[0]
        return {('any' if 'or' in joins else 'all'): out}

    def children(self, number, in_loop=False):
        code, raw = strip_code(self.lines[number - 1]).strip(), self.raw(number)
        found = []
        for m in re.finditer(r'(\w+):setStorageValue\(\s*([\w.\[\]]+)\s*,', raw):
            target = self.resolve_storage(m.group(2))
            if target is None:
                continue
            track = self.track(target)
            value = raw[m.end():].split(')')[0].strip()
            child = {'owner': 'Quest', 'request': 'set_progress', 'progress': track,
                     **({'to': int(value)} if re.fullmatch(r'-?\d+', value) else {'value_source_line': number})}
            key = self.transitions.get((norm(track_of(target)), script_of(self.path), number))
            if key:
                child['transition'] = key
            found.append(child)
        for m in re.finditer(r'Game\.setStorageValue\(\s*"?([\w.\[\]]+)"?\s*,\s*([^)]*)\)', raw):
            key = m.group(1)
            if re.search(r'\[[^\d\]]', key):
                continue  # a runtime index (a loop counter, a role field): not a fixed world-state name
            value = m.group(2).strip()
            found.append({'owner': 'Quest', 'request': 'set_world_state', 'key': f'{self.namespace}:world-state/{slug(key)}',
                          **({'to': int(value)} if re.fullmatch(r'-?\d+', value) else {'value_source_line': number})})
        if (m := re.search(r'Game\.createMonster\(\s*"([^"]+)"\s*,\s*(.*)', raw)):
            pos = POSITION.search(m.group(2))
            found.append({'owner': 'Ability', 'effect': 'summon',
                          'creature': ref('Creature', f'{self.namespace}:creature/{slug(m.group(1))}'),
                          **({'anchor': self.anchor(pos.groups())} if pos else {'anchor_source_line': number})})
        if 'teleportTo(' in code and (tm := TELEPORT_CALL.search(raw)):
            target_args = split_args(argument(raw, tm.end()))
            target = target_args[0] if target_args else None
            pos = POSITION.fullmatch(target) if target else None
            if pos:
                # D37: a relocation to a named anchor, in the current scope (VSL-MOVE-01/ChannelRuntime);
                # cross-Channel/Instance relocation (SCOPE_HANDOFF) has no source signal here and stays
                # out of scope until that contract exists (proposal §3).
                found.append({'owner': 'Movement', 'request': 'relocate', 'scope': 'in_scope',
                              'target': {'kind': 'anchor', 'anchor': self.anchor(pos.groups())}})
            elif target == 'fromPosition':
                found.append({'owner': 'Movement', 'request': 'relocate', 'scope': 'in_scope',
                              'target': {'kind': 'previous_position'}})
            else:
                # a computed target (an offset, a table lookup, anything short of the bare previous-
                # position variable itself as its own complete argument): stays blocked until its
                # definition can name an anchor, or a DUR-04 component proposes the target.
                found.append({'owner': 'Movement', 'status': 'blocked', 'reason': BLOCKED_MOVEMENT, 'to_source_line': number})
        # D38 invariant: an operation is typed only when its call is recognized EXACTLY -- the keyword
        # itself confirmed in `code` (never a string literal), its own argument list confirmed complete
        # on this line, and the specific typed field itself a fully-delimited literal (never a
        # substring, a list-order guess, or a partial/computed expression). Anything short of that
        # stays its own blocked WorldObject child with an explicit reason; nothing is ever guessed.
        # Every operation below is gated on a match in `code` (comments/string contents stripped)
        # before `raw` is ever searched for it, so a look-alike inside a string literal (e.g.
        # `player:say("Game.createItem(2793)")`) is never mistaken for the real call; `raw` is then
        # searched only to parse the call `code` already confirmed is really there. The argument may
        # hold one nested call (`removeItem(ids[math.random(#ids)], 1)`).
        removal = REMOVE_METHOD.search(code) and re.search(r'([\w.]+(?:\([^()]*\))?):(remove|removeItem)\(((?:[^()]|\([^()]*\))*)\)', raw)
        consumed = removal and self.consumed(removal, number)
        if consumed:
            found.append(consumed)
        elif removal and self.creature(removal.group(1)):
            self.unresolved.append({'line': number, 'reason': 'creature removal without an accepted owner'})
            return found
        elif removal:
            receiver = removal.group(1)
            literal_pos = POSITION.fullmatch(receiver) if '(' in receiver else None
            if '(' in receiver and not literal_pos:
                # a computed receiver (a call whose own result is not itself a fully-delimited literal
                # position, e.g. a tile lookup): its identity cannot be proven, so it stays blocked
                # rather than typed against an unprovable target (invariant: not recognized exactly).
                found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_WORLD_OBJECT, 'source_line': number})
            else:
                # D38: removing a non-value-bearing map object (a wall, a stone, a barrier), never a
                # carried item (DUR-03 consumption above) or a creature (unresolved, C4). A receiver
                # that is itself a fully-delimited literal position binds a pre-authored anchor.
                child = {'owner': 'WorldObject', 'operation': 'REMOVE', 'value_source_line': number, '_identity': receiver}
                if literal_pos:
                    child['anchor'] = self.anchor(literal_pos.groups())
                found.append(child)
        elif TRANSFORM_CALL.search(code) and (m := TRANSFORM_CALL.search(raw)):
            # from/to are rarely both literal in one call (the current id is usually implicit, read from
            # the object the script already holds), so this stays with its source line rather than guessed.
            found.append({'owner': 'WorldObject', 'operation': 'TRANSFORM', 'value_source_line': number,
                         '_identity': m.group(1)})
        elif CREATE_CALL.search(code) and (m := CREATE_CALL.search(raw)):
            constructor = CREATE_ITEM.match(code)
            args_text = call_complete(raw, m.end())
            if constructor and constructor.group(1) in self.created_items:
                # a reward-container constructor (this local is a hand-out/contents item, DUR-03), never
                # a world placement, even though the call itself is `Game.createItem(...)`.
                pass
            elif args_text is None:
                # the call's argument list never closes on this line (a multi-line call): never typed
                # from partial args (invariant: not recognized exactly stays blocked, never guessed).
                found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_INCOMPLETE_CALL, 'source_line': number})
            else:
                args = split_args(args_text)
                item_id = args[0] if args and re.fullmatch(r'\d+', args[0]) else None
                extra = args[1:]
                # engine signature `createItem(itemId, count/subtype, position)`: a bare integer among
                # the extra arguments is the count/subtype, never a placement; a literal Position is the
                # anchor; anything else could be a placement expression (C3: no dynamic geometry), so it
                # stays blocked.
                literal_pos = next((a for a in extra if POSITION.fullmatch(a)), None)
                ambiguous = any(not re.fullmatch(r'\d+', a) and not POSITION.fullmatch(a) for a in extra)
                literal = {'def': ref('Item', f'{self.namespace}:item/{item_id}')} if item_id else {'value_source_line': number}
                identity = (assigned := ASSIGNED_LOCAL.match(code)) and assigned.group(1)
                if ambiguous:
                    found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_WORLD_OBJECT, 'source_line': number})
                elif literal_pos:
                    # C3: CREATE only binds a pre-authored anchor with a fixed footprint; a literal
                    # position argument names one.
                    child = {'owner': 'WorldObject', 'operation': 'CREATE',
                             'anchor': self.anchor(POSITION.fullmatch(literal_pos).groups()), **literal}
                    if identity:
                        child['_identity'] = identity
                    found.append(child)
                else:
                    child = {'owner': 'WorldObject', 'operation': 'CREATE', **literal}
                    if identity:
                        child['_identity'] = identity
                    found.append(child)
        elif RETAG_CALL.search(code) and (m := RETAG_CALL.search(raw)):
            # D38/coordinator decision 1c: a transition between two states of the same collision class;
            # the action id itself is not modeled as data.
            found.append({'owner': 'WorldObject', 'operation': 'RETAG', 'value_source_line': number,
                         '_identity': m.group(1)})
        elif CHAINED_OBJECT_CALL.search(code):
            # the object is reached through a call (a tile or position lookup): its identity cannot be
            # proven, so this stays a blocked WorldObject child, never typed and never lost to unresolved
            found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_WORLD_OBJECT, 'source_line': number})
        else:
            is_revert, receiver, pos, delay, scheduled, incomplete = parse_revert(code, raw)
            if incomplete:
                found.append({'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED_INCOMPLETE_CALL, 'source_line': number})
            elif is_revert:
                # never its own child (D38 §4): `convert()` attaches it to the operation it provably
                # reverts (the preceding typed operation in this same statement list with the same
                # receiver/anchor identity), or leaves it blocked when none matches deterministically,
                # or when it is scheduled (addEvent) with no literal delay to record (fail closed).
                found.append({'owner': 'WorldObject', '_revert': True, '_revert_receiver': receiver,
                              '_revert_position': pos, '_revert_delay_ms': delay, '_revert_scheduled': scheduled,
                              '_source_line': number})
        if (m := CONTAINER_FILL.match(raw)) and m.group(1) in self.containers:
            # a plain item added to a reward container built earlier in this same callback (DUR-03): its contents
            self.containers[m.group(1)].setdefault('contents', []).append(
                {'item': ref('Item', f'{self.namespace}:item/{m.group(2)}'), 'count': int(m.group(3) or 1)})
            return found
        if (m := CONTAINER_FILL_EX.match(raw)) and m.group(1) in self.containers:
            # membership alone only proves this local is a positionless reward constructor (never a
            # world CREATE, see self.created_items above); its item id is resolvable here only when
            # that constructor's own id argument was itself a literal.
            item_id = self.created_items.get(m.group(2))
            if item_id:
                self.containers[m.group(1)].setdefault('contents', []).append(
                    {'item': ref('Item', f'{self.namespace}:item/{item_id}'), 'count': 1})
            else:
                self.unresolved.append({'line': number, 'reason': 'container reward item is not a plain literal item type'})
            return found
        if (m := re.search(r'(\w+):addItem\(\s*(\d+)?\s*(?:,\s*(\d+)\s*)?', raw)) and m.group(1) in self.players | {'player'}:
            child = {'owner': 'Item', 'request': 'hand_out',
                     **({'item': ref('Item', f'{self.namespace}:item/{m.group(2)}'), 'count': int(m.group(3) or 1)}
                        if m.group(2) and raw[m.end():m.end() + 1] == ')' else {'value_source_line': number})}
            found.append(child)
            if 'item' in child and (alias := re.match(r'local\s+(\w+)\s*=', raw)):
                # this local now names the container, so a later plain `NAME:addItem(...)`/`:addItemEx(...)` in the
                # same callback is that reward's contents, not a separate unowned effect
                self.containers[alias.group(1)] = child
        if re.search(r':addAchievement\(', raw):
            if (m := re.search(r':addAchievement\(\s*"([^"]+)"\s*\)', raw)):
                found.append({'owner': 'Achievement', 'request': 'grant',
                              'achievement': ref('Achievement', f'{self.namespace}:achievement/{slug(m.group(1))}')})
            else:
                # a table-driven achievement id (e.g. `reward.achievement[1]`): the achievement itself is not literal
                found.append({'owner': 'Achievement', 'request': 'grant', 'value_source_line': number})
        if (m := re.search(r'(\w+):addOutfitAddon\(\s*"?(\d+)"?\s*(?:,\s*"?(\d+)"?\s*)?', raw)) and m.group(1) in self.players | {'player'}:
            found.append({'owner': 'Outfit', 'request': 'grant',
                          **({'looktype': int(m.group(2)), **({'addon': int(m.group(3))} if m.group(3) else {})}
                             if m.group(2) else {'value_source_line': number})})
        if (m := re.search(r'(\w+):addOutfit\(\s*"?(\d+)"?\s*(?:,\s*"?(\d+)"?\s*)?', raw)) and m.group(1) in self.players | {'player'}:
            found.append({'owner': 'Outfit', 'request': 'grant',
                          **({'looktype': int(m.group(2)), **({'addon': int(m.group(3))} if m.group(3) else {})}
                             if m.group(2) else {'value_source_line': number})})
        if (m := re.search(r'(\w+):addMount\(\s*"?(\d+)"?\s*\)?', raw)) and m.group(1) in self.players | {'player'}:
            found.append({'owner': 'Mount', 'request': 'grant',
                          **({'mount': int(m.group(2))} if m.group(2) else {'value_source_line': number})})
        if (m := re.search(r'(\w+):addExperience\(\s*(\d+)?', raw)) and m.group(1) in self.players | {'player'}:
            found.append({'owner': 'Experience', 'request': 'grant',
                          **({'amount': int(m.group(2))} if m.group(2) else {'value_source_line': number})})
        if re.search(r':addMapMark\(', code):
            # the label is reserved (LICENSE-ASSETS.md); the mark's position and type are kept out too, since D36
            # keeps this owner-neutral (no `Presentation` child otherwise carries map coordinates)
            found.append({'owner': 'Presentation', 'effect': 'map_mark', 'authoritative': False, 'source_line': number})
        if re.search(r'(?i)sendmagiceffect\(', code):
            # both the method (`:sendMagicEffect(`) and the procedural (`doSendMagicEffect(`) forms
            found.append({'owner': 'Presentation', 'effect': 'magic_effect', 'authoritative': False})
        if MESSAGE.search(code):
            # the text itself is reserved (LICENSE-ASSETS.md); only its source line is kept
            found.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False, 'source_line': number})
        if not found and re.search(r'\baddEvent\(', code):
            self.unresolved.append({'line': number, 'reason': 'delayed callback (addEvent) without a scheduler owner'})
        elif not found and LOGGER.match(code):
            pass  # debug/telemetry logging, never a gameplay effect
        elif not found and BOSS_COOLDOWN.search(code):
            self.unresolved.append({'line': number, 'reason': 'boss cooldown without an accepted owner'})
        elif not found and CONDITION_CALL.search(code):
            self.unresolved.append({'line': number, 'reason': 'condition without an accepted owner'})
        elif not found and KV_WRITE.search(code):
            self.unresolved.append({'line': number, 'reason': 'kv write without an accepted owner'})
        elif not found and not NOOP.match(code) and not self.read_only(code):
            self.unresolved.append({'line': number, 'reason': 'statement outside the transcribed vocabulary'})
        for child in found:
            if in_loop:
                child['repeated'] = True
        return found

    def consumed(self, removal, number):
        """A removal that takes an item from its holder is DUR-03 consumption, never map state (D38): the player's
        `removeItem`, or `remove` on the item used (`onUse`) or dropped onto the edge (`onAddItem`)."""
        receiver, method, args = removal.groups()
        if method == 'removeItem' and receiver in self.players | {'player'}:
            m = re.fullmatch(r'\s*(\d+)\s*(?:,\s*(\d+)\s*)?', args)
            return {'owner': 'Item', 'request': 'consume',
                    **({'item': ref('Item', f'{self.namespace}:item/{m.group(1)}'), 'count': int(m.group(2) or 1)}
                       if m else {'value_source_line': number})}
        role = self.roles.get(receiver)
        if method == 'remove' and ((role == 'source' and self.callback == 'onUse') or role == 'contact'):
            return {'owner': 'Item', 'request': 'consume', 'object': 'used_item' if role == 'source' else 'contact'}
        return None

    def creature(self, receiver):
        """A receiver that holds a creature, not an item."""
        return (self.roles.get(receiver) == 'actor' or receiver in self.players
                or re.search(r'(?i)creature|monster|boss|npc|summon|spectator', receiver) is not None)

    def revert(self, out, child):
        """Attach a `_revert` sentinel (from `children()`) to the operation it provably reverts: the
        preceding typed WorldObject operation, anywhere earlier in this same statement list (not only
        the immediately preceding one), whose own identity (receiver, or a `local X = ...` constructor's
        name) or pre-authored anchor the revert's own receiver/literal position matches (D38 §4: the
        revert is the same object's own later operation, so it is transcribed as a field on that
        operation, never its own child). List position is never itself the match: a revert with no
        matching candidate, or with more than one equally plausible candidate, stays blocked instead --
        as does a scheduled (addEvent) revert with no literal delay, which fails closed rather than
        merging silently with no `revert_after_ms` (only an inherently undelayed revert may do that).
        A second scheduled revert that provably targets an operation which already carries a
        `revert_after_ms` (from an earlier one, in source order) never overwrites it -- the operation's
        actual revert delay cannot be inferred from two conflicting schedules -- and stays blocked
        instead, with its own explicit reason distinct from an unresolved delay or no candidate at all."""
        receiver, pos = child['_revert_receiver'], child['_revert_position']
        candidates = [c for c in out if c.get('owner') == 'WorldObject' and 'operation' in c
                     and self.same_target(c, receiver, pos)]
        unresolved_delay = child['_revert_scheduled'] and child['_revert_delay_ms'] is None
        duplicate = (len(candidates) == 1 and child['_revert_delay_ms'] is not None
                    and 'revert_after_ms' in candidates[0])
        if len(candidates) != 1 or unresolved_delay or duplicate:
            if duplicate:
                reason = BLOCKED_DUPLICATE_SCHEDULED_REVERT
            elif len(candidates) == 1 and unresolved_delay:
                reason = BLOCKED_SCHEDULED_REVERT_DELAY
            else:
                reason = BLOCKED_WORLD_OBJECT
            blocked = {'owner': 'WorldObject', 'status': 'blocked', 'reason': reason,
                      'source_line': child['_source_line']}
            if child.get('repeated'):
                blocked['repeated'] = True
            out.append(blocked)
        elif child['_revert_delay_ms'] is not None:
            candidates[0]['revert_after_ms'] = child['_revert_delay_ms']

    def append_children(self, out, children):
        for child in children:
            if child.pop('_revert', False):
                self.revert(out, child)
            else:
                out.append(child)

    def convert(self, nodes):
        out = []
        for node in nodes:
            if node[0] == 'stmt':
                self.append_children(out, self.children(node[1]))
            elif node[0] == 'block':
                inner = node[1][1:-1]
                control = [n for n in inner if CONTROL.match(strip_code(self.lines[n - 1]).strip())]
                for number in inner:
                    if number not in control:
                        self.append_children(out, self.children(number, in_loop=True))
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
            strip_internal(rules)
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


def apply_overrides(rules, key, used):
    """Replace an unresolved condition with a curated override's equivalent (`interaction_overrides.json`), for a
    script-local table or loop `ots_interactions.py` cannot read statically (e.g. a sibling `lib/quests/*.lua`
    table indexed by a role field or a world state). Every replacement is recorded in `used` so a stale override
    (naming a line that is no longer unresolved) is caught by `unused_overrides` rather than silently ignored."""
    overrides = OVERRIDES.get(key, {})
    if not overrides:
        return

    def replace(cond):
        if 'all' in cond or 'any' in cond:
            combinator = 'all' if 'all' in cond else 'any'
            cond[combinator] = [replace(c) for c in cond[combinator]]
            return cond
        line = cond.get('unresolved', {}).get('line')
        override = overrides.get(str(line)) if line is not None else None
        if override:
            used.add((key, str(line)))
            return json.loads(json.dumps(override['condition']))
        return cond

    def walk(nodes):
        for rule in nodes:
            if 'branch' in rule:
                for arm in rule['branch']:
                    arm['when'] = replace(arm['when'])
                    walk(arm['then'])
                walk(rule.get('otherwise', []))
    walk(rules)


def unused_overrides(used):
    """Fail loudly when a recorded interaction override no longer matches an unresolved line."""
    stale = sorted(f'{key}#{line}' for key, lines in OVERRIDES.items() for line in lines
                  if (key, line) not in used)
    if stale:
        raise SystemExit(f'interaction overrides without a matching unresolved line: {stale}')


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
    interactions, manifest_entries, used, used_overrides = [], [], set(), set()
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
        if primary['identity']['key'] in OVERRIDES:
            primary = dict(primary, rules=json.loads(json.dumps(primary['rules'])))
            apply_overrides(primary['rules'], primary['identity']['key'], used_overrides)
            status = 'unresolved_semantics' if primary['unresolved'] or unresolved_conditions(primary) else 'mapped'
        interactions.append({k: v for k, v in primary.items() if k not in ('script', 'callback_line')})
        manifest_entries.append({'destination': primary['identity']['key'], 'status': status, 'resolution': resolution,
                                 'sources': [{'source': n, 'path': SOURCES[n]['datapack'] + '/' + i['script'],
                                              'callback_line': i['callback_line'],
                                              'blob_sha1': git_blob(repos[n], SOURCES[n]['datapack'] + '/' + i['script'])}
                                             for n, i in pair.items()]})
    unused_decisions('interactions', used)
    unused_overrides(used_overrides)
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
            # D37: relocation children by their target kind (anchor or the previous tile); a computed
            # target is not one of these, it stays counted under blocked_children.
            'relocation_children': dict(sorted(Counter(
                c['target']['kind'] for c in children if c.get('owner') == 'Movement' and c.get('request') == 'relocate').items())),
            # D38: world-object overlay operations by kind, and how many of those carry a revert_after_ms
            'overlay_operations': dict(sorted(Counter(
                c['operation'] for c in children if c.get('owner') == 'WorldObject' and 'operation' in c).items())),
            'overlay_operations_with_revert_after': sum(
                1 for c in children if c.get('owner') == 'WorldObject' and 'revert_after_ms' in c),
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
