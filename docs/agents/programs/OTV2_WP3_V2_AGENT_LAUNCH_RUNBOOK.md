# OTV2 WP3-v2 Agent Launch Runbook

Use these aliases as discovery shortcuts. The live repository/task state remains authority.

## Aliases

```text
A0  Oteryn: astra wp3-v2 programme coordinator
A1  Oteryn: astra wp3-v2 architecture lead
A2  Oteryn: sol wp3-v2 evidence auditor
A3  Oteryn: astra platform native evidence hardening
A4  Oteryn: astra wp3-v2 implementation lead
A5  Oteryn: astra child-b durability lead
A6  Oteryn: astra wp5 source composition lead
A7  Oteryn: sol server seam lead
```

A7 is the existing canonical Server Seam alias and must use `docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md` / #247 rather than a replacement prompt or branch.

## Current Gate 1 baseline

Protected `main` contains merged PR #590 and Revision 3 of `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`. The artifact explicitly remains `CANDIDATE / NOT ACCEPTED`; merge/readback alone does not grant A4 implementation authority.

Revision 3 already selects first-slice Option B and freezes the root ownership/overlap and recovery-trigger semantics. A1 must close/validate that exact successor rather than create a second B-vs-C decision. A2 supplies read-only exact-source closure evidence. A4 remains gated on architecture acceptance plus fresh #162/#364 implementation allocation/path custody.

## Launch order

### Start / current architecture-closure wave

Launch A0 as coordinator.

Then, when A0's fresh readback confirms their start conditions, run these as separate sessions:

```text
A1 + A2 + A3
```

A1 closes the protected Revision-3 architecture candidate. A2 is read-only evidence support. A3 targets `Oteryn/Oteryn-Platform` and must start under Platform's own live authority/task rules.

Do not ask A1 to draft another superseding decision unless new material evidence invalidates Revision 3.

### After Revision-3 architecture acceptance + A4 allocation

Launch:

```text
A4
```

Both gates are required:

```text
architecture accepted through repository controls
+
fresh #162/#364 A4 implementation allocation/path custody
```

Protected presence of #590 is insufficient by itself.

A5 may be started only for read-only preparation while shared WP3/Cargo/Durability custody remains unavailable.

### After protected WP3 implementation

Launch/activate in parallel when live custody permits:

```text
A5 + A6 + A3 final interop qualification
```

Do not create a replacement #335 lineage.

### After protected Child B/WP4

Continue:

```text
A6 S2 + S3 + real source composition
```

A6 must establish fresh G0, not merely local source PASS.

### After fresh G0

Launch/resume:

```text
A7
```

A7 resumes `agent/otv2-gameplay-server-seam-01`; do not create a new Server Seam worker.

## Mandatory owner-facing successor footer

Every A0-A7 agent must end its completed or blocked final response with an explicit instruction telling the owner which agent to launch next. The recommendation must be resolved from live state and current gates, not copied mechanically from this runbook.

Required shape:

```text
NEXT_AGENT: <one exact alias, up to three independent aliases, or NONE>
RUN_WHEN: <exact gate/state making the launch valid>
WHY: <one concise dependency reason>
```

Rules:

- use the exact aliases from this runbook;
- never recommend a mutating worker before its live start gate is true;
- when several independent lanes are legal, list at most three in launch order;
- when a blocker prevents launch, state the intended next alias plus the missing gate in `RUN_WHEN`;
- after `FRESH_G0_READY`, A6 should normally point to `Oteryn: sol server seam lead`;
- after `SERVER_SEAM_READY_FOR_INTEGRATION`, A7 must use `NEXT_AGENT: NONE` unless live programme state still requires coordinator action;
- uncertain or cross-lane routing returns to `Oteryn: astra wp3-v2 programme coordinator` rather than guessing.

## Concurrency ceiling

Prefer no more than 2-3 active agents at once. Only one mutating agent may own a material shared path set. Read-only A2 may overlap A1/A4 as evidence support when useful.

## Full programme reference

Use `docs/agents/programs/OTV2_WP3_V2_MULTI_AGENT_DELIVERY_PROGRAMME.md` for the dependency DAG, gates, scope boundaries and terminal criteria.