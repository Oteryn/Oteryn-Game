#!/usr/bin/env python3
"""Shared declarative cases for the exact-candidate routing contract.

This module is test/validation data, not routing authority. The production
classifier must not import it. Both focused regressions and the hosted routing
contract validator should consume the same cases so the behavioral matrix has
one reviewed representation.
"""
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RoutingContractCase:
    """One classifier behavior case with optional exact-consumer edges."""

    name: str
    paths: tuple[str, ...]
    consumer_edges: tuple[tuple[str, tuple[str, ...]], ...]
    expected_fields: tuple[tuple[str, object], ...]

    def reference_consumers(self) -> dict[str, set[str]]:
        result = {path: set() for path in self.paths}
        for path, consumers in self.consumer_edges:
            if path not in result:
                raise ValueError(f"{self.name}: consumer edge targets undeclared path {path}")
            result[path].update(consumers)
        return result

    def expected(self) -> dict[str, object]:
        return dict(self.expected_fields)


def routing_contract_cases(classifier) -> tuple[RoutingContractCase, ...]:
    """Return the canonical routing decision matrix for a classifier module."""

    server = f"{classifier.REQUIRED[classifier.SERVER]}/src/lib.rs"
    client = f"{classifier.REQUIRED['oteryn-client']}/src/lib.rs"
    shared = "crates/foundation/src/lib.rs"
    evidence = "docs/agents/evidence/runtime-input.json"
    helper = "tools/content/helper.py"
    unknown = "unowned/input.bin"
    atlas = sorted(classifier.ATLAS_FULLWORLD_PATHS)[0]
    incident = (
        ".github/workflows/item-wiki-first-census.yml",
        "docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json",
        "docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md",
        "tools/reference-world-corridor-census/item_wiki_first_census.py",
        "tools/reference-world-corridor-census/item_wiki_first_census_self_test.py",
    )

    return (
        RoutingContractCase(
            name="server-only-product",
            paths=(server,),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", False),
                ("surface", "server"),
                ("reason", "server-only-exact-consumer-closure"),
            ),
        ),
        RoutingContractCase(
            name="client-product",
            paths=(client,),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "client"),
                ("reason", "windows-consumer-affected"),
            ),
        ),
        RoutingContractCase(
            name="shared-product",
            paths=(shared,),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("reason", "windows-consumer-affected"),
            ),
        ),
        RoutingContractCase(
            name="unconsumed-auxiliary",
            paths=(evidence,),
            consumer_edges=(),
            expected_fields=(
                ("rust", False),
                ("windows", False),
                ("surface", "auxiliary"),
                ("reason", "unconsumed-auxiliary-inputs"),
            ),
        ),
        RoutingContractCase(
            name="server-consumed-auxiliary",
            paths=(evidence,),
            consumer_edges=((evidence, (classifier.SERVER,)),),
            expected_fields=(
                ("rust", True),
                ("windows", False),
                ("surface", "server"),
                ("reason", "server-only-exact-consumer-closure"),
            ),
        ),
        RoutingContractCase(
            name="client-consumed-auxiliary",
            paths=(evidence,),
            consumer_edges=((evidence, ("oteryn-client",)),),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "client"),
                ("reason", "windows-consumer-affected"),
            ),
        ),
        RoutingContractCase(
            name="canonical-product-ci-consumer",
            paths=(helper,),
            consumer_edges=((helper, (classifier.CONTROL_CONSUMER,)),),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "control-plane"),
                ("reason", "canonical-control-consumer-affected"),
            ),
        ),
        RoutingContractCase(
            name="build-input",
            paths=("Cargo.lock",),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "dependencies-build"),
                ("reason", "explicit-build-or-dependency-input"),
            ),
        ),
        RoutingContractCase(
            name="canonical-control-input",
            paths=(".github/workflows/merge-gate.yml",),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "control-plane"),
                ("reason", "explicit-build-or-control-input"),
            ),
        ),
        RoutingContractCase(
            name="pr-803-unconsumed-census",
            paths=incident,
            consumer_edges=(),
            expected_fields=(
                ("rust", False),
                ("windows", False),
                ("surface", "auxiliary"),
                ("reason", "unconsumed-auxiliary-inputs"),
            ),
        ),
        RoutingContractCase(
            name="unknown-input-fail-closed",
            paths=(unknown,),
            consumer_edges=(),
            expected_fields=(
                ("rust", True),
                ("windows", True),
                ("surface", "unknown"),
                ("reason", "unmodelled-input"),
            ),
        ),
        RoutingContractCase(
            name="atlas-fullworld",
            paths=(atlas,),
            consumer_edges=(),
            expected_fields=(
                ("rust", False),
                ("windows", False),
                ("surface", "atlas-fullworld"),
                ("reason", "audited-atlas-fullworld-source"),
            ),
        ),
    )

def verify_routing_contract_cases(classifier, metadata: dict) -> tuple[str, ...]:
    """Run every canonical case and fail on any behavioral contract drift."""

    names: list[str] = []
    seen: set[str] = set()
    for case in routing_contract_cases(classifier):
        if not case.name or case.name in seen:
            raise ValueError(f"invalid or duplicate routing contract case name: {case.name!r}")
        seen.add(case.name)
        if not case.paths or len(set(case.paths)) != len(case.paths):
            raise ValueError(f"{case.name}: paths must be non-empty and unique")

        expected = case.expected()
        if (
            len(expected) != len(case.expected_fields)
            or not {"rust", "windows"} <= set(expected)
            or not set(expected) <= {"rust", "windows", "surface", "reason"}
        ):
            raise ValueError(f"{case.name}: invalid expected field set")

        records = [
            {"filename": path, "status": "modified"}
            for path in case.paths
        ]
        result = classifier.classify(
            records,
            len(records),
            metadata,
            candidate_modes_verified=True,
            reference_consumers=case.reference_consumers(),
        )
        mismatches = {
            key: {"expected": value, "actual": result.get(key)}
            for key, value in expected.items()
            if result.get(key) != value
        }
        if mismatches:
            raise ValueError(
                f"{case.name}: routing contract drift {mismatches}; full result={result}"
            )
        names.append(case.name)
    return tuple(names)
