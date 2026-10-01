"""Find where OTS Lua scripts set a player storage, and read each write as a candidate quest transition (D35).

For every `setStorageValue(<storage>, <value>)` the reader records:
- the owner: the script kind of the enclosing callback (movement, action, creature event, global event, talk action,
  event callback), `npc` for NPC files, `library` for lib files, `other` otherwise;
- the target: a literal value (`to`), a step on the same storage (`increment`), or a computed value (a
  `timestamp` from `os.time()`, i.e. a cooldown, or another `expression`);
- the guard: the nearest comparison of the same storage before the write in the enclosing function, read through
  its if-block (kept in `then`, negated in `else` or after an early `return`), as `from`;
- the registrations of the script object (`:uid`, `:aid`, `:id`, `:position`, `:type`).
The guard is a line-level heuristic, not an evaluation: a missing or nested condition leaves `from` unknown.
"""
import re
from functools import lru_cache

import lua_tables
import scoped_storage_aliases

WRITE = re.compile(r'setStorageValue\(\s*(Storage\.[A-Za-z0-9_.]+(?:\[\d+\])?|\d+)\s*,')
PROCEDURAL_WRITE = re.compile(r'(?<![\w.:])setPlayerStorageValue\s*\(')
STORAGE_TARGET = re.compile(r'(?:Storage\.[A-Za-z0-9_.]+(?:\[\d+\])?|\d+)')
FUNCTION = re.compile(r'^\s*(?:local\s+)?function\s+([A-Za-z_]\w*)(?:[.:]([A-Za-z_]\w*))?\s*\(')
OBJECT = re.compile(r'^\s*local\s+(\w+)\s*=\s*(MoveEvent|Action|CreatureEvent|GlobalEvent|TalkAction|EventCallback)\s*\(')
REGISTRATION = re.compile(r'^\s*(\w+):(uid|aid|id|position|type)\((.*)\)\s*$')
KINDS = {'MoveEvent': 'movement', 'Action': 'action', 'CreatureEvent': 'creature_event', 'GlobalEvent': 'global_event',
         'TalkAction': 'talk_action', 'EventCallback': 'event_callback'}


def argument(text, start):
    """The rest of a call's argument list from `start`, up to its closing parenthesis (strings are not special)."""
    depth, i = 0, start
    while i < len(text):
        c = text[i]
        if c in '({[':
            depth += 1
        elif c in ')}]':
            if depth == 0:
                return text[start:i]
            depth -= 1
        elif c == '\n' and depth == 0:
            return text[start:i]
        i += 1
    return text[start:]


NEGATE = {'==': '~=', '~=': '==', '<': '>=', '>=': '<', '>': '<=', '<=': '>'}
OPENER = re.compile(r'\b(if|for|while|function|repeat)\b')
CLOSER = re.compile(r'\b(end|until)\b')


def mask_code(text, literals=True, long_literals=False):
    """Preserve lexical offsets while blanking comments and optionally Lua strings."""
    out, pos = [], 0
    while pos < len(text):
        match = lua_tables.TOKEN.match(text, pos)
        if not match:
            out.append(text[pos])
            pos += 1
            continue
        token = match.group(0)
        masked = (match.group('comment') is not None or match.group('lcomment') is not None
                  or literals and match.group('string') is not None
                  or (literals or long_literals) and match.group('lstring') is not None)
        out.append(''.join('\n' if c == '\n' else ' ' for c in token) if masked else token)
        pos = match.end()
    return ''.join(out)


def strip_code(line):
    return mask_code(line)


def builtin_binding_is_pristine(lines, root):
    return _builtin_binding_is_pristine(tuple(lines), root)


