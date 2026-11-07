"""Bound third-party text retained in imbuement evidence packets.

Packets keep digests of the complete captured bytes as provenance, not the
bodies. Every retained capture, quote, verbatim claim or code anchor is a
bounded excerpt: at most 450 characters per passage (passages are joined by
" … ") and at most 1,000 characters per source record or code anchor.
"""
from __future__ import annotations

import hashlib
import re

EXCERPT_SCOPE = "Bounded excerpt: quoted passages only, whitespace normalized"
CODE_EXCERPT_SCOPE = "Bounded excerpt: leading lines of the anchored source range, whitespace preserved"
EXCERPT_SEPARATOR = " … "
MAX_EXCERPT_CHARS = 1000
MAX_PASSAGE_CHARS = 450
# Quote-bearing fields are recognised by name, so plural, prefixed and anchor
# variants (quotes, selected_quote, verbatim_anchor, question_context, ...) are
# bounded too. Digest, scope, reference and flag companions do not match.
TEXT_FIELD = re.compile(r"(?:^|_)(?:quotes?|verbatim|excerpts?|text|snippets?|transcripts?|source_lines|contexts?)"
                        r"(?:_anchor)?$")
CODE_FIELDS = ("quote", "exact_source_lines")


def is_text_field(key: str, value) -> bool:
    """A quote-bearing field holds text or a list of texts; id-keyed records are not fields."""
    return not isinstance(value, dict) and TEXT_FIELD.search(key) is not None


def _strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from _strings(item)
    elif isinstance(value, dict):
        for item in value.values():
            yield from _strings(item)


def _retained(node) -> int:
    """Characters of third-party text retained anywhere below one record."""
    if isinstance(node, list):
        return sum(_retained(item) for item in node)
    if not isinstance(node, dict):
        return 0
    return sum(sum(map(len, _strings(value))) if is_text_field(key, value) else _retained(value)
               for key, value in node.items())


def _digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def validate(packet: dict, name: str = "packet") -> None:
    def require(condition, message):
        if not condition:
            raise ValueError(f"{name}: {message}")

    def visit(node):
        if isinstance(node, list):
            for item in node:
                visit(item)
            return
        if not isinstance(node, dict):
            return
        for key, value in node.items():
            if is_text_field(key, value):
                for text in _strings(value):
                    require(all(len(passage) <= MAX_PASSAGE_CHARS for passage in text.split(EXCERPT_SEPARATOR)),
                            "captured excerpt passage exceeds its bound")
        if "captured_text" in node:
            text = node["captured_text"]
            require(node.get("captured_text_scope") == EXCERPT_SCOPE, "captured text must be a bounded quoted excerpt")
            require(len(text) <= MAX_EXCERPT_CHARS, "captured excerpt exceeds its source bound")
            require(_digest(text) == node.get("captured_text_sha256"), "captured public text digest mismatch")
        if "source_ref" in node and any(field in node for field in CODE_FIELDS):
            require(node.get("excerpt_scope") == CODE_EXCERPT_SCOPE, "code anchor must be a bounded excerpt")
            require(_retained(node) <= MAX_EXCERPT_CHARS, "code anchor excerpt exceeds its bound")
            for field in CODE_FIELDS:
                if field in node:
                    require(_digest(node[field]) == node.get("excerpt_sha256"), "code anchor excerpt digest mismatch")
        for value in node.values():
            visit(value)

    visit(packet)
    sources = packet.get("sources") if isinstance(packet, dict) else None
    records = sources.values() if isinstance(sources, dict) else sources if isinstance(sources, list) else []
    for record in records:
        require(_retained(record) <= MAX_EXCERPT_CHARS, "captured excerpt exceeds its source bound")
