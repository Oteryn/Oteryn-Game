#!/usr/bin/env python3
"""Capture pinned BossLever candidates without evaluating arbitrary Lua.

This tool emits source facts, including explicit incomplete/unknown closures.
Only the existing ContentActivationController can qualify a runtime template.
Run with the exact Crystal checkout and the existing compiled creature profile
document; generated candidates never insert a gameplay catalogue row.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re
import sys

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY_ROOT / "tools/content-migration"))
import boss_reward_item_profiles

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
POSITION = r"Position\(\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*\)"

def sha(value):
    return hashlib.sha256(value).hexdigest()

def number(expression):
    node = ast.parse(expression.strip(), mode="eval").body
    def visit(value):
        if isinstance(value, ast.Constant) and type(value.value) is int:
            return value.value
        if isinstance(value, ast.BinOp) and isinstance(value.op, ast.Mult):
            return visit(value.left) * visit(value.right)
        raise ValueError("only literal integers and source multiplication are admitted")
    result = visit(node)
    if not 0 <= result <= 2**32 - 1:
        raise ValueError("source integer outside supported bounds")
    return result

def table(text, marker):
    match = re.search(marker + r"\s*=\s*\{", text)
    if not match:
        return None
    start = match.end() - 1
    depth, quote, escaped = 0, None, False
    for index in range(start, len(text)):
        char = text[index]
        if quote:
            if escaped: escaped = False
            elif char == "\\": escaped = True
            elif char == quote: quote = None
            continue
        if char in "\"'": quote = char
        elif char == "{": depth += 1
        elif char == "}":
            depth -= 1
            if not depth: return text[start:index+1]
    return None

def position(text, marker):
    found = re.search(marker + r"\s*=\s*" + POSITION, text or "")
    return [int(value) for value in found.groups()] if found else None

def policy(config, name, default):
    match = re.search(r"\b" + name + r"\s*=\s*([^,\n}]+)", config)
    return number(match.group(1)) if match else default

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()

def native_callback_closure(key, profiles):
    """Only a complete pinned native donor capture can close an empty hook set.

    Canary47df sugar_daddy.lua:97-132 has only native attacks/defenses and no
    event scripts or summons. Crystal boss_lever.lua:241 always installs the
    implemented BossLeverOnDeath callback; a missing native hook is not ignored.
    """
    raw = (REPOSITORY_ROOT / boss_reward_item_profiles.PATH).read_bytes()
    if sha(raw) != boss_reward_item_profiles.CAPTURE_SHA256:
        raise ValueError("BOSS_NATIVE_CALLBACK_CAPTURE_DRIFT")
    closure = json.loads(raw)["callback_closure"]
    if (key != closure["creature_key"] or key not in profiles
            or sha(canonical(profiles[key])) != closure["native_profile_sha256"]):
        return [], False
    if (closure["source_events"] != [] or closure["source_summons"] is not False
            or closure["required_callbacks"] != ["BossLeverOnDeath"]):
        raise ValueError("BOSS_NATIVE_CALLBACK_CLOSURE")
    return closure["required_callbacks"], True


# Crystal 201189f the_primal_menace.lua:66-69,339-347 and the two
# death handlers: the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua:1-3,21
# and primal_ordeal_quest/bosses_killed.lua:6-7,18. These are source facts, not native support.
PRIMAL_HOOK_BLOBS = {
    "data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua": "76adae125dd1f8eb099878b6fa5f8b96f91445e8",
    "data-global/scripts/quests/the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua": "3cafb041e31f9f068b2ec118af2b5575aee6d2bc",
    "data-global/scripts/quests/primal_ordeal_quest/bosses_killed.lua": "cb0c34b64010470546b125959f2a63690bedb64b",
}


def capture_primal_hooks(root):
    sources = []
    for name, expected in PRIMAL_HOOK_BLOBS.items():
        path = root / name
        if not path.is_file():
            return [], False
        raw = path.read_bytes()
        actual = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        if actual != expected:
            return [], False
        clean = re.sub(r"--\[\[.*?\]\]", "", raw.decode(), flags=re.S)
        sources.append(re.sub(r"--[^\n]*", "", clean))
    monsters = [text for text in sources if table(text, r"\bmonster\.events") is not None]
    if len(monsters) != 1:
        return [], False
    events = re.findall(r'"([^"\n]+)"', table(monsters[0], r"\bmonster\.events"))
    declared = {name for text in sources for name in re.findall(r'CreatureEvent\("([^"\n]+)"\)', text)}
    think = re.search(r"\bmType\.onThink\s*=\s*function", monsters[0]) is not None
    hooks = sorted(set(events) | ({"mType.onThink"} if think else set()))
    return hooks, bool(events) and set(events).issubset(declared) and think


def capture(root, path, profiles, defaults):
    raw = path.read_bytes()
    text = raw.decode()
    # Comments carry examples and cannot define gameplay candidates.
    clean = re.sub(r"--\[\[.*?\]\]", "", text, flags=re.S)
    clean = re.sub(r"--[^\n]*", "", clean)
    if not re.search(r"\bBossLever\s*\(", clean):
        return None
    config = table(clean, r"\blocal\s+config")
    boss = table(config or "", r"\bboss")
    name = re.search(r'\bname\s*=\s*"([^"\n]+)"', boss or "")
    key = "oteryn:creature." + re.sub(r"[^a-z0-9]+", "_", name.group(1).lower()).strip("_") if name else None
    players = table(config or "", r"\bplayerPositions")
    pairs = re.findall(r"\{\s*pos\s*=\s*"+POSITION+r"\s*,\s*teleport\s*=\s*"+POSITION, players or "")
    lever = [[int(v) for v in pair[:3]] for pair in pairs]
    entry = [[int(v) for v in pair[3:]] for pair in pairs]
    area = table(config or "", r"\bspecPos")
    blockers = []
    if not config: blockers.append("nonliteral_or_multiple_configuration")
    for hook in ["createFunction", "onUseExtra", "encounter", "exitTeleporter"]:
        if re.search(r"\b"+hook+r"\s*=", config or ""): blockers.append("config."+hook)
    # Crystal 201189f actions/bosses_levers/the_primal_manace.lua:19 explicitly
    # retains the default cooldown. Only that literal is a qualified no-op.
    disabled = re.search(r"\bdisableCooldown\s*=\s*([^,\n}]+)", config or "")
    if disabled and disabled.group(1).strip() != "false":
        blockers.append("config.disableCooldown")
    if table(config or "", r"\bmonsters") not in [None,"{}"]: blockers.append("extra_monster_closure")
    if "zone:addArea" in clean: blockers.append("additional_zone_closure")
    if len(re.findall(r"\bBossLever\s*\(",clean)) != 1: blockers.append("multiple_lever_constructors")
    hooks, callbacks_known = native_callback_closure(key, profiles)
    if key == "oteryn:creature.faceless_bane":
        monster = root/"data-global/monster/quests/the_dream_courts/bosses/faceless_bane.lua"
        callbacks = root/"data-global/scripts/quests/the_dream_courts/creaturescripts_faceless_bane.lua"
        if monster.exists() and callbacks.exists():
            events = table(monster.read_text(), r"\bmonster\.events") or ""
            hooks = sorted(set(re.findall(r'"([^"\n]+)"',events)+re.findall(r'(?:CreatureEvent|registerEvent)\("([^"\n]+)"\)',callbacks.read_text())))
            callbacks_known = True
    if key == "oteryn:creature.the_primal_menace":
        hooks, callbacks_known = capture_primal_hooks(root)
    row = {
        "path":path.relative_to(root).as_posix(),"sha256":sha(raw),
        "line":text.count("\n",0,text.find("local config"))+1,
        "boss_name":name.group(1) if name else None,"creature_key":key,
        "boss_position":position(boss,r"\bposition"),"lever_positions":lever,"entry_positions":entry,
        "zone_from":position(area,r"\bfrom"),"zone_to":position(area,r"\bto"),
        "exit_position":position(config,r"\bexit"),"blockers":blockers,
        "required_callbacks":hooks,"callbacks_known":callbacks_known,
        "native_profile_sha256":sha(canonical(profiles[key])) if key in profiles else None,
    }
    for target, source, default in [
        ("minimum_players","minPlayers",1),("minimum_level","requiredLevel",0),
        ("cooldown_seconds","timeToFightAgain",defaults["cooldown"]),
        ("time_limit_seconds","timeToDefeat",defaults["timeout"]),("after_kill_seconds","timeAfterKill",60),
    ]:
        try: row[target] = policy(config or "",source,default)
        except (ValueError,SyntaxError): row[target]=None;blockers.append("nonliteral."+source)
    row["complete_literal_layout"] = bool(name and row["boss_position"] and lever and row["zone_from"] and row["zone_to"] and row["exit_position"])
    return row

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--source-root",type=Path,required=True)
    parser.add_argument("--native-profiles",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args=parser.parse_args()
    config=(args.source_root/"config.lua.dist").read_text()
    defaults={key:number(re.search(r"^"+name+r"\s*=\s*([^\n-]+)",config,re.M).group(1))
        for key,name in [("cooldown","bossDefaultTimeToFightAgain"),("timeout","bossDefaultTimeToDefeat")]}
    profiles={r["profile"]["target"]["key"]:r for r in json.loads(args.native_profiles.read_bytes())["records"]}
    records=[]
    for path in sorted(args.source_root.rglob("*.lua")):
        row=capture(args.source_root,path,profiles,defaults)
        if row: records.append(row)
    output={"schema":"OTERYN_CRYSTAL_BOSS_LEVER_CANDIDATES/v1","source_repository":"zimbadev/crystalserver","source_revision":PIN,
        "defaults_source":"config.lua.dist:658-659;data/libs/functions/boss_lever.lua:63-76","records":records}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_bytes(canonical(output)+b"\n")
    print(json.dumps({"records":len(records),"complete_literal_layouts":sum(r["complete_literal_layout"] for r in records),"output_sha256":sha(args.output.read_bytes())}))

if __name__ == "__main__": main()