@lru_cache(maxsize=4096)
def _builtin_binding_is_pristine(lines, root):
    """Global library assumptions require no shadow, mutation or root-table escape.
    Detect complete assignment LHS lists, including fields before a comma."""
    try:
        tokens = [t for t in lua_tables.tokenize('\n'.join(lines))
                  if t[0] not in ('comment', 'lcomment')]
    except lua_tables.LuaError:
        return False

    def after_path(index):
        index += 1
        while index < len(tokens):
            if tokens[index][1] == '.' and index + 1 < len(tokens) and tokens[index + 1][0] == 'name':
                index += 2
            elif tokens[index][1] == '[':
                depth = 1
                index += 1
                while index < len(tokens) and depth:
                    depth += (tokens[index][1] == '[') - (tokens[index][1] == ']')
                    index += 1
            else:
                break
        return index

    calls = []
    for index, (kind, token, _) in enumerate(tokens):
        if token == '(':
            k = index - 1
            while k >= 2 and tokens[k - 1][1] in ('.', ':'):
                k -= 2
            calls.append(k >= 0 and (tokens[k][1] == 'function' or k > 0 and tokens[k - 1][1] == 'function'))
        elif token == ')' and calls:
            calls.pop()
        if kind != 'name' or token != root or index and tokens[index - 1][1] in ('.', ':'):
            continue
        if calls and calls[-1] or index and tokens[index - 1][1] == 'function':
            return False  # parameter or method/function declaration
        following = tokens[index + 1][1] if index + 1 < len(tokens) else None
        if following not in ('.', '[' , ':'):
            return False  # declaration, root assignment, alias/argument/return escape
        end = after_path(index)
        while end < len(tokens) and tokens[end][1] == ',':
            end += 1
            if end >= len(tokens) or tokens[end][0] != 'name':
                break
            end = after_path(end)
        if (end < len(tokens) and tokens[end][1] == '='
                and not (end + 1 < len(tokens) and tokens[end + 1][1] == '=')):
            return False
    return True


def guard_at_write(lines, guard_line, write_line, op, value):
    """The condition on the storage that holds when the write runs, read from the if-block of the guard.

    The write inside the `then` branch keeps the comparison; inside `else`, or after an if-block whose `then`
    branch returns, it is negated; anything else (elseif, a loop, a block that falls through) leaves it unknown.
    A compound condition (`and`/`or`) marks the result as not exact.
    """
    condition = strip_code(lines[guard_line - 1])
    exact = not re.search(r'\b(and|or)\b', condition)
    depth, branch, returned = 0, 'then', False
    for number in range(guard_line, write_line):
        code = strip_code(lines[number - 1])
        if number == guard_line:
            depth = 1 if re.search(r'\bif\b', code) else 0
            if depth == 0:
                return None
            if CLOSER.search(code):  # one-line `if ... then ... end`
                depth, returned = 0, bool(re.search(r'\breturn\b', code))
            continue
        if depth == 1 and re.match(r'\s*elseif\b', code):
            return None
        if depth == 1 and re.match(r'\s*else\b', code):
            branch = 'else'
            continue
        if depth == 1 and branch == 'then' and re.match(r'\s*return\b', code):
            returned = True
        depth += len(OPENER.findall(code)) - len(CLOSER.findall(code))
        if depth <= 0:
            break
    if depth >= 1:
        keep = branch == 'then'
    elif returned and branch == 'then':
        keep = False
    else:
        return None
    return {'op': op if keep else NEGATE[op], 'value': value, 'exact': exact}


def dialogue_at_write(lines, start, write_line):
    """The player keywords (`MsgContains`) and dialogue topics of the if-blocks that enclose an NPC write.

    A stack of open blocks is kept from the callback start: `if`/`elseif` put their line on top, `else` and
    loops put nothing, `end` pops. Only the conditions still open at the write count.
    """
    stack = []
    for number in range(start, write_line):
        code = strip_code(lines[number - 1]).strip()
        if re.match(r'elseif\b', code) and stack:
            stack[-1] = number
        elif re.fullmatch(r'else', code) and stack:
            stack[-1] = None
        else:
            opened = len(OPENER.findall(code)) - len(CLOSER.findall(code))
            for _ in range(max(opened, 0)):
                stack.append(number if re.match(r'if\b', code) else None)
            for _ in range(min(-opened, len(stack))):
                stack.pop()
    # strip_code blanks string literals, so keywords come from the raw condition lines
    conditions = [re.sub(r'--.*$', '', lines[n - 1]) for n in stack if n]
    keywords = list(dict.fromkeys(k for c in conditions for k in re.findall(r'MsgContains\(\s*\w+\s*,\s*"([^"]+)"', c)))
    topics = sorted({int(t) for c in conditions for t in re.findall(r'[Tt]opic\w*(?:\[[^\]]*\]|\([^)]*\))?\s*==\s*(\d+)', c)})
    return {'keywords': keywords, 'topics': topics}


ALIAS = re.compile(r'\s*local\s+(\w+)\s*=\s*((?:Global)?Storage\.[\w.\[\]]+)\s*(--.*)?$')


def unconditional_prefix(lines):
    """Unconditional file declarations before any function/control flow; lexical tokens
    exclude quoted/commented keywords and table entries are not declarations."""
    allowed, depth = set(), 0
    try:
        tokens = lua_tables.tokenize('\n'.join(lines))
    except lua_tables.LuaError:
        return allowed
    for kind, value, number in tokens:
        if kind in ('comment', 'lcomment', 'string', 'lstring'):
            continue
        if kind == 'name' and value in ('function', 'if', 'for', 'while', 'repeat', 'do'):
            break
        if depth == 0:
            allowed.add(number)
        if value in ('{', '[', '('):
            depth += 1
        elif value in ('}', ']', ')'):
            depth -= 1
    return allowed


