# OTV2-20261004-client-neg-1

```yaml
task_id: OTV2-20261004-client-neg-1
title: Client capability negotiation (CAP-NEG-1 client side)
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/otv2-20261004-client-neg-1
pr: 1734
owned_paths:
  - crates/session/**
  - crates/session-tcp/**
  - tools/dev-client/**
  - docs/agents/tasks/archive/OTV2-20261004-client-neg-1.md
```

## Result

`Admission.supported_capabilities` (default `[13]`), kept selected set, resume never widens it, capability 13 `TOO_EARLY` with backoff retry, generic gated snapshot/command/delta routing. `apps/client/**` unchanged.

## Validation

- cargo fmt --check: pass
- cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-session-tcp --all-targets -- -D warnings: pass
- cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-session-tcp: pass
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass
