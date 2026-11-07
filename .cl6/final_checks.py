"""Final validation after the filtered PostgreSQL qualification completes."""
import subprocess
import sys

cargo = "/home/mole/oteryn-cache/codex-bin/cargo"
packages = ["oteryn-client", "oteryn-client-ui", "oteryn-client-runtime", "oteryn-session", "oteryn-gameplay-dispatch", "oteryn-synthetic-client-harness", "oteryn-protocol-oteryn", "oteryn-gameplay-durability"]
selection = [argument for package in packages for argument in ("-p", package)]


def run(label, *command):
    subprocess.run([sys.executable, ".cl6/run.py", label, *command], check=True)


run("fmt-final", cargo, "fmt", "--all", "--check")
run("check-final", cargo, "check", "--locked", "--all-targets", *selection)
run("clippy-final", cargo, "clippy", "--locked", "--all-targets", "--all-features", *selection, "--", "-D", "warnings")
subprocess.run([sys.executable, ".cl6/test_plan.py", "client"], check=True)
run("protocol-generated", cargo, "test", "--locked", "-p", "oteryn-protocol-oteryn", "--lib", "--", "--exact", "--nocapture",
    "registered_capabilities::tests::caps_split_1_golden_preserves_every_current_capability_fact",
    "registered_capabilities::tests::capability_tables_are_all_fragment_facts",
    "capability_tables::tests::the_gated_commands_and_domains_are_the_registry_ones",
    "tests::module_registration_block_is_sorted_and_complete",
    "contract_registries::tests::protocol_registry_is_its_base_and_fragments")
run("session-closure", cargo, "test", "--locked", "-p", "oteryn-session", "--lib", "tests::the_advertised_set_is_closed_under_the_registry_requires", "--", "--exact", "--nocapture")
run("architecture", cargo, "run", "--locked", "-p", "oteryn-architecture-check", "--", "workspace", ".")
run("pin-refresh", cargo, "run", "--locked", "-p", "oteryn-world-bundle-compiler", "--", "pin-refresh", ".")
run("pin-check", cargo, "run", "--locked", "-p", "oteryn-world-bundle-compiler", "--", "pin-check", ".", ".cl6/pin-check")
run("diff-check", "git", "diff", "--check", "origin/main")