def scalar_leaf_paths(value, root):
    """Only primitive literal leaves (plus named integer Storage constants) can
    escape by value; a table/Position/subtable reference is never one."""
    if type(value) in (int, float, str, bool):
        return {root}
    if isinstance(value, dict) and set(value) == {'expr'}:
        return {root} if re.fullmatch(r'(?:Global)?Storage\.[\w.\[\]]+', value['expr']) else set()
    out = set()
    if isinstance(value, dict):
        for key, leaf in value.items():
            suffix = f'[{key}]' if type(key) is int else f'.{key}'
            out.update(scalar_leaf_paths(leaf, root + suffix))
    elif isinstance(value, list):
        for index, leaf in enumerate(value, 1):
            out.update(scalar_leaf_paths(leaf, root + f'[{index}]'))
    return out


def static_name_is_immutable(lines, name, references=False, scalar_paths=(), copy_calls=()):
    """Reject writes/shadows anywhere, including inline else. For tables/Positions also
    reject alias escape, returned references and arguments to unproven functions.
    This is intentionally stricter than a Lua scope/effect analysis."""
    try:
        tokens = [t for t in lua_tables.tokenize('\n'.join(lines))
                  if t[0] not in ('comment', 'lcomment', 'string', 'lstring')]
    except lua_tables.LuaError:
        return False
    if copy_calls and not builtin_binding_is_pristine(lines, 'Game'):
        copy_calls = ()
    writes, calls, definitions = 0, [], []
    reads = {'getStorageValue', 'setStorageValue', 'getItemCount', 'teleportTo',
             'Position', 'sendMagicEffect', 'addItem', 'removeItem'}
    for i, (kind, value, number) in enumerate(tokens):
        if value == '(':
            caller = tokens[i - 1][1] if i and tokens[i - 1][0] == 'name' else None
            if i >= 3 and tokens[i - 2][1] == '.' and tokens[i - 3][1] == 'Game':
                qualified = 'Game.' + str(caller)
                if qualified in copy_calls:
                    caller = qualified
            calls.append(caller)
            k = i - 2
            while k >= 1 and tokens[k][1] in ('.', ':') and tokens[k - 1][0] == 'name':
                k -= 2
            definitions.append(caller == 'function' or (k >= 0 and tokens[k][1] == 'function'))
        elif value == ')' and calls:
            calls.pop()
            definitions.pop()
        if kind != 'name' or value != name or (i and tokens[i - 1][1] == '.'):
            continue
        if definitions and definitions[-1]:
            return False  # a function parameter shadows the supposedly static local
        before = [t[1] for t in tokens[:i] if t[2] == number]
        if 'for' in before and 'in' not in before and '=' not in before:
            return False  # generic-for variable shadows a scalar prefix local
        j = i + 1
        while j < len(tokens):
            if tokens[j][1] == '.' and j + 1 < len(tokens) and tokens[j + 1][0] == 'name':
                j += 2
            elif tokens[j][1] == '[':
                depth = 1
                j += 1
                while j < len(tokens) and depth:
                    depth += (tokens[j][1] == '[') - (tokens[j][1] == ']')
                    j += 1
            else:
                break
        following = tokens[j][1] if j < len(tokens) else None
        assignment = following == '=' and (j + 1 == len(tokens) or tokens[j + 1][1] != '=')
        if assignment:
            writes += 1
        if not references or assignment:
            continue
        path = ''.join(t[1] for t in tokens[i:j])
        if path in scalar_paths:
            continue  # primitive values cannot carry an alias to their parent table
        copied = bool(calls and calls[-1] in copy_calls)
        if not copied and i and tokens[i - 1][1] in ('=', 'return'):
            return False  # multiline alias/return escape
        # A reference used on an assignment RHS may alias a table or a subtable;
        # do not rely on the alias's later mutation being visible through the root.
        if not copied and any(v == '=' and (n == 0 or before[n - 1] not in ('=', '<', '>', '~'))
               and (n + 1 == len(before) or before[n + 1] != '=') for n, v in enumerate(before)):
            return False
        if 'return' in before or (calls and calls[-1] not in reads and not copied):
            return False
        if following == ':' and (j + 1 == len(tokens) or tokens[j + 1][1] != 'sendMagicEffect'):
            return False
        if following == '(':
            return False
    return writes == 1


