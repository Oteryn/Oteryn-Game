"""Build the proposed investigation catalogue in the existing native loader format.

Evidence only: keys are unallocated candidates; this never edits served content.
Run with --check to compare the committed fixture without writing.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
QUEST = "oteryn:quest.authored.shards_of_a_broken_moon_quest"
TRACK = "oteryn:quest-progress/shards-candidate/"
TRANSITION = "oteryn:quest-transition/shards-candidate/"


def build():
    tracks = {}
    transitions = []

    def fact(name):
        tracks.setdefault(name, dict(key=TRACK + name, quest=QUEST, initial=0,
                                     min=0, max=1, bounds_basis="PROPOSED_BOOLEAN_FACT",
                                     source_key=name))
        return TRACK + name

    def effect(name, before, after):
        return dict(track=fact(name), from_exact=True,
                    **{"from": dict(op="EQ", value=before)},
                    effect=dict(op="SET", value=after))

    def transition(name, guards, outcome, extra=None):
        effects = [effect(g, 1, 1) for g in guards]
        effects.append(effect(outcome, 0, 1))
        if extra:
            effects.append(effect(extra, 0, 1))
        transitions.append(dict(key=TRANSITION + name, quest=QUEST, completes=False,
                                effects=effects, source=dict(binding_event=name,
                                qualification="CANDIDATE_NOT_ADMITTED")))

    transition("start_saraki", [], "started", "start_route_saraki")
    transition("start_nilavarna_alt", [], "started", "start_route_nilavarna")
    transition("clue_yukti_sundara", ["started", "start_route_saraki"], "yukti_done")
    transition("clue_yukti_nipuna_alt", ["started", "start_route_nilavarna"], "yukti_done")
    transition("tides_tarisu", ["started"], "tides_started")
    markers = ["tide_marker_lagoon_a", "tide_marker_lagoon_b", "tide_marker_emerald"]
    for index, marker in enumerate(markers, 1):
        transition(f"tide_marker_{index}", ["tides_started"], marker)
    transition("tides_join", markers, "tides_done")
    transition("plants_dhira", ["started"], "plants_started")
    for plant in ["tide_veil", "whisper_reed"]:
        transition("plant_" + plant, ["plants_started"], plant + "_sample_obtained")
        transition("refine_" + plant, ["plants_started", plant + "_sample_obtained"], plant + "_refined")
    transition("plants_join", ["tide_veil_refined", "whisper_reed_refined"], "plants_done")
    transition("three_clues_join", ["yukti_done", "tides_done", "plants_done"], "three_clues_done")
    transition("report_saraki", ["three_clues_done", "start_route_saraki"], "report_done")
    transition("report_nilavarna_alt", ["three_clues_done", "start_route_nilavarna"], "report_done")
    return dict(schema="OTERYN_QUEST_STATE_LOWERING/v1", classification="EVIDENCE_ONLY_NOT_ADMITTED",
                family="Quest", contract="QUEST-STATE-1 / QUEST-LOWER-1",
                authoring_sources=["binding-plan.json", "q1-lowering-owner-plan.md"],
                counts=dict(quests=1, tracks=len(tracks), transitions=len(transitions)),
                quests=[dict(quest=QUEST, source_quest="SHARDS_Q1_CANDIDATE",
                completion="NOT_INCLUDED_INVESTIGATIONS_ONLY", tracks=list(tracks.values()),
                transitions=transitions)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = build()
    bindings = json.loads((ROOT / "binding-plan.json").read_text(encoding="utf-8"))
    ids = {e["id"] for e in bindings["events"]}
    assert all(t["source"]["binding_event"] in ids for t in result["quests"][0]["transitions"])
    assert all(len(t["effects"]) <= 8 for t in result["quests"][0]["transitions"])
    encoded = json.dumps(result, indent=2) + "\n"
    target = ROOT / "q1-native-catalogue-candidate.json"
    if args.check:
        if target.read_bytes() != encoded.encode("utf-8"):
            raise SystemExit("Q1 fixture drift")
    else:
        target.write_text(encoded, encoding="utf-8", newline="\n")
    print(result["counts"])


if __name__ == "__main__":
    main()
