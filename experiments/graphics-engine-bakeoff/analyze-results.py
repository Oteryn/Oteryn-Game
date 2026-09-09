#!/usr/bin/env python3
"""Aggregate physical graphics bake-off JSONL without third-party dependencies."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

METRICS = (
    "frame_mean_ms",
    "frame_p50_ms",
    "frame_p95_ms",
    "frame_p99_ms",
    "mean_fps",
    "prep_mean_ms",
    "prep_p95_ms",
    "startup_ms",
    "peak_working_set_bytes",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("--build", type=Path)
    parser.add_argument("--json-out", type=Path)
    return parser.parse_args()


def median(values: list[float]) -> float | None:
    if not values:
        return None
    return float(statistics.median(values))


def aggregate(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    groups: dict[tuple[str, str, int], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        key = (record["backend"], record["scenario"], int(record["sprite_px"]))
        groups[key].append(record)

    rows: list[dict[str, Any]] = []
    for (backend, scenario, sprite_px), group in sorted(groups.items()):
        row: dict[str, Any] = {
            "backend": backend,
            "scenario": scenario,
            "sprite_px": sprite_px,
            "runs": len(group),
        }
        for metric in METRICS:
            values = [float(item[metric]) for item in group if item.get(metric) is not None]
            row[f"median_{metric}"] = median(values)
        adapters = sorted(
            {
                str(item.get("adapter_log") or item.get("adapter") or "unknown")
                for item in group
            }
        )
        row["adapter_evidence"] = adapters
        rows.append(row)
    return rows


def fmt(value: float | None, digits: int = 3) -> str:
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}"


def print_markdown(rows: list[dict[str, Any]], build: dict[str, Any] | None) -> None:
    print("| Backend | Scenario | px | Runs | p50 ms | p95 ms | p99 ms | Mean FPS | RAM MiB | Startup ms |")
    print("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for row in rows:
        ram = row["median_peak_working_set_bytes"]
        ram_mib = None if ram is None else ram / (1024 * 1024)
        print(
            "| {backend} | {scenario} | {sprite_px} | {runs} | {p50} | {p95} | {p99} | {fps} | {ram} | {startup} |".format(
                backend=row["backend"],
                scenario=row["scenario"],
                sprite_px=row["sprite_px"],
                runs=row["runs"],
                p50=fmt(row["median_frame_p50_ms"]),
                p95=fmt(row["median_frame_p95_ms"]),
                p99=fmt(row["median_frame_p99_ms"]),
                fps=fmt(row["median_mean_fps"], 1),
                ram=fmt(ram_mib, 1),
                startup=fmt(row["median_startup_ms"], 1),
            )
        )

    by_cell = {(row["scenario"], row["sprite_px"], row["backend"]): row for row in rows}
    print("\n| Scenario | px | Bevy/custom p95 | Bevy/custom p99 | custom/Bevy FPS |")
    print("| --- | ---: | ---: | ---: | ---: |")
    for scenario in ("basic", "normal", "stress"):
        for sprite_px in (32, 64, 128):
            custom = by_cell.get((scenario, sprite_px, "custom-wgpu-30.0.0"))
            bevy = by_cell.get((scenario, sprite_px, "bevy-0.19.1"))
            if custom is None or bevy is None:
                continue
            p95_ratio = bevy["median_frame_p95_ms"] / custom["median_frame_p95_ms"]
            p99_ratio = bevy["median_frame_p99_ms"] / custom["median_frame_p99_ms"]
            fps_ratio = custom["median_mean_fps"] / bevy["median_mean_fps"]
            print(
                f"| {scenario} | {sprite_px} | {p95_ratio:.2f}x | {p99_ratio:.2f}x | {fps_ratio:.2f}x |"
            )

    if build is not None:
        print("\nBuild evidence:")
        print(json.dumps(build, indent=2, ensure_ascii=False))


def main() -> None:
    args = parse_args()
    records = [json.loads(line) for line in args.raw.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    rows = aggregate(records)
    build = None
    if args.build is not None:
        build = json.loads(args.build.read_text(encoding="utf-8-sig"))
    print_markdown(rows, build)
    if args.json_out is not None:
        args.json_out.write_text(json.dumps(rows, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
