# Native Slug corpse custody

The real caller case found `CorpseItemInadmissible` in the activated Slug reward
row. Its canonical Creature profile names Item 12526, but no native Item profile
admitted that body. A private fixture corpse cannot establish this route.

Source: https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml
The existing pinned local XML and client appearances are verified by SHA-256
in `engnpc2_slug_corpse_profiles.py`; no source hashes are edited.

The XML chain is 12526 → 12533 → 12534 → 12535 → removal. Durations are 10,
300, 300 and 60 seconds. The first corpse has 10 slots. The source importer
retains physical flags, container capacities and native absolute decay facts
for the complete chain. Every stage is admitted only to Ground; this grants
neither carried loot permission nor a scripted USE action.

The real caller fixture uses the activated Slug reward row, including its corpse
reference and Bestiary race, then mints through the existing death/Item owners
at the acting runtime player's cell. The selected Ground UUID reaches
`ComposedFreshAdmission::use_quest_tool`. Earlier private fixture cases remain
independent low-level qualifications.
