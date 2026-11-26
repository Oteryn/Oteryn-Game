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
import re
from pathlib import Path

# The reviewed spawn map revisions preserve the bound quest placements.
QUEST_PLAN_PLACEMENT_RELOCATIONS = {
    (
        "content/world/placements/index.json",
        "86d294671104ff928bc6070e4fab19b68c2f1598fbddbf9e0496ee37eee33197",
    ): (
        "content/world/placements/index.json",
        "43df88ca90c2710e5e2f858f2ceb10347dcbc86ff9fb0362b83c719947a6d9d5",
    ),
    (
        "content/world/placements/region-z07-x130-y127.b3",
        "dda5c6165990fc024b707fc6dcaca826b830a2d2b083ce98d19d0df470d3e127",
    ): (
        "content/world/placements/region-z07-x130-y127.b3",
        "b56bb3d6843ff73501174ded54acf8db9811fb25aa3eec672b9afb9cf99cf204",
    ),
    (
        "content/world/placements/region-z08-x128-y128.b3",
        "bd808a71a6bdd1284444209c02b3991b5e315ff2cccb2cc72f434c1061f3c004",
    ): (
        "content/world/placements/region-z08-x128-y128.b3",
        "87a420f93d73e369d11fb1af422923fabda2339e4b19e8f001abee6f30568100",
    ),
    (
        "content/world/placements/region-z09-x130-y125.b3",
        "2a259538a67532773231f989e72cca0bd84424524c39679e56b8c181e91216d9",
    ): (
        "content/world/placements/region-z09-x130-y125.b3",
        "b30ac2211f8d1e3ec679d513be4e67acb35f3e5160a204be612cacdcc7f31433",
    ),
    (
        "content/world/placements/region-z11-x129-y126.b3",
        "25000632406f556ca125613aa69a3ed52a6ddca472e4874c2c4b6f1761dd5f94",
    ): (
        "content/world/placements/region-z11-x129-y126.b3",
        "10e4b1ca595b43df3499715a7910e01718eef5dbf99e9b95e0d0f28e82095af4",
    ),
}

