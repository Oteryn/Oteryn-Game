"""Conservative SOURCE storage aliases at exact lexical method-argument sites.

No evaluation. Game storage calls stay unchanged: world state is not character
progress. Any alias mutation, escape or unsupported structure fails closed.
"""
import re

import lua_tables


def expand_scoped_storage_aliases(text):
    if len(text) > 1_000_000:
        return text
    try:
        lua_tables.tokenize(text)  # Reject lexical gaps before using offsets.
    except lua_tables.LuaError:
        return text
    tokens, line = [], 1
    for m in lua_tables.TOKEN.finditer(text):
        if not any(m.group(k) is not None for k in ('ws', 'nl', 'comment', 'lcomment')):
            tokens.append((m.group(), m.start(), m.end(), line))
        line += m.group().count('\n')
    if len(tokens) > 100_000 or any(t[0] in ('repeat', 'until', 'goto') for t in tokens):
        return text
    values = [t[0] for t in tokens]
    frames, bindings, edits, declaration_tokens = [{'kind': 'root', 'names': {}}], [], [], set()
    storage_mutated, storage_initializers = False, set()

    def declare(name, value, index):
        previous = find(name)
        if previous is not None:
            previous['valid'] = False  # Includes local K=K initializer escapes.
        binding = {'value': value, 'valid': value is not None, 'index': index}
        frames[-1]['names'][name] = binding
        bindings.append(binding)
        declaration_tokens.add(index)
        return binding

    def find(name):
        return next((f['names'][name] for f in reversed(frames) if name in f['names']), None)

    def path_end(index):
        end = index + 1
        while end + 1 < len(tokens) and values[end] == '.' and re.fullmatch(r'\w+', values[end + 1]):
            end += 2
        return end

    for i, (value, start, end, line) in enumerate(tokens):
        previous = values[i - 1] if i else None
        if value == 'local':
            j = i + 1
            if j < len(tokens) and values[j] == 'function':
                j += 1
                if j < len(tokens): declare(values[j], None, j)
            else:
                names = []
                while j < len(tokens) and re.fullmatch(r'[A-Za-z_]\w*', values[j]):
                    names.append(j); j += 1
                    if j >= len(tokens) or values[j] != ',': break
                    j += 1
                limit = j + 1
                while limit < len(tokens) and tokens[limit][3] == line and values[limit] != ';': limit += 1
                expression = ''.join(values[j + 1:limit]) if j < len(tokens) and values[j] == '=' else ''
                valid = bool(re.fullmatch(r'(?:Global)?Storage(?:\.[A-Za-z_]\w*)+|[1-9]\d*', expression))
                if expression.isdigit() and (len(expression) > 10 or int(expression) > 2_147_483_647): valid = False
                if valid and len(names) == 1 and not expression.isdigit():
                    storage_initializers.add(j + 1)
                for name in names: declare(values[name], expression if valid and len(names) == 1 else None, name)
        elif value == 'function':
            frames.append({'kind': 'function', 'names': {}})
            j = i + 1
            while j < len(tokens) and values[j] != '(': j += 1
            j += 1
            while j < len(tokens) and values[j] != ')':
                if re.fullmatch(r'[A-Za-z_]\w*', values[j]): declare(values[j], None, j)
                j += 1
            if j >= len(tokens): return text
        elif value in ('if', 'while', 'for', 'do'):
            if value == 'do' and frames[-1].get('header'):
                frames[-1]['header'] = False
            else:
                frames.append({'kind': value, 'names': {}, 'header': value in ('while', 'for')})
                if value == 'for':
                    j = i + 1
                    while j < len(tokens) and values[j] not in ('=', 'in'):
                        if re.fullmatch(r'[A-Za-z_]\w*', values[j]): declare(values[j], None, j)
                        j += 1
                    if j >= len(tokens): return text
        elif value in ('elseif', 'else'):
            if frames[-1]['kind'] != 'if': return text
            frames[-1] = {'kind': 'if', 'names': {}}
        elif value == 'end':
            if len(frames) == 1: return text
            frames.pop()
        if not re.fullmatch(r'[A-Za-z_]\w*', value) or previous in ('.', ':'):
            continue
        stop = path_end(i)
        following = values[stop] if stop < len(tokens) else None
        assignment = following == '=' and (stop + 1 == len(tokens) or values[stop + 1] != '=')
        if value in ('Storage', 'GlobalStorage'):
            direct_storage_call = (i >= 4 and previous == '(' and values[i - 2] in ('getStorageValue', 'setStorageValue') and following in (',', ')'))
            if assignment or i in declaration_tokens or (i not in storage_initializers and not direct_storage_call):
                storage_mutated = True
        binding = find(value)
        if binding is None or i in declaration_tokens:
            continue
        if not binding['valid']:
            continue
        method = values[i - 2] if i >= 2 and previous == '(' else None
        receiver_kind = values[i - 3] if i >= 3 else None
        receiver = values[i - 4] if i >= 4 else None
        call_argument = (method in ('getStorageValue', 'setStorageValue')
                         and receiver_kind in (':', '.') and following in (',', ')'))
        if assignment or not call_argument:
            binding['valid'] = False
            continue
        # Never rewrite dot-style module calls, especially Game global writes.
        if receiver_kind == '.' or receiver in ('Game', 'GlobalStorage') or not re.fullmatch(r'[A-Za-z_]\w*', receiver or ''):
            continue
        suffix = ''.join(values[i + 1:stop])
        if binding['value'].isdigit() and suffix:
            binding['valid'] = False
            continue
        edits.append((start, tokens[stop - 1][2], binding['value'] + suffix, binding))
    if len(frames) != 1:
        return text
    for start, end, replacement, binding in reversed(edits):
        if binding['valid'] and not (storage_mutated and not binding['value'].isdigit()):
            text = text[:start] + replacement + text[end:]
    return text
