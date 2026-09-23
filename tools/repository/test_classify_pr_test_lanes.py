#!/usr/bin/env python3
"""Behavioral fixtures for trusted-base risk classification and gate fan-in."""
from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
MODULE = Path(__file__).with_name("classify_pr_test_lanes.py")
ROUTING_CONTRACT = Path(__file__).with_name("validate_pr_routing_contract.py")


def test_aggregate():
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text()
    block = gate.split("  validate:\n", 1)[1].split("  game_gate:\n", 1)[0]
    script = textwrap.dedent(block.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    mandatory = ("SCOPE", "LANES", "GOVERNANCE", "DEPENDENCY_REVIEW", "CODEQL", "ROUTING_CONTRACT")
    rust = ("RUST_POLICY", "RUST_LINUX", "RUST_SUPPLY_CHAIN")
    conditional = ("RUST_WINDOWS", "ATLAS_FULLWORLD")
    env = dict.fromkeys(mandatory + rust + conditional, "success")
    env.update(RUST_REQUIRED="true", WINDOWS_REQUIRED="true", ATLAS_FULLWORLD_REQUIRED="true")

    def accepts(changes):
        with patch.dict(os.environ, dict(env, **changes)), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate:aggregate", "exec"), {})
                return True
            except SystemExit:
                return False

    assert accepts({})
    assert accepts({"WINDOWS_REQUIRED": "false", "RUST_WINDOWS": "skipped"}), "proven server-only lane cannot omit Windows"
    assert accepts({"ATLAS_FULLWORLD_REQUIRED": "false", "ATLAS_FULLWORLD": "skipped"})
    assert accepts(dict.fromkeys(rust + conditional, "skipped") | {
        "RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "false", "ATLAS_FULLWORLD_REQUIRED": "false"
    })
    assert accepts(dict.fromkeys(rust + ("RUST_WINDOWS",), "skipped") | {
        "RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "false", "ATLAS_FULLWORLD_REQUIRED": "true",
        "ATLAS_FULLWORLD": "success",
    }), "Atlas-only lane must not require Rust or Windows"
    for name in mandatory + rust + conditional:
        for value in ("failure", "cancelled", "skipped", ""):
            assert not accepts({name: value}), (name, value)
    for name in ("RUST_REQUIRED", "WINDOWS_REQUIRED", "ATLAS_FULLWORLD_REQUIRED"):
        for value in ("", "TRUE", "unknown", "0"):
            assert not accepts({name: value}), (name, value)
    assert not accepts({"RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "true"})
    print("Risk aggregate PASS: routing contract plus full/server/docs/Atlas controls, every selected failure and invalid output")


def fixture():
    roots = {
        "oteryn-game-server": "apps/game-server",
        "oteryn-client": "apps/client",
        "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
        "oteryn-simulation-determinism": "crates/simulation-determinism",
        "oteryn-foundation": "crates/foundation",
    }

    edges = {
        "oteryn-game-server": ["oteryn-foundation", "oteryn-simulation-determinism"],
        "oteryn-client": ["oteryn-foundation"],
        "oteryn-synthetic-client-harness": ["oteryn-foundation"],
    }
    return {
        "workspace_root": "/repo",
        "workspace_members": list(roots),
        "packages": [
            {"id": name, "name": name, "manifest_path": f"/repo/{path}/Cargo.toml",
             "dependencies": [{"name": dep, "path": f"/repo/{roots[dep]}", "kind": None, "target": None, "optional": False} for dep in edges.get(name, [])]}
            for name, path in roots.items()
        ],
    }


def test_snapshot_and_fallbacks(module):
    rows = [b"100644 blob " + b"a" * 40 + b"\tapps/client/src/lib.rs",
            b"100644 blob " + b"b" * 40 + b"\tapps/game-server/src/lib.rs"]
    with patch.object(module.subprocess, "check_output", return_value=b"\0".join(rows) + b"\0"):
        digest = module.input_digest(fixture())
    for index, changes_digest in ((0, True), (1, False)):
        changed = rows.copy()
        changed[index] = changed[index].replace(b"blob ", b"blob c", 1)
        with patch.object(module.subprocess, "check_output", return_value=b"\0".join(changed) + b"\0"):
            assert (module.input_digest(fixture()) != digest) is changes_digest
    with patch.object(module.subprocess, "check_output", return_value=b"120000 blob " + b"a" * 40 + b"\tlink\0"):
        try:
            module.input_digest(fixture())
        except ValueError:
            pass
        else:
            raise AssertionError("symlink input accepted")

    external = set(module.external_local_dependency_roots())
    expected_external = {
        "vendor/sqlx-core-0.9.0",
        "vendor/sqlx-postgres-0.9.0",
        "vendor/tokio-1.53.1",
    }
    assert expected_external <= external, external
    for root in expected_external:
        assert module.audited_input_path(fixture(), f"{root}/src/lib.rs")

    vendor_rows = rows + [
        b"100644 blob " + b"d" * 40 + b"\tvendor/tokio-1.53.1/src/lib.rs"
    ]
    with patch.object(module.subprocess, "check_output", return_value=b"\0".join(vendor_rows) + b"\0"):
        vendor_digest = module.input_digest(fixture())
    changed_vendor_rows = vendor_rows.copy()
    changed_vendor_rows[-1] = changed_vendor_rows[-1].replace(b"d" * 40, b"e" * 40)
    with patch.object(module.subprocess, "check_output", return_value=b"\0".join(changed_vendor_rows) + b"\0"):
        assert module.input_digest(fixture()) != vendor_digest
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text()
    assert "  lanes:\n" in gate, "trusted-base lane job is absent"
    block = gate.split("  lanes:\n", 1)[1].split("  governance:\n", 1)[0]
    script = textwrap.dedent(
        block.split("        run: |\n", 1)[1].split("\n      - name:", 1)[0]
    )
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "output"
        env = dict(os.environ, GITHUB_OUTPUT=str(output), RUNNER_TEMP=directory)
        result = subprocess.run(["bash", "-c", script], cwd=directory, env=env, capture_output=True, text=True)
        assert result.returncode == 0 and output.read_text() == (
            "rust=true\nwindows=true\natlas_fullworld=true\n"
            "surface=unknown\nreason=protected-base-classifier-missing\n"
            "routing_health=degraded\n"
        ), result
        output.unlink()
        invalid = Path(directory) / "metadata.json"
        invalid.write_text("{}")
        result = subprocess.run([sys.executable, str(MODULE), str(invalid)], env=env, capture_output=True, text=True)
        assert result.returncode == 0 and output.read_text() == (
            "rust=true\nwindows=true\natlas_fullworld=true\n"
            "surface=unknown\nreason=classifier-or-metadata-failure\n"
            "routing_health=degraded\n"
        ), result
    print("Risk snapshot and CLI fallbacks PASS: consumer changes, server isolation, symlinks, missing base classifier, malformed metadata")


def test_reviewed_document_consumers(module):
    # Independent reviewed input contract from the protected tree documented in
    # OTV2-20260905-doc-consumer-snapshot. Do not derive these fixtures from the
    # classifier constants or current HEAD: later consumer edits must remain
    # legitimate FULL inputs, not make repository regression validation fail.
    stale_reviewed_nonserver = (
        "9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8",
        "669052b19bf8d067d949623ba566e087b43def40b2661f8c3feb16f3d62de180",
        "2dbc1273b54b4f63653bc6c5a92ee10a1c095e3bd05dac232751efc4d358fa9d",
        "ba36d790f37791495584f1f1d2a51f382a11d629085f786007cc88d04cdff5ac",
        "c04bf8e0e010366170d76f23baa34b0abb2f69a9114157b0a6c6ebb880294594",
        "962dfe6c3c9fbe102a08b1040e3880589b6ee4cda52a4910feae3103225099fe",
        "6e6f7a9dafe5020cbfda968d49b97c47072c7ee471668d3d9e0cdc491a4af2f4",
        "118afae45f3c8fd7692e2e61ffd286e4efc3d4bfc8290304b999f0bbc3ca29ba",
    )
    reviewed_nonserver = "220e5dbe065665815eefb8219ac9d4609f5d06afbe15315d7a6d2b200839f3a6"
    stale_reviewed_docs = (
        "f8eed774249df64a5a64612b4a169a73bac093a7bcbfb21e59ea0e06dd2ddc26",
        "742350c55587ab94d652e27a4196308f350afaf5140ed3633d33d5d165e807b6",
        "051473d37842a816e9378c9769fd28cf9c7ddb49fb5483d02b3841d73c2cd403",
        "dc9381615fb7b1f7b06533ac6f6ffebb6d274b8d799894fe54882bf69f638472",
    )
    reviewed_docs = "4b37d0e2e6c70161a29f3def3891a17a9c3e48f4048b883fa457b66b20d654b3"
    for path in ("README.md", "docs/reference/finished.md"):
        result = module.classify([dict(filename=path, status="modified")], 1,
                                 fixture(), reviewed_nonserver, docs_digest=reviewed_docs,
                                 candidate_modes_verified=True)
        assert result["rust"] is False and result["windows"] is False, result
        for stale_input in stale_reviewed_nonserver:
            stale_nonserver = module.classify([dict(filename=path, status="modified")], 1,
                                              fixture(), stale_input, docs_digest=reviewed_docs,
                                              candidate_modes_verified=True)
            assert stale_nonserver["rust"] is True and stale_nonserver["windows"] is True, stale_nonserver
            assert stale_nonserver["reason"] == "unreviewed-document-consumer-inputs", stale_nonserver
        for stale_docs in stale_reviewed_docs:
            stale = module.classify([dict(filename=path, status="modified")], 1,
                                    fixture(), reviewed_nonserver, docs_digest=stale_docs,
                                    candidate_modes_verified=True)
            assert stale["rust"] is True and stale["windows"] is True, stale
            assert stale["reason"] == "unreviewed-document-consumer-inputs", stale

    # Exercise real tree hashing and classification together. A new or changed
    # consumer can add a document read without changing any Cargo edge.
    rows = [b"100644 blob " + b"a" * 40 + b"\tapps/client/src/lib.rs",
            b"100644 blob " + b"b" * 40 + b"\tapps/game-server/src/lib.rs"]
    def digests(records):
        with patch.object(module.subprocess, "check_output", return_value=b"\0".join(records) + b"\0"):
            return module.input_digest(fixture()), module.input_digest(fixture(), include_server=True)
    baseline_nonserver, baseline_docs = digests(rows)
    with patch.object(module, "AUDITED_INPUT_SHA256", baseline_nonserver), \
            patch.object(module, "AUDITED_DOC_INPUT_SHA256", baseline_docs):
        result = module.classify([dict(filename="README.md", status="modified")], 1,
                                 fixture(), baseline_nonserver, docs_digest=baseline_docs,
                                 candidate_modes_verified=True)
        assert result["rust"] is False and result["windows"] is False, result
        for root, index in (("apps/client", 0), ("apps/game-server", 1)):
            for added in (False, True):
                changed = rows.copy()
                if added:
                    changed.append(b"100644 blob " + b"c" * 40 + b"\t" + root.encode() + b"/src/document_reader.rs")
                else:
                    changed[index] = changed[index].replace(b"a" * 40 if index == 0 else b"b" * 40, b"c" * 40)
                nonserver, docs = digests(changed)
                result = module.classify([dict(filename="README.md", status="modified")], 1,
                                         fixture(), nonserver, docs_digest=docs,
                                         candidate_modes_verified=True)
                assert result["rust"] is True and result["windows"] is True, (root, added, result)
    print("Reviewed document consumers PASS: admitted docs and changed/new server/nonserver consumers FULL")


def test_trusted_job_mutations():
    spec = importlib.util.spec_from_file_location("risk_core", MODULE.with_name("validate_repository_policy_core.py"))
    assert spec is not None and spec.loader is not None
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    path = ROOT / ".github/workflows/merge-gate.yml"
    original = path.read_text()
    block = core.indented_yaml_mapping_block(original, "lanes", 2)
    assert block is not None
    read_text = Path.read_text
    mutations = [block.replace("needs.scope.outputs.base_sha", "needs.scope.outputs.target_sha"),
                 block.replace("  lanes:\n", "  lanes:\n    if: false\n"),
                 block.replace("  lanes:\n", "  lanes:\n    continue-on-error: true\n"),
                 block.replace("rust=true", "rust=false"),
                 block.replace("windows=true", "windows=false"),
                 block.replace("atlas_fullworld=true", "atlas_fullworld=false"),
                 block.replace("          python -I", "          exit 0\n          python -I"),
                 block.replace(
                     '        run: python -I tools/repository/validate_pr_routing_contract.py --protected-main "$RUNNER_TEMP/risk-metadata.json"\n',
                     "        run: python -c 'pass'\n",
                 )]
    for changed in mutations:
        assert changed != block
        mutated = original.replace(block, changed, 1)
        def read(file, *args, **kwargs):
            return mutated if file == path else read_text(file, *args, **kwargs)
        with patch.object(Path, "read_text", read), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            assert core.main() != 0, "trusted-base lane mutation passed policy"
    routing = core.indented_yaml_mapping_block(original, "routing_contract", 2)
    assert routing is not None
    routing_mutations = [
        routing.replace("  routing_contract:\n", "  routing_contract:\n    if: false\n"),
        routing.replace("  routing_contract:\n", "  routing_contract:\n    continue-on-error: true\n"),
        routing.replace("          ref: ${{ needs.scope.outputs.target_sha }}\n", "          ref: ${{ needs.scope.outputs.base_sha }}\n"),
        routing.replace("          fetch-depth: 0\n", "          fetch-depth: 1\n"),
        routing.replace(
            '        run: python -I tools/repository/validate_pr_routing_contract.py "$RUNNER_TEMP/routing-contract-metadata.json"\n',
            "        run: python -c 'pass'\n",
        ),
        routing.replace(
            '          git diff --check "$EXPECTED_BASE" "$EXPECTED_HEAD"\n',
            "          true\n",
        ),
    ]
    for changed in routing_mutations:
        assert changed != routing
        mutated = original.replace(routing, changed, 1)
        def read(file, *args, **kwargs):
            return mutated if file == path else read_text(file, *args, **kwargs)
        with patch.object(Path, "read_text", read), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            assert core.main() != 0, "routing-contract mutation passed policy"

    validator_path = ROOT / "tools/repository/validate_pr_routing_contract.py"
    validator_original = read_text(validator_path)
    validator_changed = validator_original.replace("        return 1\n", "        return 0\n", 1)
    assert validator_changed != validator_original
    def read_validator(file, *args, **kwargs):
        return validator_changed if file == validator_path else read_text(file, *args, **kwargs)
    with patch.object(Path, "read_text", read_validator), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        assert core.main() != 0, "mutated routing-contract validator passed policy"

    atlas = core.indented_yaml_mapping_block(original, "atlas_fullworld", 2)
    assert atlas is not None
    atlas_mutations = [
        atlas.replace("    if: needs.lanes.outputs.atlas_fullworld == 'true'\n", "    if: false\n"),
        atlas.replace("          ref: ${{ needs.scope.outputs.target_sha }}\n", "          ref: ${{ needs.scope.outputs.base_sha }}\n"),
        atlas.replace("python -S tools/game-atlas-fullworld-source/self_test.py", "python -S -c 'pass'"),
        atlas.replace("  atlas_fullworld:\n", "  atlas_fullworld:\n    continue-on-error: true\n"),
    ]
    for changed in atlas_mutations:
        assert changed != atlas
        mutated = original.replace(atlas, changed, 1)
        def read(file, *args, **kwargs):
            return mutated if file == path else read_text(file, *args, **kwargs)
        with patch.object(Path, "read_text", read), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            assert core.main() != 0, "Atlas fullworld gate mutation passed policy"
    print("Trusted-base job mutation family PASS: lane selection, routing contract and Atlas exact-head evidence remain fail closed")


def test_candidate_modes(module):
    assert callable(getattr(module, "candidate_modes_safe", None)), "candidate tree modes are not verified before reduced-lane selection"
    sha = "a" * 40
    for mode, safe in ((b"100644", True), (b"100755", True), (b"120000", False), (b"160000", False)):
        row = mode + b" blob " + b"b" * 40 + b"\tdocs/link.md\0"
        with patch.object(module.subprocess, "check_output", return_value=row):
            assert module.candidate_modes_safe(sha) is safe, mode
    assert module.candidate_modes_safe("not-an-exact-sha") is False
    for path in ("docs/link.md", "apps/game-server/src/link.rs"):
        result = module.classify([dict(filename=path, status="added")], 1, fixture(), module.AUDITED_INPUT_SHA256,
                                 docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=False)
        assert result["rust"] and result["windows"], "missing candidate modes permitted reduced lanes"
    print("Candidate mode family PASS: executable/regular controls, symlink/gitlink rejection and missing evidence FULL")


def test_atlas_fullworld_surface(module):
    producer = "tools/game-atlas-fullworld-source/producer.py"
    self_test = "tools/game-atlas-fullworld-source/self_test.py"
    server = "apps/game-server/src/content/reference_playable.rs"
    client = "apps/client/src/lib.rs"

    for path in (producer, self_test):
        result = module.classify(
            [dict(filename=path, status="modified")], 1, fixture(), module.AUDITED_INPUT_SHA256,
            docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
        )
        assert result == {
            "rust": False, "windows": False, "surface": "atlas-fullworld",
            "reason": "audited-atlas-fullworld-source",
        }, result
        assert module.atlas_fullworld_required([dict(filename=path, status="modified")], 1) is True

    result = module.classify(
        [dict(filename=server, status="modified"), dict(filename=producer, status="modified")],
        2, fixture(), module.AUDITED_INPUT_SHA256,
        docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
    )
    assert result["rust"] is True and result["windows"] is False, result
    assert result["surface"] == "server" and result["reason"].endswith("-plus-atlas-fullworld"), result

    result = module.classify(
        [dict(filename=client, status="modified"), dict(filename=producer, status="modified")],
        2, fixture(), module.AUDITED_INPUT_SHA256,
        docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
    )
    assert result["rust"] is True and result["windows"] is True, result

    for path in ("tools/game-atlas-fullworld-source/animated.py",
                 "tools/game-atlas-fullworld-source/README.md"):
        result = module.classify(
            [dict(filename=path, status="modified")], 1, fixture(), module.AUDITED_INPUT_SHA256,
            docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
        )
        assert result["rust"] is True and result["windows"] is True, (path, result)
        assert result["reason"] == "explicit-atlas-non-cargo-full", (path, result)

    result = module.classify(
        [dict(filename="tools/unreviewed/helper.py", status="modified")],
        1, fixture(), module.AUDITED_INPUT_SHA256,
        docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
    )
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "unmodelled-input", result

    renamed = [{"filename": "tools/unreviewed/producer.py", "status": "renamed", "previous_filename": producer}]
    result = module.classify(
        renamed, 1, fixture(), module.AUDITED_INPUT_SHA256,
        docs_digest=module.AUDITED_DOC_INPUT_SHA256, candidate_modes_verified=True,
    )
    assert result["rust"] is True and result["windows"] is True, result
    assert module.atlas_fullworld_required(renamed, 1) is True
    assert module.atlas_fullworld_required([dict(filename=server, status="modified")], 1) is False
    assert module.atlas_fullworld_required([], 0) is True
    print("Atlas fullworld surface PASS: bounded paths reduce Windows only with dedicated lane; siblings/renames stay fail closed")


def test_atlas_workflow_dispositions(module):
    triggers = set()
    for workflow in sorted((ROOT / ".github/workflows").glob("game-atlas-*.yml")):
        for line in workflow.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped.startswith("- "):
                continue
            value = stripped[2:].strip().strip("'\"")
            if value.startswith("tools/game-atlas-"):
                triggers.add(value)

    assert triggers, "no specialized Atlas tool triggers discovered"
    missing = sorted(path for path in triggers if module.atlas_path_disposition(path) is None)
    assert not missing, f"Atlas workflow trigger lacks classifier disposition: {missing}"
    for path in module.ATLAS_FULLWORLD_PATHS:
        assert path in triggers, f"dedicated Atlas path is not covered by a specialized workflow: {path}"

    assert module.atlas_path_disposition("tools/game-atlas-creatures/export.py") == "full"
    assert module.atlas_path_disposition("tools/game-atlas-semantic-search/**") == "full"
    assert module.atlas_path_disposition("tools/game-atlas-thais-fixture/export.py") == "full"
    assert module.atlas_path_disposition("tools/unreviewed/helper.py") is None
    print("Atlas workflow disposition PASS: every specialized Atlas tool trigger is explicitly dedicated or intentionally FULL")


def test_routing_health_helpers(module):
    assert module.routing_health(module.full("unreviewed-consumer-input-snapshot")) == "degraded"
    assert module.routing_health(module.full("classifier-or-metadata-failure")) == "degraded"
    assert module.routing_health(module.full("unmodelled-input")) == "unmodelled"
    assert module.routing_health(module.full("explicit-atlas-non-cargo-full", "atlas")) == "modelled"
    assert module.routing_health({
        "rust": True, "windows": False, "surface": "server",
        "reason": "server-only-reverse-closure-and-audited-inputs",
    }) == "modelled"

    assert module.audited_input_path(fixture(), "apps/client/src/lib.rs") is True
    assert module.audited_input_path(fixture(), "Cargo.lock") is True
    assert module.audited_input_path(fixture(), ".cargo/config.toml") is True
    assert module.audited_input_path(fixture(), "apps/game-server/src/lib.rs") is False
    print("Routing health helpers PASS: degraded/unmodelled/modelled states and audited input selection are explicit")


def test_routing_contract_helpers(module):
    assert ROUTING_CONTRACT.is_file(), "routing contract validator is missing"
    spec = importlib.util.spec_from_file_location("routing_contract_validator", ROUTING_CONTRACT)
    assert spec is not None and spec.loader is not None
    validator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(validator)

    validator.verify_classifier_matrix(module, fixture())
    records = [
        {"filename": "apps/game-server/src/lib.rs", "status": "modified"},
        {"filename": "apps/client/src/lib.rs", "status": "modified"},
        {"filename": "Cargo.lock", "status": "modified"},
    ]
    affected = validator.changed_audited_inputs(module, fixture(), records)
    assert affected == ["Cargo.lock", "apps/client/src/lib.rs"], affected

    health, changed = validator.evaluate_snapshot_health(
        module, fixture(), module.AUDITED_INPUT_SHA256, records
    )
    assert health == "healthy" and changed == []

    stale = "0" * 64
    health, changed = validator.evaluate_snapshot_health(
        module,
        fixture(),
        stale,
        [{"filename": "apps/client/src/lib.rs", "status": "modified"}],
    )
    assert health == "degraded-candidate" and changed == ["apps/client/src/lib.rs"]

    health, changed = validator.evaluate_snapshot_health(
        module,
        fixture(),
        stale,
        [{"filename": "apps/game-server/src/lib.rs", "status": "modified"}],
    )
    assert health == "stale-inherited" and changed == []

    health, changed = validator.evaluate_snapshot_health(
        module,
        fixture(),
        stale,
        protected_main=True,
    )
    assert health == "stale-protected-main" and changed == []

    expected_head = "a" * 40
    expected_base = "b" * 40
    with tempfile.TemporaryDirectory() as directory:
        metadata_path = Path(directory) / "metadata.json"
        metadata_path.write_text(json.dumps(fixture()), encoding="utf-8")
        with patch.object(validator, "load_classifier", return_value=module), \
                patch.object(module, "input_digest", return_value=stale), \
                patch.object(
                    validator,
                    "changed_records",
                    return_value=[{"filename": "apps/game-server/src/lib.rs", "status": "modified"}],
                ), \
                patch.object(validator.subprocess, "check_output", return_value=expected_head + "\n"), \
                patch.dict(
                    os.environ,
                    {"EXPECTED_HEAD": expected_head, "EXPECTED_BASE": expected_base},
                    clear=False,
                ), \
                patch.object(sys, "argv", [str(ROUTING_CONTRACT), str(metadata_path)]):
            assert validator.main() == 0, "inherited candidate drift must be advisory"

        with patch.object(validator, "load_classifier", return_value=module), \
                patch.object(module, "input_digest", return_value=stale), \
                patch.object(validator.subprocess, "check_output", return_value=expected_head + "\n"), \
                patch.dict(os.environ, {"EXPECTED_HEAD": expected_head}, clear=False), \
                patch.object(
                    sys,
                    "argv",
                    [str(ROUTING_CONTRACT), "--protected-main", str(metadata_path)],
                ):
            assert validator.main() == 1, "protected-main routing drift must remain fail closed"

    print("Routing contract helpers PASS: inherited candidate drift is advisory while protected-main drift stays fatal")


def test_bounded_document_consumer_drift(module):
    docs = [dict(filename="README.md", status="modified")]
    result = module.classify(docs, 1, fixture(), "stale-nonserver", docs_digest="stale-all",
                             candidate_modes_verified=True, docs_consumers_verified=True)
    assert result["rust"] is False and result["windows"] is False, result
    result = module.classify(docs, 1, fixture(), module.AUDITED_INPUT_SHA256,
                             docs_digest=module.AUDITED_DOC_INPUT_SHA256,
                             candidate_modes_verified=True, docs_consumers_verified=False)
    assert result["rust"] and result["windows"], result
    assert result["reason"] == "unreviewed-document-consumer-inputs", result

    assert not module.document_consumer_content_safe(
        "apps/game-server/src/combat.rs", b"fn damage() -> u32 { 6 }", b"fn damage() -> u32 { 7 }")
    unchanged_consumer = b'const GUIDE: &str = include_str!("guide.md");\n'
    assert module.document_consumer_content_safe(
        "apps/game-server/src/lib.rs", unchanged_consumer, unchanged_consumer + b"// harmless note\n")
    literal_context = b'''const Q1: char = '\"';\nconst Q2: u8 = b'\"';\nconst ESC: char = '\\u{1F_600}';\n'''
    assert module.document_consumer_content_safe(
        "apps/game-server/src/lib.rs", literal_context + b"// old note\n",
        literal_context + b"// changed note\n")
    malformed_unicode_context = b"const BAD: char = '\\u{_1}';\n"
    assert not module.document_consumer_content_safe(
        "apps/game-server/src/lib.rs", malformed_unicode_context + b"// old note\n",
        malformed_unicode_context + b"// changed note\n")
    lifetime_context = b"fn borrow<'a>(value: &'a str) -> &'a str { value }\n'outer: loop { break 'outer; }\n"
    assert module.document_consumer_content_safe(
        "apps/game-server/src/lib.rs", lifetime_context + b"// old note\n",
        lifetime_context + b"// changed note\n")
    for path, baseline, current in (
        ("Cargo.toml", b"[workspace]", b"[workspace]\n"),
        ("apps/game-server/build.rs", b"fn main() {}", b"fn main() { println!(); }"),
        ("apps/game-server/src/lib.rs", b"", b"use std::{fs}; fs::read(path);"),
        ("apps/game-server/src/lib.rs", b"", b"use std::{fs as storage}; storage::read(path);"),
        ("apps/game-server/src/lib.rs", b"", b'const X: &str = include_str!("fixture.sql");'),
        ("apps/game-server/src/lib.rs", b"const USE_DOCS: bool = false;", b"const USE_DOCS: bool = true;"),
        ("apps/game-server/src/lib.rs", b"fn use_docs() -> bool { false }", b"fn use_docs() -> bool { true }"),
        ("apps/game-server/src/lib.rs", b"// module", b"/// module"),
        ("apps/game-server/src/lib.rs", b"// crate", b"//! crate"),
        ("apps/game-server/src/lib.rs", b"/**\n// old doc text\n*/", b"/**\n// changed doc text\n*/"),
        ("apps/game-server/src/lib.rs", b"/*!\n// old crate docs\n*/", b"/*!\n// changed crate docs\n*/"),
        ("apps/game-server/src/lib.rs", b'let text = r#"\n// old string text\n"#;', b'let text = r#"\n// changed string text\n"#;'),
        ("apps/game-server/src/lib.rs",
         b'''const Q1: char = '\"';\nconst TEXT: &str = r#"\n// old string text\n"#;\nconst Q2: u8 = b'\"';''',
         b'''const Q1: char = '\"';\nconst TEXT: &str = r#"\n// changed string text\n"#;\nconst Q2: u8 = b'\"';'''),
        ("apps/game-server/src/lib.rs", b'let text = "\n// old string text\n";', b'let text = "\n// changed string text\n";'),
        ("apps/game-server/src/lib.rs", b"// baseline", b"/* unterminated\n// uncertain"),
        ("apps/game-server/src/lib.rs", b"// baseline", b"const BAD: char = '\\q';\n// uncertain"),
        ("apps/game-server/src/lib.rs", b"// baseline", b"const BAD: u8 = b'xy';\n// uncertain"),
    ):
        assert not module.document_consumer_content_safe(path, baseline, current), (path, current)

    complete = subprocess.CompletedProcess([], 0)
    safe_outputs = [
        ("a" * 40 + "\n").encode(),
        b"",
        b"apps/game-server/src/combat.rs\0",
        b"// damage remains audited",
        b"// damage remains audited\n// ordinary note",
    ]
    with patch.object(module.subprocess, "run", return_value=complete), \
            patch.object(module.subprocess, "check_output", side_effect=safe_outputs):
        assert module.document_consumers_safe(fixture()) is True

    unsafe_outputs = [
        ("a" * 40 + "\n").encode(),
        b"",
        b"apps/game-server/src/combat.rs\0",
        b"fn damage() -> u32 { 7 }",
        b'use std::{fs as storage}; storage::read(path);',
    ]
    with patch.object(module.subprocess, "run", return_value=complete), \
            patch.object(module.subprocess, "check_output", side_effect=unsafe_outputs):
        assert module.document_consumers_safe(fixture()) is False

    deleted_outputs = [("a" * 40 + "\n").encode(), b"apps/game-server/src/old.rs\0"]
    with patch.object(module.subprocess, "run", return_value=complete), \
            patch.object(module.subprocess, "check_output", side_effect=deleted_outputs):
        assert module.document_consumers_safe(fixture()) is False

    # Exercise the PR classifier's proof against real immutable Git objects.
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        def git(*args):
            return subprocess.check_output(["git", "-C", directory, "-c", "user.name=Fixture",
                                            "-c", "user.email=fixture@example.invalid", *args]).decode().strip()
        git("init", "-q")
        source = root / "apps/game-server/src/lib.rs"
        source.parent.mkdir(parents=True)
        lexical_contexts = (b"/**\n// old block docs\n*/\n/*!\n// old crate block docs\n*/\n"
                            b'const TEXT: &str = r#"\n// old raw string\n"#;\n'
                            b'''const Q1: char = '\"';\nconst QUOTED: &str = r#"\n// old quoted raw string\n"#;\nconst Q2: u8 = b'\"';\n''')
        baseline_source = unchanged_consumer + b"const USE_DOCS: bool = false;\n" + lexical_contexts
        source.write_bytes(baseline_source)
        for package in fixture()["packages"]:
            manifest = root / Path(package["manifest_path"]).relative_to("/repo")
            manifest.parent.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\n")
        git("add", ".")
        git("commit", "-qm", "baseline")
        baseline_sha = git("rev-parse", "HEAD")
        real_meta = copy.deepcopy(fixture())
        real_meta["workspace_root"] = directory
        for package in real_meta["packages"]:
            package["manifest_path"] = package["manifest_path"].replace("/repo", directory, 1)
            for dependency in package["dependencies"]:
                dependency["path"] = dependency["path"].replace("/repo", directory, 1)
        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            source.write_bytes(baseline_source + b"// harmless note\n")
            git("add", "."); git("commit", "-qm", "harmless")
            with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", baseline_sha):
                assert module.document_consumers_safe(real_meta) is True
            git("checkout", "-q", baseline_sha)
            source.write_bytes(baseline_source + malformed_unicode_context + b"// old note\n")
            git("add", "."); git("commit", "-qm", "malformed unicode baseline")
            malformed_base = git("rev-parse", "HEAD")
            source.write_bytes(baseline_source + malformed_unicode_context + b"// changed note\n")
            git("add", "."); git("commit", "-qm", "comment after malformed unicode")
            with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", malformed_base):
                assert module.document_consumers_safe(real_meta) is False
            for statement in (b"use std::{fs}; fs::read(path);\n",
                              b"use std::{fs as storage}; storage::read(path);\n"):
                git("checkout", "-q", baseline_sha)
                source.write_bytes(baseline_source + statement)
                git("add", "."); git("commit", "-qm", "unsafe alias")
                with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", baseline_sha):
                    assert module.document_consumers_safe(real_meta) is False
            git("checkout", "-q", baseline_sha)
            source.write_bytes(unchanged_consumer + b"const USE_DOCS: bool = true;\n")
            git("add", "."); git("commit", "-qm", "unsafe scalar gate")
            with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", baseline_sha):
                assert module.document_consumers_safe(real_meta) is False
            for doc_comment in (b"/// module docs\n", b"//! crate docs\n"):
                git("checkout", "-q", baseline_sha)
                source.write_bytes(baseline_source + doc_comment)
                git("add", "."); git("commit", "-qm", "unsafe doc attribute")
                with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", baseline_sha):
                    assert module.document_consumers_safe(real_meta) is False
            for ambiguous in (lexical_contexts.replace(b"// old block docs", b"// changed block docs"),
                              lexical_contexts.replace(b"// old crate block docs", b"// changed crate block docs"),
                              lexical_contexts.replace(b"// old raw string", b"// changed raw string"),
                              lexical_contexts.replace(b"// old quoted raw string", b"// changed quoted raw string"),
                              lexical_contexts + b"/* unterminated\n// uncertain\n"):
                git("checkout", "-q", baseline_sha)
                source.write_bytes(unchanged_consumer + b"const USE_DOCS: bool = false;\n" + ambiguous)
                git("add", "."); git("commit", "-qm", "unsafe lexical context")
                with patch.object(module, "AUDITED_DOC_CONSUMER_BASE_SHA", baseline_sha):
                    assert module.document_consumers_safe(real_meta) is False
        finally:
            os.chdir(old_cwd)
    print("Bounded document consumer drift PASS: lexical ordinary comments allowed; strings, block/docs, scalar and uncertain drift FULL")


def test_large_pr_git_fallback(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)

        def git(*args):
            return subprocess.check_output(
                ["git", "-C", directory, "-c", "user.name=Fixture",
                 "-c", "user.email=fixture@example.invalid", *args]
            ).decode().strip()

        git("init", "-q")
        git("commit", "--allow-empty", "-qm", "baseline")
        baseline = git("rev-parse", "HEAD")
        source = root / "apps/game-server/src/generated"
        source.mkdir(parents=True)
        for index in range(301):
            (source / f"large_{index:03d}.rs").write_text(f"pub const ITEM_{index}: usize = {index};\n")
        git("add", ".")
        git("commit", "-qm", "large server-only change")
        head = git("rev-parse", "HEAD")
        git("checkout", "-q", baseline)

        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            env = {
                "ENUMERATION_COMPLETE": "false",
                "CHANGED_FILE_RECORDS": "[]",
                "CHANGED_FILE_COUNT": "301",
                "EXPECTED_HEAD": head,
            }
            with patch.dict(os.environ, env, clear=False):
                files, count, complete = module.pr_file_records()
            assert complete is True and count == 301 and len(files) == 301
            assert {item["status"] for item in files} == {"added"}
            result = module.classify(
                files, count, fixture(), module.AUDITED_INPUT_SHA256,
                docs_digest=module.AUDITED_DOC_INPUT_SHA256,
                candidate_modes_verified=True,
            )
            assert result["rust"] is True and result["windows"] is False, result

            git("checkout", "-q", head)
            client = root / "apps/client/src/large_pr_probe.rs"
            client.parent.mkdir(parents=True, exist_ok=True)
            client.write_text("pub const CLIENT_PROBE: bool = true;\n")
            git("add", ".")
            git("commit", "-qm", "add client impact")
            client_head = git("rev-parse", "HEAD")
            git("checkout", "-q", baseline)
            with patch.dict(os.environ, env | {"EXPECTED_HEAD": client_head, "CHANGED_FILE_COUNT": "302"}, clear=False):
                files, count, complete = module.pr_file_records()
            assert complete is True and count == 302 and len(files) == 302
            result = module.classify(
                files, count, fixture(), module.AUDITED_INPUT_SHA256,
                docs_digest=module.AUDITED_DOC_INPUT_SHA256,
                candidate_modes_verified=True,
            )
            assert result["rust"] is True and result["windows"] is True, result

            # Complete immutable Git fallback must retain cross-surface rename
            # provenance instead of flattening it into a server-only removal.
            git("checkout", "-q", baseline)
            renamed_source = root / "apps/game-server/src/rename_probe.rs"
            renamed_source.parent.mkdir(parents=True, exist_ok=True)
            renamed_source.write_text("governance rename probe\n")
            git("add", ".")
            git("commit", "-qm", "rename baseline")
            rename_base = git("rev-parse", "HEAD")
            git("mv", "apps/game-server/src/rename_probe.rs", "AGENTS.md")
            git("commit", "-qm", "rename server input to governance")
            rename_head = git("rev-parse", "HEAD")
            git("checkout", "-q", rename_base)
            rename_env = env | {"EXPECTED_HEAD": rename_head, "CHANGED_FILE_COUNT": "1"}
            with patch.dict(os.environ, rename_env, clear=False):
                rename_files, rename_count, rename_complete = module.pr_file_records()
            assert rename_complete is True and rename_count == 1, rename_files
            assert rename_files == [{
                "filename": "AGENTS.md",
                "status": "renamed",
                "previous_filename": "apps/game-server/src/rename_probe.rs",
            }], rename_files
            result = module.classify(
                rename_files, rename_count, fixture(), module.AUDITED_INPUT_SHA256,
                docs_digest=module.AUDITED_DOC_INPUT_SHA256,
                candidate_modes_verified=True,
                docs_consumers_verified=True,
            )
            assert result["rust"] is True and result["windows"] is True, result
            assert result["reason"] == "cross-surface-rename", result
        finally:
            os.chdir(old_cwd)

    with patch.object(module.subprocess, "check_output",
                      return_value=b"T\0apps/game-server/src/lib.rs\0"):
        try:
            module.git_diff_records("a" * 40, "b" * 40)
        except ValueError:
            pass
        else:
            raise AssertionError("unsupported Git diff status accepted")

    with patch.dict(os.environ, {"ENUMERATION_COMPLETE": "unknown"}, clear=False):
        try:
            module.pr_file_records()
        except ValueError:
            pass
        else:
            raise AssertionError("invalid enumeration state accepted")

    for invalid_count in ("True", "0", "-1", "01", ""):
        with patch.dict(os.environ, {
            "ENUMERATION_COMPLETE": "false",
            "CHANGED_FILE_RECORDS": "[]",
            "CHANGED_FILE_COUNT": invalid_count,
            "EXPECTED_HEAD": "b" * 40,
        }, clear=False), patch.object(module.subprocess, "check_output") as check_output:
            try:
                module.pr_file_records()
            except ValueError:
                pass
            else:
                raise AssertionError(("invalid transported file count accepted", invalid_count))
            check_output.assert_not_called()

    print("Large-PR Git fallback PASS: >300 server-only records recover reduced Windows lane; client impact and malformed evidence fail closed")


def main() -> int:
    assert MODULE.is_file(), "dependency-aware trusted-base classifier is not implemented"
    spec = importlib.util.spec_from_file_location("risk_classifier", MODULE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    test_candidate_modes(module)
    test_atlas_fullworld_surface(module)
    test_atlas_workflow_dispositions(module)
    test_routing_health_helpers(module)
    test_routing_contract_helpers(module)
    test_large_pr_git_fallback(module)
    test_reviewed_document_consumers(module)
    test_bounded_document_consumer_drift(module)

    def classify(paths, metadata=None, digest=None, **kwargs):
        files = [dict(filename=p, status="modified") if isinstance(p, str) else p for p in paths]
        return module.classify(files, kwargs.pop("count", len(files)), fixture() if metadata is None else metadata,
                               module.AUDITED_INPUT_SHA256 if digest is None else digest,
                               docs_digest=kwargs.pop("docs_digest", module.AUDITED_DOC_INPUT_SHA256), candidate_modes_verified=True, **kwargs)

    server = "apps/game-server/src/lib.rs"
    result = classify([server])
    assert result["rust"] is True and result["windows"] is False, result
    assert result["surface"] == "server", result
    for path in ("apps/game-server/src/durability/mod.rs", "apps/game-server/migrations/0001.sql",
                 "apps/game-server/tests/support/postgres.rs", "apps/game-server/src/foundation/reconnect.rs"):
        result = classify([path])
        assert result["rust"] is True and result["windows"] is False, (path, result)

    full_paths = (
        "apps/client/src/lib.rs", "crates/foundation/src/lib.rs",
        "crates/simulation-determinism/src/lib.rs", "crates/simulation-determinism/fixtures/golden.json",
        "Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "apps/game-server/build.rs",
        "apps/game-server/Cargo.toml", ".github/workflows/rust.yml", ".github/actions/custom/action.yml",
        "tools/repository/classify_pr_test_lanes.py",
        "tools/game-atlas-fullworld-source/animated.py", "tools/game-atlas-fullworld-source/README.md",
        "docs/agents/evidence/runtime-input.json", "docs/migration/input.json",
        "unknown/input.dat", "apps/game-server/unknown.md",
    )
    for path in full_paths:
        result = classify([path])
        assert result["rust"] is True and result["windows"] is True, (path, result)

    governance_paths = (
        "AGENTS.md",
        "docs/agents/AGENTS.md",
        "docs/agents/PROJECT_LANES.json",
        "docs/agents/PROMPT_LIFECYCLE.json",
        "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md",
        "tools/agents/tests/test_meta_agent_policy_adoption.py",
    )
    for path in governance_paths:
        result = classify([path], docs_consumers_verified=True)
        assert result == {
            "rust": False,
            "windows": False,
            "surface": "agent-governance",
            "reason": "agent-governance-only",
        }, (path, result)
        result = classify([path], docs_consumers_verified=False)
        assert result["rust"] is True and result["windows"] is True, (path, result)
        assert result["reason"] == "unreviewed-document-consumer-inputs", (path, result)
        result = classify([path], digest="stale-runtime-snapshot", docs_digest="stale-doc-snapshot")
        assert result["rust"] is True and result["windows"] is True, (path, result)
        result = classify(
            [path], digest="stale-runtime-snapshot", docs_digest="stale-doc-snapshot",
            docs_consumers_verified=True,
        )
        assert result["rust"] is False and result["windows"] is False, (path, result)
    task_record_expected = {
        "rust": False,
        "windows": False,
        "surface": "agent-governance",
        "reason": "agent-task-record-only",
    }
    for path in (
        "docs/agents/tasks/active/task.md",
        "docs/agents/tasks/archive/task.md",
    ):
        result = classify(
            [path],
            digest="stale-runtime-snapshot",
            docs_digest="stale-doc-snapshot",
            docs_consumers_verified=False,
        )
        assert result == task_record_expected, (path, result)

    closeout_rename = [{
        "filename": "docs/agents/tasks/archive/task.md",
        "status": "renamed",
        "previous_filename": "docs/agents/tasks/active/task.md",
    }]
    result = classify(
        closeout_rename,
        digest="stale-runtime-snapshot",
        docs_digest="stale-doc-snapshot",
        docs_consumers_verified=False,
    )
    assert result == task_record_expected, result

    for paths in ([server, "apps/client/src/lib.rs"], [server, "unknown/input.dat"],
                  [{"filename": server, "status": "renamed", "previous_filename": "apps/client/src/old.rs"}],
                  [{"filename": "docs/new.md", "status": "renamed", "previous_filename": server}],
                  [{"filename": "AGENTS.md", "status": "renamed", "previous_filename": server}],
                  [{"filename": server, "status": "renamed", "previous_filename": "AGENTS.md"}],
                  [{"filename": server, "status": "removed"}, {"filename": "AGENTS.md", "status": "added"}],
                  [{"filename": "AGENTS.md", "status": "removed"}, {"filename": server, "status": "added"}]):
        result = classify(paths, docs_consumers_verified=True)
        assert result["rust"] and result["windows"], result
    for paths in (["README.md"], ["docs/architecture/example.md"], ["docs/reference/example.md"],
                  ["docs/agents/evidence/history.md"]):
        result = classify(paths)
        assert result["rust"] is False and result["windows"] is False, result
        result = classify(paths, digest="unreviewed-document-consumer")
        assert result["rust"] and result["windows"], "docs skip ignored changed input assumptions"
        result = classify(paths, docs_digest="server-started-reading-docs")
        assert result["rust"] and result["windows"], "docs skip ignored changed server input assumptions"

    result = classify([server, "docs/agents/tasks/active/task.md"], docs_consumers_verified=True)
    assert result["rust"] and not result["windows"], result
    result = classify([server, "docs/agents/tasks/active/task.md"], docs_consumers_verified=False)
    assert result["rust"] and result["windows"], result
    assert result["reason"] == "unreviewed-document-consumer-inputs", result
    result = classify(["apps/client/src/lib.rs", "docs/agents/tasks/active/task.md"], docs_consumers_verified=True)
    assert result["rust"] and result["windows"], result
    result = classify(["AGENTS.md", "docs/architecture/example.md"])
    assert result["rust"] is False and result["windows"] is False, result
    assert result["surface"] == "agent-governance", result
    assert result["reason"] == "agent-governance-plus-neutral-documentation", result
    result = classify([{"filename": "docs/agents/AGENTS.md", "status": "renamed", "previous_filename": "AGENTS.md"}])
    assert result["rust"] is False and result["windows"] is False, result

    invalid = (
        ([], {}), ([server], {"count": 2}), ([server], {"complete": False}),
        ([server], {"complete": "true"}), ([server, server], {}),
        ([{"filename": server, "status": "renamed"}], {}),
        ([{"filename": "../apps/game-server/lib.rs", "status": "modified"}], {}),
        ([{"filename": server, "status": "unknown"}], {}),
    )
    for paths, kwargs in invalid:
        result = classify(paths, **kwargs)
        assert result["rust"] and result["windows"], (paths, kwargs, result)
    for metadata in ({}, {"packages": []}, {"workspace_root": "/repo"}):
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], result
    # A later accepted cross-package include, symlink, build input or dependency edit
    # changes the protected-base input snapshot even without a Cargo edge change.
    for digest in ("", "0" * 64, "cross-package-include", "symlink"):
        result = classify([server], digest=digest)
        assert result["rust"] and result["windows"], result
    for kind, target, optional in ((None, None, False), ("dev", None, False),
                                   ("build", None, False), (None, "cfg(windows)", True)):
        metadata = fixture()
        metadata["packages"][1]["dependencies"].append({"name": "oteryn-game-server", "path": "/repo/apps/game-server",
                                                       "kind": kind, "target": target, "optional": optional})
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], (kind, target, result)
    for change in (
        lambda m: m["packages"].pop(),
        lambda m: m["packages"].append(copy.deepcopy(m["packages"][0])),
        lambda m: m["packages"][0]["dependencies"].append({"path": "/outside", "name": "unknown"}),
    ):
        metadata = fixture()
        change(metadata)
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], result
    print("Risk classifier fixtures PASS: surfaces, transitive dependency kinds, protected inputs and fail-closed enumeration")
    test_aggregate()
    test_snapshot_and_fallbacks(module)
    test_trusted_job_mutations()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
