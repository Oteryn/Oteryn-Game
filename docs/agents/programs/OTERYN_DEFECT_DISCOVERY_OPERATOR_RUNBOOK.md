# Oteryn Defect Discovery Operator Runbook

Status of this package: staging artifact until the prompt files and lifecycle entries are merged to protected `main`.

## Aliases after canonical merge

| Role | Alias | When to use |
|---|---|---|
| Technical supervisor | `Oteryn: defect discovery supervisor` | Read-only readiness, scope, collision and next-allocation proposal. |
| First proof implementation | `Oteryn: defect discovery p0-p3` | Only after #162 has merged an exact P0-P3 allocation. |
| Tool qualifier | `Oteryn: defect discovery tools` | After P0-P3 is protected and a bounded P4/P5 PoC allocation exists. |
| Product-module lead | `Oteryn: defect discovery <module>` | After core P0-P3 is protected and #162 has merged the exact module allocation. |

Supported initial `<module>` values: `simulation`, `protocol`, `ability`, `interaction`, `ai`, `foundation`, `durability`, `postgres`, `content`, `client`.

Alias existence never grants a write lease or starts a second control plane.

## Launch order

### Phase A — before implementation allocation

Run:

```text
Oteryn: defect discovery supervisor
```

Expected result: one read-only packet. If P0-P3 is ready, it returns an exact allocation proposal to #162. It does not create the allocation itself.

### Phase B — first implementation

After #162 merges a P0-P3 allocation naming the exact branch and paths, run in a separate implementation chat:

```text
Oteryn: defect discovery p0-p3
```

Keep one writer for that branch. The lead implements only P0-P3 and returns an exact-head qualification packet.

### Phase C — optional tool qualification

After protected P0-P3 completion, if the next allocation is about real-stack or concurrency tooling:

```text
Oteryn: defect discovery tools
```

This role must return evidence-backed `ADOPT / REJECT / DEFER`; it must not adopt the whole candidate list.

### Phase D — module rollout

For one allocated module, for example:

```text
Oteryn: defect discovery protocol
```

or:

```text
Oteryn: defect discovery durability
```

Do not launch a module merely because its alias exists. Resolve the exact current #162 allocation first.

Multiple module leads may run in parallel only when the active control plane proves their paths/shared resources are disjoint.

## Intended manual GitHub execution after implementation

The implementation target is a trusted workflow on protected `main` with manual dispatch. The tested code ref is an input, not the workflow definition itself.

Intended owner flow:

```text
GitHub → Actions → Oteryn Defect Discovery → Run workflow
```

Expected inputs after the workflow exists:

- `target_ref`: exact branch, tag or SHA to test;
- `module`: one supported product module or a reviewed composed suite;
- `suite`: module-defined suite such as property, fuzz, state, recovery or replay;
- `profile`: `quick | standard | deep | soak`;
- optional replay/finding selector where the suite supports it.

The exact UI/field names are an implementation contract horizon until P0-P3 creates and validates the workflow; they are not claims that the workflow already exists.

## Ordinary PR behavior

Opening or pushing an ordinary PR must not automatically run heavy Defect Discovery. Merge Queue must not implicitly expand into fuzz/deep/soak campaigns.

A small deterministic reproducer may later become an ordinary required regression only through a separate reviewed promotion change.

## Result interpretation

Do not collapse `PRODUCT_FAILURE`, `HARNESS_FAILURE`, `INFRA_FAILURE`, `UNSUPPORTED_CAPABILITY`, and `INCOMPLETE_CAMPAIGN`.

A campaign that executed its declared scope without an invariant violation may report success for that bounded scope; it must not claim the product is globally bug-free.

## Replay

Retain the actual input/operation trace/schedule and relevant environment identity. A seed is auxiliary.

Historical replay asks whether the original case reproduces on the original target. Fix verification asks whether the same semantic defect remains on the repaired target; exact low-level schedules may legitimately change after implementation changes.
