# Neutral documentation lane canary

This file is a one-purpose CI canary for issue #580.

It intentionally changes documentation only and must not require Rust Linux or Windows execution lanes when the protected-base classifier recognizes neutral documentation correctly.

Expected trusted-base classification:

- `surface=docs`
- `reason=neutral-documentation`
- `rust=false`
- `windows=false`

No runtime, Cargo, workflow, protection, Merge Queue, or product behavior change is intended.
