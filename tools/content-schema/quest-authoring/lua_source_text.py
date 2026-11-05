"""Bounded source-text parsing and pinned-byte checks for quest authoring.

This module does not execute Lua and contains no quest identity or source pin.
Callers provide their exact donor blob manifest and output paths.
"""

from __future__ import annotations

import ast
import hashlib
import re
from dataclasses import dataclass
from pathlib import Path


def verify_sources(donor: Path, expected: dict[str, str]) -> None:
    """Verify the actual bytes, including line endings, against the pinned Git tree."""
    for relative, wanted in expected.items():
        raw = (donor / relative).read_bytes()
        actual = hashlib.sha1(
            b"blob " + str(len(raw)).encode() + b"\0" + raw
        ).hexdigest()
        if actual != wanted:
            raise ValueError(
                f"pinned donor blob mismatch: {relative}; expected {wanted}, got {actual}"
            )


def write_or_check(path: Path, encoded: str, check: bool) -> None:
    payload = encoded.encode("utf-8")
    if check:
        if not path.is_file() or path.read_bytes() != payload:
            raise SystemExit(f"generated content drift: {path}")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)


@dataclass(frozen=True)
class Token:
    value: str
    line: int
    string: bool = False


def lex(source: str) -> list[Token]:
    out: list[Token] = []
    pos = 0
    line = 1
    while pos < len(source):
        char = source[pos]
        if char.isspace():
            line += char == "\n"
            pos += 1
            continue
        if source.startswith("--", pos):
            end = source.find("\n", pos)
            if end < 0:
                break
            pos = end
            continue
        if char in "\"'":
            start, oldline = pos, line
            pos += 1
            while pos < len(source):
                if source[pos] == "\\":
                    pos += 2
                elif source[pos] == char:
                    pos += 1
                    break
                else:
                    line += source[pos] == "\n"
                    pos += 1
            else:
                raise ValueError("unterminated Lua string")
            raw = source[start:pos]
            # Pinned source uses ordinary quoted text; reject unsupported Lua-only escapes.
            if re.search(r"\\(?:z|x|[0-9])", raw):
                raise ValueError("unsupported Lua string escape")
            out.append(Token(ast.literal_eval(raw), oldline, True))
            continue
        match = re.match(r"[A-Za-z_][A-Za-z_0-9]*|[0-9]+|\.\.|.", source[pos:])
        if match is None:
            raise ValueError("token")
        out.append(Token(match.group(), line))
        pos += len(match.group())
    return out


def arguments(tokens: list[Token], opening: int) -> tuple[list[list[Token]], int]:
    assert tokens[opening].value == "("
    depth = 0
    result: list[list[Token]] = []
    start = opening + 1
    for index in range(opening + 1, len(tokens)):
        token = tokens[index]
        if not token.string and token.value in "({[":
            depth += 1
        elif not token.string and token.value in ")}]":
            if token.value == ")" and depth == 0:
                result.append(tokens[start:index])
                return result, index
            depth -= 1
        elif token.value == "," and not token.string and depth == 0:
            result.append(tokens[start:index])
            start = index + 1
    raise ValueError("unterminated call")


def text_expression(tokens: list[Token], constants: dict[str, int]) -> dict:
    if len(tokens) == 1 and tokens[0].string:
        return {"parts": [tokens[0].value]}
    if tokens and tokens[0].value == "{" and tokens[-1].value == "}":
        if any(not token.string and token.value != "," for token in tokens[1:-1]):
            raise ValueError("unsupported multipart speech")
        return {"parts": [token.value for token in tokens[1:-1] if token.string]}
    if tokens[:3] and [t.value for t in tokens[:3]] == ["string", ".", "format"]:
        args, end = arguments(tokens, 3)
        if end != len(tokens) - 1 or len(args[0]) != 1 or not args[0][0].string:
            raise ValueError("unsupported formatted text")
        return {
            "parts": [args[0][0].value],
            "format": "lua_string_format",
            "arguments": [" ".join(t.value for t in part) for part in args[1:]],
        }
    parts: list[str] = []
    expected_value = True
    for token in tokens:
        if expected_value:
            if token.string:
                parts.append(token.value)
            elif token.value in constants:
                parts.append(str(constants[token.value]))
            else:
                raise ValueError(f"unknown text constant {token.value}")
        elif token.value != "..":
            raise ValueError("unsupported text expression")
        expected_value = not expected_value
    if expected_value or not parts:
        raise ValueError("empty/incomplete text expression")
    return {"parts": ["".join(parts)]}
