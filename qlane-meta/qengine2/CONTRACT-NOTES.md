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

- Pirate keeps the existing shared OutfitAndAddon quest ownership of its source
  base/sabre/hat and DefaultStart tracks. Do not reassign them to the title alias.
  Reuse Morgan's already-delivered firebird stage4->5; implement Duncan's exact
  three100-item hand-ins and both NPC introductions before claiming the whole
  outfit story. Only the Shattered Isles parent objective flags are arranged as
  external prerequisites in the test; no internal outfit stages are seeded.

- L1-05 Oriental: Crystal9f5a72c NPC ishina.lua:60-94 (sha2569eb2e869db4d0dfad8f04fc171773dcf9045e35647bd4069f9e9b18dc0513c71), habdel.lua:60-93 (0666e7207a7fd9565f5311ffc3d87f9b138c858117d802c089a7d00c727c7b78), miraia.lua:57-155 (7114cc53c7119cd7a42947a5be07611956e11e7f7c8833f68197f136be5e353f), razan.lua:57-149 (c12e695654ada3a5f4b1775657835fa18e5957b0870e735803a5ead2e7483acb). Exact comb5945x1; then ape fur5883x100,fish fins5895x100,wings5891x2,blue cloth5912x100. Both gender paths retain the existing shared addon owner. The door retains Oriental owner; source simultaneous write uses a generic atomic participant, never a reassigned track or detached follow-up.
- Source first introduction requires Explorer Society OrcPowder>=35; only that other lane's prerequisite is arranged by the test. Immutable sex guards all reachable authored stages; wrong-sex advisory offers confer no authority and the native commit rejects before writes/costs. Existing physical profiles for four materials are reused; only comb custody was missing. No new donor fact was inferred.

- L1-05B Norseman/Formorgar shared outfit reward: Crystal9f5a72c data-global/npc/hjaern.lua:107-155,189-197 sha2564e9e42e8daf616fe8c5ef71cfd0a4ac0ae0ae5d7bd786c80ccb2c26286d40466. At Questline39 and all four Obelisks5, take frost charm7289x1;write Questline40,Mission12=6,YakchalDoor1 and shared NorsemanOutfit1/DefaultStart1 atomically. Then exact5shards7290 lead40->41,reward41->42/addon1;exact10shards lead42->43,reward43->44/addon2. General owns NorsemanOutfit and DefaultStart;IceIslands retains other flags. Only pre-outfit parentQuestline39/Obelisks5 are arranged;no outfit stage is seeded. Father11 write remains untouched.
- Item name correction: imports/canary/items-xml/items.xml:18927 at pinned sha2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2 names7289 frost charm,19oz,no charges/duration/fluid. Earlier scratch note called it spirit container;IDs/costs were unchanged. Reuse existing7290 physical profile;promote only7289 custody.

- Canonical identity clarification: the admitted source catalogue has Pirate/Norseman outfit paths under `oteryn:quest.outfit_and_addon_quests`;there are no separate native quest records with the earlier scratch dedicated Pirate/Norseman URI spellings. Original L1 title keys/counts are retained;metadata now names the verified shared owner rather than implying a separately admitted record. Norseman progress also uses existing IceIslands owner. This changes no runtime data or finished reward behavior. Evidence: content/quests/missions/quest-state.json source track ownership and the qualified PG receipt/state assertions.

- Nobleman does have dedicated catalogue identity `oteryn:quest.nobleman_outfits_quest`;its source state still uses General. Pirate/Norseman lack separate dedicated records in the admitted definition shards. Native ownership evidence: content/quests/missions/quest-state.json:1 sha25665b74015fd8af6c4f1e4074d5166a5cc56e45d52e0e564da9a83fa202e7f6100. Catalogue identity and state owner are distinct;both are now recorded accurately.

- Hjaern continuous conversation: donor hjaern.lua:129-137 permits the root shard keyword independently of prior topic;rewards at141-155 leave topic3/4. Admit the root question at all reachable L1 topics0/3/4 with disjoint exact Questline40/42 guards. Preserve source next topics and fresh cost guards;never replicate the donor stale-topic yes branch that could charge/rewind after reward. A published-flow regression fails before the additional root offers and passes after.

- L1-06A Rookgaard Druid body: Crystal9f5a72c data-global/npc/tom.lua:230-250 sha2565079fd40c677dcee45941eac3eb4961c1260c5889635585d9d3441829de800b4. Source addon word startsBody0->1;paws/bear paws question and yes hand-in consume5896x50+5897x50,Body1->2,grantDruid addon1 for both sexes. Existing dedicated DruidRook state owner retained;existing material profiles and Tom placement reused. This source path has only body addon1;do not fabricate a second reward. Story independently expects mask1/two account facts and verifies the unearned addon refuses ordinary appearance selection.

