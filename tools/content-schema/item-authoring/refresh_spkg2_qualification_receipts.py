"""Version input-footprint receipts after W15c, preserving frozen source facts.

Run from any directory. Original qualification bytes are hash checked and never
written. Only the enumerated owner/producer footprints may change; all records,
closed scopes, source observations and qualification decisions are copied intact.
"""

import argparse
import ast
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

from historical_world_owner_context import GROUND, MARKER, SUGAR_GROUND, original_marker

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-migration"))

from source_relocations import current_path

HERE = "tools/content-schema/item-authoring/"
RECIPES = (
    (
        "lower_item_use_observation_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-use-observation-source-qualification-v1.json",
        "b4bf2c87cc5c3b81c66521023775ac499dcd7d64fb1f10f4210fb682935dbd8a",
        2,
    ),
    (
        "lower_elemental_magic_modifier_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json",
        "022b40bdbd6de3d97d938eeff4d7370d5904961edf3ba8effde700c8f5973490",
        2,
    ),
    (
        "lower_wiki_stack_default_successor8_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json",
        "2f95cad2912f2cc3e29df6f2ecb04fef50595d8b370b39e9bf85d4da0b7fded4",
        2,
    ),
    (
        "lower_wiki_stack_default_successor8_packet",
        "RECEIPT",
        "docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v9.json",
        "6e219614713d4cc5d4e2b20fc622cb9f4ab553feb7cdd08d40e353a3aade787e",
        10,
    ),
    (
        "lower_item_name15_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-name15-source-qualification-v2.json",
        "20a113d4cc4d9dd1c97c7ee4e09c9c8bb7e11a271cd3acb740f56f31ae76a1bb",
        3,
    ),
    (
        "lower_mantra_bond_modifier_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v7.json",
        "875da6c756647b528b3f4ce7c74bc8c4c384694d2478284df7c6971ce9be112a",
        8,
    ),
    (
        "lower_numeric_modifier17_packet",
        "RECEIPT",
        "docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v7.json",
        "c5d6d1a97d222b81b16d3796d19022d4396ffe32df6a8aabe2d8cb898568d319",
        8,
    ),
)
ALLOWED = {
    *(
        "docs/agents/evidence/" + name + ".json"
        for name in (
            "OTV2-20261001-item-capacity-promotion-v1",
            "OTV2-20261001-item-document-promotion-v1",
            "OTV2-20261001-item-physical-promotion-v1",
            "OTV2-20261001-item-stack-false-promotion-v1",
            "OTV2-20261002-item-elemental-magic-modifier-promotion-v1",
            "OTV2-20261002-item-hit-magic-promotion-v1",
            "OTV2-20261002-item-mantra-bond-promotion-v1",
            "OTV2-20261002-item-use-observation-promotion-v1",
            "OTV2-20261003-item-market-true-promotion-v2",
            "OTV2-20261003-item-movable-promotion-v2",
        )
    ),
    "crates/gameplay/content/src/content/item_name_promotion.rs",
    MARKER,
    "tools/content-migration/validate_world_project_v2_to_tree.py",
    "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json",
    "docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json",
    *(
        HERE + name + ".py"
        for name in (
            "check_stack_default_historical_context",
            "test_stack_default_historical_context",
            "lower_elemental_magic_modifier_packet",
            "lower_mantra_bond_modifier_packet",
        )
    ),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def refresh_pin(root, path, previous):
    raw = (root / current_path(path)).read_bytes()
    actual = digest(raw)
    if actual != previous and path not in ALLOWED:
        raise ValueError(f"unqualified input change: {path}")
    return actual, len(raw)


def refresh_list(root, pins, additional_owner_inputs=()):
    for pin in pins:
        actual, size = refresh_pin(root, pin["path"], pin["sha256"])
        pin["sha256"] = actual
        if "bytes" in pin:
            pin["bytes"] = size
    if any(pin["path"] == MARKER for pin in pins):
        for path in (
            "content/world/terrain/" + GROUND,
            "content/world/terrain/" + SUGAR_GROUND,
            HERE + "historical_world_owner_context.py",
            *additional_owner_inputs,
            "tools/content-schema/world-object-authoring/sugar_room.py",
            "tools/content-schema/world-object-authoring/samples/sugar-room-source-flags.json",
        ):
            raw = (root / path).read_bytes()
            pins.append({"path": path, "sha256": digest(raw), "bytes": len(raw)})


def refresh(
    root,
    recipe,
    additional_owner_inputs=(),
    base_head="4224c44239d0913e6dc14b6e30bcb18e1b248c43",
    reason="W15c owner footprint; original source records and closed scope unchanged",
    generator=HERE + Path(__file__).name,
    require_new_or_identical=False,
    regenerate_owned_successor=False,
):
    module, kind, source, expected, version = recipe
    raw = (root / source).read_bytes()
    if digest(raw) != expected:
        raise ValueError(f"frozen qualification changed: {source}")
    original_marker(root)  # Replay both qualified appearance-only owner additions.
    packet = json.loads(raw)
    for key in (
        "input_pins",
        "input_digests",
        "world_owner_inputs",
        "source_dependency_pins",
    ):
        pins = packet.get(key)
        if isinstance(pins, list):
            refresh_list(root, pins, additional_owner_inputs)
        elif isinstance(pins, dict):
            for path, previous in pins.items():
                pins[path] = refresh_pin(root, path, previous)[0]
            if MARKER in pins:
                for path in (
                    "content/world/terrain/" + SUGAR_GROUND,
                    HERE + "historical_world_owner_context.py",
                    "tools/content-schema/world-object-authoring/sugar_room.py",
                    "tools/content-schema/world-object-authoring/samples/sugar-room-source-flags.json",
                    *additional_owner_inputs,
                ):
                    pins[path] = digest((root / path).read_bytes())
    if "world_owner_baseline" in packet:
        refresh_list(
            root,
            packet["world_owner_baseline"]["input_digests"],
            additional_owner_inputs,
        )
    if "historical_source_proof_sha256" in packet:
        proof = root / re.sub(
            r"-v[0-9]+\.json$", f"-v{RECIPES[2][4]}.json", RECIPES[2][2]
        )
        packet["historical_source_proof_sha256"] = digest(proof.read_bytes())
    if "actual_published_parent" in packet:
        packet["actual_published_parent"] = base_head
    packet["input_footprint_refresh"] = {
        "base": base_head,
        "supersedes": source,
        "sha256": expected,
        "reason": reason,
        "generator": generator,
    }
    output = re.sub(r"-v[0-9]+\.json$", f"-v{version}.json", source)
    # Preserve the frozen source's serialization style, avoiding unrelated churn.
    if raw.count(b"\n") <= 1:
        encoded = (
            json.dumps(
                packet, sort_keys=True, ensure_ascii=False, separators=(",", ":")
            )
            + "\n"
        ).encode()
    else:
        encoded = (json.dumps(packet, indent=2, ensure_ascii=False) + "\n").encode()
    destination = root / output
    if (
        require_new_or_identical
        and destination.exists()
        and destination.read_bytes() != encoded
    ):
        # Only this producer's exact successor lineage may be recomputed. The
        # candidate above was rebuilt from the hash-checked frozen source, so
        # refreshed input pins never copy mutable successor source facts.
        existing = json.loads(destination.read_bytes())
        if (
            not regenerate_owned_successor
            or existing.get("input_footprint_refresh")
            != packet["input_footprint_refresh"]
        ):
            raise ValueError(f"successor already exists with different bytes: {output}")
    destination.write_bytes(encoded)
    compiler = root / HERE / (module + ".py")
    text = compiler.read_text()
    # Accept reruns from either original or refreshed constants, without changing checks.
    assignment = next(
        node
        for node in ast.parse(text).body
        if isinstance(node, ast.Assign)
        and isinstance(node.targets[0], ast.Name)
        and node.targets[0].id == kind
    )
    lines = text.splitlines(keepends=True)
    value = f'{kind} = "{output}"\n'
    if len(value.rstrip()) > 88 and len(output) + 6 <= 88:
        value = f'{kind} = (\n    "{output}"\n)\n'
    lines[assignment.lineno - 1 : assignment.end_lineno] = [value]
    text = "".join(lines)
    pattern = rf'({kind}_SHA = ")[0-9a-f]{{64}}(")'
    text, count = re.subn(pattern, rf"\g<1>{digest(encoded)}\2", text)
    if count != 1:
        raise ValueError(f"missing unique {kind}_SHA in {compiler}")
    compiler.write_text(text)
    print(output)


def refresh_consumer_pins(root):
    """Repin exact consumers only after their closed source producers rebuild."""
    path = root / "tools/content-migration/validate_world_project_v2_to_tree.py"
    text = path.read_text()
    for module, packet, code, expected_count in (
        (
            "lower_item_hit_magic_packet",
            "OTV2-20261002-item-hit-magic-promotion-v1.json",
            "HIT_MAGIC_PACKET_DIGEST",
            1,
        ),
        (
            "lower_item_use_observation_packet",
            "OTV2-20261002-item-use-observation-promotion-v1.json",
            "USE_OBSERVATION_PACKET_DIGEST",
            2,
        ),
    ):
        subprocess.run(
            [sys.executable, str(root / HERE / (module + ".py")), "--check"], check=True
        )
        actual = digest((root / "docs/agents/evidence" / packet).read_bytes())
        pattern = (
            r'(hashlib\.sha256\(raw\)\.hexdigest\(\) == ")[0-9a-f]{64}(", "'
            + code
            + '")'
        )
        text, count = re.subn(
            pattern, lambda match, value=actual: match[1] + value + match[2], text
        )
        if count != expected_count:
            raise ValueError(f"consumer pin shape changed: {code}")
    path.write_text(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", type=int, choices=range(len(RECIPES)), nargs="?")
    parser.add_argument("--consumer-pins", action="store_true")
    args = parser.parse_args()
    if args.consumer_pins:
        if args.target is not None:
            parser.error("--consumer-pins does not take a receipt target")
        refresh_consumer_pins(ROOT)
        return
    if args.target is None:
        parser.error("a receipt target or --consumer-pins is required")
    refresh(ROOT, RECIPES[args.target])


if __name__ == "__main__":
    main()
