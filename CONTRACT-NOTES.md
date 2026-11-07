# L2 integration decisions

- 2026-11-06: Reuse PC5's source NPC programme interpreter, item hand-ins and claim-progress bindings. L2's sixteen-track atomic writer and paid repeat identities remain authoritative.
- Native Item composition: canonical known facts and existing admitted facts take precedence over donor hypotheses. A portable canonical shell may receive source-qualified physical defaults only when its exact identity, admission, known semantics and zero-only operational attributes agree. Existing richer providers cannot be replaced. This reconciles PC5's generic canonical shells with QENGINE2's bounded source importer.
- Shared starter Items keep one native provider. Newhaven's overlay reuses the already generated starting-loadout fragment after checking identity, portability and known physical/stack/container facts; it does not mount duplicate records.
- No new L2 migration numbers are allocated by these reuse changes. PC5's already authored migrations retain their original numbers; new L2 migrations follow the coordinator-owned range1210–1219.
- Departure callbacks: `reversible` governs lever pairs. A non-lever pressure-plate departure may restore Channel-local map facts without that flag; all departure predicates continue to reject quest writes, item hand-ins, rewards, teleports and level gates. The Python authoring validator and Rust loader enforce the same rule.
- Deployment: Jorge uses the pinned Global cell (32204,32292,6); Tereban uses (33170,31807,6). The example-map Tereban cell is excluded. NPC callbacks require actual deployment as well as dialogue admission.
- Draconia reset reuse: PC6 commit `be812420e3c20d9dd8ae6c3c23a9c4acb942ceda` supplies an exact-value conditional write. Nonmatching values must remain unchanged under the existing fenced quest transaction and replay receipt; an unconditional reset is insufficient. The NPC exchange source is Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/npc/an_old_dragon_lord.lua:61-74`, SHA-256 `85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052`: one mushroom (3723) grants one fetish (3206) and flag 1; flag 1 refuses another grant.
- Draconia continuation: reuse PC6's authored recovery landing and exact conditional storage callback. The source escape consumes one fetish and resets only flag 1; missing offerings recover to (32818,31599,9). Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/scripts/quests/draconia/movement-escape.lua:9-26`, SHA-256 `53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312`.
- Draconia's other exit uses the existing composed remote-destination owner, extended with at most seven additional distinct tile conditions. The four control levers use the existing reversible lever effect; success requires appearances 2772,2773,2772,2773 at x32802..32805/y31584/z1. Failure returns to (32803,31587,1). Source: the same Crystal pin, `data-global/scripts/quests/draconia/movement-exit_teleport.lua:1-36`, SHA-256 `80af40ac459eb655670472a0391db78590d456827917d4a64e714947b64723a4`. Stateless remote routes are allowed only without quest writes, hand-ins, rewards, native rules or other effects; existing read-only quest routes keep their validation.
- Reuse PC6's already captured portable Item 3206 admission and regenerate its registration with the existing native producer. The source NPC filename `an_old_dragon_lord.lua` resolves to the accepted canonical NPC key `oteryn:npc.an_old_dragonlord`; its production cell is (32796,31558,2).
- Current Crystal world witnesses outside the legacy wardrobe scope require positive exact source lines plus a lowercase SHA-256 on the already accepted immutable 9f5a72c pin. Legacy wardrobe witnesses without source lines remain wardrobe-only; their negative admission tests remain intact.
- Exchange continuation reuses QENGINE2 payment commit `33b640335067df5d5582460bf06ce818d39b9d4d`: the existing native coin/change and account-bank fee owner participates in the same Type2 quest/item transaction. Keep all custody, epoch, ordering, conservation and source-receipt guards. A positive monetary fee is a valid cost for a repeatable item exchange; a fee and an explicit coin-item cost may not coexist.
- Gamon's exact source price is 5000 gold, with carried money and bank accepted; Quentin must introduce the stake first. Source: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/npc/gamon.lua:59-71`, SHA-256 `466de8702c51fc9c550f3f9f75e0301713eeec3a81bb8974fc848f10df5ead83`. Canonical QENGINE2 payment SQL uses1000, after checking main `ca5a91b59` and the inspected lane refs. The shared corruptor migration is renamed1201; canonical QENGINE2 payment1000 is consumed unchanged.
- Friends and Traders preserves the existing catalogue’s zero initial value: source unstarted `-1` accepts the canonical `0` only for the introductory transition. Gamon requires stage at least 1, so the zero normalization cannot bypass Quentin.
- The paid-repeat PostgreSQL regression found L2 migration 1211's material-only cost guard. Migration 1000 extends that guard with an alternative exact same-transaction monetary fee receipt; it retains the item-output restriction and all receipt, custody and positive-price checks. The paid purchase was rejected until this integration was added.
- Wooden Stake prayers reuse the existing carried-evidence participant. Quentin, Tibra, Maealil, Yberius, Isimov, Amanda, Kasmir, Rahkem and Brewster advance stages 1 through 10 without consuming the wooden stake; Tyrias alone consumes one 5941, grants one 5942 and writes stage 11. Generated bindings retain exact file:line and SHA-256 witnesses from Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d. The authoring refuses changes to the final item exchange or intermediate retention.
- The nine additional priests use the existing pinned placement candidates, each with one unique, unshared cell. No synthetic production NPC placement is used. The compact PostgreSQL fixture places all eleven story NPCs on two rows and walks the actual wire corridor; this corridor is isolated from the unrelated Behemoth lever and barriers used by the shared fixture.
- The ordered-prayer PG regression found that an ineligible reward confirmation could reach ordinary NPC fallback and acknowledge `Ok` without a quest change. Recognized, unhandled reward confirmations now refuse after every specialized native owner has had its opportunity; normal greetings and active bank confirmations retain their owners. The strict out-of-order and completed-prayer assertions remain intact.
- Reuse the existing absolute ItemType decay schedule and expiry owner for timed Quest rewards. The cup of molten gold has source duration 36000 seconds, with no stopping or decay target (pinned Canary items.xml; exact item 12804). The source authoring includes its full 36000000ms decay plan. Rust resolves the admitted generation's typed decay and requires an exact match; MINT and immutable deadline scheduling share the Quest transaction. Migration 1212 adds only the matching same-XID Quest MINT proof to the existing schedule owner and detects omitted schedules at deferred completion. Existing cast, action, event, custody, expiry and audit proofs remain intact.
- Migration 1212 was free in main 9668e0a04e1a631ebfc8752772102a1fd98ce8fa and the freshly inspected QENGINE2, PC5, PC6 and PC8 refs. Source deadline initialization is generic, including an admitted decay target; no item-specific runtime duration exception is introduced. The producer continues to reject charge and pausable time-budget definitions for this composition.
- Fixed Cyclops/Marina item admission reuses the richer existing native providers for weapons, armour, honeycomb, silk, metals, ingots and gear wheels. The L2 custody producer adds only the three missing definitions (3560, 5886, 12804); it does not replace an operational provider with an inert source shell. Its native-conflict rejection remains intact.
- The first integrated PG run reached every steel exchange and then rejected molten-gold scheduling because the reusable action scheduler requires its normalized `count: 1` evidence. The Quest composition now supplies that evidence; the scheduler's original one-item guard is unchanged.
- Concurrent L2 merge: preserve incoming fcb7ac5576a8a42fb081745c83c219dce8206201 and local cbf052888. Retain incoming read-only prerequisites, XP/level guards, source greeting conclusions, all twelve additional native item profiles and published NPC placements together with the fixed exchanges and deadline composition. Duplicate source programme blocks were removed through the authoring, and derived content was regenerated with the repository merge tool.

