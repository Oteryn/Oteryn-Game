"""Pinned sources that a reviewed structural change moved or rewrote after pinning.

A qualification record pins each source by repository path and sha256 as it was
reviewed. Generated content embeds those records, so they stay byte-identical when a
later structural change moves or rewrites a pinned source. This ledger names each such
transition once: the pinned path and digest, and the exact current path and digest.

A pin resolves through an entry only when it matches the recorded pinned digest and
the current file matches the recorded current digest exactly. Any further edit to a
relocated source therefore fails closed until that edit is reviewed and recorded here.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

# (pinned path, pinned sha256) -> (current path, current sha256)
RELOCATIONS = {
    # ADR-0019 stage 1 carve: the Item identity module moved into the gameplay content
    # crate. Only its two include_bytes! prefixes gained one `../`; the explicit
    # APPEARANCE_ONLY_ITEM_IDS cohort the pins guard is unchanged.
    (
        "apps/game-server/src/content/item_identity.rs",
        "0df52c391a1d56e5d82be269ddcd480233f923007cfdf35b9a79226bf2868dde",
    ): (
        "crates/gameplay/content-promotion/src/content/item_identity.rs",
        "40c254806d0a55b5c2e938801beb9f0d17430a6825662ac31c89ca83747f3bea",
    ),
    # The same carve: the A12 helper reads the native admission through this ledger.
    (
        "tools/content-migration/item_official_navigation.py",
        "e39d051c146e2ef0e9eca03f624438d87af303abe383f8f9c08879e0083c9dd1",
    ): (
        "tools/content-migration/item_official_navigation.py",
        "ab3cb5adc65b9afabeb15cef77f26a68febef3a31da40c57a98d734ada48aae3",
    ),
}
# pinned path -> current path, for sources a structural change moved byte-identically,
# so the recorded pin digest still names the current bytes.
VERBATIM_MOVES = {
    # CONTENT-PROMO-1 carve: the Item name promotion moved into the content-promotion
    # crate unchanged.
    "crates/gameplay/content/src/content/item_name_promotion.rs": (
        "crates/gameplay/content-promotion/src/content/item_name_promotion.rs"
    ),
}
MOVED = {
    pinned: current
    for (pinned, _), (current, _) in RELOCATIONS.items()
    if pinned != current
} | VERBATIM_MOVES


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def current_path(path: str) -> str:
    """The current repository path of a pinned source path."""
    return MOVED.get(path, path)


def read_text(root: Path, path: str) -> str:
    """Read a source a record names by path only, at its current location."""
    return (Path(root) / current_path(path)).read_text()


def read_pinned(root: Path, path: str, expected: str) -> bytes | None:
    """The current bytes of a pinned source, or None when its pin does not hold."""
    relocated = RELOCATIONS.get((path, expected))
    current, current_digest = relocated if relocated else (current_path(path), expected)
    target = Path(root) / current
    if not target.is_file():
        return None
    data = target.read_bytes()
    return data if digest(data) == current_digest else None
