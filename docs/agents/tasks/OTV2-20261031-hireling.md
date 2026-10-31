# OTV2-20261031-hireling

## Scope

Complete audit row E12 on `chatgpt/hireling`, starting from `ae4188a9c8d461acb152044814ea23075de02ed4`. Primary reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. Extend the existing Hireling, NPC bank/trade/stash and Store owners.

## Acceptance

- Regression tests for existing summon, dismiss, query and bank paths before production edits.
- Session-generation-fenced lamp placement and return, house reconciliation and restart projection.
- Donor-backed services, skill ownership and profession appearances with recorded replay.
- Existing Store purchase/delivery path extended without a second commercial authority.
- Unit and mounted PostgreSQL tests; exact commands and results on the draft PR.

## Ownership and numbers

One lead owns this branch and publication. Delegated writes are path-disjoint. Reserved migration block 0700-0703, capabilities 250-251, commands 470-473, mechanic ordinals/offer orders 610-613. Reuse capability 165 and command 220 where sufficient; existing NPC and Store commands retain their authority.

## Questions

The draft PR is the decision channel. Questions start `QUESTION:` and answers start `ANSWER:`. Unsettled donor/prompt differences are listed under `Left out` until resolved. Do not merge/rebase main, force-push, request review, or add heavy-tests.

## State

AUTHORING. Initial read-only audit found query/dismiss regressions, dynamic bank authorization gaps, donor character-wide skill/outfit ownership, and a production Store composition dependency. No implementation or test result is claimed by this initial commit.