### L2 direct reward output reuse

PC8 commit eb196696c7d2606656f07cc72bf850ecdec81e10 already implements
atomic direct backpack outputs through the complete native MINT reservation,
child identities, per-item audit and deferred complete-plan proof. L2 reuses
that implementation, retaining fee, expiry and prerequisite extensions. Its
The source backpack-output extension is allocated as L2 1213 after checking the available remote
main, engine and quest-lane refs before the coordinator registry superseded local allocation checks. This is a shared mechanic
slice; it does not alone qualify a quest. A source soul-orb trade may require
more room than available; refuse the whole exchange without consuming orbs.

### L2 Cyclops all-orb exchange

Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
`data-global/npc/a_sweaty_cyclops.lua:188-213`,
sha256 `bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a`,
consumes every carried soul orb5944 and rolls separately per orb: 1/100 yields
200 spectral bolts6528, otherwise100. The generic bulk owner preserves that
rule and records every native server roll with the exact consumed count and
output layout. A whole output set that cannot fit is refused atomically.
The isolated PostgreSQL fixture replaces only the server RNG function to drive
normal and bonus outcomes deterministically; it seeds neither quest progress
nor earned outputs. Native orb/ammunition providers are reused unchanged.
Migration1214 was checked against24 current main/engine/quest heads.

