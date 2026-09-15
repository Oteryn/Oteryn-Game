#!/usr/bin/env python3
from __future__ import annotations

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

SCENARIOS = ("basic", "normal", "stress")
DENSITIES = (32, 64, 128)
MODES = ("atlas", "array", "hybrid")


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    for number, line in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise SystemExit(f"{path}:{number}: invalid JSON: {exc}") from exc
        if not isinstance(row, dict):
            raise SystemExit(f"{path}:{number}: expected JSON object")
        rows.append(row)
    return rows


def nested(row: dict, *keys: str):
    value = row
    for key in keys:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def median(rows: list[dict], *keys: str) -> float | None:
    values = [nested(row, *keys) for row in rows]
    numeric = [float(value) for value in values if isinstance(value, (int, float)) and not isinstance(value, bool)]
    return statistics.median(numeric) if numeric else None

def reliability_ok(row: dict) -> bool:
    reliability = row.get("reliability")
    return isinstance(reliability, dict) and all(
        reliability.get(key) == 0
        for key in ("surface_timeout", "surface_occluded", "surface_outdated", "surface_lost")
    ) and reliability.get("device_loss_detected") is False


def group_primary(rows: list[dict]):
    groups: dict[tuple[str, int, str], list[dict]] = defaultdict(list)
    for row in rows:
        groups[(str(row.get("scenario")), int(row.get("density", 0)), str(row.get("resource_mode")))].append(row)
    return groups


def cell_summary(key, rows):
    return {
        "scenario": key[0],
        "density": key[1],
        "resource_mode": key[2],
        "runs": len(rows),
        "cpu_p50_ms": median(rows, "frame_ms", "p50"),
        "cpu_p95_ms": median(rows, "frame_ms", "p95"),
        "cpu_p99_ms": median(rows, "frame_ms", "p99"),
        "gpu_p50_ms": median(rows, "gpu_frame_ms", "p50"),
        "gpu_p95_ms": median(rows, "gpu_frame_ms", "p95"),
        "gpu_p99_ms": median(rows, "gpu_frame_ms", "p99"),
        "mean_fps": median(rows, "mean_fps"),
        "peak_working_set_mib": (median(rows, "peak_working_set_bytes") or 0.0) / (1024 * 1024),
        "batches_mean": median(rows, "order_preserving_batches", "mean"),
        "visible_mean": median(rows, "visible_primitives", "mean"),
        "cache_evictions": median(rows, "cache", "evictions"),
        "cache_uploads": median(rows, "cache", "uploads"),
        "overflow_fallbacks": median(rows, "cache", "overflow_fallbacks"),
        "first_frame_ms": median(rows, "first_frame_ms"),
        "pipeline_prewarm_ms": median(rows, "pipeline_prewarm_ms"),
        "scroll_p95_ms": median(rows, "camera_scroll_stall_ms", "p95"),
        "zoom_p95_ms": median(rows, "zoom_change_stall_ms", "p95"),
        "floor_p95_ms": median(rows, "floor_transition_stall_ms", "p95"),
    }

