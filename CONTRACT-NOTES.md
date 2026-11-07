# L2 integration decisions

- 2026-11-06: Reuse PC5's source NPC programme interpreter, item hand-ins and claim-progress bindings. L2's sixteen-track atomic writer and paid repeat identities remain authoritative.
- Native Item composition: canonical known facts and existing admitted facts take precedence over donor hypotheses. A portable canonical shell may receive source-qualified physical defaults only when its exact identity, admission, known semantics and zero-only operational attributes agree. Existing richer providers cannot be replaced. This reconciles PC5's generic canonical shells with QENGINE2's bounded source importer.
- Shared starter Items keep one native provider. Newhaven's overlay reuses the already generated starting-loadout fragment after checking identity, portability and known physical/stack/container facts; it does not mount duplicate records.
- No new L2 migration numbers are allocated by these reuse changes. PC5's already authored migrations retain their original numbers; new L2 migrations remain restricted to 1140–1149.
- Departure callbacks: `reversible` governs lever pairs. A non-lever pressure-plate departure may restore Channel-local map facts without that flag; all departure predicates continue to reject quest writes, item hand-ins, rewards, teleports and level gates. The Python authoring validator and Rust loader enforce the same rule.
- Deployment: Jorge uses the pinned Global cell (32204,32292,6); Tereban uses (33170,31807,6). The example-map Tereban cell is excluded. NPC callbacks require actual deployment as well as dialogue admission.
- Draconia reset reuse: PC6 commit `be812420e3c20d9dd8ae6c3c23a9c4acb942ceda` supplies an exact-value conditional write. Nonmatching values must remain unchanged under the existing fenced quest transaction and replay receipt; an unconditional reset is insufficient. The NPC exchange source is Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/npc/an_old_dragon_lord.lua:61-74`, SHA-256 `85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052`: one mushroom (3723) grants one fetish (3206) and flag 1; flag 1 refuses another grant.
- Draconia continuation: reuse PC6's authored recovery landing and exact conditional storage callback. The source escape consumes one fetish and resets only flag 1; missing offerings recover to (32818,31599,9). Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/scripts/quests/draconia/movement-escape.lua:9-26`, SHA-256 `53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312`.
- Draconia's other exit uses the existing composed remote-destination owner, extended with at most seven additional distinct tile conditions. The four control levers use the existing reversible lever effect; success requires appearances 2772,2773,2772,2773 at x32802..32805/y31584/z1. Failure returns to (32803,31587,1). Source: the same Crystal pin, `data-global/scripts/quests/draconia/movement-exit_teleport.lua:1-36`, SHA-256 `80af40ac459eb655670472a0391db78590d456827917d4a64e714947b64723a4`. Stateless remote routes are allowed only without quest writes, hand-ins, rewards, native rules or other effects; existing read-only quest routes keep their validation.
- Reuse PC6's already captured portable Item 3206 admission and regenerate its registration with the existing native producer. The source NPC filename `an_old_dragon_lord.lua` resolves to the accepted canonical NPC key `oteryn:npc.an_old_dragonlord`; its production cell is (32796,31558,2).
- Current Crystal world witnesses outside the legacy wardrobe scope require positive exact source lines plus a lowercase SHA-256 on the already accepted immutable 9f5a72c pin. Legacy wardrobe witnesses without source lines remain wardrobe-only; their negative admission tests remain intact.
- Exchange continuation reuses QENGINE2 payment commit `33b640335067df5d5582460bf06ce818d39b9d4d`: the existing native coin/change and account-bank fee owner participates in the same Type2 quest/item transaction. Keep all custody, epoch, ordering, conservation and source-receipt guards. A positive monetary fee is a valid cost for a repeatable item exchange; a fee and an explicit coin-item cost may not coexist.
- Gamon's exact source price is 5000 gold, with carried money and bank accepted; Quentin must introduce the stake first. Source: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/npc/gamon.lua:59-71`, SHA-256 `466de8702c51fc9c550f3f9f75e0301713eeec3a81bb8974fc848f10df5ead83`. New L2 payment SQL uses 1142, after checking main `ca5a91b59` and the inspected lane refs. The existing 1000 corruptor migration is retained; the reused payment migration's original 1000 number is not reused.
- Friends and Traders preserves the existing catalogue’s zero initial value: source unstarted `-1` accepts the canonical `0` only for the introductory transition. Gamon requires stage at least 1, so the zero normalization cannot bypass Quentin.
- The paid-repeat PostgreSQL regression found L2 migration 1141's material-only cost guard. Migration 1142 extends that guard with an alternative exact same-transaction monetary fee receipt; it retains the item-output restriction and all receipt, custody and positive-price checks. The paid purchase was rejected until this integration was added.
- Wooden Stake prayers reuse the existing carried-evidence participant. Quentin, Tibra, Maealil, Yberius, Isimov, Amanda, Kasmir, Rahkem and Brewster advance stages 1 through 10 without consuming the wooden stake; Tyrias alone consumes one 5941, grants one 5942 and writes stage 11. Generated bindings retain exact file:line and SHA-256 witnesses from Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d. The authoring refuses changes to the final item exchange or intermediate retention.
- The nine additional priests use the existing pinned placement candidates, each with one unique, unshared cell. No synthetic production NPC placement is used. The compact PostgreSQL fixture places all eleven story NPCs on two rows and walks the actual wire corridor; this corridor is isolated from the unrelated Behemoth lever and barriers used by the shared fixture.
- The ordered-prayer PG regression found that an ineligible reward confirmation could reach ordinary NPC fallback and acknowledge `Ok` without a quest change. Recognized, unhandled reward confirmations now refuse after every specialized native owner has had its opportunity; normal greetings and active bank confirmations retain their owners. The strict out-of-order and completed-prayer assertions remain intact.
- Reuse the existing absolute ItemType decay schedule and expiry owner for timed Quest rewards. The cup of molten gold has source duration 36000 seconds, with no stopping or decay target (pinned Canary items.xml; exact item 12804). The source authoring includes its full 36000000ms decay plan. Rust resolves the admitted generation's typed decay and requires an exact match; MINT and immutable deadline scheduling share the Quest transaction. Migration 1143 adds only the matching same-XID Quest MINT proof to the existing schedule owner and detects omitted schedules at deferred completion. Existing cast, action, event, custody, expiry and audit proofs remain intact.
- Migration 1143 was free in main 9668e0a04e1a631ebfc8752772102a1fd98ce8fa and the freshly inspected QENGINE2, PC5, PC6 and PC8 refs. Source deadline initialization is generic, including an admitted decay target; no item-specific runtime duration exception is introduced. The producer continues to reject charge and pausable time-budget definitions for this composition.
- Fixed Cyclops/Marina item admission reuses the richer existing native providers for weapons, armour, honeycomb, silk, metals, ingots and gear wheels. The L2 custody producer adds only the three missing definitions (3560, 5886, 12804); it does not replace an operational provider with an inert source shell. Its native-conflict rejection remains intact.
- The first integrated PG run reached every steel exchange and then rejected molten-gold scheduling because the reusable action scheduler requires its normalized `count: 1` evidence. The Quest composition now supplies that evidence; the scheduler's original one-item guard is unchanged.
- Concurrent L2 merge: preserve incoming fcb7ac5576a8a42fb081745c83c219dce8206201 and local cbf052888. Retain incoming read-only prerequisites, XP/level guards, source greeting conclusions, all twelve additional native item profiles and published NPC placements together with the fixed exchanges and deadline composition. Duplicate source programme blocks were removed through the authoring, and derived content was regenerated with the repository merge tool.

### L2 direct reward output reuse

PC8 commit eb196696c7d2606656f07cc72bf850ecdec81e10 already implements
atomic direct backpack outputs through the complete native MINT reservation,
child identities, per-item audit and deferred complete-plan proof. L2 reuses
that implementation, retaining fee, expiry and prerequisite extensions. Its
1108 migration is allocated as L2 1144 after checking the available remote
main, engine and quest-lane refs for 1144 collisions. This is a shared mechanic
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
Migration1145 was checked against24 current main/engine/quest heads.

### L2 world USE reward participants

Reuse PC8 d90c1bfa29fac954fbdd08e014a6473d4a33de96's finite world
USE reward composition through the existing complete native MINT owner.
Migration1146 extends the installed receipt proof without changing published
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
the mine permission and promises dwarven armor. The startup chest table74
(sha0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca)
locates the untagged chest at32455,31968,z14. Restore the promised native
armor3397 there through the shared world reward participant and an internal
once flag; no legacy unique id is fabricated. The native gate at32455,31967,z14
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
