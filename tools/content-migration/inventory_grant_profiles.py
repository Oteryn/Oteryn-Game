"""Find native Item definitions used by the repository's inventory grants."""
from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "oteryn:item.tibia.i"
PROVIDERS = ("item_profiles", "starting_loadout_item_profiles", "item_profile_updates")
PHASES = "apps/game-server/src/qualification/client_phases"


def read_json(root, path):
    return json.loads((root / path).read_bytes())


def item_reference(value):
    if isinstance(value, int):
        return f"{PREFIX}{value}", None
    if isinstance(value, str) and value.startswith("oteryn:item."):
        return value, None
    if isinstance(value, dict) and value.get("family") == "Item":
        return value.get("key", value.get("production_key")), value.get("revision", value.get("revision_ref"))
    raise ValueError(f"Unrecognized inventory grant Item: {value!r}")


def profile_index(root):
    manifest = read_json(root, "content/spells.manifest.json")

    def pinned(field):
        pin = manifest[field]
        path = root / "content" / pin["path"]
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != pin["sha256"]:
            raise ValueError(f"Native manifest pin drift: {path}")
        return json.loads(raw)

    revisions = dict(pinned("item_keys")["records"])
    profiles = {}
    for provider in PROVIDERS:
        document = pinned(provider)
        if (provider == "item_profile_updates"
                and document["base_sha256"] != manifest["item_profiles"]["sha256"]):
            raise ValueError("Native Item updates target a different base")
        seen = set()
        for row in document["records"]:
            definition = row["production_definition"]
            key = (definition["production_key"], definition["revision_ref"])
            if definition["family"] != "Item" or revisions.get(key[0]) != key[1]:
                raise ValueError(f"Unexpected native Item definition: {definition}")
            if key in seen or (key in profiles and provider != "item_profile_updates"):
                raise ValueError(f"Duplicate native Item provider: {key}")
            seen.add(key)
            profiles[key] = row
    return revisions, profiles


def inventory_admissible(row):
    if row is None:
        return False
    admission = row["admission"]
    return (admission["materializable"] is True
            and admission["stack_class"] in ("NonStackable", "StackCapable")
            and "CharacterInventory" in admission["legal_destinations"])


def item_tree(value):
    """A grant can mint an outer container together with all of its children."""
    if isinstance(value, list):
        for child in value:
            yield from item_tree(child)
    elif isinstance(value, dict):
        if "item" in value:
            yield value["item"]
        if "item_id" in value:
            yield value["item_id"]
        if value.get("family") == "Item":
            yield value
        for key in ("items", "contents", "root", "children", "entries", "alternatives"):
            if key in value:
                yield from item_tree(value[key])


def fixture_grants(root, profiles):
    # Literal IDs and local u32 constants are resolved directly from the USE-fixture
    # callers. The bounded variable sources below are the loops using those helpers.
    constant = re.compile(r"const\s+(\w+)\s*:\s*u32\s*=\s*([0-9_]+)\s*;")
    call = re.compile(r"(?:character_prep::grant_item|\bseed_item_pair)\((?!\s*\n?\s*client:)[^,]+,\s*([^,]+),\s*([^,\n)]+)")
    for path in sorted((root / PHASES).glob("*.rs")):
        source = path.read_text()
        literals = {name: int(value.replace("_", "")) for name, value in constant.findall(source)}
        for hit in call.finditer(source):
            args = hit.group()
            operand = hit[2].strip() if args.startswith("seed_item_pair") else hit[1].strip()
            if re.fullmatch(r"[0-9_]+", operand):
                yield int(operand.replace("_", "")), path
            elif operand in literals:
                yield literals[operand], path
            elif operand == "id" and path.name == "full_e2ef3_e2ei-d.rs":
                bag_list = source.split("const SEEDED_BAGS:", 1)[1].split("];", 1)[0]
                for value in re.findall(r"\(I\d+,\s*([0-9_]+),", bag_list):
                    yield int(value.replace("_", "")), path
            elif operand == "id" and path.name == "full_e2ef5.rs":
                # seed_item_pair's own grant receives the same IDs as its callers.
                if args.startswith("seed_item_pair"):
                    table = source.split("for (id, acquire, refusal, cast, map, block, expiry) in [", 1)[1]
                    for value in re.findall(r"\(\s*([0-9_]+),", table.split("] {", 1)[0]):
                        yield int(value.replace("_", "")), path
            elif operand in ("id", "character_prep::item_id(key)?") and path.name in (
                    "full_e2ef3_e2ei-a.rs", "full_e2ef7.rs"):
                if operand == "id":
                    # P21 chooses its target from materializable profiles whose
                    # slot count and Strike tier match the live selector.
                    targets = []
                    for (key, _), row in profiles.items():
                        imbuement = row["semantics"].get("imbuement") or {}
                        value = imbuement.get("value", {})
                        slots = value.get("slot_count", {}).get("value", 0)
                        tiers = value.get("allowed_family_tiers", {}).get("value", [])
                        if (row["admission"]["materializable"] and 1 <= slots <= 3
                                and any(tier["family"] == "CRITICAL_HIT" for tier in tiers)):
                            targets.append(key)
                    if not targets:
                        raise ValueError("The P21 Strike fixture has no native equipment target")
                    yield from ((key, path) for key in targets)
                else:
                    recipe = root / "crates/gameplay/items/src/imbuement_catalogue.rs"
                    basic = recipe.read_text().split("family: 1,\n        tier: 1,", 1)[1].split("},", 1)[0]
                    for material in re.findall(r"\bMATERIAL_([0-9]+)\b", basic):
                        yield int(material), recipe
            else:
                raise ValueError(f"Unresolved qualification inventory grant in {path}: {operand}")
        for value in re.findall(r'"item-([0-9]+)-[0-9]+"', source):
            yield int(value), path

    fixture = root / "apps/game-server/src/node/serve/qualification_fixture.rs"
    source = fixture.read_text()
    legacy = source.split("let legacy = [", 1)[1].split("];", 1)[0]
    for value in re.findall(r"\(([0-9]+),\s*[0-9]+\)", legacy):
        yield int(value), fixture
    for value in re.findall(r'"item-([0-9]+)-[0-9]+"', source):
        yield int(value), fixture