- Generic greeting fallback: Tom.lua:61-87 selects four quest-specific greetings and sex-specific farewells. Shared reward greetings now use the pinned NPCHandler neutral defaults whenever a source NPC has no unique static message;functional conditionals retain their owned selectors. Each emitted default cites its exact handler source line/hash. No arbitrary quest branch or gender is guessed,and reward eligibility remains independently fenced.

- L1-06B Citizen Rook: Crystal9f5a72c data-global/npc/amber.lua:69-122 sha25600a81c0a699f019923ec1d906166c7dae13f7f60b4e3fcc321aff41c9b32a7f9;norma.lua:80-118 sha256c1eb0cbfdf9520fe110dd90b07674729d70aeaf69926f8f40a5f65d2983e2105. Amber takes100minotaurleathers5878,thenstrictlymorethan7200seconds before backpack addon1. Internal timer is normalized to start timestamp plus ElapsedAtLeast7201;the public wait is unchanged. Ref lacks native declaration:conservative sharedGeneral owner/initial0/nonnegative i32 bounds,source-backed writable declaration. Increment is native checkedAdd1;finish resets0 as donor math.min does forpositive refs,while retaining0 ifalreadyreset,nevercreatingnegative UI sentinels. Exact quest stages remain independent of Ref.
- Norma takes1legionhelmet3374,50honeycombs5902,100chickenfeathers5890 foraddon2. Source finalMissionHat0 could allow repeated intro;retain completedHat2 guard to prevent restarting/charging a finished outfit. No source costs,body waits or required stages are skipped. Clock-only fixture transition simulates elapsed time after proving fresh stamp/refusal;all outfit stages run literally. Existing materials/NPC placements reused.
- State-operation declarations:default remains read-only;writable requires explicitflag,provenance and every declared writer matching the named owner. Existing native definitions must match exactly. Combinedowned/participant writes remain<=8. Core checkedAdd/SetNow/Elapsed comparison,sortedlocks,capacity and source proofs are reused;SQL1002 validates exact operation preconditions/results andsame physical source transaction.

Beggar source: Crystal9f5a72c data-global/npc/hugo.lua:54-199 sha25660069ed647e4379a2e1177d32652602d86d8d50ad8da066f7dc65529b92fa55d;data-global/npc/simon_the_beggar.lua:84-222 sha2567ad2665ab2a053412536f0dd15762c98fb07122f0db20e020180fecd89003e74. Hugo's strict deadline os.time()+86400 becomes native start+ElapsedAtLeast86401,Simon male uses exact432000>= after beard request;only clocks advance in test fixtures. Native General remains original owner. FirstAddonDoor is absent from the native catalogue:explicit source-backed writable declaration General/initial0/min-1/max1 treats native0 as source-unset-1;both refuse once1. Simon's female gypsy branch grants addon1 but omits advancement,blocking his staff branch;conservative completion writes stage10/door1 like the male completed reward. Shared paid hand-in stamps the existing addon clock for both sexes;female never reads it,male beard request resets it exactly as source. Source staff replay lacks a completion guard;retain its written SecondAddon2 as a one-shot bound. Premium Current is required for Hugo introduction/base and Simon paid/staff hand-ins;Simon first introduction requires the native earned/starter outfit proof. No premium or outfit ownership is inferred from conversation cache.

