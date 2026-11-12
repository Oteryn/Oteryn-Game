#!/usr/bin/env python3
"""Retain source-defined failure reasons and Rust summaries; suppress other text.

An allowlist avoids trying to recognize every possible opaque secret spelling.
Raw responses, headers, values, panic payloads and compiler excerpts are discarded.
The test drivers must still never print secrets in their private input logs.
"""
import os
from pathlib import Path
import re
import sys


def vocabulary():
    tests = Path(__file__).resolve().parents[3] / "crates/platform-client/tests"
    sources = list(tests.glob("*.rs")) + list((tests / "support").glob("*.rs"))
    words = {value for source in sources
             for value in re.findall(r'"([a-z][a-z0-9_]*)"', source.read_text())}
    # This fixed prefix is assembled with a source-defined label and SQLSTATE.
    words.add("postgres_assertion_query_failed")
    return words


def redact(contents, words):
    result = []
    for line in contents.splitlines():
        reason = re.fullmatch(r'Error: "?([a-z][a-z0-9_]*(?::[a-z][a-z0-9_]*)?(?::[A-Z0-9]{5})?)"?', line)
        parts = reason[1].split(":") if reason else []
        safe_reason = (parts and "_" in parts[0] and parts[0] in words
                       and (len(parts) < 2 or parts[1] in words))
        summary = re.fullmatch(
            r'(?:running [0-9]+ tests?|test [a-zA-Z0-9_:]+ \.\.\. (?:ok|FAILED)|'
            r'test result: (?:ok|FAILED)\. [0-9]+ passed; [0-9]+ failed; '
            r'[0-9]+ ignored; [0-9]+ measured; [0-9]+ filtered out; finished in [0-9.]+s)', line)
        if not line or safe_reason or summary:
            result.append(line)
        else:
            result.append("[REDACTED]")
    return "\n".join(result) + "\n"


def main():
    source, destination = map(Path, sys.argv[1:])
    sanitized = redact(source.read_text(errors="replace"), vocabulary())
    # Never copy raw bytes, even if parsing or writing fails. Private permissions
    # also apply if this helper is run independently of the lane's umask.
    descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "w") as output:
        output.write(sanitized)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError):
        print("test_log_redaction_failed", file=sys.stderr)
        sys.exit(1)
