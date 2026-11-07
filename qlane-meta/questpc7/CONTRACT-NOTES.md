# questpc7 contract decisions

- Scope is the owner's25 ACTIVE ledger entries, with entries already QUALIFIED
  on another branch skipped. Fresh qengine2 head9d3c3869f03a523601d5567cdb352d40854f3517
  qualifies Festive Outfits (RESUME.md:44-55, PG18.6 case89.658220585s).
  It is recorded as external qualification, not as locally reimplemented work.
- Reuse the reward engine's NPC value owner first. Preserve this branch's
  existing world callback signatures and read-only hoist semantics while
  integrating NPC exchanges; world effects are a separate subsequent slice.
- Explicit fluid and charge operands remain owned by the existing exact-match
  owners. New ordinary static reward constructors specify no fluid/charge
  instance override; they never infer a charge grant from an unrelated item.
- Read-only prerequisites belong in `requires`, retain their original quest
  ownership, and are checked under the same root and sorted track locks.
- No donor fact has been invented in this slice. Investigation is capped at
  ten minutes per fact; any later conservative authored rule must be recorded
  here before its focused PostgreSQL proof and publication.

- Wire-level outfit integration uses qengine2's fresh-character fixture with a
  creation-bound sex and the appearance packet rebound to the fixture content
  revision. Historical sex-less characters are not valid appearance-capability
  test actors. Keep the normal capability25, catalogue delivery and selection
  assertions; do not bypass the appearance owner or disable the capability.

- Palimuth is now globally qualified by the reused reward catalogue. Its older
  unavailable-greeting expectation becomes exact source greeting/farewell/
  walkaway assertions; the unrequested Spectulus refusal remains unchanged.

- Golden uses the pinned selected recipe's100000/50000/50000 gold-unit prices
  (`content/quests/definitions/quests-00100-00199.json:1#/records/52/definition`,
  SHA256afe08860377d1027a1698e4dca459f617d34f39a5ddbf987ae5af25a73be5e50).
  King Tibianus' donor accepts carried money plus Bank balance at
  `data-global/npc/king_tibianus.lua:131-189`, Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
  SHA256e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd.
  Preserve that existing settlement policy with the chosen Oteryn prices;
  do not substitute the donor's500m/250m/250m economy. Each character's
  commission is once-only; earned appearance facts retain account ownership.

- Payment support reuses qengine2 commit33b640335 (head e06058c74),
  including the closed fee cause, exact price/bank policy, same-transaction
  source bridge and immutable receipts. Migration1099 preserves all previous
  carried conservation, custody and revision predicates; no new wallet is added.
  Golden's selected final confirmation writes completion atomically through
  the reusable callback `completes` flag.
- This checkout's hand-maintained fee-tag golden inventory additionally needs
  tag18 and its own codec fixture. Added both, retaining every pre-existing
  golden byte and exact inventory assertion. The imported qengine commit
  did not include this checkout-specific fixture update.