### L2 world USE reward participants

Reuse PC8 d90c1bfa29fac954fbdd08e014a6473d4a33de96's finite world
USE reward composition through the existing complete native MINT owner.
Migration1215 extends the installed receipt proof without changing published
migrations; 24 current main/engine/quest heads were checked for allocation.
World metadata must identify the same USE, quest, storage and complete output
plan. The weaker context-free writer cannot execute a world value grant.
Existing NPC fee, bulk and absolute-deadline proof arms remain active.

Item-id registrations use a positive appearance selector and the resolved
server map stack, including untagged placements; they require no fabricated
position, action or unique id. The source Father corpse registrations are
`data-global/scripts/quests/fathers_burden/actions_corpse.lua:2-3,13-19`,
Crystal9f, sha256 ab1477d63aba338744198875b5746abc4c7092d90318331af3a2bad4732fa9a1.
They issue scale11550/sinew11548 once per character, without a mission-start
prerequisite, and retain the corpse. This slice proves the world grant
participant; it does not independently qualify the complete Father quest.

### L2 Sam and Outfox native access composition

Source9f Sam111-117 (sha ed713aa63d597133355328a16e9c36d1b382c0e1d7e40dc199211f4fc4d14e6b)
consumes3244 and refers the player to Kroox. Kroox63-67 (sha
080ef330bea12ad63d9d9aecdf259e669df0eb448287907f583aa04fa5e64497) grants
the mine permission and promises dwarven armor. The startup chest table3176-3178
(sha0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca)
locates chest9226 at32455,31968,z14. The shared world reward participant grants
armor3397 there, requires the mine permission and NPC stage2, and atomically
writes NPC stage3 with an internal once flag. The NPC completion follows
`data-global/scripts/actions/other/others/quest_system2.lua:97-103,378`
(sha04990d261d85479bf9a75de040f6c158bc6eb557018a7ad359374400c2a49635).
The native gate at32455,31967,z14
retains appearance1644/action40603 and its exact named storage.

Budrik61-106 (sha0bad10f01b039dfb0a8f0460b36a0429c1eb498cb504b378ef4aaba005056aa5)
issues Outfox, writes the Questline field, and exchanges139 for875. The old
callback omitted `/questline`; the canonical door action41492 already reads
the correct field. Its existing helmet claim6257 remains the reward owner.

The full source-world fixture exposed a routing guard before qualified bundle
owners, preventing native door/chest USE. Apply that guard only to the legacy
fallback without a bundle. Qualified bundle handlers retain their digest,
revision, actor/session, reach, storage, custody and MINT checks.

