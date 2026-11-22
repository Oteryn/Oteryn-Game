#!/usr/bin/env python3
"""Run one fast-test archive/shard and report evidence without saving a ledger."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


# A newly updated PR workflow can qualify an older exact head. Its helper is
# then checked out from the immutable workflow SHA in a separate control tree.
ROOT = Path(os.environ.get("GITHUB_WORKSPACE", Path(__file__).resolve().parents[2])).resolve()
ROLES = {"all", "bins", "lib", *(f"{kind}-{n}" for kind in ("lib", "dispatch") for n in range(1, 4))}
PRIORITIES = '''[profile.rust-fast.junit]
path = "junit.xml"

[[profile.rust-fast.overrides]]
filter = 'test(/attack_chase_budget_tests::/) | test(/tutorial_current_world_compiles_twice/) | test(/compiler_writes_only_v7_and_v4_to_v6/)'
priority = 100

[[profile.rust-fast.overrides]]
filter = 'test(/equipment_effects_owner::tests::/) | test(/typed_item_codec_tests::bed_set_/) | test(/summer_dialogue_tests::/) | test(/npc_promotion::tests::/) | test(/eqmod2_content_forge_tiers/) | test(/item_look::tests::reading_map_/) | test(/imbuement_allowance_of_activated_native_content/) | test(/writable_profiles_survive_native_artifact/) | test(/bundle_spawn_owner_tests::/) | test(/the_chase_search_drain_gives_the_same_routes/) | test(/active_content_mints_every_reward_bag_output/) | test(/prey_catalogue_draws_only_wire_encodable/)'
priority = 50
'''


def packages(value):
    if (not isinstance(value, list)
            or any(not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", name) for name in value)
            or len(value) != len(set(value))):
        raise ValueError("invalid fast-test package selection")
    return set(value)


def context():
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if not re.fullmatch(r"[a-f0-9]{40}", head) or head != os.environ["EXPECTED_SHA"]:
        raise ValueError("fast-test checkout differs from the planned head")
    base = os.environ["BASE_SHA"]
    if not re.fullmatch(r"[a-f0-9]{40}", base):
        raise ValueError("invalid fast-test base SHA")
    role, shard, slug = (os.environ[name] for name in ("ARCHIVE", "SHARD", "SLUG"))
    if role not in ROLES or not re.fullmatch(r"[1-5]/5", shard) or slug != f"tests-{shard[0]}-5":
        raise ValueError("invalid fast-test archive or shard")
    selection = json.loads(os.environ["RUST_SELECTION"])
    selected = packages(selection["packages"])
    if selection["mode"] not in ("workspace", "packages") or not selected:
        raise ValueError("fast-test archive requires selected packages")
    planned = packages(json.loads(os.environ["TEST_SHARDS"])[slug])
    if not planned <= selected:
        raise ValueError("unit-test shard selection disagrees with selected packages")
    image = os.environ.get("ImageVersion", "unknown")
    platform = f'{os.environ["RUNNER_OS"]}-{os.environ["RUNNER_ARCH"]}'
    build_image, build_platform = os.environ["BUILD_IMAGE"], os.environ["BUILD_PLATFORM"]
    planner = os.environ["PLAN_IMAGE"], os.environ["PLAN_PLATFORM"]
    # Both the runner and builder are part of the planned package ledger key.
    if (image, platform) != planner or (build_image, build_platform) != planner or image in ("", "unknown"):
        planned = selected
        print("::notice::Runner identity differs from the ledger plan; testing every selected package")
    run, attempt = os.environ["GITHUB_RUN_ID"], os.environ["GITHUB_RUN_ATTEMPT"]
    if not all(re.fullmatch(r"[1-9][0-9]*", value) for value in (run, attempt)):
        raise ValueError("invalid fast-test workflow identity")
    return {
        "head": head, "base_sha": base, "role": role, "shard": shard, "slug": slug,
        "run_id": run, "run_attempt": attempt,
        "image": image, "platform": platform, "build_image": build_image, "build_platform": build_platform,
        "pending_packages": sorted(planned), "exit_code": None, "completed": False,
    }


def write_result(directory, result):
    directory.mkdir(parents=True, exist_ok=True)
    temporary = directory / "result.json.tmp"
    temporary.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(directory / "result.json")


def plan(directory):
    result = context()
    hit = not result["pending_packages"]
    if hit:
        result.update(exit_code=0, completed=True)
    (directory / "junit.xml").unlink(missing_ok=True)
    write_result(directory, result)
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"hit={str(hit).lower()}\n")


def run(directory, artifact, extract):
    result = context()
    stored = json.loads((directory / "result.json").read_text(encoding="utf-8"))
    if any(stored[key] != value for key, value in result.items() if key not in ("exit_code", "completed")):
        raise ValueError("fast-test inputs changed after package selection")
    if not result["pending_packages"]:
        return 0
    write_result(directory, result)  # A killed process leaves incomplete evidence.
    for filename, expected in (("runner-image", result["build_image"]), ("runner-platform", result["build_platform"])):
        if (artifact / filename).read_text(encoding="utf-8").strip() != expected:
            raise ValueError("fast-test archive builder identity does not match its job")
    nextest = artifact / "cargo-nextest"
    if not nextest.is_file() or not (artifact / "unit-tests.tar.zst").is_file():
        raise ValueError("fast-test archive executable or test bundle is missing")
    nextest.chmod(0o755)
    expression = "|".join(f"package(={name})" for name in result["pending_packages"])
    if re.fullmatch(r"(?:lib|dispatch)-[23]", result["role"]):
        expression = f"({expression}) & test(gameplay_transport::monk_save::postgres_tests::)"
    extract.mkdir(parents=True, exist_ok=True)
    configuration = extract.with_suffix(".toml")
    configuration.write_text(PRIORITIES, encoding="utf-8")
    junit = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "nextest/rust-fast/junit.xml"
    junit.unlink(missing_ok=True)
    (directory / "junit.xml").unlink(missing_ok=True)
    code = subprocess.run([
        "cargo", "+1.100.0", "nextest", "run", "--archive-file", str(artifact / "unit-tests.tar.zst"),
        "--workspace-remap", ".", "--extract-to", str(extract),
        "--config-file", str(configuration), "--profile", "rust-fast", "-E", expression,
        "--partition", f'hash:{result["shard"]}', "--no-tests=pass", "--no-fail-fast",
    ], cwd=ROOT, stdin=subprocess.DEVNULL, check=False).returncode
    if junit.is_file() and junit.stat().st_size:
        shutil.move(junit, directory / "junit.xml")
    result.update(exit_code=code, completed=code in (0, 100))
    write_result(directory, result)
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("plan", "run"):
        child = commands.add_parser(command)
        child.add_argument("--out", type=Path, required=True)
        if command == "run":
            child.add_argument("--artifact", type=Path, required=True)
            child.add_argument("--extract", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "plan":
            plan(args.out)
            return 0
        return run(args.out, args.artifact, args.extract)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        parser.exit(1, f"fast-test archive: {error}\n")


if __name__ == "__main__":
    raise SystemExit(main())
