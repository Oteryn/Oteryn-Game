#!/usr/bin/env python3
"""Shared behavioral cases for exact-candidate routing validation.

This is validation data, not production routing authority. The classifier must
not import it; focused tests and the hosted routing-contract validator do.
"""
from __future__ import annotations


def routing_contract_cases(classifier):
    server = f"{classifier.REQUIRED[classifier.SERVER]}/src/lib.rs"
    client = f"{classifier.REQUIRED['oteryn-client']}/src/lib.rs"
    evidence = "docs/agents/evidence/runtime-input.json"
    helper = "tools/content/helper.py"
    incident = (
        ".github/workflows/item-wiki-first-census.yml",
        "docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json",
        "docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md",
        "tools/reference-world-corridor-census/item_wiki_first_census.py",
        "tools/reference-world-corridor-census/item_wiki_first_census_self_test.py",
    )

    def case(name, paths, expected, consumers=()):
        return name, tuple(paths), tuple(consumers), expected

    return (
        case("server-only", (server,), {
            "rust": True, "windows": False, "surface": "server",
            "reason": "server-only-exact-consumer-closure",
        }),
        case("client", (client,), {
            "rust": True, "windows": True, "surface": "client",
            "reason": "windows-consumer-affected",
        }),
        case("shared", ("crates/foundation/src/lib.rs",), {
            "rust": True, "windows": True, "reason": "windows-consumer-affected",
        }),
        case("governance-aux", ("AGENTS.md",), {
            "rust": False, "windows": False, "surface": "agent-governance",
            "reason": "unconsumed-auxiliary-inputs",
        }),
        case("docs-aux", ("docs/architecture/example.md",), {
            "rust": False, "windows": False, "surface": "docs",
            "reason": "unconsumed-auxiliary-inputs",
        }),
        case("evidence-aux", (evidence,), {
            "rust": False, "windows": False, "surface": "auxiliary",
            "reason": "unconsumed-auxiliary-inputs",
        }),
        case("workflow-aux", (".github/workflows/offline-content.yml",), {
            "rust": False, "windows": False, "reason": "unconsumed-auxiliary-inputs",
        }),
        case("tool-aux", ("tools/reference-world-corridor-census/offline.py",), {
            "rust": False, "windows": False, "reason": "unconsumed-auxiliary-inputs",
        }),
        case("server-consumed-aux", (evidence,), {
            "rust": True, "windows": False,
            "reason": "server-only-exact-consumer-closure",
        }, ((evidence, (classifier.SERVER,)),)),
        case("client-consumed-aux", (evidence,), {
            "rust": True, "windows": True, "reason": "windows-consumer-affected",
        }, ((evidence, ("oteryn-client",)),)),
        case("canonical-ci-consumer", (helper,), {
            "rust": True, "windows": True, "surface": "control-plane",
            "reason": "canonical-control-consumer-affected",
        }, ((helper, (classifier.CONTROL_CONSUMER,)),)),
        case("cargo-lock", ("Cargo.lock",), {
            "rust": True, "windows": True, "surface": "dependencies-build",
            "reason": "explicit-build-or-dependency-input",
        }),
        case("merge-gate-control", (".github/workflows/merge-gate.yml",), {
            "rust": True, "windows": True, "surface": "control-plane",
            "reason": "explicit-build-or-control-input",
        }),
        case("merge-group-control", (".github/workflows/merge-group-gate.yml",), {
            "rust": True, "windows": True, "reason": "explicit-build-or-control-input",
        }),
        case("rust-workflow-control", (".github/workflows/rust.yml",), {
            "rust": True, "windows": True, "reason": "explicit-build-or-control-input",
        }),
        case("repository-tool-control", ("tools/repository/classify_pr_test_lanes.py",), {
            "rust": True, "windows": True, "reason": "explicit-build-or-control-input",
        }),
        case("pr-803-regression", incident, {
            "rust": False, "windows": False, "surface": "auxiliary",
            "reason": "unconsumed-auxiliary-inputs",
        }),
        case("unknown-fail-closed", ("unowned/input.bin",), {
            "rust": True, "windows": True, "surface": "unknown",
            "reason": "unmodelled-input",
        }),
        case("atlas-fullworld", (sorted(classifier.ATLAS_FULLWORLD_PATHS)[0],), {
            "rust": False, "windows": False, "surface": "atlas-fullworld",
            "reason": "audited-atlas-fullworld-source",
        }),
    )


def verify_routing_contract_cases(classifier, metadata):
    names = set()
    for name, paths, edges, expected in routing_contract_cases(classifier):
        if not name or name in names or not paths or len(set(paths)) != len(paths):
            raise ValueError(f"invalid routing contract case: {name!r}")
        names.add(name)
        references = {path: set() for path in paths}
        for path, consumers in edges:
            if path not in references:
                raise ValueError(f"{name}: consumer edge targets undeclared path")
            references[path].update(consumers)

        result = classifier.classify(
            [{"filename": path, "status": "modified"} for path in paths],
            len(paths),
            metadata,
            candidate_modes_verified=True,
            reference_consumers=references,
        )
        mismatch = {
            key: (value, result.get(key))
            for key, value in expected.items()
            if result.get(key) != value
        }
        if mismatch:
            raise ValueError(f"{name}: routing contract drift {mismatch}; result={result}")
    return tuple(sorted(names))