L2 Into the Bone Pit source contract (Crystal
9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d):
`data-global/npc/muriel.lua:61-85,133-141`, SHA256
`7bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc`,
requires level 35, issues 4852, and returns a death ring 6299 for sample 131.
`data-global/scripts/lib/register_actions.lua:502-512`, SHA256
`4a9c9f217f034d69372170a50bdfbbad3427453fc98fb233e0bd29c8779cf069`,
uses a carried shovel on appearance 103/action 4205 and adds remains 3133/action
4206 for 60 seconds without replacing the pit.
`data-global/scripts/quests/others/actions_ectoplasm_container.lua:4-12`,
SHA256 `584ec08275530b23bfc4222e6294acfaf84d9a1235d1d0738f77a04306666e28`,
transforms the existing container to 131 and removes those remains. Both new
world effects use the existing Channel callback projection and lifetime owner;
carried tool evidence is locked inside the fenced quest transaction.
The pinned Muriel script contains no experience grant. The owner explicitly
requests an XP reward and authorizes a documented likely behaviour when facts
are missing; this lane restores a one-time 1000 XP completion reward. That
amount is a lane contract choice, not a claim about the donor script. Item,
progress, and the XP completion obligation share the existing atomic NPC reward
transaction; the existing refresh settles that obligation before the NPC reply.
Bone Pit's imported canonical track has initial/minimum 0 in
`content/quests/missions/quest-state.json`; the donor's absent-storage sentinel
is -1. Mission issuance admits both unstarted representations (-1..0), matching
other lane issuance contracts, and advances them to source stage 1.
The Bone Pit startup binding is literal
`data-global/startup/tables/item.lua:449-452`, SHA256
`677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd`:
appearance 103 at (32408,32107,10). The served B3 tile retains that appearance
without the startup action attribute. As with Sam's untagged chest, the quest
callback restores the exact source position/appearance selector; it does not
invent a world action attribute. The full PG fixture uses that source cell and
an untagged pit too.
The full Bone Pit PG run also exposed a shared projection gap: world callbacks
scheduled a quest-copy refresh but applied committed changes immediately only
for Monk shrines. This left a completed container action invisible to the next
NPC command. All world callback commits now update the admitted quest copy
through its existing `apply` owner before publication. Recovered older receipts
cannot overwrite a copy that no longer has their original revision; the ordinary
refresh still reconciles recovery and XP obligations.

L2 An Interest in Botany reuses the carried-evidence and direct-output owners.
Crystal9f `data-global/npc/rabaz.lua:145-184`, SHA256
`b74ee3591cc967215c463d104d887ace6d448f3ea46d7521f0a0a264f5347643`,
sets DefaultStart/Questline to 1 and ChestDoor to 0 on acceptance. Its final
hand-in consumes 11699 and grants Sedge Hat 11700, ten platinum coins 3035
(1000 gold), 3000 XP and Questline 4. Hat and coins use one complete backpack
MINT reservation; the PG journey compares their audit `created_xact_id` values
to prove that both outputs were inserted in the same PostgreSQL transaction.
`data-global/startup/tables/chest.lua:2807-2813`, SHA256
`0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca`,
places book 11699 in appearance-2469 chest (33004,31530,10). The served chest
has no unique attribute, so the world callback binds its exact source cell and
appearance. The native door at (33007,31536,10) is appearance 7721/action
42965, mapped to the named ChestDoor track. Source value 0 grants entry under
the existing native quest-door rule; absent storage remains locked.
`data-global/scripts/actions/tools/skinning.lua:224-250,349`, SHA256
`2065a1ff806ab025efac6add9646e0a91084015d507b994e3dcf384365d37861`,
requires Obsidian Knife 5908 for the plant branch and a carried almanach 11699.
Appearance 10735 advances stage 1 to 2; only then does appearance 10697 advance
2 to 3. Both tools and plants remain intact. Rabaz is deployed at the reviewed
source promotion-candidate cell (33009,31532,10).

L2 shared callback state owners reuse QENGINE2 commits 6ad33a4b1 and
e9d98947f: bounded counter operations, server-time stamps/elapsed predicates,
and up to three explicitly declared foreign quest participants. Migration
Canonical QENGINE2 migrations1001/1002 retain their deferred proofs; L2 extensions are in1216.
The combined operation limit remains sixteen, preserving the eleven-track
Father completion. Foreign writes retain the same fenced Character revision,
source plan, content pins and PostgreSQL transaction as their primary reward.
The focused reference journey uses the pinned Amber/Norma Rook Citizen story,
not an L2 quest qualification. Only its elapsed timer is advanced by a test-only
native transition; assertions keep every other progress value unchanged.
Source file/line/SHA references are generated into each callback and the
writable foreign-track declaration from the committed donor corpus.

