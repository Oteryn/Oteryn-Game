"""A small, strict Lua-literal reader for the GameStore catalog sources.

This is not a Lua interpreter. It tokenizes a `.lua` file and recognizes exactly the
statement/expression shapes the Canary catalog modules and Crystal `init.lua` /
`gamestore.lua` actually use:

  - `local NAME = <expr>`
  - `NAME(.NAME)* = <expr>`        (dotted assignment, e.g. `GameStore.OfferTypes = {...}`)
  - `return <expr>`
  - `if ... then ... end`, `for ... do ... end`, `function NAME(...) ... end`
    are recognized as blocks and *skipped* (never evaluated); this is a deliberate,
    documented omission for config-dependent branches (e.g. the VIP Shop naming
    override in `premium_time.lua` / Crystal's inline copy of it), not a silent drop:
    every skipped block is recorded in `Chunk.skipped` with its opening keyword,
    starting line and the identifier that follows it.
  - expressions: string/number/boolean/nil literals, `{ ... }` array or record
    tables (never both in the same table), dotted names (`GameStore.States.STATE_NEW`),
    and `string.format("...%s...", ARG)` (the only function call this data uses).

Anything else at the top level, or any other function call, is a hard parse error
(`LuaParseError`) — fail closed, never a silent guess.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

BLOCK_OPENERS = ("if", "for", "function", "while")
LITERAL_KEYWORDS = {"true": True, "false": False, "nil": None}
# `{}[](),;=.` are structurally meaningful (parsed); `<>~+*/%:` only ever occur inside
# a skipped `if`/`for`/`function`/`while` block, where they are scanned but never
# interpreted, so a single generic token per character is enough to keep tokenizing.
PUNCT = set("{}[](),;=.<>~+*/%:-#^&|")


class LuaParseError(Exception):
    pass


@dataclass
class Token:
    kind: str  # STRING, NUMBER, NAME, EOF, or a punctuation character
    value: Any
    line: int


class Unresolved:
    """A source expression this reader recognized but could not (and, for its
    field, need not) reduce to a literal value: a bare identifier that is neither a
    known local nor a known dotted constant, or a call other than `string.format`.
    Carries the raw dotted/expr text for provenance; never invented as a value."""

    def __init__(self, text: str):
        self.text = text

    def __repr__(self):
        return f"Unresolved({self.text!r})"

    def __eq__(self, other):
        return isinstance(other, Unresolved) and self.text == other.text


def tokenize(text: str) -> list[Token]:
    tokens: list[Token] = []
    i, n = 0, len(text)
    line = 1
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
            continue
        if c in " \t\r":
            i += 1
            continue
        if text[i : i + 4] == "--[[":
            end = text.find("]]", i + 4)
            if end == -1:
                raise LuaParseError(f"line {line}: unterminated block comment")
            line += text.count("\n", i, end)
            i = end + 2
            continue
        if c == "-" and i + 1 < n and text[i + 1] == "-":
            nl = text.find("\n", i)
            i = nl if nl != -1 else n
            continue
        if c in "\"'":
            quote = c
            j = i + 1
            buf: list[str] = []
            while j < n and text[j] != quote:
                if text[j] == "\\":
                    nxt = text[j + 1 : j + 2]
                    esc = {"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'"}
                    if nxt not in esc:
                        raise LuaParseError(
                            f"line {line}: unsupported escape \\{nxt!r}"
                        )
                    buf.append(esc[nxt])
                    j += 2
                else:
                    if text[j] == "\n":
                        line += 1
                    buf.append(text[j])
                    j += 1
            if j >= n:
                raise LuaParseError(f"line {line}: unterminated string literal")
            tokens.append(Token("STRING", "".join(buf), line))
            i = j + 1
            continue
        if c.isdigit():
            j = i
            if c == "0" and i + 1 < n and text[i + 1] in "xX":
                j = i + 2
                while j < n and text[j] in "0123456789abcdefABCDEF":
                    j += 1
                tokens.append(Token("NUMBER", str(int(text[i:j], 16)), line))
                i = j
                continue
            while j < n and (text[j].isdigit() or text[j] == "."):
                j += 1
            tokens.append(Token("NUMBER", text[i:j], line))
            i = j
            continue
        if c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            tokens.append(Token("NAME", text[i:j], line))
            i = j
            continue
        if c in PUNCT:
            tokens.append(Token(c, c, line))
            i += 1
            continue
        raise LuaParseError(f"line {line}: unexpected character {c!r}")
    tokens.append(Token("EOF", None, line))
    return tokens


@dataclass
class Chunk:
    assignments: dict[str, Any] = field(default_factory=dict)
    skipped: list[str] = field(default_factory=list)


class Parser:
    def __init__(self, tokens: list[Token]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Token:
        return self.tokens[self.pos]

    def advance(self) -> Token:
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def expect(self, kind: str) -> Token:
        tok = self.advance()
        if tok.kind != kind:
            raise LuaParseError(
                f"line {tok.line}: expected {kind!r}, found {tok.kind!r} ({tok.value!r})"
            )
        return tok

    def parse_chunk(self) -> Chunk:
        chunk = Chunk()
        while self.peek().kind != "EOF":
            self.parse_statement(chunk)
        return chunk

    def parse_dotted_name(self) -> str:
        parts = [self.expect("NAME").value]
        while self.peek().kind == "." and self.tokens[self.pos + 1].kind == "NAME":
            self.advance()
            parts.append(self.expect("NAME").value)
        return ".".join(parts)

    def parse_statement(self, chunk: Chunk) -> None:
        tok = self.peek()
        if tok.kind != "NAME":
            raise LuaParseError(
                f"line {tok.line}: unexpected top-level token {tok.kind!r}"
            )
        if tok.value == "local":
            self.advance()
            if self.peek().kind == "NAME" and self.peek().value == "function":
                fn_tok = self.advance()
                chunk.skipped.append(f"local function (line {fn_tok.line})")
                self._skip_to_matching_end(fn_tok.line, "local function")
                return
            name = self.expect("NAME").value
            self.expect("=")
            chunk.assignments[name] = self.parse_expr(chunk)
            return
        if tok.value == "return":
            self.advance()
            chunk.assignments["__return__"] = self.parse_expr(chunk)
            return
        if tok.value in BLOCK_OPENERS:
            self.skip_block(chunk, tok)
            return
        start_line = tok.line
        dotted = self.parse_dotted_name()
        if self.peek().kind == "(":
            # A bare call statement (e.g. `dofile(CORE_DIRECTORY .. "...")`): engine
            # bootstrap, never catalog data. Its argument list may use operators this
            # reader does not evaluate (string concatenation, and so on), so it is
            # skipped by bracket balance rather than parsed as an expression.
            chunk.skipped.append(f"{dotted}(...) call (line {start_line})")
            self._skip_parens()
            return
        self.expect("=")
        chunk.assignments[dotted] = self.parse_expr(chunk)

    def _skip_parens(self) -> None:
        self.expect("(")
        depth = 1
        while depth > 0:
            tok = self.advance()
            if tok.kind == "EOF":
                raise LuaParseError("unterminated parenthesized argument list")
            if tok.kind == "(":
                depth += 1
            elif tok.kind == ")":
                depth -= 1

    def skip_block(self, chunk: Chunk, opener: Token) -> None:
        following = self.tokens[self.pos + 1]
        chunk.skipped.append(
            f"{opener.value} (line {opener.line}, next token {following.value!r})"
        )
        self.advance()
        self._skip_to_matching_end(opener.line, opener.value)

    def _skip_to_matching_end(self, start_line: int, what: str) -> None:
        # `if`/`for`/`function`/`while` all close with a single `end`; nesting any of
        # them again increments the same depth counter (the standard trick — the
        # interstitial `then`/`do`/parameter-list tokens never affect balance).
        depth = 1
        while depth > 0:
            tok = self.advance()
            if tok.kind == "EOF":
                raise LuaParseError(f"line {start_line}: unterminated {what} block")
            if tok.kind == "NAME" and tok.value in BLOCK_OPENERS:
                depth += 1
            elif tok.kind == "NAME" and tok.value == "end":
                depth -= 1

    def parse_expr(self, chunk: Chunk):
        left = self.parse_primary(chunk)
        while self.peek().kind == "." and self.tokens[self.pos + 1].kind == ".":
            self.advance()
            self.advance()
            right = self.parse_primary(chunk)
            left = ("concat", left, right)
        return left

    def parse_primary(self, chunk: Chunk):
        tok = self.peek()
        if tok.kind == "STRING":
            self.advance()
            return ("str", tok.value)
        if tok.kind == "NUMBER":
            self.advance()
            return ("num", float(tok.value) if "." in tok.value else int(tok.value))
        if tok.kind == "{":
            return self.parse_table(chunk)
        if tok.kind == "NAME" and tok.value in LITERAL_KEYWORDS:
            self.advance()
            return ("lit", LITERAL_KEYWORDS[tok.value])
        if tok.kind == "NAME" and tok.value == "function":
            # An anonymous function value (e.g. `GameStore.isItsPacket = function(byte) ... end`):
            # engine behavior, never catalog data. Its body is skipped like a statement-level
            # `function` block; recorded in `chunk.skipped`, never evaluated.
            self.advance()
            chunk.skipped.append(f"function value (line {tok.line})")
            self._skip_to_matching_end(tok.line, "function value")
            return ("unresolved", "function(...)")
        if tok.kind == "NAME":
            dotted = self.parse_dotted_name()
            if self.peek().kind == "[":
                # Indexed lookup (e.g. `HIRELING_SKILLS.COOKING[1]`): the table it
                # indexes is defined elsewhere in the engine, out of this reader's
                # scope. Recorded as unresolved, never guessed.
                self.advance()
                self.parse_expr(chunk)
                self.expect("]")
                return ("unresolved", f"{dotted}[...]")
            if self.peek().kind == "(":
                self.advance()
                args = []
                if self.peek().kind != ")":
                    args.append(self.parse_expr(chunk))
                    while self.peek().kind == ",":
                        self.advance()
                        args.append(self.parse_expr(chunk))
                self.expect(")")
                return ("call", dotted, args)
            return ("name", dotted)
        raise LuaParseError(
            f"line {tok.line}: unexpected token in expression: {tok.kind!r}"
        )

    def parse_table(self, chunk: Chunk):
        self.expect("{")
        record: dict[str, Any] = {}
        array: list[Any] = []
        while self.peek().kind != "}":
            if self.peek().kind == "[":
                self.advance()
                key_expr = self.parse_expr(chunk)
                self.expect("]")
                self.expect("=")
                value = self.parse_expr(chunk)
                if key_expr[0] != "str" and key_expr[0] != "num":
                    raise LuaParseError("computed table key must be a literal")
                record[str(key_expr[1])] = value
            elif self.peek().kind == "NAME" and self.tokens[self.pos + 1].kind == "=":
                name = self.advance().value
                self.advance()
                record[name] = self.parse_expr(chunk)
            else:
                array.append(self.parse_expr(chunk))
            if self.peek().kind in (",", ";"):
                self.advance()
            elif self.peek().kind == "}":
                break
            else:
                tok = self.peek()
                raise LuaParseError(
                    f"line {tok.line}: expected ',' or '}}' in table, found {tok.kind!r}"
                )
        self.expect("}")
        if record and array:
            raise LuaParseError("mixed array/record table literal is not supported")
        return (
            ("table_record", record) if record or not array else ("table_array", array)
        )


def parse_chunk(text: str) -> Chunk:
    return Parser(tokenize(text)).parse_chunk()


def resolve(expr, locals_: dict, constants: dict):
    kind = expr[0]
    if kind in ("str", "num", "lit"):
        return expr[1]
    if kind == "table_array":
        return [resolve(e, locals_, constants) for e in expr[1]]
    if kind == "table_record":
        return {k: resolve(v, locals_, constants) for k, v in expr[1].items()}
    if kind == "unresolved":
        return Unresolved(expr[1])
    if kind == "concat":
        left = resolve(expr[1], locals_, constants)
        right = resolve(expr[2], locals_, constants)
        if isinstance(left, (str, int, float)) and isinstance(right, (str, int, float)):
            return f"{left}{right}"
        return Unresolved(f"{left!r} .. {right!r}")
    if kind == "name":
        dotted = expr[1]
        if dotted in locals_:
            return locals_[dotted]
        if dotted in constants:
            return constants[dotted]
        return Unresolved(dotted)
    if kind == "call":
        fn, args = expr[1], expr[2]
        resolved_args = [resolve(a, locals_, constants) for a in args]
        if (
            fn == "string.format"
            and resolved_args
            and isinstance(resolved_args[0], str)
        ):
            fmt = resolved_args[0]
            rest = resolved_args[1:]
            if fmt.count("%s") == len(rest) and all(
                isinstance(a, (str, int, float)) for a in rest
            ):
                out = fmt
                for a in rest:
                    out = out.replace("%s", str(a), 1)
                return out
        return Unresolved(f"{fn}(...)")
    raise LuaParseError(f"unresolvable expression kind {kind!r}")


def flatten_constants(
    assignments: dict[str, Any], locals_: dict[str, Any]
) -> dict[str, Any]:
    """Flattens `NAME.GROUP = {A = 0, B = 1}`-shaped assignments (as produced by
    `constants.lua` / `init.lua`) into a single dotted-name -> literal map, e.g.
    `GameStore.OfferTypes.OFFER_TYPE_HOUSE` -> 9."""
    flat: dict[str, Any] = {}
    for dotted, expr in assignments.items():
        if dotted == "__return__":
            continue
        value = resolve(expr, locals_, flat)
        if isinstance(value, dict):
            for k, v in value.items():
                if isinstance(v, (int, float, str, bool)) or v is None:
                    flat[f"{dotted}.{k}"] = v
        elif isinstance(value, (int, float, str, bool)) or value is None:
            flat[dotted] = value
    return flat
