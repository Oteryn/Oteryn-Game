#!/usr/bin/env python3
"""Preserve diagnostics and mask only secret-bearing substrings, on every line."""
import argparse
from collections import Counter
import math
import os
from pathlib import Path
import re
import sys

MASK = "[REDACTED]"
KEYS = r"access_token|refresh_token|id_token|code|state|password|secret|client_secret|code_verifier|device_credential|PGPASSWORD|password_hash|cookie|set-cookie|(?:[a-z0-9]+[_-])*(?:game_?login_?|native_?|admission_?|login_?)?tickets?(?:_?(?:token|secret|value))?"


def value_end(text, start):
    """Find the complete value, including nested arrays and escaped JSON strings."""
    def quoted_end(position):
        opening = position
        while position < len(text) and text[position] == "\\":
            position += 1
        if position >= len(text) or text[position] not in "\"'":
            return None
        quote = text[position]
        layer = position - opening
        position += 1
        while position < len(text):
            if text[position] == quote:
                backslashes = 0
                previous = position - 1
                while previous >= 0 and text[previous] == "\\":
                    backslashes += 1
                    previous -= 1
                if backslashes % (2 * (layer + 1)) == layer:
                    return position + 1
            position += 1
        return len(text)  # Incomplete secret values must fail closed.

    quoted = quoted_end(start)
    if quoted is not None:
        return quoted
    if start < len(text) and text[start] in "[{":
        stack = []
        position = start
        while position < len(text):
            if text[position] in "\"'\\":
                quoted = quoted_end(position)
                if quoted is not None:
                    position = quoted
                    continue
            char = text[position]
            if char in "[{":
                stack.append(char)
            elif char in "]}":
                if not stack or (stack.pop(), char) not in [("[", "]"), ("{", "}")]:
                    return len(text)
                if not stack:
                    return position + 1
            position += 1
        return len(text)
    # Quotes embedded in an unquoted value are data, not terminators.
    # If its structure is ambiguous, consume the whole token up to a delimiter.
    match = re.match(r'''[^\s,;&)}\]]+''', text[start:])
    return start + len(match[0]) if match else start


def redact_fields(text):
    key = re.compile(r'''(?i)(?:\\*["'])?\b(?:''' + KEYS + r''')\b(?:\\*["'])?\s*[:=]\s*''')
    result = []
    position = 0
    while match := key.search(text, position):
        result.append(text[position:match.end()])
        end = value_end(text, match.end())
        result.append(MASK)
        position = max(end, match.end())
    result.append(text[position:])
    return "".join(result)


def redact(contents, evidence=()):
    # Evidence is supplied by the caller, never inferred from untrusted log text.
    approved = {value.removeprefix("sha256:") for value in evidence if re.fullmatch(r"(?:sha256:)?(?:[a-fA-F0-9]{40}|[a-fA-F0-9]{64})", value)}

    def userinfo(match):
        # Authority boundaries, not quote heuristics, determine URL userinfo.
        # Use the final @ so unusual/ambiguous credentials fail closed.
        authority = match[2]
        if "@" not in authority:
            return match[0]
        return match[1] + MASK + "@" + authority.rsplit("@", 1)[1]

    contents = re.sub(r"(?i)\b([a-z][a-z0-9+.-]*://)([^\s/?#]*)", userinfo, contents)
    contents = re.sub(r'''(?i)\bpostgres(?:ql)?://[^\s"'<>\\]+''', MASK, contents)
    contents = re.sub(r'''(?i)(\b(?:Bearer|OterynDevice)\s+)[^\s"'<>},;\\]+''', lambda m: m[1] + MASK, contents)
    contents = re.sub(r"\botd1\.[A-Za-z0-9_-]+", MASK, contents)
    contents = re.sub(r"(?im)(\b(?:Set-Cookie|Cookie)[ \t]*[:=][ \t]*)[^\r\n]+", lambda m: m[1] + MASK, contents)
    contents = redact_fields(contents)

    contents = re.sub(r"(?<![a-fA-F0-9])[a-fA-F0-9]{32,}(?![a-fA-F0-9])",
                      lambda m: m[0] if m[0] in approved else MASK, contents)

    def opaque(match):
        value = match[0]
        if value in approved:
            return value
        if re.fullmatch(r"[a-fA-F0-9]{32,}", value):
            return MASK
        entropy = -sum((n / len(value)) * math.log2(n / len(value)) for n in Counter(value).values())
        # Rust snake_case identifiers are not opaque tokens. Random base64url
        # runs have a larger alphabet/distribution; hex is always handled above.
        return MASK if entropy >= 4.3 else value

    return re.sub(r"(?<![A-Za-z0-9_-])[A-Za-z0-9_-]{32,}(?![A-Za-z0-9_-])", opaque, contents)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--evidence", action="append", default=[])
    args = parser.parse_args()
    sanitized = redact(args.source.read_text(errors="replace"), args.evidence)
    descriptor = os.open(args.destination, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "w") as output:
        output.write(sanitized)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError):
        print("test_log_redaction_failed", file=sys.stderr)
        sys.exit(1)
