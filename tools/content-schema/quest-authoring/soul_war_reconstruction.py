"""Validate the bounded Soul War reconstruction and faithful recipe followup."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

CRYSTAL = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
CANARY = "04b83b512114bfd888000d6e1433ed8ecaec7c5b"
PACKET = "tools/content-schema/quest-authoring/samples/soul-war-reconstruction/reconstruction.json"
SCHEMA = "tools/content-schema/quest-authoring/soul_war_reconstruction.schema.json"

REQUIRED_MECHANICS = {
    "claustrophobic_inferno_three_raids",
    "claustrophobic_inferno_crystal_bad_spawn_token",
    "malice_soul_cage",
    "malice_white_safe_tiles",
    "mirrored_nightmare_access",
    "greed_soul_sphere_cycle",
    "ebb_and_flow_dynamic_map",
    "spite_soul_fire",
    "spite_fear",
    "rotten_wasteland_shrines",
    "hatred_burning_cycle",
    "hatred_torment_management",
    "furious_crater_energy_access",
    "cruelty_greedy_eye",
    "cruelty_mortal_essence_greedy_maw",
    "megalomania_aspect_vulnerability",
    "megalomania_splinters_and_sanity",
    "mirror_image",
}
BOSSES = {
    "Goshnar's Malice",
    "Goshnar's Greed",
    "Goshnar's Spite",
    "Goshnar's Cruelty",
    "Goshnar's Hatred",
}
TAINTS = [
    (1, "taints-teleport"),
    (2, "taints-spawn"),
    (3, "taints-damage"),
    (4, "taints-heal"),
    (5, "taints-loss"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(root: Path, relative: str):
    return json.loads((root / relative).read_text(encoding="utf-8"))


def require(ok: bool, message: str):
    if not ok:
        raise ValueError(message)


def validate(root: Path):
    packet = load(root, PACKET)
    schema = load(root, SCHEMA)
    Draft202012Validator(schema).validate(packet)

    require(packet["runtime_activation"] is False, "Soul War runtime activation must remain false")
    require(packet["quest"] == {
        "key": "oteryn:quest.soul_war_quest",
        "revision": "quest-r1",
        "display_name": "Soul War Quest",
    }, "Soul War canonical identity differs")
    policy = packet["source_policy"]
    require(policy["primary_revision"] == CRYSTAL, "Crystal summer-update pin differs")
    require(policy["crosscheck_revision"] == CANARY, "Canary cross-check pin differs")

    source_files = packet["source_files"]
    require(len(source_files) == len({row["path"] for row in source_files}), "duplicate Soul War donor path")
    require(all(row["repository"] == "zimbadev/crystalserver" and row["revision"] == CRYSTAL for row in source_files),
            "Soul War donor substitution")
    source_paths = {row["path"] for row in source_files}
    for required in {
        "data-global/lib/quests/soul_war.lua",
        "data-global/scripts/quests/soul_war/soul_war_mechanics.lua",
        "data-global/scripts/quests/soul_war/moveevent-soul_war_entrances.lua",
        "data-global/scripts/quests/soul_war/moveevent-claustrophobic-inferno-raid.lua",
        "data-global/scripts/quests/soul_war/action-reward_soul_war.lua",
        "data-global/scripts/quests/soul_war/globalevent-ebb_and_flow_change_maps.lua",
        "data-global/npc/flickering_soul.lua",
    }:
        require(required in source_paths, f"missing Soul War donor source: {required}")

    videos = packet["video_sources"]
    require({row["part"] for row in videos} == set(range(1, 7)), "Soul War video parts differ")
    require(len({row["video_id"] for row in videos}) == 6, "Soul War video ids are not unique")
    video_ids = {row["video_id"] for row in videos}

    progression = packet["progression"]
    require(progression["admission"] == {"minimum_level": 250, "premium": True, "party_slots": 5},
            "Soul War admission differs")
    require(set(progression["mini_bosses"]) == BOSSES, "Soul War mini-boss set differs")
    require([(row["level"], row["key"]) for row in progression["taints"]] == TAINTS,
            "Soul War taint sequence differs")
    require(progression["taint_reset_seconds"] == 14 * 24 * 60 * 60, "Soul War taint reset differs")
    require(progression["mini_boss_retry_seconds"] == 20 * 60 * 60, "Soul War boss retry differs")
    require(progression["megalomania_retry_seconds"] == 72 * 60 * 60, "Soul War final retry differs")

    mechanics = packet["mechanics"]
    by_key = {row["key"]: row for row in mechanics}
    require(len(by_key) == len(mechanics), "duplicate Soul War mechanic key")
    require(REQUIRED_MECHANICS <= set(by_key), "Soul War mechanic coverage differs")
    require(by_key["mirror_image"]["classification"] == "OTERYN_ACCEPTED", "Mirror Image accepted ruling lost")
    require(by_key["spite_fear"]["classification"] == "OTERYN_ACCEPTED", "Spite Fear accepted ruling lost")
    require(by_key["claustrophobic_inferno_crystal_bad_spawn_token"]["classification"] == "CONFLICT",
            "Crystal bare Pos defect must remain a conflict")
    require(by_key["furious_crater_energy_access"]["chosen"]["floor_thresholds"] == [40, 55, 70],
            "Cruelty access thresholds differ")
    require(by_key["spite_soul_fire"]["chosen"]["create_interval_seconds"] == 14
            and by_key["spite_soul_fire"]["chosen"]["per_player_reuse_seconds"] == 56,
            "Spite fire timing differs")
    require(by_key["cruelty_mortal_essence_greedy_maw"]["chosen"]["per_player_use_cooldown_seconds"] == 30
            and by_key["cruelty_mortal_essence_greedy_maw"]["chosen"]["defense_grace_seconds"] == 15,
            "Cruelty Greedy Maw timing differs")
    require(by_key["megalomania_aspect_vulnerability"]["chosen"]["aspect_respawn_seconds"] == 5
            and by_key["megalomania_aspect_vulnerability"]["chosen"]["return_to_immune_seconds"] == 70,
            "Megalomania aspect timing differs")

    for mechanic in mechanics:
        for evidence in mechanic["evidence"]:
            if evidence["kind"] == "CRYSTAL_SOURCE":
                require(evidence["path"] in source_paths, f"unbound Crystal evidence: {evidence['path']}")
            elif evidence["kind"] == "VIDEO":
                require(evidence["video_id"] in video_ids, f"unknown Soul War video evidence: {evidence['video_id']}")
            elif evidence["kind"] in {"OTERYN_DECISION", "OTERYN_FORMAT"}:
                require((root / evidence["path"]).is_file(), f"missing Oteryn evidence: {evidence['path']}")

    defect = next((row for row in packet["known_donor_defects"]
                   if row["path"] == "data-global/lib/quests/soul_war.lua"
                   and "bare Pos" in row["defect"]), None)
    require(defect is not None and defect["revision"] == CRYSTAL, "Crystal bare Pos defect witness lost")
    require(packet["runtime_holds"], "Soul War runtime holds must remain explicit")

    import quest_soul_war_followup as followup
    follow_raw = (root / followup.PATH).read_bytes()
    require(hashlib.sha256(follow_raw).hexdigest() == followup.SHA256, "Soul War recipe followup digest differs")
    follow = json.loads(follow_raw)
    followup._validate_packet(follow)
    recon = follow["reconstruction"]
    require(recon["path"] == PACKET and recon["sha256"] == sha256(root / PACKET),
            "Soul War recipe followup reconstruction pin differs")
    require(recon["schema_path"] == SCHEMA and recon["schema_sha256"] == sha256(root / SCHEMA),
            "Soul War recipe followup schema pin differs")
    require(follow["donor_source_refs"] == source_files, "Soul War recipe donor set differs from reconstruction")
    return packet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = validate(args.root.resolve())
    print(f"Soul War reconstruction: {len(packet['mechanics'])} mechanics, 6 videos, runtime=false")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
