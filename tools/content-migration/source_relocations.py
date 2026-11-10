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
    # QMINT3's tree-first reference view changes only the Forge289 compiler and
    # mapper fingerprints. All454 binding rows and289 literal promotions are
    # byte-equivalent; the consumer still rebuilds and checks their whole scope.
    (
        "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json",
        "a85c850e31342f8466c4f7a7d5165425a51468bb56aaea01fd0b995592185f57",
    ): (
        "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json",
        "7614b2bcbb2c0fa11271570cc62028d01f395976f746db53a1cd273a9eb6ac48",
    ),
    # RewardClaim validator gains the bounded multi-item form; Item qualifiers stay unchanged.
    ('crates/gameplay/content/src/content/reference_playable.rs', 'f090002566f663b57fae0d37ab0a35e4db1117d8871d8aa021488ad8b90261c2'): ('crates/gameplay/content/src/content/reference_playable.rs', 'e559f5bdff432d83198bfa4e4d447ec993abcad9ad928a4d86403b8793162593'),
    # QMINT3: exact merged geometry footprints and compatibility-only producer
    # changes. Frozen record scopes and current ownership checks remain enforced.
    ('content/world/objects/index.json', '1e2f278f385c502dcd59e6859fab3bb096f71d6c3f327b1b2e56e85f85e1b431'): ('content/world/objects/index.json', '1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30'),
    ('content/world/objects/index.json', 'a0dbd7f2c0c0b09b5457b189128c7cbd7dd2a4553e2a3c00063092372c3b65d7'): ('content/world/objects/index.json', '1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30'),
    ('content/world/objects/index.json', 'f23cb0a1a89c20e9957834fe63e1e8a01531cbba6ecb76a1e0ffde5c90a6b7cc'): ('content/world/objects/index.json', '1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30'),
    ('content/world/terrain/index.json', '0549b3567749436b9e6f91f79751d32d9ea4431fe6b1be7505566d50b797e27d'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/index.json', '22cb600cca441ac68526501c8e3576ec9c1dd07f77b276db470c70b996028fec'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/index.json', '360e65beb939450179cb6c99b5c19a0e2842051fd9c3beaf21e6366b9c9245a4'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/index.json', '89ae879da01b6a81e1e6a1c6a06b2632a2678066e0eb71404a5a7882bd516c44'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/index.json', 'abffe2dd83dfec1ce1afd657bd0fcaae3760a10a509f8e1034e35834a714ba79'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/index.json', 'baf9f79b334c9478895ae824c67aad32e06eb4e7b09435570389dd64ad455836'): ('content/world/terrain/index.json', '704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768'),
    ('content/world/terrain/terrain-05500-05999.json', 'c5081d0589d6e65a40baf89178c4e29f81089147d463d168474ac286d546a66a'): ('content/world/terrain/terrain-05500-05999.json', '7ba587fc470273980e0a75f0d597a13543b827d39e7f948ec469d1473a40d6c9'),
    ('content/world/terrain/terrain-08614-08668.json', 'dfd54e670a7de3eb8b5a444df9c8efc15faf8ddafadab761347338d80141f820'): ('content/world/terrain/terrain-08614-08668.json', '0c35e7cc19b8d799b9f33cccb0f989986f13e19006f379a28b110df76f50efe5'),
    ('tools/content-schema/item-authoring/historical_world_owner_context.py', '00878d28eb56c84f57734956505f60eb1fe7d248b7d1002c19209172a5125cfd'): ('tools/content-schema/item-authoring/historical_world_owner_context.py', 'db9150a3639c4d6bcd596c040b7d2ad39047fbc3f51c925761bebfbb6def26fd'),
    ('tools/content-schema/item-authoring/historical_world_owner_context.py', '8e41e838db44b527e4a243d4be1dc5245fe74def7145d9a68cdfc1436d609d5c'): ('tools/content-schema/item-authoring/historical_world_owner_context.py', 'db9150a3639c4d6bcd596c040b7d2ad39047fbc3f51c925761bebfbb6def26fd'),
    ('tools/content-schema/item-authoring/historical_world_owner_context.py', 'ae1a4f2c09345efbda4293eee96a93f9054bb33e40f658ab483b8b0c88027477'): ('tools/content-schema/item-authoring/historical_world_owner_context.py', 'db9150a3639c4d6bcd596c040b7d2ad39047fbc3f51c925761bebfbb6def26fd'),
    ('tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py', '5969b009ed326991e6db310b3e3c48640ae5a5ac361c9c34f5619740bff8ce2e'): ('tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py', '3f6a78aece8db60a19f0114c4ed9998c28c4330cc598319936e3eb0446740182'),
    # M57 adds only owner-approved Cake bridge Item48227. The identity generator
    # reproduces every predecessor binding byte after removing that exact row.
    (
        "imports/crystalserver/bindings/items.json",
        "1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a",
    ): (
        "imports/crystalserver/bindings/items.json",
        "6e2952b3017d8f5e5ab952216387075238bd37e7f1aba5bdc501cdcf43b90b66",
    ),
    # QMINT3 offline re-key: 10 newly admitted Item IDs gained canonical keys.
    # All 13,826 observations are byte-equivalent by item_id; no source facts changed.
    (
        "imports/tibiawiki/facts/items-stats.json",
        "24e3e57fddc6c14f00aa5c4976e91117d31bc2102bd3835b3d7891d9e41fbeae",
    ): (
        "imports/tibiawiki/facts/items-stats.json",
        "4dd7557ba437421396835dafe7dde3eabfa64ae4d3d797fa3f0f05155f797705",
    ),
    # ADR-0019 stage 1 carve: the Item identity module moved into the gameplay content
    # crate. Only its two include_bytes! prefixes gained one `../`; the explicit
    # APPEARANCE_ONLY_ITEM_IDS cohort the pins guard is unchanged. DUP-GUARD-1 later
    # moved its unit tests to the content crate's item_identity_tests.rs; production
    # code and the cohort are byte-identical.
    (
        "apps/game-server/src/content/item_identity.rs",
        "0df52c391a1d56e5d82be269ddcd480233f923007cfdf35b9a79226bf2868dde",
    ): (
        "crates/gameplay/content-promotion/src/content/item_identity.rs",
        "12b998650847975760f4f5abbbf38e2f0eba1c11b51e844e5c89c74854103544",
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