def resource_layout_verdict(groups):
    comparisons = []
    atlas_wins = 0
    array_wins = 0
    hybrid_slow = 0
    hybrid_overflow = 0
    for scenario in SCENARIOS:
        for density in DENSITIES:
            atlas = groups.get((scenario, density, "atlas"), [])
            array = groups.get((scenario, density, "array"), [])
            hybrid = groups.get((scenario, density, "hybrid"), [])
            a_cpu = median(atlas, "frame_ms", "p95")
            r_cpu = median(array, "frame_ms", "p95")
            h_cpu = median(hybrid, "frame_ms", "p95")
            a_gpu = median(atlas, "gpu_frame_ms", "p95")
            r_gpu = median(array, "gpu_frame_ms", "p95")
            h_gpu = median(hybrid, "gpu_frame_ms", "p95")
            if None in (a_cpu, r_cpu, h_cpu, a_gpu, r_gpu, h_gpu):
                continue
            if r_cpu < a_cpu * 0.95 and r_gpu < a_gpu * 0.95:
                array_wins += 1
            if a_cpu < r_cpu * 0.95 and a_gpu < r_gpu * 0.95:
                atlas_wins += 1
            best_cpu = min(a_cpu, r_cpu)
            best_gpu = min(a_gpu, r_gpu)
            if h_cpu > best_cpu * 1.25 and h_gpu > best_gpu * 1.25:
                hybrid_slow += 1
            overflow = sum(int(nested(row, "cache", "overflow_fallbacks") or 0) for row in hybrid)
            if overflow > 0:
                hybrid_overflow += 1
            comparisons.append({
                "scenario": scenario, "density": density,
                "atlas_cpu_p95_ms": a_cpu, "array_cpu_p95_ms": r_cpu, "hybrid_cpu_p95_ms": h_cpu,
                "atlas_gpu_p95_ms": a_gpu, "array_gpu_p95_ms": r_gpu, "hybrid_gpu_p95_ms": h_gpu,
                "hybrid_overflow_fallbacks": overflow,
            })
    if atlas_wins >= 7 and array_wins == 0:
        primary = {"verdict": "ADOPT", "choice": "atlas_pages"}
    elif array_wins >= 7 and atlas_wins == 0:
        primary = {"verdict": "ADOPT", "choice": "texture_arrays"}
    else:
        primary = {"verdict": "INSUFFICIENT_EVIDENCE", "choice": None}
    hybrid = "REJECT" if hybrid_slow >= 7 or hybrid_overflow >= 3 else "INSUFFICIENT_EVIDENCE"
    return {**primary, "hybrid_challenger": hybrid, "comparisons": comparisons}

