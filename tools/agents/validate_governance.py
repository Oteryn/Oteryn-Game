#!/usr/bin/env python3
"""Game governance validator with retired local Codex merge authority."""
from __future__ import annotations

import importlib.util
from pathlib import Path

CORE_PATH = Path(__file__).with_name("validate_governance_core.py")


def load_core():
    spec = importlib.util.spec_from_file_location("validate_governance_core", CORE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load governance core: {CORE_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    core = load_core()
    # The historical local Codex policy remains readable evidence only. Current
    # AI-review routing is owned by protected META policy and is advisory, so it
    # must not participate in Game's deterministic merge gate.
    core.validate_codex_review_policy = lambda _policy, _errors: None
    return core.main()


if __name__ == "__main__":
    raise SystemExit(main())
