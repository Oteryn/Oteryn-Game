# Partial Cyclopedia Item observations, 13 November 2026

This packet retains observed fields and qualified gaps from Tibia Test Server
15.33.dd0955. It changes no Item definition, runtime, pin, protocol or workflow.
Screenshots remain private on the owner's Ubuntu machine. Relative proof keys
and SHA256 identify those bytes; no images, access configuration or credentials
are included. Exact names and observed field literals are preserved.

## What was actually read

| Slice | Qualified scope | Remaining boundary |
|---|---|---|
| Creature Products | Basic cards at physical UI ordinals1–134;404 proof hashes | Partial category; no end, complete trade or animation qualification |
| Armors |33 Basic cards requalified from75 newly read pages;141 proof hashes | Global physical ordinals UNKNOWN; prior manually read names retained with explicit provenance |
| Boots and Containers trade |176 retained initial panels:67 Boots and109 Containers | Initial panes only; pagination completeness is unqualified |

The trade checklist identifies19 cards requiring21 additional vertical panels,
one clipped horizontal label and two NPC names whose exact text remains unknown.
Visible empty initial panes are observations, never N/A or proof of no offers.
Only complete visible offer lines were transcribed; clipped lines were not
invented. Sell To and Buy From remain separate, including literal currency and
residence text. These observations are not executable offers or market snapshots.

The category backlog preserves Armors observed lower bound162 and Amulets lower
bound89 without inventing global ordinals. Same-name rain coat and wolf tooth
chain rows remain separate observations. Basic completeness, trade pagination,
animation and identity mapping are independent qualifications.

## Source comparison, without an ID claim

`compare_sources.py` reproduces the comparison against Game source
`5efd67da164ac9dc33c8c066434b228cebb86c2b`:69 canonical shards,
three manifest-selected profile inputs and the manifest itself (73 receipts).
It compares only Weight and Tradeable In Market, retaining every other observed
Basic field without speculative conversion. Known source weight integers are
hundredths of an ounce, as formatted by the pinned inspection implementation.

Across167 observations,141 have one name candidate,22 have none and four have
multiple candidates. Even a unique name match remains an **unqualified name
correspondence**, not a read client ID, appearance mapping or serverID. Candidate
field comparisons yield286 equal,11 unknown/unqualified and five different
values. These are candidate comparisons, not counts of proven correct Items.

The five weight differences involve amber souvenir, badger fur, bed of nails,
bloodshot giant eye and broken helmet. They remain source/version/identity
questions, not demonstrated gameplay defects. The comparison records all name
candidates, source states, selected membership and literal admission flags;
none establishes actual runtime admission. No absent field becomes zero, false,
empty or NOT_APPLICABLE. Physical UI ordinal, appearance ID, Cyclopedia ID,
source namespace and native definition reference remain distinct.

## Provenance and the legacy offset correction

Custody receipts identify the exact private source JSON before sanitization.
Only absolute owner-root prefixes were replaced with relative proof keys.
All404 Creature Products,141 Armors and176 initial-trade PNG hashes were
rechecked against private bytes on Ubuntu during preparation (721 unique files).
`packet-integrity.json` seals the sanitized repository JSON independently.
Creature Products proof keys resolve below the private `outputs` directory;
trade proof paths resolve below the private handoff root.

A separate offline calibration audited the original Creature Products collector:
one consecutive list transition, pages0143→0144 while capturing ordinal83,
moved24 pixels rather than the usual20. The legacy counter omitted4 pixels of
capture-induced movement. Every one of the134 recorded physical ordinals still
recomputes correctly from the measured screenshot chain. Original index labels
were retained. The old target's recorded offset4720 corresponds to measured
4724; the stopped restore frame independently measures4720. The private sidecar
receipt and its hash are recorded in `custody-validation.json`.

The new local collector checks list layout after capture and holds on unexpected
movement. This packet does not claim that its autonomous GUI lifecycle has
completed a real cycle. Correcting a physical scroll coordinate does not infer
Item identity or alter the observed Basic fields.

## Validation

Run from repository root:

```text
python docs/agents/evidence/items-observations-20261113/validate.py
python docs/agents/evidence/items-observations-20261113/compare_sources.py --check
python tools/agents/validate_governance.py
python -m unittest discover -s tools/agents/tests
git diff --check
```

Private proof-byte checks are an Ubuntu custody result, not a claim that Cloud
contains or reread screenshots. Source reproduction and sanitized packet
integrity are checked directly in Cloud. No complete client inventory,
every-value semantic parity, graphical/gameplay parity, deployment, merge or
CI result is asserted.
