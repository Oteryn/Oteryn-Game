#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

SCENARIOS = ("basic", "normal", "stress")
DENSITIES = (32, 64, 128)
LAYOUTS = ("atlas", "array")


def load_jsonl(path: Path) -> list[dict]:
    rows: list[dict] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise SystemExit(f"{path}:{line_number}: invalid JSON: {exc}") from exc
        if not isinstance(value, dict):
            raise SystemExit(f"{path}:{line_number}: record is not an object")
        rows.append(value)
    return rows


def median_path(rows: list[dict], *path: str) -> float | None:
    values: list[float] = []
    for row in rows:
        current: object = row
        for key in path:
            if not isinstance(current, dict) or key not in current:
                current = None
                break
            current = current[key]
        if isinstance(current, (int, float)) and not isinstance(current, bool):
            values.append(float(current))
    return statistics.median(values) if values else None


def reliability_ok(row: dict) -> bool:
    reliability = row.get("reliability")
    if not isinstance(reliability, dict):
        return False
    return all(reliability.get(key) == 0 for key in (
        "surface_lost",
        "surface_outdated",
        "surface_timeout",
        "surface_occluded",
    )) and reliability.get("device_loss_detected") is False


def full_matrix(rows: list[dict]) -> bool:
    observed = {(row.get("scenario"), row.get("density"), row.get("layout")) for row in rows}
    expected = {(s, d, l) for s in SCENARIOS for d in DENSITIES for l in LAYOUTS}
    return expected.issubset(observed)


