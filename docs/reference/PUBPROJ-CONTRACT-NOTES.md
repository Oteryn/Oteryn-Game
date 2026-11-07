# PUBPROJ contract notes

Owner authorization: 2026-11-07, “1 tak, 2 zgoda”. Base: `d34807f5b`.
Branch: `claude/happy-davinci-ijz3wy-pubproj`; migration allocation: 1350–1359;
new numeric identifiers use next free on the base plus 50. None allocated yet.
Owner extended the task on 2026-11-07 with G-12 world_daily and both G-8
statistics families; migrations 1360–1369 are additionally reserved if needed.

## Part A

The parser already has `PublicProjectionsConfig`, default-disabled gating,
purpose-separated credential/fence validation, and a production caller in
`node/serve/supervision.rs`. Install the disabled table in the Synology template
and node boot fixture. No gameplay rule changes or donor rules in this step.

The disabled table exposed eager certificate reads by Bazaar and T94. Those
reads now require public_projections.enabled, while configuration path-identity
validation remains active. The existing publisher itself was already gated.
The new missing/mistyped-field negative test passed (first build 9m14s;
execution 0.01s). Server check passed (4m15s); all-target server clippy passed.
Full changed-crate tests, pin and architecture checks remain pending in this
checkpoint. No claim of completed Part A qualification is made yet.

## Part B reconciliation

Read Platform main's
`docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`
(revision 2). Its V2 paths replace the three candidate V1 families; the existing
V1 watermark enum stays closed. The owner additionally requires stable killer
identity: player CharacterId and native creature content key, historical names
for display only. This amendment must preserve §7 privacy and requires durable
cause capture before a creature identity can be emitted. Producers, acceptance
text, fixtures and execution evidence are still pending in this checkpoint.

### V2 wire checkpoint

The owner acceptance is recorded in the V1 status line and the separate V2
profile. V2 has typed player/creature killer identities, stable guild member
ids, and exact house status variants. The fixture generator is
`tools/contracts/generate_public_projection_fixtures.py`; it emits nine byte
fixtures. Five focused wire tests passed (build 14.58s, tests <0.01s).
V2 acknowledgements are version-bound; the closed V1 watermark stays unchanged.

This checkpoint compiles, but B is not complete: the durable store methods,
source capture migrations, PostgreSQL cases and resource-limit registration
are pending. The production store defaults to unavailable for the V2 methods,
so it cannot publish fabricated facts before those source paths are installed.
Recent historical receipts without immutable classification must withhold
freshness, rather than become guessed PvE or a known empty death history.
C and D implementation has not started.

### Durable capture checkpoint

Reserved migrations:1350 (V2 heads/outbox/source seam),1351 (Guild/house),1352
(death capture). Resource ordinals10069–10076: base maximum10018, next10019
plus lane offset50. No command ids, capability bits or mechanic ordinals added.

The reconciled durable read/ACK/facts adapter and native death capture compile
with the server (4m08s). Native admission-source tests:151 passed,0 failed,
0 ignored (0.14s); native all-target clippy passed (16s). Source migration PG
execution, house catalogue boot integration, delayed-condition cause provenance
and full changed-crate validation remain pending. This is a compiling WIP.

A's full server tests exposed a pre-existing stale reward-claim golden: actual
311 versus expected308. ITEMHOLD-Q commit ca5a91b59 admitted exactly
ornamentedshield, powerring2 and triangletower; see
`docs/architecture/reviews/ITEMHOLD-Q.md:651–659`. The correction preserves the
original308 cohort and additionally requires each of those three exact ids.
No content or binder rule changed. Its focused rerun remains pending.

Pin-refresh, pin-check and architecture-check completed successfully. Physical
node-boot qualification reached the exact Platform checkout but failed on the
host prerequisites: Docker buildx tried a read-only home path, and cleanup
reported `sudo: command not found`. No node-boot PASS is claimed.

The new ADMINAN obligation reserves creature keys player and unknown. Future
admin player-death analytics uses World × primary killer key × UTC day, with
all player killers collapsed to player and unattributed causes to unknown;
private G-4 CharacterIds never enter that admin feed. Known invalid source
metadata must remain quarantined, not be relabelled unknown. C/D source and
retention implementation is in progress; no ADMINAN producer is implemented.
