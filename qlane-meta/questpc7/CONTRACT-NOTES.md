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