def authored_grants(root):
    content = root / "content"
    for path in sorted((content / "starter").glob("starter-kits-*.json")):
        for record in read_json(root, path)["records"]:
            yield record["definition"]["item"], path

    for path in sorted((content / "interactions/reward_claims").glob("*.json")):
        for record in read_json(root, path).get("records", []):
            definition = record["definition"]
            if definition.get("quest", {}).get("key") == "canary:quest/dawnport_quest":
                continue
            variant = definition.get("source_variant", {})
            key_reward = variant.get("native_lowering", {}).get("pending_implementation") == [
                "KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED"]
            if definition.get("readiness") != "ready" and not (key_reward or definition.get("native_contents")):
                continue
            for placement in definition["placements"]:
                for item in item_tree(placement.get("reward")):
                    yield item, path
            for placement in (definition.get("native_contents") or {}).get("placements", []):
                for item in item_tree(placement):
                    yield item, path

    for path in sorted((content / "quests").rglob("*.json")):
        if path.parent.name.endswith("-held"):
            continue
        document = read_json(root, path)
        for callback in document.get("callbacks", []) if isinstance(document, dict) else []:
            if callback.get("quest") == "oteryn:quest.dawnport_quest":
                continue
            effects = callback.get("effects")
            world_reward = effects.get("item_reward") if isinstance(effects, dict) else None
            for reward in (callback.get("give"), callback.get("item_reward"), world_reward):
                for item in item_tree(reward):
                    yield item, path

    rules_source = (root / "crates/gameplay/quests/src/quest/npc_rules.rs").read_text()
    runtime = rules_source.split("pub fn runtime(", 1)[1].split("pub fn programs(", 1)[0]
    live_quests = set(re.findall(r'"(oteryn:quest\.[a-z0-9_]+)"', runtime))
    live_programs = set(re.findall(r'"(oteryn:quest-npc-program\.[a-z0-9_.-]+)"', runtime))
    supplemental = set(re.findall(r'content/dialogues/quest-rules-supplemental/([^"]+)', runtime))
    for path in sorted((content / "dialogues/quest-rules").glob("quest-*.json")) + sorted(
            (content / "dialogues/quest-rules-supplemental").glob("*.json")):
        document = read_json(root, path)
        if document.get("quest") == "oteryn:quest.dawnport_quest":
            continue
        if (document["schema"] == 1 and path.name not in supplemental
                and document.get("quest") not in live_quests
                and document.get("program_key") not in live_programs):
            continue
        for rule in document["rules"]:
            for effect in rule.get("effects", []):
                if effect["kind"] == "give_item":
                    yield effect["item"], path

    catalogue = "tools/content-schema/native-gameplay/reward-bag-catalogue.json"
    for bag in read_json(root, catalogue)["bags"]:
        if bag["playable"]:
            for reward in bag["rewards"]:
                yield reward["item"], root / catalogue

    recipes = content / "craft/recipes.json"
    for record in read_json(root, recipes)["records"]:
        recipe = record["recipe"]
        for item in item_tree(recipe["outputs"]):
            yield item, recipes
        for operand in recipe["operands"]:
            # World-station transforms alter scenery; a used tool or ingredient
            # transforms in its existing carried slot.
            if operand["role"] == "USED" and operand["effect"] == "TRANSFORM_TO":
                yield operand["output_item_id"], recipes

    cuisine = content / "quests/hot-cuisine-batch3.json"
    for recipe in read_json(root, cuisine)["recipes"]:
        for reward in item_tree(recipe.get("reward")):
            yield reward, cuisine


def missing_inventory_profiles(root=ROOT):
    revisions, profiles = profile_index(root)
    missing = defaultdict(set)
    for grants in (fixture_grants(root, profiles), authored_grants(root)):
        for value, source in grants:
            key, revision = item_reference(value)
            current = revisions.get(key)
            if current is None or (revision is not None and current != revision):
                raise ValueError(f"Inventory grant references an unindexed Item: {value} ({source})")
            if not inventory_admissible(profiles.get((key, current))):
                missing[key].add(str(source.relative_to(root)))
    return dict(sorted(missing.items()))


if __name__ == "__main__":
    missing = missing_inventory_profiles()
    print(json.dumps({key: sorted(paths) for key, paths in missing.items()}, indent=2))
    raise SystemExit(bool(missing))
