# OTV2 DEFECT DISCOVERY MODULE LEAD

Parameterized short invocation after canonical merge and after the core P0-P3 proof is protected:

```text
Oteryn: defect discovery <module>
```

Supported initial product modules:

```text
simulation
protocol
ability
interaction
ai
foundation
durability
postgres
content
client
```

A module name selects product scope only. Discovery method and execution environment come from the live allocation and module capability contract.

```yaml
prompt_id: OTV2_DEFECT_DISCOVERY_MODULE_LEAD
prompt_version: "1.0"
prompt_mode: DEFECT_DISCOVERY_MODULE_IMPLEMENTATION
repository: Oteryn/Oteryn-Game
short_invocation_pattern: "Oteryn: defect discovery <module>"
merge_authority: false
production_authority: false
cross_repository_write_authority: false
```

## Mission

Extend the protected Defect Discovery core for exactly one allocated product module. Reuse the common finding/replay/corpus contracts and do not create a second dispatcher, reporter, oracle schema, or E2E platform.

Without a current merged allocation naming the module and exact writable paths, remain read-only and return `WAITING_ALLOCATION`.

## Module contract

Before implementation, resolve and record:

```yaml
module:
target_entrypoints: []
accepted_contract_sources: []
methods: []
environment:
  real_postgres:
  real_tls:
  windows:
  native_client:
  network_simulation:
  special_toolchain:
required_capabilities: []
explicitly_not_proven: []
campaign_profiles: [quick, standard, deep, soak]
```

Only expose campaign/profile combinations that have truthful prerequisites. Missing required capability is never a silent PASS.

## Discovery principles

- Generate semantically valid base states before applying controlled negative mutations where the property requires it.
- Prefer independent reference/state models and metamorphic relations over duplicating the implementation under test.
- Preserve positive/liveness controls so a fail-closed implementation that rejects everything cannot appear correct.
- Record state/transition/oracle/fault coverage where meaningful; line coverage is diagnostic only.
- For concurrency, retain the actual operation/schedule trace in addition to any seed.
- For parsers/artifacts, keep malformed/raw and semantically valid/deep campaigns separate.
- For persistence, distinguish real PostgreSQL/TLS qualification from simulation.
- Do not automatically make randomized/deep/soak campaigns required PR gates.
- A confirmed stable minimal reproducer may be proposed for regression promotion through a separate reviewed change.

## Finding handling

Do not repair a found product defect unless a separate exact repair allocation names the owning product paths. Preserve the finding, reproducer, affected contract/oracle, tested SHA, environment, and next safe routing target.

Do not automatically open Issues unless the current reporter policy/allocation explicitly allows it.

## Completion packet

```yaml
state: READY_FOR_INTEGRATION_REVIEW | WAITING_ALLOCATION | WAITING_DEPENDENCY | BLOCKED_CAPABILITY_UNAVAILABLE | SHARED_LEASE_REQUIRED | ARCHITECTURE_ESCALATION_REQUIRED | POLICY_CONFLICT
module:
head_sha:
methods_qualified: []
environments_qualified: []
oracles_qualified: []
positive_controls:
negative_controls:
replay_proven:
known_limitations: []
findings: []
regression_candidates: []
next_action: <exactly one concrete action>
```
