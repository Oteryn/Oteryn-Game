# L2 finished slice on current main

This slice reuses the published, PostgreSQL-tested White Raven Monastery and
both Djinn faction implementations from questl2b be6626b4. It excludes that
branch's unfinished stories and migrations. The complete 51-quest packet ledger
remains on questl2b; this separate branch contains three quests for collection.

Dalbrect's live source writes WhiteRaven QuestLog while Costello still names an
obsolete FamilyBrooch flag. Costello checks the atomic QuestLog completion from
the actual Brooch hand-in. Both source witnesses remain in the authoring.
Dalbrect charges ten gold after friendship; friendship never waives the fee.

Djinn source menus require the corresponding faction's mission03 == 3. The
ordinary trade transaction rechecks those requirements; a displayed offer is
not durable authority. Source count=200 for necklace 3085 is its charge subtype,
not a quantity of 200 necklaces. Both faction starts are mutually exclusive.

Main f5b584ef1 already counts native shells 3029, 3035, 3205, 3212, 3234 and
3607. Their source refinement does not add six new identities. The equipment
pin tooling subtracts exactly this declared overlap and preserves raw fact tests.

Main already implements ActorConditionRequirement::Drunk. The older request
for a new drunk eligibility mechanism is withdrawn; future Kissing a Pig work
will reuse that condition. The diagonal movement request for Unnatural Selection
remains separate from this finished slice.

The isolated PostgreSQL rerun exposed two integration differences from the older
lane branch. World callbacks now cite the corpus provider `crystalserver:` with
the same immutable revision and SHA256. The 31-line committed-copy helper reuses
be6626b4's generic update so NPC hand-ins observe a committed world stage before
the next command; recovered receipts cannot overwrite newer snapshots. This
replaces main's Monk-only immediate update and preserves cadence reconciliation.

The complete focused rerun passed all four cases: White Raven 119.526543435 s,
Efreet 162.740122134 s, Marid 155.475079588 s, plus the equipment regression.
All three player journeys emitted PostgreSQL 18.6 markers and completed normally.