L1-07 pinned Crystal9f5a72c source ranges/hashes: morgan.lua:115-148 fbef2b0d61842368915e506ff5d2e1e96f00a3f55bb3cad4ee97178824832610;gregor.lua:63-211 f56640cfe0501c564d092d3572b7b065aebaa6e22cb8f5af561a524079c8db98;sam.lua:67-88 ed713aa63d597133355328a16e9c36d1b382c0e1d7e40dc199211f4fc4d14e6b;ustan.lua:63-85 441ed6570ee072e3e75ffa98af95aa653b3d6ec8836327f6a5927833c5faed89;ceiron.lua:60-187 fb4cc1a978148a1ea552764b78930c098afd735af56738433797f20aa17da780;a_majestic_warwolf.lua:64-75 9fa8cee3622cacee85aec7770960d7d1bf46d25569f3a6ef857d0605bc842572;vescu.lua:56-189 af26eb8e9e31d41d3d00f08d03a971cf3136c4c2ffc83317af414e1dcb40262b;erayo.lua:57-114 dc6c54417c8bfdd32abcea9f068322f17d029c4d7722f1acfdeb6cd795a4c48f;atrad.lua:60-105 ffdc0e1b8d0d5455be4be4c6875b3f832dca3a488440ed08020bfa20bd7bde01. All paths data-global/npc/. General remains original owner; missing primary AddonHelmet/RamsayDoor/DruidAmuletDoor/Atrad flags receive source-bounded metadata with native-unset0. Existing declared Ref nonnegative rule also applies to Gregor increment/Sam finish0. Sam strict2h deadline normalizes native start+7201. ApeCity predicate>15 admits16..18,the complete native track domain. Assassin Shipwrecked2 remains read-only under its original parent owner. Source Vescu initial greeting requiresDrunk,Atrad greeting requiresFire: bounded proof samples both existing native condition owners at the value command's admission and binds the exact command/callback/itemfence/nativegeneration. This conservatively rechecks Atrad on each value command rather than retaining an expired greeting condition; no runtime locks are acquired inside SQL. Greeting prose may use the pinned neutral NPCHandler variant,never eligibility truth. PG fixtures apply/clear real native owned conditions;no outfit stages are seeded. Source hand-in prerequisites,quest progression and gifts run literally through NPC dispatch. Knight's existing source hand-in wording is retained;new unowned-addon requirement matches Morgan's explicit hasOutfit check,including account facts from another character.
L1-07 correction from actual PG83.573871312s Druid/79.114808667s Knight failures:two omitted primary door flags inherited the generic unknown-storage sentinel-1,but offers incorrectly required0. Source assignments are unconditional under the main stage guard:explicit writable General declarations now admit initial0/min-1 and assignments accept full-1..1 domain. Knight MissionHelmet assignments likewise admit its full native domain rather than invent an extra stage predicate. Python regression checks assert both door ranges and explicit defaults. Assassin full source path already passed actualPG92.674685234s before this metadata correction. The unowned-addon check takes the existing account appearance grant lock before reading and keeps it through costs/grant,so another character cannot race the negative ownership predicate and still charge. No duplicate account or condition owner is introduced.

## L1-08 source rules and compatibility

- `data-global/npc/elane.lua:55-148`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `beb84839e9b7b878fc276659bcbc0577bd82210f3cf900f53e5ddeb25dcc2cef`.
- `data-global/npc/legola.lua:60-110`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `26084a8748e9d3a01d59745bb30e67fb3610924430ce86034828c893c9b8dfc2`.
- `data-global/npc/angelina.lua:53-80`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `9c9d66498d3880f1c587aea1b43fd44f2c4ceaa9b8b839a4192aaa3dd572ca98`.
- `data-global/npc/lynda.lua:60-245`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9`.
- `data-global/npc/sandra.lua:68-124`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `566fd1c69a69ed447fbf6f56d08603631983cfbc9f203a11c41b71b9bd47ae53`.
- `data-global/npc/myra.lua:62-330`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `5939c5f235ac5f58bff2b2bf6668145e6da0a7ded222d46c3a792c02a386f94b`.
- `data-global/npc/zoltan.lua:60-150`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `c184ead30d167e379ccd628713028fd9cc8acc24713bb7c6943deefce14032d1`.
- `data-global/npc/eruaran.lua:153-199`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `6da2bd991078e0b7ac9cf96f855f4e931800e5609ba82280ea19bd749ae49936`.

Hunter grants preserve raw male/female addon permutations. Mage grants preserve each explicit source looktype, including cross-family Summoner grants and the female Summoner hat in Zoltan. Legacy neutral account facts remain compatible for both sexes. New qualified facts require matching sex in normal appearance projection and selection.

The male Mage hat has no source quest flag: qualification uses the actual unowned addon predicate under the existing account lock, exactly one carried 5903, and a same-transaction native quest/source/grant receipt. No invented progress flag or completion is recorded. All ten wand/rod types are consumed atomically; the existing bounded recipe limit grows from eight to sixteen.

Conservative donor correction: Eruaran repeats looktype 577 twice for each Dream Warden addon; the canonical outfit family includes male 577 and female 578. The family grant therefore supports both canonical sexes. Premium and each distinct source flag remain required. Missing Lua storage -1 is normalized to native zero for initial one-shot addon flags. Winning lottery tickets are supplied through ordinary native item custody, like other required rare materials; unrelated repeatable vial trading is not an outfit quest stage.

## L1-09 source item grants and paid exchanges

- Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` `data-global/npc/gnomally.lua:75-169`, sha256 `e598782edbd947acf0de2a13e42b4791536b5b8923d5132841891a77604133b1`:20Major16129 for16252/16255,70Minor16128 for16253/16256; private config[message] selection normalized to distinct ephemeral topics101-104. These are repeatable trades, not quest-progress storage.
- Same revision `data-global/npc/ottokar.lua:61-78`, sha256 `fe8c4a6740a75448c3af035f27c3b2783b9fe91ecad7ba4d255544003370a951`: one Medicine Pouch12517 buys belongings12413 and advances Doctor! Doctor! toward100. Reuse existing achievement owner, with per-command replay protection from the quest/source receipt.
- `data/scripts/actions/items/usable_outfit_items.lua:1-10`, sha256 `69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37`; `usable_afflicted_outfit_items.lua:1-61`, sha256 `d4a9f639be295771cf2d6f5c19c21435e0ec0ca0cb1684682ce4ad09717cb391`: byte-identical in the existing201189f item-action policy packet and9f5a72c corpus. Preserve existing item-action provenance and exact canonical look-pair matching. No synthetic cosmetics catalogue.

