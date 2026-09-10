import copy
import importlib.util
import json
import pathlib
import tempfile
import unittest

HERE = pathlib.Path(__file__).parent
SPEC = importlib.util.spec_from_file_location("reference_cell", HERE / "reference_cell.py")
rc = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(rc)


def repetition(value=1_000):
    return {
        "latency_us": [value] * 100, "queue_age_us": [value] * 100,
        "cpu_utilization_basis_points": 5_000, "rss_bytes": 100_000,
        "peak_rss_bytes": 110_000, "swap_observations_bytes": [0, 0],
        "memory_growth_bytes": 10, "unexpected_rejections": 0,
        "attempted_operations": 100, "completed_operations": 100,
        "stale_references_rejected": True, "missed_windows": 0,
    }


def complete_document():
    identities = {key: "sha256:" + "a" * 64 for key in rc.IDENTITY_FIELDS}
    identities["repository"] = "Oteryn/Oteryn-Game"
    identities["source_revision"] = "d" * 40
    identities["capacity_evidence_source"] = "PHYSICAL_REFERENCE_CELL"
    fingerprint = {
        "runner_group": "game-runners", "runner_label": "oteryn-game",
        "runner_name": "oteryn-synology-game", "os_family": "Linux",
        "cpu_vendor": "sanitized-vendor", "cpu_model": "sanitized-model",
        "architecture": "x86_64", "online_logical_cpus": [0, 1],
        "topology": {"sockets": 1, "cores": 2, "threads_per_core": 1},
        "selected_cpu": 0, "smt_state": "disabled", "numa_topology": "single-node",
        "frequency_policy": "observed", "governor": "performance", "turbo_state": "disabled",
        "total_physical_ram_bytes": 2_000_000_000, "os_distribution": "sanitized-linux",
        "os_version": "1", "kernel_release": "sanitized", "kernel_architecture": "x86_64",
        "effective_cpu_limit": "1", "effective_memory_limit_bytes": 1_073_741_824,
        "rlimit_as_bytes": 1_073_741_824, "allowed_affinity": [0, 1],
        "page_size_bytes": 4096, "host_swap_present": True,
    }
    objectives = dict(zip(rc.OBJECTIVE_FIELDS, (10_000, 25_000, 50_000, 25_000, 9_000, 858_993_459, 0, 10_737_418, 0)))
    workload = {
        "window_us": 50_000, "actor_order": ["PLAYER", "CREATURE", "NPC_SYSTEM"],
        "lookup_per_actor": 1, "mutation_rate_numerator": 1, "mutation_rate_denominator": 10,
        "reuse_rate_numerator": 1, "reuse_rate_denominator": 100, "fixed_seed": 7,
        "warmup_windows": 10, "measured_windows": 100, "population_plan": [10, 20],
        "percentile_method": "nearest-rank", "minimum_samples_per_metric": 100,
        "minimum_repetitions": 5, "soak_duration_seconds": 1800,
        "world_id": "world-reference", "channel_id": "channel-reference",
        "session_generation_owner": "single-logical-mutation-owner",
        "exact_current_reference_lookup": True, "scan_or_candidate_collection": False,
        "same_slot_reuse": True, "generation_advance_checked": True,
        "stale_reference_check": True, "kind_balance_max_difference": 1,
        "rational_accumulator": True, "rotation_covers_all_before_repeat": True,
        "remove_reuse_preserves_population": True,
        "excluded_payloads": ["VARIABLE_BEHAVIOR", "COMBAT", "AI", "INVENTORY", "PATHFINDING", "LOOT", "NETWORK", "PERSISTENCE"],
        "process_reset_between_repetitions": True,
    }
    return {
        "schema_version": 1, "contract": rc.CONTRACT, "state": "MEASUREMENT_INCOMPLETE",
        "identities": identities, "comparison_baseline_identities": copy.deepcopy(identities),
        "cell_fingerprint": {"expected": fingerprint, "observed": copy.deepcopy(fingerprint)},
        "enforcement": {
            "execution_mode": "native-release-process", "container": "none",
            "affinity_set_succeeded": True, "affinity_readback": [0],
            "rlimit_as_soft_bytes": 1_073_741_824, "rlimit_as_hard_bytes": 1_073_741_824,
            "rlimit_readback_proven": True, "process_swap_observations_bytes": [0, 0],
            "exclusive_job_proven": True, "benchmark_instances": 1,
            "workflow_run_id": "run", "workflow_job_id": "job", "native_process_proven": True,
            "process_start_invariants_proven": True, "host_background_activity_recorded": True,
            "contention_absent": True,
        },
        "objectives": objectives, "workload": workload,
        "populations": [
            {"population": 10, "repetitions": [repetition() for _ in range(5)]},
            {"population": 20, "repetitions": [repetition(60_000) for _ in range(5)]},
        ],
        "soak": {"population": 10, "duration_seconds": 1800, "measurement": repetition()},
        "lower_coupled_limits": [12],
        "boundary": {
            "configured_maximum": 10, "occupancy_m_admitted": True, "m_plus_one_rejected": True,
            "rejected_before_mutation": True, "no_eviction_or_recycle": True,
            "generation_advanced": True, "stale_references_rejected": True,
            "state_unchanged_after_rejection": True, "overflow_rejected_before_allocation": True,
        },
    }