def storage_aliases(lines):
    """Only immutable unconditional prefix locals; callback/branch/sibling aliases
    cannot leak into the file-wide substitution used by scan()."""
    prefix = unconditional_prefix(lines)
    aliases, declarations = {}, {}
    code = [strip_code(line) for line in lines]
    for number, line in enumerate(lines, 1):
        if number not in prefix:
            continue
        m = ALIAS.match(line)
        numeric = re.fullmatch(r'\s*local\s+(\w+)\s*=\s*(\d+)\s*(?:--.*)?', line)
        if m and static_name_is_immutable(lines, m.group(1), references=True):
            aliases[m.group(1)] = m.group(2)
        elif numeric and static_name_is_immutable(lines, numeric.group(1)):
            name, value = numeric.groups()
            reads = re.compile(rf'(?:get|set)StorageValue\(\s*{re.escape(name)}\s*[,)]')
            uses = [n for n, text in enumerate(code, 1) if reads.search(text)]
            if uses and min(uses) > number:
                aliases[name] = value
        table = re.match(r'\s*local\s+(\w+)\s*=\s*\{', line)
        if table:
            declarations[table.group(1)] = number
    text = '\n'.join(lines)
    paths = re.findall(r'(?:get|set)StorageValue\(\s*(\w+(?:\.\w+|\[\d+\])+)\s*[,)]', '\n'.join(code))
    for path in sorted(set(paths)):
        root = re.match(r'\w+', path).group()
        if root not in declarations:
            continue
        uses = [n for n, line in enumerate(code, 1) if path in line and re.search(r'(?:get|set)StorageValue\(', line)]
        if min(uses) <= declarations[root]:
            continue
        try:
            value = lua_tables.as_python(lua_tables.assignments(text, {root})[root])
            if not static_name_is_immutable(lines, root, references=True, scalar_paths=scalar_leaf_paths(value, root)):
                continue
            for field, index in re.findall(r'\.(\w+)|\[(\d+)\]', path[len(root):]):
                value = value[field] if field else value[int(index) - 1] if isinstance(value, list) else value[int(index)]
            if isinstance(value, dict) and set(value) == {'expr'} and re.fullmatch(r'(?:Global)?Storage\.[\w.\[\]]+', value['expr']):
                aliases[path] = value['expr']
            elif type(value) is int and value >= 0:
                aliases[path] = str(value)
        except (lua_tables.LuaError, KeyError, IndexError, TypeError, ValueError):
            continue
    return aliases


def expand_aliases(line, aliases):
    """A line with its storage aliases written out. An alias that shadows `Storage` itself is expanded only
    where it stands alone, so the full `Storage.…` paths of the same file stay as they are."""
    if ALIAS.match(line):
        return line
    declaration = re.match(r'\s*local\s+\w+\s*=', line)
    prefix, line = (line[:declaration.end()], line[declaration.end():]) if declaration else ('', line)
    for alias, path in aliases.items():
        follow = r'(?![\w.\[])' if alias in ('Storage', 'GlobalStorage') else r'(?=[.\[\s,)])'
        # Strings carry evidence/text, never identifier references. Do not expand their
        # contents even when an alias is followed by spaces or punctuation there.
        pieces = re.split(r'("(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')', line)
        for index in range(0, len(pieces), 2):
            pieces[index] = re.sub(rf'(?<![\w.]){re.escape(alias)}{follow}', lambda _: path, pieces[index])
        line = ''.join(pieces)
    return prefix + line


def guard_pattern(target):
    return re.compile(r'getStorageValue\(\s*' + re.escape(target) + r'\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)')


def pristine_procedure(text, name):
    """A bare legacy builtin call is source evidence only while its binding is untouched.
    Any declaration, assignment, parameter, alias or escape leaves that name unknown.
    Qualified table methods are separate symbols, never this builtin."""
    try:
        tokens = [t for t in lua_tables.tokenize(text) if t[0] not in ('comment', 'lcomment')]
    except lua_tables.LuaError:
        return False
    for i, (kind, value, _) in enumerate(tokens):
        if kind != 'name' or value != name or i and tokens[i - 1][1] in ('.', ':'):
            continue
        if (i and tokens[i - 1][1] == 'function'
                or i + 1 >= len(tokens) or tokens[i + 1][1] != '('):
            return False
    return True


def call_arguments(text, start):
    """Split a complete call with lexical offsets intact; commas in nested calls are not separators."""
    code, depth, begin, parts = mask_code(text), 0, start, []
    for i in range(start, len(code)):
        char = code[i]
        if char in '([{':
            depth += 1
        elif char in ')]}':
            if depth == 0:
                return parts + [text[begin:i].strip()] if char == ')' else None
            depth -= 1
        elif char == ',' and depth == 0:
            parts.append(text[begin:i].strip())
            begin = i + 1
    return None


