#!/usr/bin/env python3
"""Validate the upstream JSON report. Logs are never used as service discovery."""
import json
import sys
from pathlib import Path
from urllib.parse import urlsplit

CLIENTS = {
    "native-evidence", "runtime-status", "scope-assignment", "bootstrap-intents",
    "lcfa", "character-commands", "character-services", "restore-notice", "store",
    "premium", "loyalty-snapshot", "loyalty-stream", "coins", "sanctions",
    "security-revocations", "public-projections",
}


def validate(report, work):
    if report["status"] != "ready" or not report["database"].startswith("portal_e2e_"):
        raise ValueError("not_ready")
    root = (work / "portal").resolve()
    if Path(report["outputDirectory"]).resolve() != root:
        raise ValueError("wrong_output_directory")
    for name, scheme in [("public", "http"), ("internal", "https"), ("gateway", "http"), ("nativeAdmission", "http")]:
        url = urlsplit(report["baseUrls"][name])
        if url.scheme != scheme or url.hostname != "127.0.0.1" or not url.port or url.username or url.password:
            raise ValueError("nonlocal_listener")
    if report["internalServerName"] != "localhost" or set(report["clients"]) != CLIENTS:
        raise ValueError("missing_client_identity")
    paths = [report["caFile"]]
    identities = set()
    for client in report["clients"].values():
        identities.add(client["san"])
        paths += [client["cert"], client["key"]]
    if len(identities) != 16:
        raise ValueError("overlapping_client_identities")
    paths += [report["logs"][role] for role in ["migrate", "serve", "worker", "gateway"]]
    for value in paths:
        path = Path(value).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            raise ValueError("missing_harness_artifact")
    for name in ["accountA", "accountB", "world", "nativeClient", "freshTrust", "recoveryTrust"]:
        if "error" in report["seed"][name]:
            raise ValueError("seed_failed")
    for item in report["pending"]:
        if item.get("entity") not in {"channel", "route"} or item.get("error") != "pending":
            raise ValueError("unexpected_pending_seed")
    if report["pending"]:
        (work / "route-pending").touch(mode=0o600)


def main():
    try:
        validate(json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]))
    except (OSError, ValueError, KeyError, TypeError):
        sys.exit(1)


if __name__ == "__main__":
    main()
