# Codex Cloud helpers

These scripts are operational helpers for the Oteryn-Game Codex Cloud environment. They do not grant task or repository authority.

For the long-running WP3 SQLx/rustls/Tokio lane configure the Codex Cloud environment with:

```text
Setup:       bash tools/codex/cloud-wp3-setup.sh
Maintenance: bash tools/codex/cloud-wp3-maintenance.sh
```

The setup intentionally warms only the four WP3 dependency manifests for `x86_64-unknown-linux-gnu`. The maintenance command intentionally performs no dependency fetch or workspace build.

See `docs/agents/CODEX_CLOUD_WP3_STABILITY_PROFILE.md` for the atomic-invocation and failure-handling profile.