The chosen finite quests accept ordinary legal possession of rare addon vouchers and plague accessories; tests do not assert a loot probability or turn supply packages/belongings into a guaranteed rare drop. The full selected token trades, six-piece combination, Ottokar trade and native USE stages are exercised. A cost-backed value-only receipt is valid only with nonempty positive input and an exact native MINT in the same fenced transaction; the original source completion trigger continues to prove every consumption and MINT. A free gift or detached receipt is not admitted by this arm.

Paid exchange identity: the existing RewardClaim MINT owner remains one-shot per claim, including its pending/reservation exclusion. A source exchange with no storage/state operations/participants and positive inputs plus a MINT gets a deterministic per-command claim key: SHA256(version prefix, fixed-width Character UUID, Session UUID, colon-terminated decimal unsigned CommandId, source callback key). SQL derives the same key and still proves exact same-transaction command, input count and MINT item/quantity. Ordinary phase-bound quest claims retain their existing identity. No claim ledger row is deleted or made reusable.

Scope clarification for L1-09B: the selected Soil Guardian s7 and Crystal Warlord s6 explicitly require supply-package acquisition. Whole-entry qualification therefore also requires ordinary USE of15698 to produce the rare voucher. The original chance table is `data-global/scripts/actions/valuables/random_items.lua:49-77`, Crystal9f5a72c, sha256 `14fc3cf10ca10338aa4479a5371bb3dd7b9b0dede3353a4dec92ffdd11c7023b`. Preserve its1..10001 draw and exact intervals; a deterministic test witness is not a changed drop probability. The earlier legal-possession discussion does not waive these selected package steps.

L1-09B package implementation preserves every one of the25 contiguous source intervals, their exact quantities, and the10001-point roll domain. The existing generic reward-bag consume/MINT owner executes the captured row. Existing richer potion/ammunition profiles are reused; only six missing physical custody profiles are added. Gnomally's source `supply` trade costs2 Minor Crystalline Tokens, gives15698, and is selected by normalized ephemeral topic105; its existing canonical parent is `oteryn:quest.bigfoot_s_burden_quest`.

The PG acquisition witness uses the actual NPC-minted package UUID. Read-only evaluation finds a future ordinary CommandId with the rare outcome, and real harmless NPC commands advance admission to that ID. Neither the source item identity nor the production roll/probabilities is rewritten, and no command admission is skipped. The package is then consumed by ordinary USE; replay cannot consume or mint again.

L1-10 Golden commission: the selected definition in
content/quests/definitions/quests-00100-00199.json#/records/52/definition/oteryn_recipe/payload/recipe/stages
explicitly chooses 100000 gold base, 50000 boots/gloves, then 50000 helmet,
with no Premium requirement. Preserve those authored values and sequence instead
of donor 500000000/250000000/250000000 and helmet-before-boots. Donor
Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/king_tibianus.lua:117-216, SHA256
e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd,
supplies King dialogue topics, paired looks1210/1211, addon1 boots and addon2
helmet, existing GoldenOutfit storage, and carried-or-bank monetary semantics.
The donor-only decoration kit is outside the selected outfit programme and is
not an authored reward. Existing fee owner accepts equivalent coin denominations;
no synthetic inventory grant or duplicate monetary owner is introduced.
