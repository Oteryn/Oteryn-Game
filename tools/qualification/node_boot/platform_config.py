"""Install the Platform harness identities into an unprivileged node-boot stack."""

import json
from pathlib import Path
from shutil import copyfile
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "game_platform"))
from composition import node_documents


def install(source, destination, mode):
    copyfile(source, destination)
    destination.chmod(mode)


def configure(work, node, base):
    report = json.loads((work / "portal-report.json").read_text())
    clients = report["clients"]
    for role in ("runtime-status", "lcfa", "bootstrap-intents"):
        install(clients[role]["cert"], base / "node/secrets" / f"{role}.crt", 0o600)
        install(clients[role]["key"], base / "node/secrets" / f"{role}.key", 0o600)
        install(clients[role]["cert"], base / "ops" / f"{role}.crt", 0o644)
    assignment = clients["scope-assignment"]
    install(assignment["cert"], base / "ops/scope-assignment.crt", 0o644)
    install(assignment["key"], base / "ops/scope-assignment.key", 0o600)
    install(report["caFile"], base / "ops/platform-ca.pem", 0o644)

    # The same stack renders configs for several node incarnations. Preserve
    # the epoch fence across them, and refuse nonregular files and symlinks.
    fence = base / "run/lcfa-epoch-fence"
    marker = work / "lcfa-epoch-fence"
    if marker.exists():
        if fence.is_symlink() or not fence.is_file():
            raise ValueError("invalid existing epoch fence")
    else:
        with fence.open("x") as output:
            output.write("0\n")
        fence.chmod(0o600)
        marker.touch()

    text, config = node_documents(report, node.read_text(), base)
    node.write_text(text)
    report_config = base / "ops/assignment-report.toml"
    report_config.write_text(config)
    report_config.chmod(0o644)


if __name__ == "__main__":
    try:
        configure(*map(Path, sys.argv[1:]))
    except (OSError, KeyError, ValueError):
        sys.exit("node_boot_platform_configuration_failed")
