# OTV2-20260930-mq-server-only-windows

```yaml
task_id: OTV2-20260930-mq-server-only-windows
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
owner_authorization: D248 (#162 comment 5919785239)
packet: "#162 comment 5919546372"
stage_a:
  pr: 1421
  branch: claude/mq-server-only-windows-a
  change: merge-authority-audit.yml EXPECTED_MERGE_GROUP_GATE_BLOB 82b0d535 -> 9e7f083a
stage_b:
  pr: 1422
  branch: claude/mq-server-only-windows-b
  gate_blob: 9e7f083a7e2a58a1d03910d90a9423dd7ec5478c
writer: control plane session_018szkUA14PiwXp5JWX1zdx4
jira: KAN-20 (parent KAN-9)
ownership_released: on merge of #1422
```

## Scope

The Merge Queue deselects only Windows for a synthetic candidate that the protected-base exact-consumer classifier proves server-only (`rust=True`, `windows=False`, reason `server-only-exact-consumer-closure`, surface `server` or `durability`). Linux, PostgreSQL 17.6, supply chain and the always-required gates stay selected. Every other result, and incomplete or malformed evidence, stays FULL. Node-boot and Server-Seam selection is unchanged. Merge Queue settings 5/1 are unchanged.

## Validation

- `DERIVED`: the packet author ran the routing canaries RED on the old gate and GREEN on the new one. The aggregate tests, the queue credibility suite (28 workflow mutations, lifecycle discovery, 4 PowerShell native failure positions), the classifier fixtures, repository policy and governance all passed. Hosts: Windows for the PowerShell canaries, Ubuntu 24.04 for the rest.
- `PROVEN`: the control plane checked the patches against `main@c70d899a`. They apply cleanly, the gate blob is `9e7f083a`, and the governance and repository-policy validators, `tools/agents/tests` and the three lane-classifier tests pass. The `*_pg_sim` suites need `pwsh` and are proven by exact-head CI.
- Review: Codex on each exact frozen head (D245). Integration: Merge Queue (D247), stage A first.

## Follow-up

After #1422 merges, verify with the first real server-only merge-group run that Linux and PostgreSQL were required and Windows was skipped. Also verify that a mixed or shared change stays FULL.
