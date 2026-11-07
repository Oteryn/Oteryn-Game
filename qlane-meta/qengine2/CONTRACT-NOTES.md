# qengine2 contract notes

Owner rule: bound each investigation or missing-fact search to ten minutes.
Use the documented conservative behavior, implement and test it, and continue.
Push each finished green quest/mechanism slice immediately; do not hold green
work longer than thirty minutes. Only focused checks, no full workspace suite.

Accepted implemented decisions (all PostgreSQL qualified and pushed):

- Dedoras: charge exactly five Sturdy Books or twenty Epaulettes, matching his
  dialogue and the selected canonical recipe. Repair donor strict-greater-than
  checks and the unreachable epaulettes keyword branch. Crystal9f5a72c,
  data-global/npc/dedoras.lua:198-224,
  sha25675b3c6e0215ce23ab7f2f6812a77d71a80d3f4ff1565a33eb3308f3c21310fd1.
- Muriel: his source offers require the addon flag that only those offers set.
  The corpus has no other writer. Bootstrap stage1 only after base completion
  and matching carried evidence, without consuming or granting; the next exact
  hand-in earns the addon. Completed stage2 cannot restart. Crystal9f5a72c,
  data-global/npc/muriel.lua:97-149,
  sha2567bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc.
- Oramond VotingPoints: declare only the missing read-only catalogue fact,
  retaining Oramond ownership. Donor absence is-1; use the conservative signed
  32-bit ceiling2147483647. No player progress is written by this declaration.
  Exact thresholds3/6 and second-addon priority come from Crystal9f5a72c,
  data-global/npc/ezebeth.lua:93-103,
  sha256280ef0cf3d3342ed33e7f156a029f096e32e7a593981099e522256fe47c14e82.

All source blobs remain in the committed pinned donor corpus; no donor copies
or binaries are added. QUEST-STATUS.json retains exact tests and case markers.

- Nobleman: the donor implements the150000 carried-money choice for each addon.
  Its10000-instalment text has no executable payment branch; preserve that
  implemented one-shot behavior. Dress/coat are aliases for the same canonical
  first-addon effect; the appearance owner uses immutable sex for the actual
  look. Hat is the second addon. No bank balance pays a carried-only request.
  Crystal9f5a72c, data-global/npc/irmana.lua:50-135 and male counterpart.

  Exact Nobleman donor citation: crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/irmana.lua:sha256:08e9d63f771a43f2fa147e58f7fb474deef6b541a3a749258ec9a040c8993c03.

- Native Quest payments reuse the complete bounded carried-coin proof profile.
  The new cause must also have its exact same-transaction Quest fee/source
  receipt; no legacy custody, ordering, epoch or Character proof is skipped.

- Royal Costume: donor XML tags looks1456/1457 as store, but the pinned source
  NPC and owner-selected quest recipe explicitly grant them for in-game tokens.
  Admit only Royal Costume as a native quest path; leave other store looks out.
  Both client appearance records exist in the admitted corpus. Exact source
  costs are15000 Silver Tokens22516 +12500 Gold Tokens22721 for base, then
  7500+6250 each for shield and crown, in that order. Do not use the total-package
  30000/25000 dialogue number as the base-only price. Source Crystal9f5a72c,
  data-global/npc/percybald.lua:149-228,
  sha256b617d51111abedf07a4b5de72acc938153152695517ef46b894b7b37873fdadc.
