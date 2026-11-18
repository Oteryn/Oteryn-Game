# QCX57 Children of the Revolution Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Compared the imported records against the pinned quest page and spoiler, then against Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. [Quest page](https://tibia.fandom.com/?curid=41842&oldid=683807) · [Spoiler](https://tibia.fandom.com/?curid=41844&oldid=1160862)

| Part | Comparison |
| --- | --- |
| Requirements | Global requires Premium and The New Frontier mission 8. Level 0 is the minimum and 70+ is recommended. The donor door UID 3170 also closes once Wrath mission 11 reaches 2. No active Children door callback binds that source selector. Boxes/parcels or Levitate, optional rope and shovel, and a melee weapon for the bars are route equipment. |
| Prove Your Worzz | Global requires the Tactical Map within less than one hour. The prepared Zalamon topics have no active Children opener or map hand-in. The inspected Crystal NPC consumes one map (637), with no deadline check in the NPC branch. |
| Corruption | Global requires entering the Temple of Equilibrium basement and reporting it. The prepared report is inactive and UID 3163 has no active investigation callback. |
| Imperial Zzecret Weaponzz | Global requires three building visits and a report. Action IDs 8009–8011 have no active spy callbacks and the prepared report is inactive. The first Tome claim matches one item 10217 at UID 6289. |
| Zee Killing Fieldzz | Global obtains poison in Zalamon's storage room and uses it in the northern rice field. The donor grants one item 10183 at UID 3164 and consumes it at AID 8012. Both interactions and the prepared report are missing from active runtime. |
| Zze Way of Zztonezz | Global permits early symbol inspection after poisoning, then the NPC explanation, oil and a solved lever puzzle. The symbol visit and prepared explanation are inactive. The oil shelf already delivers one item 10189 at UID 6290 but has no mission gate. The active AID 8013 oil use correctly restricts application to the lever, where Crystal's Lua predicate is ineffective. The five puzzle levers and the NPC Tome (one item 10217) are already present. |
| Phantom Army | The two position-bound blue-flame passages already require existing access. The prepared offer lacks an active acknowledgement to grant access. Global requires four players, a 10–30 second wait, timed stairs and survival for ten minutes. The inspected main has no binding for this encounter. The second Tome claim matches one item 10217 at UID 6291. The final NPC report was missing. |
| Finale | Added the donor Zalamon report at questline 20. It atomically advances to 21 and Mission05 to 3 and gives one Serpent Crest (10199) and 10,000 XP. The existing Warmaster exchange consumes the Crest for addon 2. The main quest page lists no separate achievement. [Reward and transcript](https://tibia.fandom.com/?curid=41844&oldid=1160862) |
| Zzuppliezz and Collect | The respective Zalamon/Chartan daily task offers, material collection, hand-ins, variable rewards and Vive la Resistance / Extreme Degustation achievements are missing. Global requires two Corned Fish, one delivered to the prisoner, and a crate plus the other fish reported to the NPC. Collect uses seven plant items and vocation-based rewards, with XP options detailed in the spoiler. |
| Zze Art of War | The 6-day 20-hour repeat, NPC handoff after Wrath, return to the encounter, Lizard Gate Guardian and 10,000 XP task reward are missing. Crystal's inspected Zalamon and Chartan scripts do not implement these tasks. |

BLOCKER Phantom Army coordinated occupancy, timed raid waves and survivor ejection/credit `data-global/scripts/quests/children_of_the_revolution/movements_click.lua:16-74`.

OPEN-FACTS: Global runtime IDs, exact scanner boundaries for the rice field and room entrances, timed-map expiry/reset rules beyond the one-hour limit, precise stair timeout, task reward odds and any additional task placement IDs are unspecified in the fetched wiki excerpts. Crystal source selectors and item identities are cited as compatibility evidence where available.

PARTIAL: Entry access and recruitment, timed map, Temple checkpoint/report, Chaochai visits/report, poison acquisition/use/report, symbol inspection/briefing and oil-shelf gate, Phantom Army access grant/encounter, and all three post-quest tasks.
