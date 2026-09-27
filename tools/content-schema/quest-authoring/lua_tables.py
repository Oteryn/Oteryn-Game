"""Read Lua table constructors from OTS data files without a Lua interpreter.

Covers the literal subset used by startup tables: numbers, strings (short and long), booleans,
nil, dotted names (kept as {'expr': text}) and nested tables. Comments are kept per entry so a
source label such as `-- Katana Quest` stays attached to the entry that follows it.
"""
import re

TOKEN = re.compile(r'''
    (?P<ws>[ \t\r]+) | (?P<nl>\n)
  | (?P<lcomment>--\[(?P<lc_eq>=*)\[.*?\](?P=lc_eq)\])
  | (?P<comment>--[^\n]*)
  | (?P<lstring>\[(?P<ls_eq>=*)\[.*?\](?P=ls_eq)\])
  | (?P<string>"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*')
  | (?P<number>0[xX][0-9a-fA-F]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)
  | (?P<name>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<symbol>[{}\[\]=,;.():#+\-*/<>~^%])
''', re.S | re.X)


class LuaError(ValueError):
    pass


def tokenize(text):
    tokens, line, pos = [], 1, 0
    while pos < len(text):
        m = TOKEN.match(text, pos)
        if not m:
            raise LuaError(f'line {line}: cannot read {text[pos:pos + 20]!r}')
        kind = next(k for k in ('ws', 'nl', 'lcomment', 'comment', 'lstring', 'string', 'number', 'name', 'symbol')
                    if m.group(k) is not None)
        value = m.group(kind)
        if kind not in ('ws', 'nl'):
            tokens.append((kind, value, line))
        line += value.count('\n')
        pos = m.end()
    return tokens


def _string(token):
    kind, value, _ = token
    if kind == 'lstring':
        body = value[value.index('[', 1) + 1:value.rindex(']', 0, -1)]
        return body[1:] if body.startswith('\n') else body
    body = value[1:-1]
    body = re.sub(r'\\z\s*', '', body)
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), body)


class Parser:
    def __init__(self, text):
        self.tokens = tokenize(text)
        self.i = 0

    def peek(self, offset=0):
        j = self.i
        seen = 0
        while j < len(self.tokens):
            if self.tokens[j][0] not in ('comment', 'lcomment'):
                if seen == offset:
                    return self.tokens[j]
                seen += 1
            j += 1
        return ('eof', '', self.tokens[-1][2] if self.tokens else 0)

    def take_comments(self):
        comments = []
        while self.i < len(self.tokens) and self.tokens[self.i][0] in ('comment', 'lcomment'):
            comments.append(self.tokens[self.i])
            self.i += 1
        return comments

    def next(self):
        self.take_comments()
        token = self.tokens[self.i]
        self.i += 1
        return token

    def expect(self, value):
        token = self.next()
        if token[1] != value:
            raise LuaError(f'line {token[2]}: expected {value!r}, got {token[1]!r}')
        return token

    def value(self):
        kind, text, line = self.peek()
        if text == '{':
            return self.table()
        if text == '-' and self.peek(1)[0] == 'number':
            self.next()
            return -self._number(self.next()[1])
        self.next()
        if kind == 'number':
            return self._number(text)
        if kind in ('string', 'lstring'):
            return _string((kind, text, line))
        if kind == 'name':
            if text in ('true', 'false'):
                return text == 'true'
            if text == 'nil':
                return None
            parts = [text]
            while self.peek()[1] == '.' and self.peek(1)[0] == 'name':
                self.next()
                parts.append(self.next()[1])
            return {'expr': '.'.join(parts), 'line': line}
        raise LuaError(f'line {line}: unsupported value {text!r}')

    @staticmethod
    def _number(text):
        return int(text, 16) if text.lower().startswith('0x') else (float(text) if '.' in text or 'e' in text.lower() else int(text))

    def table(self):
        """A table as {'line', 'head', 'fields'}; `head` holds comments on the line of the opening brace.

        Each field is {'key', 'value', 'line', 'comments', 'trailing'}: `comments` precede the field,
        `trailing` follow it on its last line. List items get their 1-based position as key.
        """
        start = self.expect('{')[2]
        head = [c[1] for c in self.take_comments_on_line(start)]
        fields, position = [], 1
        while True:
            leading = [c[1] for c in self.take_comments()]
            if self.peek()[1] == '}':
                self.next()
                return {'line': start, 'head': head, 'fields': fields}
            kind, text, line = self.peek()
            if text == '[':
                self.next()
                key = self.value()
                self.expect(']')
                self.expect('=')
            elif kind == 'name' and self.peek(1)[1] == '=':
                self.next()
                self.next()
                key = text
            else:
                key = position
                position += 1
            value = self.value()
            last = self.tokens[self.i - 1][2]
            if self.peek()[1] in (',', ';'):
                last = self.next()[2]
            trailing = [c[1] for c in self.take_comments_on_line(last)]
            fields.append({'key': key, 'value': value, 'line': line, 'comments': leading, 'trailing': trailing})

    def take_comments_on_line(self, line):
        taken = []
        while self.i < len(self.tokens) and self.tokens[self.i][0] == 'comment' and self.tokens[self.i][2] == line:
            taken.append(self.tokens[self.i])
            self.i += 1
        return taken


def assignments(text, names):
    """Top-level `Name = { ... }` constructors for the requested names."""
    parser = Parser(text)
    found = {}
    while parser.peek()[0] != 'eof':
        kind, text_, _ = parser.next()
        if kind == 'name' and text_ in names and parser.peek()[1] == '=' and parser.peek(1)[1] == '{':
            parser.next()
            found[text_] = parser.table()
    return found


def as_python(value):
    """Drop line/comment bookkeeping: tables become dicts or lists."""
    if isinstance(value, dict) and 'fields' in value:
        fields = value['fields']
        if fields and all(isinstance(f['key'], int) and f['key'] == i + 1 for i, f in enumerate(fields)):
            return [as_python(f['value']) for f in fields]
        return {f['key']: as_python(f['value']) for f in fields}
    if isinstance(value, dict) and 'expr' in value:
        return {'expr': value['expr']}
    return value
