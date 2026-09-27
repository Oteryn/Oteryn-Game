"""Split a Lua callback body into if/elseif/else branches and plain statements, line by line.

Enough for OTS quest scripts, where one statement or block keyword starts each line: `if … then`, `elseif … then`,
`else`, `end`, loops and local functions. A loop or nested function is kept as one opaque statement block. A
`then` branch that ends in `return` makes the statements after its if-block the implicit `else`, as the early-exit
guards of the seal scripts do. Nothing is evaluated.
"""
import re

from lua_writers import CLOSER, OPENER, strip_code


def function_body(lines, start):
    """Line numbers (1-based) of the body of the function that starts at `start`, without its final `end`."""
    depth, body = 0, []
    for number in range(start, len(lines) + 1):
        code = strip_code(lines[number - 1])
        depth += len(OPENER.findall(code)) - len(CLOSER.findall(code))
        if number > start:
            if depth <= 0:
                return body
            body.append(number)
    return body


def parse(lines, numbers):
    """A list of nodes: ('stmt', line) | ('block', [lines]) | ('deferred', [lines]) |
    ('if', [(condition, line, [nodes])], else_nodes)."""
    nodes, i = [], 0
    while i < len(numbers):
        number = numbers[i]
        code = strip_code(lines[number - 1]).strip()
        one_line = re.fullmatch(r'if\s+(.*?)\s+then\s+(.*?)\s*end', code)
        if one_line:
            body = [('return',)] if re.match(r'return\b', one_line.group(2)) else [('stmt', number)]
            if body == [('return',)]:
                nodes.append(('if', [(one_line.group(1), number, body)], parse(lines, numbers[i + 1:])))
                return nodes
            nodes.append(('if', [(one_line.group(1), number, body)], []))
            i += 1
            continue
        if re.match(r'if\b', code) and not re.search(r'\bend\b', code):
            branches, else_nodes, depth, current, cond_line = [], None, 1, [], number
            condition = re.sub(r'^if\s+|\s+then$', '', code)
            i += 1
            while i < len(numbers) and depth:
                inner = strip_code(lines[numbers[i] - 1]).strip()
                if depth == 1 and re.match(r'elseif\b', inner):
                    branches.append((condition, cond_line, parse(lines, current)))
                    condition, cond_line, current = re.sub(r'^elseif\s+|\s+then$', '', inner), numbers[i], []
                elif depth == 1 and re.fullmatch(r'else', inner):
                    branches.append((condition, cond_line, parse(lines, current)))
                    condition, current = None, []
                else:
                    depth += len(OPENER.findall(inner)) - len(CLOSER.findall(inner))
                    if depth:
                        current.append(numbers[i])
                i += 1
            if condition is None:
                else_nodes = parse(lines, current)
            else:
                branches.append((condition, cond_line, parse(lines, current)))
            rest = numbers[i:]
            last = branches[-1][2] if branches else []
            if else_nodes is None and len(branches) == 1 and last and last[-1] == ('return',):
                # early exit: what follows the if-block runs only when the condition fails
                nodes.append(('if', branches, parse(lines, rest)))
                return nodes
            nodes.append(('if', branches, else_nodes or []))
            continue
        if re.match(r'(for|while)\b|local\s+function\b|function\b|repeat\b', code):
            depth, block = 0, []
            while i < len(numbers):
                inner = strip_code(lines[numbers[i] - 1])
                depth += len(OPENER.findall(inner)) - len(CLOSER.findall(inner))
                block.append(numbers[i])
                i += 1
                if depth <= 0:
                    break
            nodes.append(('block', block))
            continue
        if len(OPENER.findall(code)) > len(CLOSER.findall(code)):
            # a statement that opens a function literal, e.g. `addEvent(function() ... end, 1000)`: its body
            # runs later, never inline, so it stays one deferred block
            depth, block = 0, []
            while i < len(numbers):
                inner = strip_code(lines[numbers[i] - 1])
                depth += len(OPENER.findall(inner)) - len(CLOSER.findall(inner))
                block.append(numbers[i])
                i += 1
                if depth <= 0:
                    break
            nodes.append(('deferred', block))
            continue
        if re.fullmatch(r'return\b.*', code):
            nodes.append(('return',))
        elif code:
            nodes.append(('stmt', number))
        i += 1
    return nodes
