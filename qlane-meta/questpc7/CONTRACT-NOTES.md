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

- Friends and Traders follows the chosen canonical linear programme at
  content/quests/definitions/quests-00100-00199.json:1#/records/47/definition.
  It retains all ten named priests in the exact selected order and folds
  collect stages into exact carried-item handins; no quest-state seeds are used.
  The recipe does not replace Gamon's price: retained Crystal9f5a72c
  data-global/npc/gamon.lua:64-68 (SHA256466de8702c51fc9c550f3f9f75e0301713eeec3a81bb8974fc848f10df5ead83)
  charges5000 via carried+Bank before giving Item5941. The same exact fee owner
  is reused. Sweaty Cyclops source:109-137 (SHAbc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a)
  consumes3 Item3560 and unlocks3381->5887; Marina:105-116
  (SHA8574dd5721f258bef8d649657ed428537e3e351e540a990fe72bcc21c89b8682)
  consumes50 Item5902 and unlocks10 Item5879->1 Item5886.
  Source citations for every prayer are embedded with exact file lines and SHAs.
- Repeatable unlocked services use a separate bounded monotone receipt counter,
  preserving the finite programme track and its completed revision. Maximum1e12
  is the documented engine bound; every exchange must produce one fenced Add(1)
  receipt and cannot advance past its bound. Existing QuestEffectKind::Add owns
  the arithmetic and overflow checks. No alternative item/fee writer is added.

- The first counted-service PG attempt proved counter1 but the second legitimate
  exchange hit the shared MINT owner's once-only claim key. Reuse the ordered
  NPC exchange pattern: counted callbacks receive a deterministic claim identity
  scoped to callback/session/command; ordinary callbacks keep their existing
  once-only key. Migration1100 retains the exact native MINT proof and binds the
  command-scoped key plus an exact same-transaction bounded Add(1) quest receipt.
  No global claim policy, prior receipt, input predicate or constraint is removed.

- Unavailable-stage prayer and repeat-blessing words can receive ordinary NPC
  chat acknowledgement (NpcIntentDisposition::Ok) without a quest action.
  The PG contract checks zero new quest receipts, unchanged programme progress
  and unchanged stake quantities; reply category alone is not a grant proof.
  The fixture allocates14 actor slots for13 selected NPCs plus its player,
  using the existing bounded constructor; production carrier checks stay intact.

- Eligibility reuses the native transaction-scoped Premium latch (the existing
  quest premium trait), not a client/session boolean. Level is locked native
  progression; completed-parent facts are read-only under the Character fence.
  Make Believe canonical is `oteryn:quest.authored.make_believe_quest`,
  content/quests/definitions/quests-00300-00351.json:1#/records/15/definition.
- Illuminated source lookup was timeboxed. Retained Summer00ce02a57ca5a12e48f32a3476e37471167e4c3f:
  data-global/npc/wayland_smythers.lua:112-114 SHA9a6b7b1ff8c351bc44275c053a6992093e69984f6ae77a519dcdc9040be7b851
  confirms2500/1250/1250; its script and XML contain no Illuminated grant IDs.
  https://www.tibiawiki.com.br/Illuminated_Warrior_Outfits?action=raw and
  https://www.tibiawiki.com.br/Illuminated_Warrior_Outfits_Quest?action=raw
  confirm Premium, Make Believe and counts, with no look ID.
  https://www.tibiawiki.com.br/Scraps_of_a_Radiant_Attire?action=raw confirms
  weight0.30oz/nonstackable; no numeric Item ID is published there. Current
  https://raw.githubusercontent.com/zimbadev/crystalserver/main/data/XML/outfits.xml
  also has no Illuminated Warrior row. Do not spend further time repeating this
  lookup: author distinct Oteryn compatibility identities using existing admitted
  appearance references, preserving known weight/stack behavior and exact totals;
  add a bounded escrow if totals exceed the existing carried-tree bound. This is
  an explicit native adaptation, not a claim of an unknown original numeric ID.

- Exact canonical parent source: content/quests/definitions/quests-00300-00351.json:1#/records/15/definition SHA2564a24de17cc2af26d6122fea2502b95974c8b5db55682e3540da813c0f28ebe6a. Parent completion is a read-only prerequisite, and its fenced fixture precondition does not qualify that other-lane quest.

## World wardrobe adapter — 2026-11-07

Reuse qengine2's fenced world-outfit transaction pattern (origin/claude/happy-davinci-ijz3wy-qengine2 at412fba411), without replacing this lane's read-only hoist/remote selector or asymmetric passage rules. Only the outfit participant is admitted here; unrelated portal/encounter effects are not implicitly enabled. The retained shared wardrobe producer emits an explicit top-level outfit_reward. The regression uses the retained Warmaster wardrobe and a fenced fixture of its independently owned prior mission; it does not qualify or claim that other quest.

Formal Dress fact budget is closed: Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data-global/scripts/quests/25_years_of _tibia_quest/lord_retro_death.lua:3-10 (sha256019afff6aac0e7ad9bd6e53edfb60ecf79094ab5a8393594238ac37c5ae78545) grants base looks1460/1461 from Lord Retro; usable_outfit_items.lua:51-52 (sha25669b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37) grants plume addon1 and button addon2. The selected quest instead explicitly calls for an exhibition display. Its display is an authored Oteryn adaptation of that selected recipe, not a claimed donor display location. Preserve existing consumptive item owners and their exact source grants.