class ReferenceCellTests(unittest.TestCase):
    def assert_code(self, document, code):
        with self.assertRaises(rc.EvidenceError) as caught:
            rc.process(document)
        self.assertEqual(caught.exception.code, code)

    def test_complete_packet_is_provisional_and_perf_remains_open(self):
        result = rc.process(complete_document())
        self.assertEqual(result["candidate_M"], 10)  # min(floor(14), P=10, lower=12)
        self.assertEqual(result["state"], "MEASUREMENT_COMPLETE_NOT_ACCEPTED")
        self.assertEqual(result["perf_01_status"], "OPEN")
        self.assertIsNone(result["production_capacity"])

    def test_placeholder_cannot_emit_capacity(self):
        placeholder = {"state": "PLACEHOLDER_UNBOUND", "candidate_M": 1}
        self.assert_code(placeholder, "PLACEHOLDER_CAPACITY_FORBIDDEN")

    def test_placeholder_without_capacity_is_unbound(self):
        result = rc.process({"state": "PLACEHOLDER_UNBOUND", "candidate_M": None})
        self.assertIsNone(result["candidate_M"])

    def test_all_contract_inputs_are_required(self):
        for section, field in (("objectives", "latency_p50_us_max"), ("workload", "window_us"), ("identities", "source_revision")):
            document = complete_document(); del document[section][field]
            self.assert_code(document, "REQUIRED_INPUT_MISSING")

    def test_fingerprint_is_order_independent_and_drift_rejected(self):
        document = complete_document()
        document["cell_fingerprint"]["observed"] = dict(reversed(list(document["cell_fingerprint"]["observed"].items())))
        rc.process(document)
        document["cell_fingerprint"]["observed"]["cpu_model"] = "drift"
        self.assert_code(document, "CELL_FINGERPRINT_MISMATCH")

    def test_missing_fingerprint_is_incomplete(self):
        document = complete_document(); del document["cell_fingerprint"]["observed"]["governor"]
        self.assert_code(document, "CELL_FINGERPRINT_INCOMPLETE")

    def test_identity_revision_drift_rejected(self):
        for field in rc.IDENTITY_FIELDS:
            document = complete_document(); document["identities"][field] += "drift"
            self.assert_code(document, "EVIDENCE_IDENTITY_MISMATCH")

    def test_enforcement_is_fail_closed(self):
        for field, value in (("affinity_readback", [0, 1]), ("rlimit_as_hard_bytes", 1_073_741_825), ("process_swap_observations_bytes", [0, 1]), ("exclusive_job_proven", False), ("native_process_proven", False)):
            document = complete_document(); document["enforcement"][field] = value
            self.assert_code(document, "REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE")

    def test_nearest_rank_and_insufficient_samples(self):
        self.assertEqual(rc.nearest_rank(list(range(1, 101)), 95, 100), 95)
        with self.assertRaises(rc.EvidenceError) as caught:
            rc.nearest_rank([1] * 99, 99, 100)
        self.assertEqual(caught.exception.code, "SAMPLE_SET_INCOMPLETE")
        document = complete_document(); document["populations"][0]["repetitions"][0]["rss_bytes"] = "unknown"
        self.assert_code(document, "SAMPLE_SET_INCOMPLETE")

    def test_repetitions_and_inconclusive_are_rejected(self):
        document = complete_document(); document["populations"][0]["repetitions"].pop()
        self.assert_code(document, "MEASUREMENT_INCOMPLETE")
        document = complete_document(); document["populations"][1]["repetitions"][0] = repetition()
        self.assert_code(document, "POPULATION_INCONCLUSIVE")

    def test_soak_and_progression_are_required(self):
        document = complete_document(); document["soak"]["duration_seconds"] = 1799
        self.assert_code(document, "SOAK_INCOMPLETE")
        document = complete_document(); document["workload"]["population_plan"] = [10, 15, 20]
        self.assert_code(document, "PROGRESSIVE_EVIDENCE_INCOMPLETE")

    def test_boundary_is_atomic(self):
        document = complete_document(); document["boundary"]["rejected_before_mutation"] = False
        self.assert_code(document, "BOUNDARY_EVIDENCE_INCOMPLETE")

    def test_checked_arithmetic_and_lower_caps(self):
        self.assertEqual(rc.checked_candidate(100, 60, [55, 70]), 55)
        with self.assertRaises(rc.EvidenceError) as caught:
            rc.checked_candidate(rc.U64_MAX, 1, [1])
        self.assertEqual(caught.exception.code, "CHECKED_INTEGER_OVERFLOW")

    def test_synthetic_sources_rejected(self):
        for source in rc.FORBIDDEN_SOURCES:
            document = complete_document(); document["identities"]["capacity_evidence_source"] = source
            document["comparison_baseline_identities"]["capacity_evidence_source"] = source
            self.assert_code(document, "NON_CAPACITY_EVIDENCE_REJECTED")

    def test_json_is_byte_stable(self):
        value = {"z": 1, "a": {"y": 2, "x": 3}}
        self.assertEqual(rc.canonical_bytes(value), b'{"a":{"x":3,"y":2},"z":1}\n')
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "input.json"
            output = pathlib.Path(directory) / "output.json"
            path.write_text(json.dumps({"state": "PLACEHOLDER_UNBOUND", "candidate_M": None}))
            self.assertEqual(rc.main([str(path), "--output", str(output)]), 0)
            self.assertEqual(rc.main([str(path), "--output", str(output), "--check"]), 0)
            output.write_text("{}\n")
            self.assertEqual(rc.main([str(path), "--output", str(output), "--check"]), 2)


if __name__ == "__main__":
    unittest.main()
