"""Capture Soulpit's source-only selection facts; never admit a native identity.

Run with the existing lupa==2.8 authoring dependency and a clean exact Crystal
checkout. The normal canary_batch Lua recorder evaluates the source tables.
Only the reviewed CakeQuest.Items data and spell-script Combat recorder are
supplied for two source dependencies; no game callbacks or engine run.
"""
import argparse
import hashlib
import json
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

import canary_batch as capture
import spell_scripts

REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
MONSTERS = "data-global/monster"
CAKE = MONSTERS + "/quests/a_piece_of_cake/cake_golem.lua"
MUSHROOM = MONSTERS + "/quests/rotten_blood_quest/mushroom.lua"
CAKE_CONFIG = "data-global/scripts/lib/a_piece_of_cake_config.lua"
PRIMAL = "data-global/lib/quests/the_primal_ordeal.lua"
SUPPORT = (
    "config.lua.dist", "src/crystalserver.cpp", "src/lua/scripts/scripts.cpp",
    "src/game/game.hpp", "src/game/game.cpp", "src/creatures/monsters/monsters.hpp",
    "src/creatures/monsters/monsters.cpp", "src/creatures/creatures_definitions.hpp",
    "src/lua/functions/creatures/monster/monster_type_functions.cpp",
    "src/lua/functions/core/game/game_functions.cpp",
    "data/scripts/lib/register_monster_type.lua", "data/core.lua", "data/global.lua",
    "data-global/lib/lib.lua", "data-global/lib/quests/quest.lua", CAKE_CONFIG, PRIMAL,
)
OUTSIDE = ("data/lib", "data/libs", "data/scripts", "data-global/lib",
           "data-global/scripts", "data-global/startup")
NON_MONSTERS = {
    MONSTERS + "/quests/rotten_blood_quest/rottenEssences.lua",
    MONSTERS + "/quests/the_secret_library/bosses/grand_master_oberon_functions.lua",
}


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()


def git(root, *args, allow_no_match=False):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=False)
    if result.returncode and not (allow_no_match and result.returncode == 1):
        raise ValueError(result.stderr.decode(errors="replace")[:600])
    return result.stdout


def pinned(root, path):
    data = (root / path).read_bytes()
    oid = git(root, "rev-parse", REVISION + ":" + path).decode().strip()
    if capture.blob_id(data) != oid:
        raise ValueError("source worktree differs from pinned blob: " + path)
    return data.decode(), {"path": path, "git_blob": oid, "sha256": hashlib.sha256(data).hexdigest()}


def outside_creators(root):
    # Examine the exact Git tree, including non-materialized sparse-checkout paths.
    result = git(root, "grep", "-n", "-E", "createMonsterType|:raceId\\(", REVISION,
                 "--", *OUTSIDE, allow_no_match=True).decode().splitlines()
    creators, setters = [], []
    for line in result:
        _, path, number, text = line.split(":", 3)
        if "createMonsterType" in text:
            creators.append({"path": path, "line": int(number), "text": text.strip()})
        if re.search(r":raceId\(\s*[^)\s]", text):
            setters.append({"path": path, "line": int(number), "text": text.strip()})
    if [row["path"] for row in creators] != [PRIMAL]:
        raise ValueError("unqualified out-of-monster type creator")
    if [row["path"] for row in setters] != ["data/scripts/lib/register_monster_type.lua"]:
        raise ValueError("unqualified out-of-monster Bestiary setter")
    return {"scopes": list(OUTSIDE), "creators": creators, "setters": setters}


def supported_capture(root, relative, source, cake_items):
    original = capture.LUA_PRELUDE
    extra = ""
    if relative == CAKE:
        extra = "CakeQuest = {}\n" + cake_items + "\n"
    elif relative == MUSHROOM:
        # Existing authoring recorder: constructors only. Mushroom stores callbacks;
        # they are never invoked, and neither recorder changes the monster table.
        extra = "(function()\n" + spell_scripts.LUA_SANDBOX + "\nend)()\n"
    if extra:
        capture.LUA_PRELUDE = original.replace("return registered, callbacks, capture_error",
                                             extra + "return registered, callbacks, capture_error")
    errors = []
    try:
        name, raw, _ = capture.load_monster(root / relative, errors)
    finally:
        capture.LUA_PRELUDE = original
    if errors:
        if raw.get("raceId") is not None:
            # All 11 calls follow the original registration. The pinned helper
            # copies that table and removes membership from its generated type.
            if (len(errors) != 1 or "RegisterPrimalPackBeast" not in errors[0]
                    or errors[0].startswith("PRE_REGISTER:")
                    or source.count("RegisterPrimalPackBeast(monster)") != 1):
                raise ValueError("unqualified Bestiary capture error: " + relative)
        elif "raceId" in source or "Bestiary" in source:
            raise ValueError("unqualified absent-membership proof: " + relative)
        # A created type without any raceId/Bestiary assignment has the source
        # defaults regardless of later quest callback construction dependencies.
    return name, raw, errors


