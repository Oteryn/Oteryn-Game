"""Pinned proficiency sources and the admitted canonical Proficiency definitions.

PROFICIENCY-0 §4.1 (PROF-CONTENT-1d): a definition is admitted from the pinned 15.30
client source alone. The admitted definitions are the committed `content/proficiencies/`
family (`oteryn:proficiency.tibia.p<ProficiencyId>`), so an Item binds one only through
the client crosswalk (`CLIENT_PROFICIENCY_SOURCE`, external id = `ProficiencyId`, source
version = its `Version`). Canary and Crystal crosswalks are optional corroboration and are
admitted only where pinned here (Magic Sword, 238/3). The levels and perks live in the
definition, never inline on the Item.
"""

import json
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PROFICIENCY_CONTENT = REPO_ROOT / "content" / "proficiencies"

CLIENT_PROFICIENCY_SOURCE = {
    "source_profile": "cipsoft_client_15_30_proficiencies_v1",
    "repository": "Oteryn/Oteryn-Game",
    "revision": "client-15.30",
    "path": "content/assets/files/proficiencies-7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015.json",
    "digest_sha256": "7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015",
}

CANARY_PROFICIENCY_SOURCE = {
    "source_profile": "canary_47dfd51_item_definition_v1",
    "repository": "opentibiabr/canary",
    "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
    "path": "data/items/proficiencies.json",
    "digest_sha256": "1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22",
}

CRYSTAL_PROFICIENCY_SOURCE = {
    "source_profile": "crystal_ff7ede5_item_definition_v1",
    "repository": "zimbadev/crystalserver",
    "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    "path": "data/json/proficiencies.json",
    "digest_sha256": "1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22",
}

MAGIC_SWORD_PROFICIENCY_REF = {
    "family": "Proficiency",
    "key": "oteryn:proficiency.tibia.p238",
    "revision": "definition-r1",
}

MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES = frozenset(
    {
        (CLIENT_PROFICIENCY_SOURCE["source_profile"], "238", 3),
        (CANARY_PROFICIENCY_SOURCE["source_profile"], "238", 3),
        (CRYSTAL_PROFICIENCY_SOURCE["source_profile"], "238", 3),
    }
)


def admitted_client_crosswalks():
    """(client profile, ProficiencyId, Version) -> ident of every admitted definition."""

    index = json.loads((PROFICIENCY_CONTENT / "index.json").read_text(encoding="utf-8"))
    admitted = {}
    for shard in index["shards"]:
        payload = json.loads((REPO_ROOT / shard).read_text(encoding="utf-8"))
        for row in payload["records"]:
            definition = row["definition"]
            source = definition["source"]
            admitted[
                (
                    CLIENT_PROFICIENCY_SOURCE["source_profile"],
                    str(source["proficiency_id"]),
                    source["version"],
                )
            ] = (
                "Proficiency",
                definition["identity"]["key"],
                definition["identity"]["revision"],
            )
    return admitted


def proficiency_crosswalk(source):
    """Return one exact source-to-canonical Magic Sword proficiency crosswalk."""

    return {
        **deepcopy(source),
        "external_id": "238",
        "source_version": 3,
        "target": deepcopy(MAGIC_SWORD_PROFICIENCY_REF),
    }


def magic_sword_proficiency():
    """Return the Magic Sword binding; its levels and perks are the definition's.

    The threshold class is the one `content/proficiencies/bindings.json` gives Item 3288.
    """

    return {
        "profile_binding": deepcopy(MAGIC_SWORD_PROFICIENCY_REF),
        "threshold_class": "standard",
    }
