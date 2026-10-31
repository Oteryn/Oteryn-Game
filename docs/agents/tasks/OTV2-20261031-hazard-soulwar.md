# W10 Hazard / Primal Ordeal and W11 Soul War / Rotten Blood

Task ID: OTV2-20261031-hazard-soulwar
Status: AUTHORING
Branch: `chatgpt/hazard-soulwar`
Base: `ae4188a9c8d461acb152044814ea23075de02ed4`
Reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`

## Accepted scope

Extend existing Hazard, Quest, encounter and durable authorities for hazard unlock,
monster spell scaling, production zone qualification, Primal Ordeal entry, Soul War
taints and experience, and Soul War / Rotten Blood entry, progression, reset and rewards.
The shared boss/activity owner remains the sole encounter admission authority.
Global / TibiaWiki evidence wins the W11 conflicts identified in the re-audit.

## Allocation

- Hazard combat, unlock and tests; migration 0708.
- Quest-owned Soul War taints, per-recipient experience and tests; migration 0709.
- Quest-owned Rotten Blood progression and tests; migration 0710.
- Integration, command routes and admission consumers; migration 0711 if needed.
- Reserved capability IDs 254-255, command IDs 478-481 and ordinals 618-621;
  existing protocol IDs are reused where sufficient. Number allocation is rechecked before publication.

## Verification and delivery

The branch starts from the immutable base above and is never rebased or merged
with main during this task. One lead commits and publishes. The pull request stays
in draft; no review request, heavy-tests label or merge is authorized by this task.

Every unresolved value or scope decision is recorded as one QUESTION comment on
the draft pull request. Answers are re-read before each new implementation step.
The parity table, exact executed checks and remaining gaps are recorded in that PR.

No implementation or test success is claimed by this initial coordination commit.