def facts(name, raw, races):
    bestiary = raw.get("Bestiary") or {}
    if not isinstance(bestiary, dict):
        raise TypeError("unqualified Bestiary table: " + name)
    race = bestiary.get("race")
    race = 0 if race is None else races[race.removeprefix("@")]
    stars = bestiary.get("Stars", 0)
    race_id = raw.get("raceId")
    if type(stars) is not int or not 0 <= stars <= 255:
        raise ValueError("unqualified star value: " + name)
    if race_id is not None and (type(race_id) is not int or not 0 <= race_id <= 65535):
        raise ValueError("unqualified race ID: " + name)
    result = {"registration_name": name, "name": raw.get("name", name),
              "race_id": race_id, "class": bestiary.get("class", ""), "race": race, "stars": stars}
    if not all(isinstance(result[key], str) for key in ("registration_name", "name", "class")):
        raise ValueError("unqualified source name/class: " + name)
    return result


def capture_roster(root, repository):
    if git(root, "rev-parse", "HEAD").decode().strip() != REVISION:
        raise ValueError("source HEAD differs from exact Crystal pin")
    texts, support = {}, []
    for path in SUPPORT:
        text, evidence = pinned(root, path)
        texts[path] = text
        support.append(evidence)
    for fragment in ("primalMonster.raceId = nil", "primalMonster.Bestiary = nil",
                     "local primalMonster = table.copy(template)"):
        if fragment not in texts[PRIMAL]:
            raise ValueError("Primal copy membership proof changed")
    tables = re.findall(r"(?ms)^CakeQuest\.Items = \{\n.*?^\}", texts[CAKE_CONFIG])
    if len(tables) != 1:
        raise ValueError("source CakeQuest.Items table is not uniquely bounded")
    races = {name: int(value) for name, value in re.findall(
        r"\b(BESTY_RACE_[A-Z_]+)\s*=\s*(\d+)", texts["src/creatures/creatures_definitions.hpp"])}
    if sorted(races.values()) != list(range(22)):
        raise ValueError("Bestiary race enum changed")
    outside = outside_creators(root)
    files = []
    for raw in git(root, "ls-tree", "-rz", REVISION, "--", MONSTERS).split(b"\0"):
        if not raw:
            continue
        meta, path = raw.split(b"\t", 1)
        _, kind, oid = meta.decode().split()
        path = path.decode()
        if kind == "blob" and path.endswith(".lua"):
            files.append((path, oid))
    rows, evidence, skipped = [], [], []
    for path, oid in sorted(files):
        source, proof = pinned(root, path)
        if proof["git_blob"] != oid:
            raise ValueError("source tree mismatch")
        if Path(path).name.startswith("#") or Path(path).parent.name in ("lib", "events"):
            skipped.append({**proof, "reason": "source_loader_exclusion"})
            continue
        if path in NON_MONSTERS:
            if "createMonsterType" in source or "raceId" in source:
                raise ValueError("non-monster helper acquired registration")
            skipped.append({**proof, "reason": "no_type_creation_or_bestiary_mutation"})
            continue
        creators = re.findall(r'Game\.createMonsterType\("([^"\n]+)"\)', source)
        if len(creators) != 1 or not re.search(r'^local mType = Game\.createMonsterType\("', source, re.MULTILINE):
            raise ValueError("unqualified dynamic/aliased creator: " + path)
        if len(re.findall(r"\bcreateMonsterType\s*\(", source)) != 1:
            raise ValueError("unqualified additional type creator: " + path)
        name, raw, errors = supported_capture(root, path, source, tables[0])
        if name != creators[0]:
            raise ValueError("registered and created name differ")
        row = facts(name, raw, races)
        rows.append(row)
        evidence.append({**proof, "registration_name": name, "capture_errors": errors,
                         "qualification": "registered_metadata" if not errors else
                         ("registered_before_non_bestiary_primal_copy" if row["race_id"] is not None
                          else "created_type_has_no_bestiary_assignments")})
    registry = {}
    duplicates = defaultdict(list)
    for row in rows:
        key = row["registration_name"].lower()
        if key in registry and row != registry[key]:
            raise ValueError("load-order-dependent registry metadata: " + key)
        registry[key] = row
        duplicates[key].append(row["registration_name"])
    monsters = [registry[key] for key in sorted(registry)]
    by_race = defaultdict(list)
    for index, row in enumerate(monsters):
        if row["race_id"] is not None:
            by_race[row["race_id"]].append(index)
    core_document = repository / "tools/content-schema/native-gameplay/soulpit-echo-item-profiles.json"
    core_data = json.loads(core_document.read_text())
    core_names = {row["name"].lower() for row in core_data["core_catalogue"]}
    duplicate_races, indices = [], []
    for race_id, members in sorted(by_race.items()):
        metadata = [monsters[index] for index in members]
        if len(members) > 1:
            values = {(row["race"], row["class"], row["stars"]) for row in metadata}
            names = {row["name"] for row in metadata}
            # The unsorted filesystem loader has two first-wins collisions.
            # Equal output names are equivalent; NONE/0 alternatives both lacking
            # any core also have the same refusal in both selection consumers.
            missing_zero = values == {(0, "", 0)} and all(
                name.lower() + " soul core" not in core_names for name in names)
            if len(values) != 1 or (len(names) != 1 and not missing_zero):
                raise ValueError("non-equivalent first-wins Bestiary collision")
            duplicate_races.append({"race_id": race_id,
                                    "members": [row["registration_name"] for row in metadata],
                                    "equivalence": "same_source_output_name" if len(names) == 1
                                    else "same_default_race_stars_and_no_source_core"})
        indices.append(members[0])
    runtime = {"monsters": monsters, "bestiary_indices": indices}
    digest = hashlib.sha256(canonical(runtime)).hexdigest()
    return {
        "schema": "OTERYN_SOULPIT_SOURCE_BESTIARY/v1", "source_revision": REVISION,
        "scope": "Source selection metadata only; no Creature or Item identity admission",
        "source_monster_tree": git(root, "rev-parse", REVISION + ":" + MONSTERS).decode().strip(),
        "roster_sha256": digest, "runtime": runtime, "duplicate_races": duplicate_races,
        "equivalent_duplicate_registry_names": sorted(key for key, values in duplicates.items() if len(values) > 1),
        "source_files": evidence, "source_loader_exclusions": skipped, "support": support,
        "outside_monster_closure": outside,
        "capture_tool_sha256": hashlib.sha256(Path(capture.__file__).read_bytes()).hexdigest(),
        "core_catalogue_sha256": hashlib.sha256(core_document.read_bytes()).hexdigest(),
    }


