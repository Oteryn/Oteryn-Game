# Run-8 whole-claim native dependency census and bounded repair

133 selected claims (frozen132 + explicitly Mintwallin-routed key3620); 151 distinct exact Item/family/revision tuples across every reward root and child.
Read-only census: `python3 .questpc/r8-catalogue-native-audit.py`; full selectors/uses and issue list in `.questpc/r8-catalogue-native-audit.json`.

Only missing profile: `Item/oteryn:item.tibia.i2973/definition-r1`. Its sole selected use is root reward of `oteryn:reward-claim.quest.key.id3520`, quantity1, keynumber3520.
Other150 tuples each have exactly one profile; CharacterInventory admission/materializable, known portable weight, movable/pickupable true, supported quantity/stack maximum, matching known native root container capacity and valid per-instance key numbers. No second sibling missing profile is left for another review round.

Root cause: `quest_callback_profiles.admit` gathers leaf items only for ready definitions (or native_contents closures). This key3520 claim retains `readiness=waiting_implementation` and has no native_contents. Existing key owner/World binder supports it, and canonical item is already admitted; native profile collection omitted its item. Do not broaden all waiting definitions or native lookup fallback.

Minimal repair proposal for lead-owned `tools/content-migration/quest_callback_profiles.py`: add an explicitly closed `keys.add('oteryn:item.tibia.i2973')` near the existing bounded Rotten Blood/Falcon cohorts, with exact key3520 source comment. The existing canonical/exactbinding/sourceXML/portability checks and native profile builder then do the full profile construction. Regenerate via ordinary register pipeline and manifest pinning; do not hand-edit generated provider/digest. No new item policy/trait/protocol owner required.

Canonical: `content/items/definitions/items-17000-17499.json`, i2973, materializabletrue, NonStackable, physical100centi-oz/movabletrue/pickupabletrue, namebonekey, revisiondefinition-r1.
Binding: `imports/crystalserver/bindings/items.json` external2973→exacti2973 definition-r1, sourceff7ede593c69d4c658b382c97443e8155926924a.
Existing digest-bound core admission: `docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json` admissions[44], client2973 flags.take=true, unmove/cumulative absent(false), no holds, WikiBoneKey page3268/revid1043653; Canary/Crystal/CrystalSummerXML all typekey/weight100. Current native provider has no i2973 tuple.
Current retained sourceXML: `imports/canary/items-xml/items.xml` item2973 namebonekey/typekey/primarytypekeys/weight100.
Chest donors: `.questpc/sources/crystal/data-global/startup/tables/chest.lua:508-513`; `.questpc/sources/canary/data-otservbr-global/startup/tables/chest.lua:233-238`; uid5018, appearance2473,32376/31802/7, reward2973x1, Storage.Quest.Key.ID3520.
Fresh Wiki (working official helper): `.questpc/r8-catalogue-wiki/Bone Key.wiki` item2973 weight1.00 pickupableyes; `Key 3520.wiki` Bone/CarlinCryptKey/FanfareQuestKey, storagebuilding nearCarlinboat.

Validation state: real batch failed before key3520 with DefinitionNotFound; no partial132 qualification. Mintwallin key3620 individual test is already positive with marker6.438445793s in root repaired log. Require final successful whole132 test after mountingi2973.