def atlas_array_verdict(groups: dict[tuple[str, int, str], list[dict]]) -> dict:
    evidence: list[dict] = []
    array_wins = 0
    atlas_wins = 0
    comparable = 0
    for scenario in SCENARIOS:
        for density in DENSITIES:
            atlas = groups.get((scenario, density, "atlas"), [])
            array = groups.get((scenario, density, "array"), [])
            a_cpu = median_path(atlas, "cpu_frame_ms", "p95")
            r_cpu = median_path(array, "cpu_frame_ms", "p95")
            a_gpu = median_path(atlas, "gpu_frame_ms", "summary", "p95")
            r_gpu = median_path(array, "gpu_frame_ms", "summary", "p95")
            if None in (a_cpu, r_cpu, a_gpu, r_gpu):
                continue
            comparable += 1
            assert a_cpu is not None and r_cpu is not None and a_gpu is not None and r_gpu is not None
            cpu_ratio = r_cpu / a_cpu if a_cpu else 0.0
            gpu_ratio = r_gpu / a_gpu if a_gpu else 0.0
            if cpu_ratio < 0.95 and gpu_ratio < 0.95:
                array_wins += 1
            elif cpu_ratio > 1.05 and gpu_ratio > 1.05:
                atlas_wins += 1
            evidence.append({
                "scenario": scenario,
                "density": density,
                "atlas_cpu_p95_ms": a_cpu,
                "array_cpu_p95_ms": r_cpu,
                "atlas_gpu_p95_ms": a_gpu,
                "array_gpu_p95_ms": r_gpu,
            })
    if comparable == 9 and array_wins >= 7 and atlas_wins == 0:
        return {"verdict": "ADOPT", "choice": "texture_arrays", "evidence": evidence}
    if comparable == 9 and atlas_wins >= 7 and array_wins == 0:
        return {"verdict": "ADOPT", "choice": "atlas_pages", "evidence": evidence}
    return {"verdict": "INSUFFICIENT_EVIDENCE", "choice": None, "evidence": evidence}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("family_smoke", type=Path)
    parser.add_argument("cold_prewarm", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    rows = load_jsonl(args.raw)
    family_rows = load_jsonl(args.family_smoke)
    prewarm_rows = load_jsonl(args.cold_prewarm)
    groups: dict[tuple[str, int, str], list[dict]] = defaultdict(list)
    for row in rows:
        groups[(str(row.get("scenario")), int(row.get("density", 0)), str(row.get("layout")))].append(row)

    cells = []
    for key in sorted(groups):
        group = groups[key]
        cells.append({
            "scenario": key[0],
            "density": key[1],
            "layout": key[2],
            "runs": len(group),
            "cpu_p50_ms_median": median_path(group, "cpu_frame_ms", "p50"),
            "cpu_p95_ms_median": median_path(group, "cpu_frame_ms", "p95"),
            "cpu_p99_ms_median": median_path(group, "cpu_frame_ms", "p99"),
            "gpu_p95_ms_median": median_path(group, "gpu_frame_ms", "summary", "p95"),
            "mean_fps_median": median_path(group, "mean_fps"),
            "peak_working_set_bytes_median": median_path(group, "host", "peak_working_set_bytes"),
            "batches_mean_median": median_path(group, "draw_batches", "mean"),
            "visible_mean_median": median_path(group, "visible_primitives", "mean"),
            "cache_evictions_median": median_path(group, "cache", "evictions"),
            "upload_p95_ms_median": median_path(group, "cache", "upload_ms", "p95"),
            "scroll_stall_p95_ms_median": median_path(group, "camera_scroll_stall_ms", "p95"),
        })

    signatures = {str(row.get("gameplay_signature_xor")) for row in family_rows}
    family_semantics_equal = len(signatures) == 1 and len(family_rows) >= 3
    reliability = all(reliability_ok(row) for row in rows + family_rows + prewarm_rows)
    readability = all(
        isinstance(row.get("readability"), dict)
        and row["readability"].get("critical_visible_min", 0) > 0
        for row in rows
    )
    environment = all(
        isinstance(row.get("environment_seen"), dict)
        and row["environment_seen"].get("rain") is True
        and row["environment_seen"].get("snow") is True
        and row["environment_seen"].get("winter") is True
        for row in rows
    )
    churn = any(
        isinstance(row.get("cache"), dict)
        and row["cache"].get("evictions", 0) > 0
        and row["cache"].get("upload_bytes", 0) > 0
        for row in rows
    )

    verdicts = {
        "atlas_vs_texture_arrays_vs_hybrid": atlas_array_verdict(groups),
        "ktx2_vs_dds_or_other_container": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "prototype benchmarks decoded GPU pages; it does not benchmark runtime container decode/transcode/IO",
        },
        "streaming_cache_model": {
            "verdict": "ADOPT" if churn and reliability else "INSUFFICIENT_EVIDENCE",
            "choice": "bounded_visible_working_set_with_lru_style_eviction" if churn and reliability else None,
            "reason": "physical camera-driven upload/eviction churn completed without renderer failure" if churn and reliability else "churn or reliability evidence incomplete",
        },
        "particle_implementation_direction": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "only CPU-prepared instanced particles were exercised; no compute/GPU-simulation challenger was measured",
        },
        "light_vfx_budgets": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "prototype records active counts and frame cost but no accepted product frame-time/readability budget exists",
        },
        "renderer_batching_thresholds": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "order-preserving contiguous-state batching is measured, but no threshold challenger matrix was run",
        },
        "ram_budget": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "one physical host can establish working-set evidence, not a cross-platform production budget",
        },
        "vram_budget": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "no trustworthy per-process VRAM counter is accepted by this harness",
        },
        "filtering_mipmap_direction": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "Classic nearest and Enhanced/HD linear sampling are exercised, but mip/filter alternatives are not qualified",
        },
    }

    output = {
        "report": "oteryn-world-vfx-prototype-physical-summary-v1",
        "matrix_complete": full_matrix(rows),
        "raw_runs": len(rows),
        "family_smoke_runs": len(family_rows),
        "prewarm_runs": len(prewarm_rows),
        "reliability_pass": reliability,
        "readability_policy_pass": readability,
        "environment_coverage_pass": environment,
        "family_gameplay_semantics_equal": family_semantics_equal,
        "cache_churn_exercised": churn,
        "cells": cells,
        "family_signatures": sorted(signatures),
        "cold_prewarm": prewarm_rows,
        "verdicts": verdicts,
    }
    args.output.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