def rust_catalogue(document):
    q = lambda text: json.dumps(text, ensure_ascii=False)
    lines = ["//! Generated by capture_soulpit_bestiary.py from exact Crystal201189f.",
             "//! Source metadata only; native admission remains with its existing owners.",
             "use super::SourceMonsterFacts;", "",
             "pub const SOURCE_BESTIARY_ROSTER_SHA256: &str = " + q(document["roster_sha256"]) + ";",
             "pub(super) const MONSTERS: &[SourceMonsterFacts] = &["]
    for row in document["runtime"]["monsters"]:
        race_id = "None" if row["race_id"] is None else "Some(" + str(row["race_id"]) + ")"
        lines.append("    SourceMonsterFacts { registration_name: " + q(row["registration_name"])
                     + ", name: " + q(row["name"]) + ", race_id: " + race_id
                     + ", class: " + q(row["class"]) + ", race: " + str(row["race"])
                     + ", stars: " + str(row["stars"]) + " },")
    lines.extend(["];", "", "pub(super) const BESTIARY_INDICES: &[usize] = &["])
    indices = document["runtime"]["bestiary_indices"]
    lines.extend("    " + ", ".join(map(str, indices[index:index + 16])) + ","
                 for index in range(0, len(indices), 16))
    return "\n".join([*lines, "];", ""])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--repository", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--rust-out", type=Path, required=True)
    args = parser.parse_args()
    document = capture_roster(args.source, args.repository)
    args.out.write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n")
    args.rust_out.write_text(rust_catalogue(document))
    subprocess.run(["rustfmt", "--edition", "2024", str(args.rust_out)], check=True)
    candidates = [document["runtime"]["monsters"][index] for index in document["runtime"]["bestiary_indices"]]
    print(json.dumps({"roster_sha256": document["roster_sha256"],
                      "source_registry_names": len(document["runtime"]["monsters"]),
                      "bestiary_candidates": len(candidates),
                      "stars": dict(sorted(Counter(row["stars"] for row in candidates).items())),
                      "duplicate_races": document["duplicate_races"]}, indent=2))


if __name__ == "__main__":
    main()
