"""Run only exact unit and PostgreSQL cases selected for the CL6/CL7 package."""
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parent
cargo = "/home/mole/oteryn-cache/codex-bin/cargo"


def run(label, *arguments):
    subprocess.run([sys.executable, str(root / "run.py"), label, cargo, *arguments], check=True)


def names_in(path):
    return re.findall(r"#\[test\]\s+fn (\w+)", Path(path).read_text())


client_names = []
for feature in ("0006-world-spatial-entities", "0019-world-light", "0082-creature-push", "0179-hotkey-equip", "0195-weapon-proficiency"):
    for path in Path(f"apps/client/src/features/{feature}/tests").glob("*.rs"):
        prefix = "scene::live_entities::tests" if path.name == "scene.rs" else "play::tests::feature_" + feature.replace("-", "_")
        client_names.extend(prefix + "::" + name for name in names_in(path))
client_names.append("play::tests::feature_local_status_flows::cl6_mechanics_status_native_queue_refusal_replay_logout")
plans = {
    "client": ("oteryn-client", "--lib", client_names),
    "ui": ("oteryn-client-ui", "--lib", ["cl6_all_owner_projections_reach_native_text_and_refusals_retain_confirmation"]),
    "protocol": ("oteryn-protocol-oteryn", "--lib", [module + "::tests::" + name for module in ("mechanics_status", "weapon_proficiency") for name in names_in(f"crates/protocol-oteryn/src/{module}.rs")]),
    "session": ("oteryn-session", "--lib", [
        "the_advertised_set_carries_chat_and_stays_closed",
        "derived_gated_routes_match_previous_table",
        "appearance_result_first_and_observation_first_converge_through_session_dispatch",
        "creature_push_is_advertised_and_routed_with_its_prerequisites",
        "proficiency_browse_is_advertised_and_routed_with_item_identity",
        "hotkey_equip_carried_is_advertised_with_its_command_owner",
    ]),
    "synthetic": ("oteryn-synthetic-client-harness", "--bin", names_in("tools/synthetic-client-harness/src/native_map.rs")),
    "pg": ("oteryn-gameplay-dispatch", "--lib", [
        "cl6_status_postgres_real_dispatch_existing_owners_and_fences",
        "party_effects_real_dispatch_postgres_membership_and_shared_eligibility",
        *names_in("crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/condition_potions.rs"),
        "magic_shield_potion_postgres_use_capacity_expiry_requirements_replay",
        "antidote_potion_postgres_use_cures_poison_flask_cooldown_replay",
        "w22b_wealth_duplex_ignores_wall_clock_time_spent_offline",
        "w22b_wealth_duplex_ends_after_one_online_hour",
        "proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence",
        "creature_push_real_dispatch_moves_and_refuses_on_postgres",
    ]),
}
for plan in sys.argv[1:] or list(plans):
    package, target, wanted = plans[plan]
    selector = [target] if target == "--lib" else [target, package]
    features = ["--all-features"] if plan == "pg" else []
    prefix = ["test", "--locked", "-p", package, *selector, *features]
    if not (root / f"{plan}-list.log").exists():
        run(f"{plan}-list", *prefix, "--", "--list")
    available = re.findall(r"^(\S+): test$", (root / f"{plan}-list.log").read_text(), re.M)
    exact = []
    for name in wanted:
        matches = [test for test in available if test == name or ("::" not in name and test.rsplit("::", 1)[-1] == name)]
        if len(matches) != 1:
            raise RuntimeError(f"expected one exact test for {name}, got {matches}")
        exact.extend(matches)
    if plan == "pg":
        # Each invocation stays within its own mechanic and has visible execution markers.
        for name in exact:
            label = "pg-" + name.rsplit("::", 1)[-1]
            run(label, *prefix, name, "--", "--exact", "--nocapture", "--test-threads=1")
            log = (root / f"{label}.log").read_text()
            elapsed = re.search(r"test result: ok\. 1 passed;.*finished in ([\d.]+)s", log)
            marker = re.search(r"(?:POSTGRES_|PG_MARKER|CL6_TIMER)", log)
            needs_marker = "creature_push_real_dispatch" not in name
            if "is not configured" in log or not elapsed or float(elapsed[1]) <= 0 or (needs_marker and not marker):
                raise RuntimeError(f"PostgreSQL execution not proven for {name}")
    else:
        run(f"{plan}-tests", *prefix, "--", "--exact", "--nocapture", *exact)
        log = (root / f"{plan}-tests.log").read_text()
        if f"test result: ok. {len(exact)} passed;" not in log:
            raise RuntimeError(f"expected {len(exact)} passing exact tests in {plan}")
