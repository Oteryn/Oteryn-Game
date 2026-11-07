# R24 spell import completion

This batch closes data and tooling work on the existing test-content path.
It preserves the r23 qualified evidence rather than relabelling old runs as r24.

The player catalog contains 246 definitions; six removed spells remain quarantined.
The native population remains 1,508 Creature/Presentation profiles. A separately
attested Crystal fallback fills only previously disabled physical-melee slots
whose exact existing native health, attack ordinal and schedule match. Existing
Canary-enabled selections and native identities remain unchanged.

Approximate monster melee still selects one slot, omits explicitly flagged secondary
conditions, applies no armor/shield mitigation and preserves the player health floor
of one. Ranged attacks, defenses, full custom callbacks and chase are not activated
by these files. Full quest execution and the missing Premium/house composition
remain coordinator integration work.

The scenario matrix lists the real prerequisite and refusal conditions for each
player definition. It is test input, not 246 successful gameplay receipts. Source
access records distinguish ordinary HTTP/Git from public Chrome/CDP browser reads.
No computer project files or system settings are modified by browser research.

## Import preparation

The pinned manifest recipe and evidence/validation-receipt.json record the current
checks. Content preparation does not grant live activation or deployment authority.
Use the existing Thalom map and normal Game/Platform admission path.

```sh
python docs/reference/spells/r24-candidate/materialize_manifest.py --out /tmp/oteryn-r24-inputs
bash tools/qualification/spells/run.sh map /tmp/oteryn-r24-inputs/manifest.json
```

## Final results

- 246 player definitions, 241 selected and five inactive source aliases.
- 1253 approximate monster melee profiles, including 22 new Crystal closures;
  255 explicitly disabled, each with source observations and a disposition.
- Scenario planning: 101 selected definitions require real fixtures on existing
  paths, 140 require runtime-owner integration. No scenario gameplay passes claimed.
- Game library 1593 passed/four ignored; player tools 224 passed; monster tools
  19 passed; qualification tools 12 passed. Full manifest admission passed.
- Required Game Clippy still fails with 859 library-test diagnostics, down from
  864 after five unused-import repairs. No warnings were suppressed.
- Independent batch review passed after fixing duplicate fallback acceptance and
  the Find Fiend scenario classification.

The previous r23 physical test is historical evidence, not a physical r24 run.
Fresh targeted BR/Fandom wiki reads used public Chrome/CDP after HTTP blocks;
full catalog wiki verification is not claimed. The owner's Premium-based soul
limits are retained as an explicit product choice; fresh wiki ties them to promotion.
