# OTV2 DEFECT DISCOVERY TOOL QUALIFIER

Short invocation after canonical merge and after P0-P3 is protected:

```text
Oteryn: defect discovery tools
```

```yaml
prompt_id: OTV2_DEFECT_DISCOVERY_TOOL_QUALIFIER
prompt_version: "1.0"
prompt_mode: DEFECT_DISCOVERY_BOUNDED_TOOL_QUALIFICATION
repository: Oteryn/Oteryn-Game
short_invocation: "Oteryn: defect discovery tools"
merge_authority: false
production_authority: false
cross_repository_write_authority: false
```

## Mission

Qualify optional Defect Discovery tools against real Oteryn needs after P0-P3 is protected. Adopt nothing by reputation alone.

Without an exact merged PoC allocation, operate read-only and return the proposed smallest PoCs.

## Candidate families

Evaluate only candidates material to the allocated problem. Current candidates include Turmoil, Toxiproxy or equivalent, Shuttle, Loom, cargo-mutants, cargo-llvm-cov, Kani, cargo-nextest and Rust CodeQL where applicable.

## Qualification rules

For each allocated candidate:

1. Name the exact Oteryn target and problem it is expected to solve.
2. Prove what real production behavior remains exercised and what the tool replaces or simulates.
3. Measure integration surface, runtime/build cost, reproducibility, evidence quality, and maintenance cost.
4. Demonstrate at least one useful positive and one controlled negative case when practical.
5. Do not claim simulated evidence qualifies the real PostgreSQL/TLS/Tokio/native-client stack.
6. Reject a tool that requires disproportionate production-code distortion purely for testing convenience.
7. Prefer the smallest set of tools that covers distinct evidence needs.

## Output

For every candidate return:

```yaml
tool:
target_problem:
disposition: ADOPT | REJECT | DEFER
tested_sha:
real_behavior_exercised: []
behavior_simulated_or_replaced: []
integration_cost:
runner_cost:
replay_quality:
evidence:
limitations: []
recommended_scope_if_adopted: []
```

Then return exactly one programme next action. A tool disposition grants no new product/write scope. Broad adoption requires a separate exact allocation.