def procedural_writes(text):
    """setPlayerStorageValue(receiver, storage, value): storage is the second argument.
    Keep the source receiver verbatim; it is not a resolved player identity. Anonymous
    storage expressions stay unknown instead of joining another numeric argument."""
    if not pristine_procedure(text, 'setPlayerStorageValue'):
        return []
    found = []
    for match in PROCEDURAL_WRITE.finditer(mask_code(text)):
        args = call_arguments(text, match.end())
        if (args and len(args) == 3 and args[0] and args[2] and STORAGE_TARGET.fullmatch(args[1])
                and (not args[1].startswith('Storage.')
                     or builtin_binding_is_pristine(text.splitlines(), 'Storage'))):
            found.append((match.start(), args[1], args[2], args[0]))
    return found


def scan(text, path):
    text = scoped_storage_aliases.expand_scoped_storage_aliases(text)
    aliases = storage_aliases(text.split('\n'))
    text = '\n'.join(expand_aliases(line, aliases) for line in text.split('\n'))
    lines = text.split('\n')
    objects, functions, registrations = {}, [], {}
    for number, line in enumerate(lines, 1):
        if (m := OBJECT.match(line)):
            objects[m.group(1)] = KINDS[m.group(2)]
        if (m := FUNCTION.match(line)):
            functions.append((number, m.group(1), m.group(2)))
        if (m := REGISTRATION.match(line)):
            registrations.setdefault(m.group(1), []).append(f'{m.group(2)}({m.group(3).strip()[:60]})')
    writes = []
    candidates = [(m.start(), m.group(1), argument(text, m.end()).strip(), None)
                  for m in WRITE.finditer(text)] + procedural_writes(text)
    for offset, target, value, receiver in sorted(candidates):
        line = text.count('\n', 0, offset) + 1
        enclosing = next(((n, obj, method) for n, obj, method in reversed(functions) if n <= line), None)
        start = enclosing[0] if enclosing else 1
        guard = None
        pattern = guard_pattern(target) if receiver is None else re.compile(
            r'(?<![\w.:])getPlayerStorageValue\(\s*' + re.escape(receiver)
            + r'\s*,\s*' + re.escape(target) + r'\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)')
        # Equal expression text does not prove the same player: calls may return
        # different actors and identifiers may be reassigned or shadowed. Only a
        # literal CID (or an already-proved immutable literal alias) is stable here.
        reader_known = receiver is None or (bool(re.fullmatch(r'\d+', receiver))
                                           and pristine_procedure(text, 'getPlayerStorageValue'))
        for number in range(line - 1, start - 1, -1):
            g = pattern.search(strip_code(lines[number - 1])) if reader_known else None
            if g:
                guard = guard_at_write(lines, number, line, g.group(1), int(g.group(2)))
                break
        step = re.fullmatch(r'(?:\w+:)?getStorageValue\(\s*' + re.escape(target) + r'\s*\)\s*\+\s*(\d+)', value)
        if receiver is not None:
            step = re.fullmatch(r'getPlayerStorageValue\(\s*' + re.escape(receiver)
                               + r'\s*,\s*' + re.escape(target) + r'\s*\)\s*\+\s*(\d+)', value) if reader_known else None
        if re.fullmatch(r'-?\d+', value):
            effect = {'to': int(value)}
        elif step:
            effect = {'increment': int(step.group(1))}
        else:
            timestamp = ('os.time' in value if receiver is None else
                         bool(re.search(r'\bos\.time\s*\(', mask_code(value)))
                         and builtin_binding_is_pristine(lines, 'os'))
            effect = {'computed': 'timestamp' if timestamp else 'expression'}
        obj = enclosing[1] if enclosing else None
        if '/npc/' in path:
            owner = 'npc'
        elif '/lib/' in path or path.startswith('lib/'):
            owner = 'library'
        else:
            owner = objects.get(obj, 'other')
        writes.append({'line': line, 'target': target, 'value': value[:60], **effect, 'from': guard, 'owner': owner,
                       'callback': enclosing[2] if enclosing else None,
                       'registrations': sorted(set(registrations.get(obj, [])))[:6],
                       **({'dialogue': dialogue_at_write(lines, start, line)} if owner == 'npc' else {}),
                       **({'call_form': 'legacy_procedure', 'receiver_expression': receiver}
                          if receiver is not None else {})})
    return writes