def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: analyze-results.py RAW.jsonl FAMILY.jsonl SUMMARY.json")
    raw_path, family_path, output_path = map(Path, sys.argv[1:])
    rows = load_jsonl(raw_path)
    family_rows = load_jsonl(family_path)
    groups = group_primary(rows)
    expected = {(s, d, m) for s in SCENARIOS for d in DENSITIES for m in MODES}
    matrix_complete = len(rows) == 81 and set(groups) == expected and all(len(groups[key]) == 3 for key in expected)
    fixed_family = {row.get("presentation_family") for row in rows} == {"enhanced"}
    workloads = {str(row.get("workload")) for row in rows + family_rows}
    workload_consistent = len(workloads) == 1
    real_atlas_workload = workloads == {"real_atlas_fullworld_slice"}
    commit_shas = {str(row.get("commit_sha")) for row in rows + family_rows if row.get("commit_sha")}
    exact_head_consistent = len(commit_shas) == 1
    gpu_reliable = all(row.get("gpu_timestamp_reliable") is True and isinstance(row.get("gpu_frame_ms"), dict) for row in rows)
    reliability = all(reliability_ok(row) for row in rows + family_rows)
    family_signatures = {str(row.get("gameplay_signature_xor")) for row in family_rows}
    family_semantics_equal = len(family_rows) == 3 and len(family_signatures) == 1
    families_seen = {row.get("presentation_family") for row in family_rows}
    family_axis_complete = families_seen == {"classic", "enhanced", "hd"}
    churn = any((nested(row, "cache", "evictions") or 0) > 0 and (nested(row, "cache", "uploads") or 0) > 0 for row in rows)
    churn_cells = sum(
        1 for cell_rows in groups.values()
        if any((nested(row, "cache", "evictions") or 0) > 0 and (nested(row, "cache", "uploads") or 0) > 0 for row in cell_rows)
    )
    overflow_free = all((nested(row, "cache", "overflow_fallbacks") or 0) == 0 for row in rows + family_rows)
    readability = all((nested(row, "max_semantics", "critical_vfx") or 0) > 0 for row in rows)
    multi_page = all((nested(row, "active_resource_pages", "max") or 0) > 1 for row in rows)
    cells = [cell_summary(key, groups[key]) for key in sorted(groups)]
    layout = resource_layout_verdict(groups)
    verdicts = {
        "atlas_vs_texture_arrays": layout,
        "ktx2_vs_dds_or_other_runtime_container": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "decoded GPU page layout was measured; container IO/decode/transcode challengers were not",
        },
        "streaming_cache_model": {
            "verdict": "ADOPT" if real_atlas_workload and churn and reliability and multi_page and overflow_free else "INSUFFICIENT_EVIDENCE",
            "choice": "bounded_visible_working_set_with_eviction" if real_atlas_workload and churn and reliability and multi_page and overflow_free else None,
            "reason": "real Atlas FullWorld viewport semantics drove multi-page upload/eviction churn without renderer failure or resource overflow" if real_atlas_workload and churn and reliability and multi_page and overflow_free else "real-world churn/reliability/overflow evidence incomplete",
        },
        "particle_implementation_direction": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "CPU-prepared instanced particles were exercised without a compute/GPU-simulation challenger",
        },
        "light_vfx_budgets": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "physical cost/readability counts exist but no accepted cross-platform product frame budget exists",
        },
        "renderer_batching_thresholds": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "order-preserving batching was measured without a threshold challenger matrix",
        },
        "ram_budget": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "Molehill process working-set evidence alone cannot freeze a production cross-platform RAM budget",
        },
        "vram_budget": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "no trustworthy per-process VRAM counter was available",
        },
        "filtering_mipmap_direction": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "reason": "Classic nearest and Enhanced/HD linear sampling are exercised but no mip/filter challenger matrix is complete",
        },
    }
    report = {
        "report": "oteryn-world-vfx-prototype-physical-summary-v3",
        "primary_runs": len(rows),
        "family_smoke_runs": len(family_rows),
        "matrix_complete": matrix_complete,
        "primary_family_fixed": fixed_family,
        "workloads": sorted(workloads),
        "workload_consistent": workload_consistent,
        "real_atlas_workload": real_atlas_workload,
        "exact_head_consistent": exact_head_consistent,
        "commit_shas": sorted(commit_shas),
        "gpu_timestamp_reliable": gpu_reliable,
        "reliability_pass": reliability,
        "readability_pass": readability,
        "multi_page_pass": multi_page,
        "cache_churn_exercised": churn,
        "cache_churn_cells": churn_cells,
        "overflow_fallback_free": overflow_free,
        "family_axis_complete": family_axis_complete,
        "family_gameplay_semantics_equal": family_semantics_equal,
        "family_signatures": sorted(family_signatures),
        "cells": cells,
        "verdicts": verdicts,
    }
    output_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    markdown = [
        "# Oteryn World + VFX Prototype â€” physical summary",
        "",
        f"Primary matrix: **{len(rows)} runs**, complete={matrix_complete}, fixed Enhanced={fixed_family}.",
        f"Workload={','.join(sorted(workloads))}; real Atlas={real_atlas_workload}; exact-head consistent={exact_head_consistent}.",
        f"GPU timestamps reliable={gpu_reliable}; reliability pass={reliability}; cache churn={churn} across {churn_cells}/27 cells; overflow-free={overflow_free}.",
        f"Classic/Enhanced/HD independent family smoke: complete={family_axis_complete}, gameplay signatures equal={family_semantics_equal}.",
        "",
        "| Scenario | Density | Mode | CPU p95 ms | GPU p95 ms | FPS | RAM MiB | Batches mean | Evictions | Overflow |",
        "|---|---:|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for cell in cells:
        markdown.append(
            "| {scenario} | {density} | {resource_mode} | {cpu_p95_ms:.4f} | {gpu_p95_ms:.4f} | {mean_fps:.1f} | {peak_working_set_mib:.1f} | {batches_mean:.1f} | {cache_evictions:.0f} | {overflow_fallbacks:.0f} |".format(**cell)
        )
    markdown += ["", "## Verdicts", ""]
    for topic, verdict in verdicts.items():
        choice = verdict.get("choice")
        suffix = f" â€” {choice}" if choice else ""
        markdown.append(f"- **{topic}**: `{verdict['verdict']}`{suffix}")
        if verdict.get("reason"):
            markdown.append(f"  - {verdict['reason']}")
        if topic == "atlas_vs_texture_arrays" and verdict.get("hybrid_challenger"):
            markdown.append(f"  - hybrid challenger: `{verdict['hybrid_challenger']}`")
    output_path.with_name("SUMMARY.md").write_text("\n".join(markdown) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