L2 Vampire Hunter follows Crystal9f `data-global/npc/storkus.lua:99-199`,
SHA256 `8f1b7eb209871b9a61e561714cf678e92640352cb96c49ac4540d555cbcf03cd`.
Six hand-ins consume 1/4/5/10/30/50 tokens 8192 and award
1000/5000/10000/20000/50000/100000 XP. The final hand-in grants doll 8191,
rank 6 and Door 1. Inquisition Questline >=11 remains a read-only prerequisite.
The PG character arranges only that separate prerequisite through a fenced
transition; no Vampire rank, access, reward or achievement is seeded.
Absent donor rank -1 is normalized to the new bounded track's unstarted 0.
The donor omits the achievement requested by the owner. This lane restores
`oteryn:achievement/razing` using `content/achievements/achievements-00000-00499.json:1`,
SHA256 `d097a1833981e27ec326f274651c39b5f6dc6f06ea79afc6cb6123e953a81341`: its description explicitly identifies the
vampire hunter who has gathered numerous tokens. The final reward uses the
existing atomic achievement obligation, not an independent script grant.
The native access door is the existing action 41731/appearance 8259 at
(32953,31460,9), bound by the source door-table owner to the same Door track.

L2 selected carried USE reuses QENGINE2 `97cb9db79` through the existing
quest/value transaction. The exact selected UUID must be in fenced carried
custody; matching another instance of the same definition is insufficient.
The consumption proof stores that UUID and the bounded, encoded USE ingress.
Migration 1005 reuses the deferred same-transaction ingress proof; all seventy
fetched remote heads were checked for number clashes before authoring.
NPC selection excludes carried callbacks and NPC admissions supply no carried
ingress. Recovery compares immutable bytes and never restores live authority.
The reference journey uses Crystal9f `data-global/npc/the_bone_master.lua:61-99`,
SHA256 `65a6ff562c38b78383ad4e9a4ba13ede2e875cd07686955f7c7741308d8fe3b1`: join
the Brotherhood, issue document 768, award Bone Brother, and set the foreign
Knightwatch Tower door. Its document USE follows
`data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua:27-65`,
SHA256 `1e3d15bb73efa6f3b26bdd1d0882a579279c0cff39c2fb166d98d03b3ed6be16`.
Only an unowned base outfit consumes the selected document. Account grants are
locked while checking negative ownership. A separate bounded claim preserves
one-time document bookkeeping under the same quest transaction. This is a
focused shared-mechanism proof; it does not qualify another lane's full outfit
quest. The player starts with no Brotherhood quest progress or outfit grants.

L2 delivery batch source contract: Ahmet follows Crystal9f
`data-global/npc/ahmet.lua:118-159`, SHA256
`7141a6ed014f4ca77031a6c83f41f7ff9603bf35a645004be53754e7bfa62797`.
The source bag 235 hand-in grants 100 gold 3031 and 100 brown mushrooms 3725.
`data-global/scripts/quests/others/actions_steal_from_thieves.lua:4-16`, SHA256
`cf029fb4b5f0d9a8adb119c1cae81da2eef80a6d9f768e25e3af28cd8dcbc9ec`,
checks UID19910 but registers AID19910; the donor map report cannot find that ID.
Under the owner's documented-choice instruction, restore this at the served
Darama box appearance2473 at (33131,32661,7), verified in `content/world/placements/region-z07-x129-y127.b3`
SHA256 `3d8be37fc38619e95490267428993d658fac808cbd8eb64e82beb3f26d3108de`.
This is an explicit restoration choice, not a claim that the source ID existed
on that box. The selector uses its exact native position and appearance.

Troll Sabotage follows Jerom67-84 SHA8f300ec0a5d3e609aee8772993cb8df06d4c1d510c432c662f706969690535b3
and Chief Grarkharok73-116 SHA4e2c0d750d572355b89e4b2cad56e2def8cff3e527ea666bc6aece91a538df09
in the pinned `data-global/npc/*.lua` sources. The arbitrary lady name is
restored as the explicit dialogue word `grubokk`. The chief issues necklace7754;
Jerom consumes it and grants elvenhair rope646. Native equipment gates query
the held Character's durable slot edges under the Character root lock, not
a supplied inventory assertion. Head slot1 admits the source three diving
helmets; durable feet slot4 requires boots3079. Source movement script1-76 SHA
6f804525bdbe061efb3172968183a7724e318d077c2d85105fe375eb5096c870 and
startup tile table2032-2041 SHA f6e1d120063105d19303c9e9c8a57b40a6419cc51c4998be58984c42dc67a2a7
bind the six exact trigger cells. A server-time stamp plus age<=1 second is
equivalent to the source JumpTimer=now+1 and JumpTimer>=now check. Future
stamps and expired stamps refuse. The vortex1949 at (33229,31770,9) already
returns to (33235,31748,7) through the imported native teleport owner.

