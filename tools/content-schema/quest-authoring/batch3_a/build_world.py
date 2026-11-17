"""Materialize exact batch-three A gates into the shared world callback schema.

The source documents preserve complete donor parameters. Publication is held
where the unchanged parser, selector or map cannot execute those parameters.
Coordinator R-CALLBACK-SCHEMA=B forbids extending the shared callback contract.
"""

import argparse
import json

from build_dialogues import HERE, PIN, QUESTS, REPO


def documents():
    import importlib.util

    result = {}
    for quest in sorted(QUESTS):
        path = HERE / f"capture_world_{quest}.py"
        if not path.is_file():
            continue
        spec = importlib.util.spec_from_file_location(f"batch3_a_world_{quest}", path)
        if spec is None or spec.loader is None:
            raise ValueError(f"Cannot load quest source author: {path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        document = module.document()
        if any(row["quest"] != QUESTS[quest] for row in document["callbacks"]):
            raise ValueError(f"Wrong world source owner: {quest}")
        result[quest] = document
    return result

def publication_holds(quest, row):
    """Closed evidence-based refusals; never lower a read-only guard to SET."""
    pr = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-"
    holds = []
    missing = []
    if any(state.get("read_only") for state in row["storage"]):
        missing.append("storage.read_only")
    if any(state.get("failure_message") is not None for state in row["storage"]):
        missing.append("storage.failure_message")
    if row.get("failure_teleport") is not None:
        missing.append("failure_teleport")
    if row.get("success_message") is not None:
        missing.append("success_message")
    if missing:
        semantics = {
            "in_service_of_yalahar": ("actions_mechanism.lua", 22, 33,
                "The donor only reads Questline; replacing the guard with SET0 or SET(threshold) would erase later progress (for example72 at a minimum21 gate)."),
            "bigfoot_burden": ("movements_gnomebase_teleport.lua", 78, 98,
                "The donor reads QuestLine/rank and distinguishes Xelvar refusal from low rank. SET would change player progress and one common refusal would discard the source branch order."),
            "the_hidden_city_of_beregar": (
                "moviments_ore_wagon.lua" if row.get("success_message") else "moviments_elevator.lua",
                9, 11 if row.get("success_message") else 23,
                "The donor emits the discovery text after the flag write; omitting it changes the interaction." if row.get("success_message") else
                "The source tests GoingDown==2 and never writes it; replacing a read-only guard by SET0 would remove completed elevator access."),
        }
        source, first, last, proof = semantics[quest]
        holds.append({"missing_fields": missing,
                      "sources": [f"{PIN}data-global/scripts/quests/{quest}/{source}:{first}-{last}"],
                      "proof": proof, "question": pr+"6589971017", "decision": pr+"6590300355"})
    selector = row["selector"]
    for kind, proven_missing in (("unique", {*range(3091, 3103), 1018, 1019}),
                                 ("action", {25018, 50092, 50093, 50111, 40029, 50091})):
        value = selector.get(kind)
        if value is None:
            continue
        if value not in proven_missing:
            raise ValueError(f"Unreviewed {kind} binding: {value}")
        holds.append({"missing_fields": [f"map.attrs.{kind}={value}"],
                      "sources": row["sources"],
                      "proof": "All1208 native map regions have no matching attribute. BundleFacts::push clones the sector attrs and quest_callback_matches requires the original unique/action. A position or appearance is not substituted.",
                      "map_index": "Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/world/placements/index.json:1",
                      "map_index_sha256": "9d492647ae0fbe15eccfe89dd888a27c51611cafbc848d6e3917b4b033118393",
                      "question": pr+"6590278878", "decision": pr+"6589517205"})
    if row["trigger"] == "use" and selector.get("position") is not None and all(
            selector.get(kind) is None for kind in ("unique", "action", "appearance")):
        holds.append({"missing_fields": ["existing position-only Use selector accepts only ordinal0"],
                      "sources": row["sources"] + [f"{PIN}data-global/scripts/quests/the_way_of_the_monk/actions_blue_valley_teleports.lua:15-25"],
                      "proof": "The clicked source item is above ground (bells49948 ordinal1/3; Rookgaard50310 ordinal2). Baseline matches rejects those items. No artificial itemid/UID or ground-only replacement is authorized.",
                      "question": pr+"6590420434", "decision": pr+"6590300355"})
    if quest == "bigfoot_burden" and row["key"].endswith(("gnomebase-31", "gnomebase-32")):
        holds.append({"missing_fields": ["ordered distinct failures for the same QuestLine"],
                      "sources": [f"{PIN}data-global/scripts/quests/bigfoot_burden/movements_gnomebase_teleport.lua:78-98"],
                      "failure_branches": [
                          {"QuestLine": "<1", "message": "You have no idea on how to use this device. Xelvar in Kazordoon might tell you more about it."},
                          {"QuestLine": "1..24", "message": "Your rank among the Gnomes is too low."}],
                      "proof": "One guard per track cannot retain both source refusals; requirement25 is not relaxed.",
                      "question": pr+"6589971017", "decision": pr+"6590300355"})
    return holds


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quest", choices=sorted(QUESTS))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for quest, document in documents().items():
        if args.quest and args.quest != quest:
            continue
        reasons = {row["key"]: publication_holds(quest, row) for row in document["callbacks"]}
        held_keys = {key for key, holds in reasons.items() if holds}
        active = {"schema": 1, "callbacks": [row for row in document["callbacks"] if row["key"] not in held_keys]}
        held = [row for row in document["callbacks"] if row["key"] in held_keys]
        targets = [(HERE / "world" / f"{quest}.json", document),
                   (REPO / "content/quests/world-callbacks" / f"batch3-a-{quest}.json", active)]
        if held:
            targets.append((REPO / "content/quests/world-callbacks-held" / f"batch3-a-{quest}.json", {
                "schema": 1, "runtime_enabled": False, "callbacks": held,
                "reason": "Complete source parameters retained; no unapproved shared-contract change or invented map binding is published.",
                "holds": [{"callback": key, "reasons": value} for key, value in reasons.items() if value],
                "question": "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589971017",
                "decision": "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6590300355",
            }))
        for path, value in targets:
            payload = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
            if args.check:
                if not path.exists() or path.read_text() != payload:
                    raise ValueError(f"Stale generated world callbacks: {path}")
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(payload)
        print(f"{quest}: {len(active['callbacks'])} active, {len(held)} held world callbacks")


if __name__ == "__main__":
    main()