# (pinned path, pinned sha256) -> (current path, current sha256)
RELOCATIONS = {
    **QUEST_PLAN_PLACEMENT_RELOCATIONS,
    (
        "content/world/terrain/index.json",
        "ce986ca83b92f5ea2b7aeaa5bdc75b591cb28d63ab1a93dce0077215ce90979f",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "96b24f0cbedb168d73aeca26644536693d97b7fe2d2f49ecbd150bdfd4f3037b",
    ): (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "bb5e66f244a6a512baf9044e7accca7e4c74dfc642debc5d2ab28ea224cf0262",
    ),
    # The source-qualified Anomaly ground retains its pre-existing Item route;
    # the complete terrain row otherwise remains byte-identical.
    (
        "content/world/terrain/terrain-05500-05999.json",
        "7ba587fc470273980e0a75f0d597a13543b827d39e7f948ec469d1473a40d6c9",
    ): (
        "content/world/terrain/terrain-05500-05999.json",
        "08e0c9e30efbdfb15637e43cda5f5e20b5e862035e459a6f9b25436cdedf142b",
    ),
    # Exact QMECH owner-input transitions preserve closed Item rows. The helper
    # aliases below cover their reviewed formatting/import cleanup.
    (
        "content/world/terrain/index.json",
        "704991a60238795986358b6d2bcb7ac25f2172a04723b541e3f959a50eb9d768",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/terrain-05500-05999.json",
        "c5081d0589d6e65a40baf89178c4e29f81089147d463d168474ac286d546a66a",
    ): (
        "content/world/terrain/terrain-05500-05999.json",
        "08e0c9e30efbdfb15637e43cda5f5e20b5e862035e459a6f9b25436cdedf142b",
    ),
    (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "db9150a3639c4d6bcd596c040b7d2ad39047fbc3f51c925761bebfbb6def26fd",
    ): (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "bb5e66f244a6a512baf9044e7accca7e4c74dfc642debc5d2ab28ea224cf0262",
    ),
    (
        "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py",
        "3f6a78aece8db60a19f0114c4ed9998c28c4330cc598319936e3eb0446740182",
    ): (
        "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py",
        "ebfe6d0f0f1a141f5b3cf3a3606edba091845c0913acc570953fa29d46f55fde",
    ),
    (
        "tools/content-schema/item-authoring/check_stack_default_historical_context.py",
        "d0b52da99de9ffff5ab0804ac13f293cc77f9d654b9291a59f56119f47d21575",
    ): (
        "tools/content-schema/item-authoring/check_stack_default_historical_context.py",
        "0304232dbc524a5088976178acbe2e7a86a1b607cf6c13a7952d084fb73be529",
    ),
    (
        "tools/content-schema/item-authoring/test_stack_default_historical_context.py",
        "bf74e7cb74521fa6ed7e506dba9d32a1c552c09a670de62cbe152df79d18d14b",
    ): (
        "tools/content-schema/item-authoring/test_stack_default_historical_context.py",
        "b7dc47b1f179fd919950be1cb0beadece76db594e0968f41bb53356172325210",
    ),
    # ITEMGLOB3 supersedes only the Wiki modifier rows for i47376 and i49872.
    # Neither Item belongs to the closed stack or elemental-magic cohorts.
    # The merged World refresh changes only the packet's map-owner digests.
    (
        "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json",
        "82cd7cc71421911cfb85f0fac5375bf9a225706793dd5debca91b147b91215ba",
    ): (
        "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json",
        "5ef32cfa01e7a319e4ef056592df1aa66e7fbff30b6d86604f5354316436aa3d",
    ),
    (
        "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json",
        "f7e73f663dcafae8d757300cd324d7b1f3f1691b0dbc2b11b068f7f861b92684",
    ): (
        "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json",
        "5ef32cfa01e7a319e4ef056592df1aa66e7fbff30b6d86604f5354316436aa3d",
    ),
    # Numeric17 reuses the Mantra/Bond compiler's source readers. Their behavior
    # is unchanged by the exact Global overlay check on live Mantra/Bond rows.
    (
        "tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py",
        "70b2a00b1fd6512a7ec02b6f0b6ecc4548a29d06bd093a93df607626df05cbea",
    ): (
        "tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py",
        "b56128eee9602582a3020db96e9dd003f733773921b532b2af89be3d3889a30e",
    ),
    # The closed139 name packet changes only terrain fingerprints. Its Rust
    # consumer changes only that packet's digest; qualification and rows are exact.
    (
        "docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json",
        "a3c59faa4b6ccab7e0b4029004c0a1bbe22a8e246c6029ae479d63029972d0f9",
    ): (
        "docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json",
        "2442cec4dabda2c89593259138ed41e41a6e5edf59ab8bd9abdc2340fb6e7539",
    ),
    (
        "crates/gameplay/content/src/content/item_name_promotion.rs",
        "36b822f96bab04e4ac6566a5742ee29f8616d6c283fb67e92f1528ee8e8eee0d",
    ): (
        "crates/gameplay/content-promotion/src/content/item_name_promotion.rs",
        "29c05a9d6a5dbee23213ed047510efc41cbdb2723e38a787da64b8d0c9856502",
    ),
    (
        "crates/gameplay/content-promotion/src/content/item_name_promotion.rs",
        "36b822f96bab04e4ac6566a5742ee29f8616d6c283fb67e92f1528ee8e8eee0d",
    ): (
        "crates/gameplay/content-promotion/src/content/item_name_promotion.rs",
        "29c05a9d6a5dbee23213ed047510efc41cbdb2723e38a787da64b8d0c9856502",
    ),
    # QMINT3's tree-first reference view changes only the Forge289 compiler and
    # mapper fingerprints. All454 binding rows and289 literal promotions are
    # byte-equivalent; the consumer still rebuilds and checks their whole scope.
    # QMECH formatting updates only those same compiler and mapper fingerprints.
    (
        "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json",
        "a85c850e31342f8466c4f7a7d5165425a51468bb56aaea01fd0b995592185f57",
    ): (
        "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json",
        "69bf44d0e7aea723e634f2fba199bdd39b2e5c80a3a64a3d74597d68aafe95aa",
    ),
    # RewardClaim validator gains the bounded multi-item form; Item qualifiers stay unchanged.
    (
        "crates/gameplay/content/src/content/reference_playable.rs",
        "f090002566f663b57fae0d37ab0a35e4db1117d8871d8aa021488ad8b90261c2",
    ): (
        "crates/gameplay/content/src/content/reference_playable.rs",
        "e559f5bdff432d83198bfa4e4d447ec993abcad9ad928a4d86403b8793162593",
    ),
    # QMINT3: exact merged geometry footprints and compatibility-only producer
    # changes. Frozen record scopes and current ownership checks remain enforced.
    # QMECH restores source-qualified activity i23049 and composes owner notes
    # deterministically. The helper aliases retain their reviewed compatibility
    # changes; the terrain shard also resolves through its qualified source pin.
    (
        "content/world/objects/index.json",
        "1e2f278f385c502dcd59e6859fab3bb096f71d6c3f327b1b2e56e85f85e1b431",
    ): (
        "content/world/objects/index.json",
        "1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30",
    ),
    (
        "content/world/objects/index.json",
        "a0dbd7f2c0c0b09b5457b189128c7cbd7dd2a4553e2a3c00063092372c3b65d7",
    ): (
        "content/world/objects/index.json",
        "1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30",
    ),
    (
        "content/world/objects/index.json",
        "f23cb0a1a89c20e9957834fe63e1e8a01531cbba6ecb76a1e0ffde5c90a6b7cc",
    ): (
        "content/world/objects/index.json",
        "1a7e2a2a99b2dd5b59cb52970928e58d46296884c3fa519319da905d4b62bb30",
    ),
    (
        "content/world/terrain/index.json",
        "0549b3567749436b9e6f91f79751d32d9ea4431fe6b1be7505566d50b797e27d",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/index.json",
        "22cb600cca441ac68526501c8e3576ec9c1dd07f77b276db470c70b996028fec",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/index.json",
        "360e65beb939450179cb6c99b5c19a0e2842051fd9c3beaf21e6366b9c9245a4",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/index.json",
        "89ae879da01b6a81e1e6a1c6a06b2632a2678066e0eb71404a5a7882bd516c44",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/index.json",
        "abffe2dd83dfec1ce1afd657bd0fcaae3760a10a509f8e1034e35834a714ba79",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/index.json",
        "baf9f79b334c9478895ae824c67aad32e06eb4e7b09435570389dd64ad455836",
    ): (
        "content/world/terrain/index.json",
        "8229c3e86315d22bfcb2841d7e024ecafdf63c7110265ccb97da797c41cdffea",
    ),
    (
        "content/world/terrain/terrain-08614-08668.json",
        "dfd54e670a7de3eb8b5a444df9c8efc15faf8ddafadab761347338d80141f820",
    ): (
        "content/world/terrain/terrain-08614-08668.json",
        "0c35e7cc19b8d799b9f33cccb0f989986f13e19006f379a28b110df76f50efe5",
    ),
    (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "00878d28eb56c84f57734956505f60eb1fe7d248b7d1002c19209172a5125cfd",
    ): (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "bb5e66f244a6a512baf9044e7accca7e4c74dfc642debc5d2ab28ea224cf0262",
    ),
    (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "8e41e838db44b527e4a243d4be1dc5245fe74def7145d9a68cdfc1436d609d5c",
    ): (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "bb5e66f244a6a512baf9044e7accca7e4c74dfc642debc5d2ab28ea224cf0262",
    ),
    (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "ae1a4f2c09345efbda4293eee96a93f9054bb33e40f658ab483b8b0c88027477",
    ): (
        "tools/content-schema/item-authoring/historical_world_owner_context.py",
        "bb5e66f244a6a512baf9044e7accca7e4c74dfc642debc5d2ab28ea224cf0262",
    ),
    (
        "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py",
        "5969b009ed326991e6db310b3e3c48640ae5a5ac361c9c34f5619740bff8ce2e",
    ): (
        "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py",
        "ebfe6d0f0f1a141f5b3cf3a3606edba091845c0913acc570953fa29d46f55fde",
    ),
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
    # code and the cohort are byte-identical. #3250 then made the key rule apply to a
    # candidate draft and added the promotion-time unique Item key check; both are
    # additions only, and the cohort is unchanged. The promotion tests now live
    # in item_identity_promotion_tests.rs.
    (
        "apps/game-server/src/content/item_identity.rs",
        "0df52c391a1d56e5d82be269ddcd480233f923007cfdf35b9a79226bf2868dde",
    ): (
        "crates/gameplay/content-promotion/src/content/item_identity.rs",
        "21ad89460c333ee7f61bc16f70e7a81cd33f65bb8d18be513db937e11007d41e",
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


def retain_quest_plan_citations(rows: list[dict]) -> list[dict]:
    """Preserve published plan identities for exact, reviewed placement changes."""
    historical_citations = {
        f"{current_path}:1:sha256:{current}": f"{path}:1:sha256:{historical}"
        for (path, historical), (
            current_path,
            current,
        ) in QUEST_PLAN_PLACEMENT_RELOCATIONS.items()
    }
    return [
        {
            **row,
            "sources": [
                historical_citations.get(source, source) for source in row["sources"]
            ],
        }
        for row in rows
    ]


def refresh_current(paths: list[str]) -> None:
    """Refresh the destination digests of explicitly reviewed paths."""
    ledger = Path(__file__).resolve()
    root = ledger.parents[2]
    text = ledger.read_text()
    for relative in paths:
        count = sum(current == relative for current, _ in RELOCATIONS.values())
        if not count:
            raise ValueError(f"no relocation target: {relative}")
        current = digest((root / relative).read_bytes())
        if (relative, current) in RELOCATIONS:
            raise ValueError(f"relocation target matches a historical pin: {relative}")
        pattern = rf"""(:\s*\(\s*['"]{re.escape(relative)}['"]\s*,\s*['"])[0-9a-f]{{64}}(['"])"""
        text, updated = re.subn(pattern, rf"\g<1>{current}\2", text)
        if updated != count:
            raise ValueError(f"unexpected number of relocation entries: {relative}")
    ledger.write_text(text)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=refresh_current.__doc__)
    parser.add_argument("--refresh-current", nargs="+", required=True, metavar="PATH")
    refresh_current(parser.parse_args().refresh_current)
