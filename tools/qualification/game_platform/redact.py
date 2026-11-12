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
VALUE = r'''(?:\[REDACTED\]|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|[^\s,;&}\]]+)'''


def redact(contents, evidence=()):
    # Evidence is supplied by the caller, never inferred from untrusted log text.
    approved = {value.removeprefix("sha256:") for value in evidence if re.fullmatch(r"(?:sha256:)?(?:[a-fA-F0-9]{40}|[a-fA-F0-9]{64})", value)}
    contents = re.sub(r'''(?i)\bpostgres(?:ql)?://[^\s"'<>\\]+''', MASK, contents)
    contents = re.sub(r'''(?i)(\b(?:Bearer|OterynDevice)\s+)[^\s"'<>},;\\]+''', lambda m: m[1] + MASK, contents)
    contents = re.sub(r"\botd1\.[A-Za-z0-9_-]+", MASK, contents)
    contents = re.sub(r"(?im)(\b(?:Set-Cookie|Cookie)[ \t]*[:=][ \t]*)[^\r\n]+", lambda m: m[1] + MASK, contents)
    keys = r"access_token|refresh_token|id_token|code|state|password|secret|client_secret|code_verifier|device_credential|game_login_ticket|PGPASSWORD|password_hash|cookie|set-cookie"
    contents = re.sub(r'''(?i)(["']?\b(?:''' + keys + r''')\b["']?\s*[:=]\s*)''' + VALUE,
                      lambda m: m[1] + MASK, contents)

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