Cartography101's donor Rotem Valos NPC is a registered shell. This lane binds
the existing authored68 three-landmark recipe, map22107 (adventurer's map),
and the existing Cartography101 achievement. Exact named landmark coordinates
come from `content/world/areas/hunting-places/hunting-places-00000-00444.json:1`, SHA256
`cafc1aa9f937f5e07b794e1a297732aee616f6c85beb1a333f4c45d945489652`. Each native landmark
USE requires the carried issued map and writes only its own one-time survey
mark. The final NPC hand-in requires all three marks and consumes that map.
These are chosen Oteryn bindings; the source NPC shell is not presented as
original quest logic. PG fixtures compress only unrelated walking/climbing
with ordinary fixture teleports; quest gates retain their actual coordinates.
World callbacks loaded before classic NPC overlays reuse the overlay owner's
exact track declaration. The later overlay merge still rejects any mismatch;
callback guards never widen a canonical owner's domain. Test-world extents are
derived from the fixture tiles with half-open upper bounds and sorted floors.
The shared PG harness initializes players one tile west of its anchor. L2 delivery
anchors account for that offset, and every journey asserts its initial player cell
before issuing commands. Equipment predicates use durable semantic slot codes
from `crates/protocol-oteryn/src/item_view.rs:210`, rather than wire ordinals.
Native hotkey equipment negotiates capability179 and dependencies12/14. Slot
exhaustion uses separate non-stackable blank scrolls2815 (0.5oz each): 100-coin
stacks exceeded carrying capacity with the 400oz quest bag, while one-coin mint
outputs coalesced. Full-slot and atomic two-output assertions are retained.
Both nested moves select actual main-backpack custody.
Boots3079 had a known FEET slot but unknown requirements, which correctly refused
native equipment. The pinned Canary items.xml moveevent declares feet with no
level/vocation restriction. Source qualification now refines only equipment
UNKNOWN fields; it preserves every other native provider field and rejects
contradictory known requirements. XML SHA256 is recorded in the generated proof.
Boots source witness: `imports/canary/items-xml/items.xml:7224-7236`, SHA256
`1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`.
The explicit moveevent feet slot has no level/vocation attributes; the existing
native source importer records its constructor defaults as level0 and no vocations.
Troll Sabotage initializes both native NPC actors once at their source cells
(Jerom33240,31757,7 and Chief33231,31754,1). The player walks back to the same
Jerom actor; the fixture does not reinstall or move NPCs during the journey.

Coordinator migration registry (2026-11-07) supersedes prior local range checks.
QENGINE2 0999–1006 are byte-identical to its fetched branch. Shared QBASE files
are renamed1200–1205 without content changes. L2 owns1210–1216, including only
L2 extensions; Father's eleven-track write remains one physical transaction.
Renaming QBASE after QENGINE2 exposed a historical nonempty receipt constraint;
1210 retains that shared content and restores QENGINE2's guarded value-only arm.

Next L2 batch source contract: Yonan four-piece Regalia assembly is pinned in
`data-global/npc/yonan.lua:198-209` (Crystal9f), SHA256
`ab32c38fec7fc99be97b91ed19d79e64880c9ff955d58d8eee3504282f6e3848`:
parts31572–31575 produce31576, with no source fee or quest prerequisite.
Alliance supply requirements are pinned in the repository's structured wiki
source record for An Uneasy Alliance Quest, spoiler revision891001 SHA256
`2788b3c0b943e0b71687494a5e7bc5aa81542e0b1407793c165cd64e00fc2dfd`,
lines7–9: bucket of milk, crocodile steak, wooden flute. Where quantities are
not captured, the owner-authorized restoration uses one of each. Accepted XML
identities are steak10219 and flute2948; subtype checks use the native fluid owner.
Curos's source mission states and Tome reward remain at curos.lua:71-101, SHA256
`6feee8de2475788bca144210fd47cbf857b5dce3a16215ff2647e971bcd3a42e`.
The rebel death witness is creaturescripts_renegade_orc_kill.lua:1-15 SHA256
`cfbb7215d0d987b18e8c3e53dabf2954c4b523527121cd8185739745f73c5c99`;
crystal-ball action40030 is actions_crystal_ball.lua:1-25 SHA256
`6371998f8ecfe0c5d0c14f8dd6eb7e493a54145ebb0a016659ab3330e5b49211`.
Cael's source tome stages and incremental unlocks are cael.lua:84-128, SHA256
`4d4d0c4aa36722ebb85925c685b7631471889dd1f51e641711cee51568336dcd`.
Existing QENGINE2, PC3, PC5 and PC8 heads were checked for reusable fluid hand-in
and generic monster callbacks; no corresponding implementation was found there.
The three new PG tests first prove missing bindings, then require full native
journey execution; binding presence alone can never qualify these entries.
Fluid selectors extend the existing Item/count shape with an optional native
subtype. Empty inputs use subtype0; empty outputs keep the prior absent-state
encoding. Different fluids of one Item are distinct selectors, and overlapping
unqualified/qualified selectors are rejected. Root and child fluid outputs use
QBASE1200's existing MINT initialization in the same transaction. L2 migration
1217 binds exact subtype input counts and the complete output fluid plan without
changing any canonical QENGINE2 migration or the native fluid state owner.

Yonan stateless exchange restoration reuses QENGINE2's canonical paid-exchange
claim identity (1003/1004) and its bounded Rust admission. No synthetic quest
track is introduced: the four source parts are the payment, native MINT is
the output, and generic transition callers cannot execute value-only receipts.
The catalogue exception forbids XP-only empty transitions. Native player
journey qualification remains pending until the existing RED test passes.

Regalia31576 keeps accepted canonical weight3400 against pinned donor XML850,
using the existing finite weight-precedence check. Parts31572–31575 retain
850 each. Yonan's three conditional greeting branches repeat the same literal
text; the producer deduplicates identical literals and still rejects differing
ambiguous greetings. Success text is the exact donor line209.

Native creature callbacks bind canonical typed transitions through shared-kill-
callbacks.json. Renegade Orc uses the existing creature_event_1 transition
(quest_door 0 -> 1), with the pinned Crystal onDeath witness above. Credits
are descendants of native settled deaths, reuse captured recipient occurrences,
and filter positive captured damage plus original session/lease identity.
The writer rejects a command or USE cause for registered creature callbacks.
Full native Alliance kill proof is pending; the mechanism is not yet qualified.

The missing second Alliance tome is the source tower chest: Crystal
data-global/startup/tables/chest.lua:1375-1380, SHA256
0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca,
chest6104 at33154,31297,z3, one10217, TomesOfKnowledge.TopTower.
The same source table:2740-2804 provides eight more one-tome chests;
container_quest.lua:92-98, SHA256
c18f0caf3d2c9f1d0c084722dfcb55523dbb1811166863a4f72d052bd2ae24b9,
adds the chest at33343,31610,z1. Together these ten physical books and
Curos/Zalamon's one-tome NPC grants supply Cael's twelve source deliveries.

Alliance provision restoration: the pinned wiki lists milk, crocodile steak
and wooden flute but does not bind their use; Curos Lua omits them. In the
owner-authorized restoration, Curos accepts one of each when issuing the
mission, requires exact milk subtype6, and returns an empty bucket. This is
an explicit Oteryn binding, not a claim that the donor NPC consumed them.
His donor mission sequence and one-tome final reward remain unchanged.

Alliance PG fixture shortens unrelated navigation by placing Curos near the
source tower chest and a device carrying the real global action40030 binding.
The native Renegade Orc is reduced to one health before the player's real
attack; captured positive damage and the native death settlement remain real.
Only the foreign New Frontier prerequisite is arranged through the fenced
quest writer; no Alliance progress, death credit or Tome reward is seeded.
Timed transformations reuse revision-bound movement decay; the PG journey
checks the state immediately before its deadline and after advancing the
Channel owner clock, without changing durable quest progress.
