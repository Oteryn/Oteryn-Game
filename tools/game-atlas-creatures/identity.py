#!/usr/bin/env python3
"""Stable creature entity identity shared by Game-owned Atlas projections."""
from __future__ import annotations

import hashlib


def stable_creature_entity_id(kind: str, normalized_name: str) -> str:
    """Return the existing public creature entity ID without changing its hash seam."""
    if kind not in {"npc", "monster"}:
        raise ValueError(f"unsupported creature kind: {kind}")
    entity_kind = f"{kind}-entity"
    payload = "\0".join((entity_kind, normalized_name)).encode("utf-8")
    return f"{entity_kind}:{hashlib.sha256(payload).hexdigest()[:32]}"