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

## Launch order

### Start first

Launch A0 as coordinator.

Then launch these three as separate sessions when A0's fresh readback confirms their start conditions:

```text
A1 + A2 + A3
```

A1 and A2 work on the WP3 decision/evidence split. A2 is read-only. A3 targets `Oteryn/Oteryn-Platform` and must start under Platform's own live authority/task rules.

### After WP3-v2 architecture acceptance/allocation

Launch:

```text
A4
```

Do not launch A4 as a mutating worker before Gate 1.

A5 may be started only for read-only preparation while shared WP3/Cargo/Durability custody remains unavailable.

### After protected WP3

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

## Concurrency ceiling

Prefer no more than 2-3 active agents at once. Only one mutating agent may own a material shared path set. Read-only A2 may overlap A1/A4 as evidence support when useful.

## Full programme reference

Use `docs/agents/programs/OTV2_WP3_V2_MULTI_AGENT_DELIVERY_PROGRAMME.md` for the dependency DAG, gates, scope boundaries and terminal criteria.