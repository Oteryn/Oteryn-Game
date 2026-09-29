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

WRITE = re.compile(r'setStorageValue\(\s*(Storage\.[A-Za-z0-9_.]+(?:\[\d+\])?|\d+)\s*,')
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


def strip_code(line):
    line = re.sub(r'--.*$', '', line)
    return re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', '""', line)


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
    keywords = [k for c in conditions for k in re.findall(r'MsgContains\(\s*\w+\s*,\s*"([^"]+)"', c)]
    topics = sorted({int(t) for c in conditions for t in re.findall(r'[Tt]opic\w*(?:\[[^\]]*\]|\([^)]*\))?\s*==\s*(\d+)', c)})
    return {'keywords': keywords, 'topics': topics}


ALIAS = re.compile(r'\s*local\s+(\w+)\s*=\s*((?:Global)?Storage\.[\w.\[\]]+)\s*(--.*)?$')


def storage_aliases(lines):
    """`local ThreatenedDreams = Storage.Quest.U11_40.ThreatenedDreams` and the like, by alias name."""
    return {m.group(1): m.group(2) for line in lines if (m := ALIAS.match(line))}


def expand_aliases(line, aliases):
    """A line with its storage aliases written out. An alias that shadows `Storage` itself is expanded only
    where it stands alone, so the full `Storage.…` paths of the same file stay as they are."""
    if ALIAS.match(line):
        return line
    for alias, path in aliases.items():
        follow = r'(?![\w.\[])' if alias in ('Storage', 'GlobalStorage') else r'(?=[.\[\s,)])'
        line = re.sub(rf'(?<![\w.]){alias}{follow}', path, line)
    return line


def guard_pattern(target):
    return re.compile(r'getStorageValue\(\s*' + re.escape(target) + r'\s*\)\s*(==|~=|<=|>=|<|>)\s*(-?\d+)')


def scan(text, path):
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
    for match in WRITE.finditer(text):
        line = text.count('\n', 0, match.start()) + 1
        target, value = match.group(1), argument(text, match.end()).strip()
        enclosing = next(((n, obj, method) for n, obj, method in reversed(functions) if n <= line), None)
        start = enclosing[0] if enclosing else 1
        guard = None
        for number in range(line - 1, start - 1, -1):
            g = guard_pattern(target).search(lines[number - 1])
            if g:
                guard = guard_at_write(lines, number, line, g.group(1), int(g.group(2)))
                break
        step = re.fullmatch(r'(?:\w+:)?getStorageValue\(\s*' + re.escape(target) + r'\s*\)\s*\+\s*(\d+)', value)
        if re.fullmatch(r'-?\d+', value):
            effect = {'to': int(value)}
        elif step:
            effect = {'increment': int(step.group(1))}
        else:
            effect = {'computed': 'timestamp' if 'os.time' in value else 'expression'}
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
                       **({'dialogue': dialogue_at_write(lines, start, line)} if owner == 'npc' else {})})
    return writes
