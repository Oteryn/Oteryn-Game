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

L1-10B Brotherhood/Nightmare: reuse the existing account appearance and Quest
transaction owners for ordinary carried USE; never select the branch from item
number alone. Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d source:
- data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua:1-69,
  SHA256 1e3d15bb73efa6f3b26bdd1d0882a579279c0cff39c2fb166d98d03b3ed6be16:
  follower768/base, officer769/addon1, leader770/addon2; rank1/2/3;
  Brotherhood rank greater than Nightmare selects Brotherhood. Joining remains
  mutually exclusive, making that comparison unambiguous. Owned grants retain
  the document; addon use requires the base.
- data-global/npc/the_bone_master.lua:61-119, SHA256
  65a6ff562c38b78383ad4e9a4ba13ede2e875cd07686955f7c7741308d8fe3b1.
- data-global/npc/the_dream_master.lua:61-121, SHA256
  eac3bc09ddfe9b4fb83bb9751fc684f1687cc2f940365892cac5755ab50fe6f7.
  Join/yes/yes creates follower; subsequent advancement consumes500 then1000
  demonic essences6499 and creates officer then leader. No fixture-created
  documents count as the selected acquisition programme. Donor-only shield and
  rank4 advancement are outside these selected three outfit milestones.
The named Dreamer's Challenge prerequisite is the completed puzzle route, chosen
stage s4 (Tic Tac Toe). Its chosen stages s5-s7 already include joining Nightmare
and a document reward, so requiring s7 before joining would create a circular
condition and break the source's actual initiation. Read s4 through the original
Dreamer owner, without writing it or its Knightwatch/world effects here. PG only
arranges that separate parent's qualifying fact. Both order ranks and account
ownership are rechecked before costs. New per-character document claim facts
record the successful one-shot USE; immutable account ownership still prevents
another character paying for an already owned outfit or addon.

L1-11 selected level programmes (no missing-fact hold):
- Dream Warrior: Crystal9f data-global/npc/vanys.lua:83-157 SHA256
  fbc0bb272ed67594e0d385ae0baf41271c81aca511b28455bd896c51f083e59a:
  WardStones.Questline>=3 and SummerCourt=1 grant1146/1147; five
  pomegranates30169 and one ice shield30168 grant addons1/2. The bounded
  claim selector "dream warrior outfit" addresses that completed-task reward
  without intercepting the separate parent's talk/task initiation menu. Its
  readonly original tracks remain owned by The Dream Courts. The selected
  programme explicitly adds level250/Premium. Refuse already-owned grants
  before charging, correcting the donor's repeat-addon-charge behaviour.
- Citizen of Issavi: the authored recipe names an unresolved attire-keeper
  ROLE, not a source NPC. Bind it conservatively to Eshaya, the actual Wanted
  report contact. NPC identity/greeting: Crystal9f data-global/npc/eshaya.lua
  SHA256 0b84914661498605630d5e98bfb74579760e035d280e5395862c14b975e749db.
  The original Kilmaresh selected s21 reports the three outlaw victories;
  use that existing parent's chosen completion signal read-only. Do not
  invent independent outlaw kills or rewrite the parent. Selected base level250,
  Current Premium; eye-embroidered veil37003 addon1 then tagralt-inlaid
  scabbard37002 addon2, one each. Item identities are pinned Canary items.xml
  lines68951-68958 SHA2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
  The addon-slot order is an explicit conservative binding of the selected
  ordered accessory programme (the source recipe leaves native encoding open).
- Makeshift: the recipe explicitly authors a Gnome Deep Hub qualification desk.
  Bind that role to the admitted Gnomadness; reaching this physical NPC within
  normal interaction range witnesses the selected exploration. Salvage request
  and confirmation execute the authored apprenticeship before the base reward.
  Crystal9f data-global/npc/gnomadness.lua SHA256
  e3411f8704927cc665340418f3b65fa40fdafa6d607364fde90827516b9f7279
  supplies NPC identity, not a falsely recovered outfit callback. The recipe's
  requirements.min_level=100 wins over its contradictory adaptation text saying
  zero; preserve the stricter explicit requirement and Current Premium.
  Crude wood planks27657/addon1 and tinged pot27656/addon2 match
  Crystal9f usable_outfit_items.lua:14-15 SHA256
  69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37.
  Enforce the chosen first-before-second accessory sequence and award
  Cobbled and Patched only when the first addon is already owned.
Shared implementation: current persisted character level is read while its root
is locked; required addons use the existing compatible account facts. Storage-free
base claims require an exact neutral target/unowned-base predicate under the
existing account grant lock, with SQL1006 retaining all original proof arms.
Carried addons use the existing one-shot account predicate and exact input/ingress
proof without manufacturing quest progress flags.

L12 native hunting trials (AUTHORING; not yet qualified): reuse generic native
kill settlement from questpc3 commit5e72a3f0b0e16803bc5cd999298e4ec83f05eed9,
without importing its other-lane quest bindings. Frozen native death eligibility,
original party observation, occurrence identity, live lease and mission epoch
checks use the existing bounded kill queue and durable revision sequencer.
Dragon Slayer: selected20dragons +20dragon lords,10green/10red scales; base only.
Cave Explorer: selected20Spike creatures is conservatively a fixed Crystal Spider
assignment. Chosen level80 overrides source lower thresholds. Retain actual source
prices1000base/2000first/3000second (5000both addons): Crystal9f
9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data-global/npc/gnommander.lua:75-97,
SHA25662e3088421dd6f06c1decf2f2ce0c3c296d4a106b670cfa8927eb936e0bb69eb.
Fire-Fighter: selected10fiery creatures is conservatively a fixed Lava Lurker
assignment; Gnomadness is the physical Gnomprona qualification contact, as actually
located in Gnomprona Gardens by the same pinned source npc/gnomadness.lua:102-105,
SHA256e3411f8704927cc665340418f3b65fa40fdafa6d607364fde90827516b9f7279.
These fixed trials are explicit authored rules, not claims of recovered donor
outfit scripts. Firefighting axe39544 and portable flame39545 retain existing
canonical identities and native profiles. NPC dialogue accepts trials before any
counter credit; no direct counter writes are used in the positive PG stories.
Dragon Slayer appearance: pinned Crystal9f outfits.xml marks admitted looks
female1289/male1288 as store. The owner explicitly selected an authored Dragon
Slayer quest and authorizes missing bindings; promote this pair for its quest
base grant as an explicit exception, retaining exact captured appearance hashes.
Do not infer a recovered donor quest or enable unrelated store grants. The
canonical outfit now participates in the existing account grant/appearance owner.
A unique NPC exchange owns its exact keyword even with unmet predicates: after
eligible conversation offers, route that unique exchange to the locked durable
writer for a stage refusal instead of generic legacy-dialogue fallthrough.
Cyclops identity evidence: same Crystal9f data-global/npc/a_sweaty_cyclops.lua:1-14,
SHA256bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a.
The authored20+20trial is an owner-selected supplement, not a donor script claim.

L13 endgame accessories: pinned Crystal9f9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
 data/scripts/actions/items/usable_outfit_items.lua:26-31 SHA256
69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37 binds
Revenant skull34075 to native bit2 and bracelet34076 to bit1. Keep the chosen
skull-before-bracelet sequence using those actual slot bits; do not swap the
native encoding because the recipe calls its ordered steps first/second.
Cursed wood43899 bit1 then darklight geode43900 bit2. Min250/Current Premium
and original compatible native base facts precede consumption. Parent Soul War
and Rotten Blood grant mechanisms and their account provenance remain unchanged.
New PG variants compose at their earned native base reward checkpoint; the
original parent cases retain all their assertions and historical-state checks.
Rift Warrior: Cledwyn Lua:168-204 SHA256
915bab4d2c1e44b43cc4855c7b896c64e31cb3abe6598af461155bcbf8143431 demands
100silver tokens22516 for EACH addon, source looks845/846. Conservatively bind
the chosen completed-Ferumbras base attestation to Cledwyn as an explicit role;
his donor script itself only supplies addon exchanges. Parent completion remains
read-only under the original Ferumbras quest owner; do not duplicate its world
or boss mechanisms. Source NPC identities: flickering_soul.lua:1-6 SHA256
67a704c084d2490ea37ad337d472a7f3075375f0a3a043b1a615be12d5ac2405;
bloodshade_rotten.lua:1-6 SHA256
153105638eeaa55321f4e30caf4f5796291c89329092b32b3d4e6d6f9103df7c.

L13 restart reuse audit: questpc7 1f47892721766f4317426ee8445fea5e3f125818
crates/gameplay/durability/src/durability/quest_state/writer.rs:493 supplies the
existing root-locked read-only completed-character-revision query pattern.
Apply it to bounded requires_completed_quests in the shared reward participant;
no invented parent track or parent progress writer. Native parent bases still
come from original Soul War/Rotten Blood flows in the new PG variants.
All four source accessory items already have active physical custody profiles
in content/items/definitions/spell-native-profiles.json, from the existing
event-item profile producer. Reuse these; the older Canary XML lacks43899/43900
and is not evidence for replacing or inventing their physical facts.

L13 chosen numeric requirements: Rift recipe requires Premium and completed
Ferumbras but explicitly min_level0; omit its minimum_level predicate and qualify
its token story at creation level. The two parent-earned accessory programmes
retain their explicitly chosen min250. Rift's chosen stage sequence first/second
is preserved even though Cledwyn's original source permits either addon order.

L13 immutable base facts: accepted generic grant_callback_outfit contract
(crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs:11-29;
SQL1003:17-23,79-85) includes canonical base with addon grants. A previously
earned native base remains a separate compatible immutable receipt. Qualifying
assertions require exactly canonical0/1/2 plus that unchanged native receipt,
not a broad global count that incorrectly conflates their distinct sources.
Rotten parent Dependencies admission starts item_fence=None
(monk_save_postgres_tests.rs:213), unlike SoulWar source delivery. Initialize
the carried-USE phase from the fresh fenced owner read; no authority check bypass.

L13 final appearance carrier: failed Decay test database Character root
content_revision=content-1 while stock active cosmetics document=s3b-content-1.
The normal appearance owner correctly rejected that mismatch. Outfit-only
variant now uses the existing production_generation_with_source content-1
helper, preserving original parent-only native fixture and all production
validation. Actual final selection subsequently passed on PostgreSQL.

S06 stock mount population (2026-11-07): active authoring had no mounts although
51 qualified taming rules existed. Native rules retain their receipt-bearing
mount identities, including legacy spellings; their source display names join
to the mount XML after punctuation/suffix normalization. Other mounts reuse the
existing declared identity when names agree, otherwise use the source name.
No duplicate alias ownership is introduced. The sole non-name join is wind-up
loco: crystal9f5a72c data-global/scripts/actions/mounts/mounts.lua:70 explicitly
names mount194; data/XML/mounts.xml names194 Gloothomotive, look1459. This source
identity is retained rather than guessing from the action label. Action sha
bf241ac7938e044199b989a8c46a4b1ba787328dc44aab5d382fcb0ea7ec100f;
mount XML sha e5ed55b01274d1e29472a43224ccdb579684f8967244e50e8927ce3858a392b8.
All250 XML appearances are admitted by the existing pinned client corpus.
Existing item-action grants additionally require equal mount ID and client look
across their older source packet and this source; mismatches fail generation.
Declaring a store mount never grants it: only existing proven acquisition
receipts and provenance map to ownership. Source speed and Premium fields remain.

S06 construction audit: the four legacy item-target rows were never exercised
by the creature-only PG pair loop. Their placeholder targets name the tool
itself. Pinned crystal9f5a72c data/items/items.xml:29950/30010 identifies tin key
12305 versus inoperative tin lizzard12319; :31157/31160 identifies inoperative
uniwheel12800 versus golden can of oil12801; :70198/70201 identifies wind-up
key37397 versus wind-up loco37398. Item XML sha
c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb.
Mount action :40..46 lists all seven Glooth Glider recipe components, :115..141
refuses missing pieces before removing every one and granting mount71 plus
Fabled Construction/Natural Born Cowboy; :230..250 checks actual target name
and additionally consumes loco37398. Source action sha is recorded above.
Implement missing construction checks and atomic recipe/target composition in a
shared owner; these four rows cannot be qualified by the creature-only proof.

S06 source outfit inconsistencies: the captured addon-doll table pairs578/579,
618/619 and620/621, but pinned outfits.xml:64..66 and196..198 explicitly pairs
Dream Warden577/578, Glooth Engineer610/618 and Jersey619/620. XML sha
02a4f73b60237946109da1ec6ccb34257b246d9f3f746b5935caeacf2e9a0757.
Newly Wed328/329 has zero addon patterns in the admitted client appearance
records. Entire-rule qualification correctly refuses before charging when any
choice is unbound. Keep that guard. Native admission will retain each explicit
source family draw, use the XML's exact sex pair, and deliver base only for an
addonless family. The immutable capture is preserved; corrected native material
carries source packet digest, XML citation and explicit owner decisions.

S06 final source-family resolution supersedes the earlier Jersey assumption:
addon_doll.lua:37/38 have incorrect Dream Warden/Glooth Engineer pairs; use their
explicit family labels with the pinned XML pairs. :39 names Jungle Explorer,
female620/male621 (the table's first field is female, confirmed by Citizen136/128
at :4). This family was missing from the enabled XML catalogue and is now
admitted from the exact script row and client observations. :27's Wedding label
uses the already admitted Newly Wed328/329 pair. Wedding and Jungle Explorer
have no admitted addon patterns: their weighted family draws deliver the base,
without claiming invisible addons. All108 source draw branches and family
weights remain; 201used native grants are bound exactly. Original capture is
unchanged; 8 explicit lowering decisions carry original/native keys and sources.
New Jungle Explorer starts locked for both sexes and is Premium to wear, an
explicit conservative owner admission decision because its acquisition script
does not state wearing Premium. Its source item keeps its original Premium rule.

W15b additional roots: preserve the original primary reward and optional filled
container. Additional items are independent fresh backpack entries, never an
invented wrapping bag. Bound at most19additional entries by the existing20-slot
ceiling, with current occupied slots and all incoming weight checked together.
Every item has a domain-separated frozen transaction/event/instance identity;
keys, source text and one-item audit use the common existing mint participant.
The complete additional vector joins the semantic intent only when nonempty,
so old empty intents remain byte-identical. Receipt/reservation additional plans
must match, and deferred guards prove every item, custody, audit and metadata
before any commit. New guard arms preserve all installed owner arms.

S06 construction source review (2026-11-07, next step, not qualification):
Crystal revision9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
`data-global/scripts/actions/mounts/mounts.lua:20-36,40-41,70,234-253`,
sha256bf241ac7938e044199b989a8c46a4b1ba787328dc44aab5d382fcb0ea7ec100f,
contains the full seven-component Glider recipe, tin/uniwheel item-target rules,
and locomotive target consumption. `data/items/items.xml:29950,30010,31157,31160,70198,70201`,
sha256c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb,
resolves the exact named target definitions12319,12800,37398. The current capture's
self-target placeholders and guaranteed tin/uniwheel chance do not express these facts.
Owner normalization decision: use the explicit config CHANCE40 as the success
probability, consistent with the generic taming probability contract; the donor
TYPE_ITEM branch reverses its comparison at237 and would otherwise fail even
CHANCE100 locomotives almost always. Keep that donor defect as evidence, rather
than adopting it as the native probability contract. Preserve both authored oil
failure branches and their independent break/no-spend actions. Source-item
normalization must retain all47 creature rules plus3 item targets and route
Glider through one complete seven-piece recipe; never silently omit a source row.
A clicked Ground Glider blueprint is only the trigger, following the donor's
carried getItemCount/removeItem loop; consume the seven carried ingredients.

S06 Glider physical admission: added only the six absent component IDs to
the existing qengine_reward_item_profiles producer. Its pinned
imports/canary/items-xml/items.xml SHA256
1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2
and client appearance SHA256
2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2
supply exact physical/stack/custody facts; known canonical facts retain priority.
Item XML locations: 21898:line43247, 21899:line43250, 21901:line43258, 21902:line43262, 21905:line43275, 21906:line43279.
Canonical Item declarations and source captures remain unchanged; the existing
native overlay admits the six portable plain components. Focused source test
GREEN14.278s verifies every component's materializable inventory/Ground policy.

S06 explicit source acquisition after main1ae39c7fb: main cosmetic_item_use.rs:133 and consume_transform_cosmetics.rs:81 correctly keep donor-only random sources refused even with complete grant mappings. The full focused PostgreSQL run found exactly five stock outfit refusals (56.553534s Wedding,72.182964363s Dream Warden,99.31327755s Glooth Engineer,102.569482848s Jungle Explorer,74.933472322s default draw); other74 cases passed. Keep those guards through generic, optional, independently OWNER_DECISION-promoted item_acquisitions in the native cosmetics document. Admission binds exact Item production key, exact donor action and SHA256 of the entire native action catalogue, plus source citation and owner decision. No source look, family label, profile revision or complete synthetic mapping pool alone permits a source. Emit only the current source-backed addon-doll admission; other unpromoted random sources remain refused. The existing OT-only PostgreSQL refusal test remains unchanged and is added to the rerun. No SQL schema or legacy provenance changes.

Authored engine decision 2026-11-07: keep source identity resolution explicit and catalogue-scoped; no NPC/item target is inferred from recipe prose. Reuse questpc3 bounded carried exchange input proof from commit9936a44dd27374dfcd9c5430888c972b0d35e5ef, apps/game-server/migrations/1226_quest_carried_exchange_inputs.sql:4-25, and selected-definition filtering from crates/gameplay/durability/src/durability/quest_world_callback.rs:144-157 at that commit. Existing QENGINE2 legacy tool test requires a give mutation to remain refused. Add an explicit default-false carried_exchange opt-in rather than relaxing legacy tool admission; preserve the original exact selected-instance proof and same-transaction reward MINT proof. New SQL goes in own1552 range; collector must reconcile the peer1226 equivalent when collecting that lane, retaining one installed implementation.

PREPUSH-2 regeneration coverage decision: run regenerate_content.py --resolve --skip-checks for generation, including the World package, inventory/NPC pins, Item+Mount content tree, native registrations and final world pin-refresh. Required changed-crate unit tests and own PostgreSQL cases are separate commands, not omitted by generation-only mode. Run tools/contracts/update_item_audit_tags.py for operation tag lists and wire golden, quest_binding_authoring.py for NPC/quest binding evidence, validate_world_project_v2_to_tree.py and validate_world_metadata.py after generation. Rust module lists regenerate through repository build scripts. Never hand-edit a pin. The separate raw-donor world-metadata recapture tool requires Crystal00ce02a57ca5a12e48f32a3476e37471167e4c3f:data-global/world/world.otbm (SHA256dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7; convert_world_metadata.py:52-69,381-389); no raw OTBM was found in checkout/environment source files. Preserve its committed normalized capture as an input; regenerate and validate shipped metadata through the World pipeline, without inventing donor bytes or changing captured facts.

QNPCFX integration decision: source mainf641d8d6c introduces opaque NPC program hashes and predecessor/default maps. Preserve them for unaffected quests in authored attachment and with_quest. A changed authored quest hashes the prior native-program binding and drops NPC-only grandfathering entries; no implicit migration of changed semantics. Regression native_program_hashes_and_predecessors_survive_authored_attachment was genuinelyRED on the state-only hash beforefix; full quests suite283/283 GREEN41.627s. Additive merge driver initially placed authored_bindings in NpcDefinitionPredecessor; restored it to QuestStateCatalogue before semantic tests.
PREPUSH-2 second-main regeneration: reuse full World package just regenerated at9d7946b7c because no content/world or materializer source changes, item aliases/ranged/drown inputs identical, and canonical Item shards identical SHA2561eba2b725075e304b8396bc3a8a2921cfb7dc4c28f67f7c71879eb78398847a9. Refresh all changed native providers, Item/Mount tree, registrations, grizzly rooms, audit tags, NPC binding evidence and world pins with repo tooling. Evidence /tmp/qengine2-qnpcfx-world-input-reuse.json. This is the supported unchanged-World native regeneration mode, not hand-edited pins.

QNPCFX/MAINRED integration: source mainf641 new NPC consume_npc_inputs still called apply_input with3arguments, while e1ca3e003 requires SpellLanePermit + fence for ground retirement. Do not manufacture a spell permit or remove that guard. Extract the common item-row update into carried_input::update; keep ground deletion and channel permit check in apply_input; NPC hand-ins use carried_input::apply, which validates a carried location class before any mutation and cannot select Ground. Same NPC receipt/consumption proof remains. New main reward grants initialize additional_items empty, preserving single-root behavior and Father atomic11-track writer.
Incremental generation follow-up: world_project_v2_to_tree resets derived family registrations, so rerun every standard AUTHORING_TOOLS producer plus grizzly_rooms afterward. Missing that normal post-step caused LOCK_COUNTS (Encounter.Raid absent); rerunning raid_catalogue and the normal authorers restores239raids. No lock or pin was hand-edited.

Remote-tracking decision: this checkout has only main in remote.origin.fetch. A named-branch fetch fills FETCH_HEAD but does not refresh origin/qengine2; use an explicit refs/heads/lane:refs/remotes/origin/lane refspec before comparing/merging. Publication279a9cff8717636d82b7dfb37be8d0bc46ad1b7d was independently verified via ls-remote and the lane tracking ref refreshed. Shared-engine reuse audit refreshed ten live peer branches; qworld and qnpcfx remote branches are deleted and their canonical owners already merged in main. Added three source-cited engine requests from current questpc3/questpc8 packets without taking their quest ownership.

23:20UTC grouped repair: bounded unit run found exactly3 assertion failures before the PREPUSH-2 budget stop (1601passed,2interrupted,2672notrun; no full-suite success claimed). Two failures require retaining Glider21897 in the51-row source catalogue, while own qengine_rewards.py previously removed it to prevent a one-piece fallback. Retain the row with required_items derived from the already source-bound atomic pieces program (mounts.lua:20-36,112-139); all single-item taming selectors refuse construction rows, leaving the existing complete recipe owner. Preserve all other50rows and strengthen population/cost assertions. Equipment audit expected53 but source producer IDS now60; add repo regeneration tooling rather than hand-editing the derived count. Inspected collector4a08b3ebbedac6e9197d77ac65f216d2514deaf5 equipment WORLDCHG/SRV9 witnesses; those cohorts are not shipped on this branch, so their additions were not retained. Native source scope remains unchanged. Main47db34f09d760b2f435860c2f8d4ddf12a893e05 now canonically owns NPC apply_input_rows: merged as05911c4e6 and deleted the lane-local duplicate helper, preserving own selected-definition exchange filter. Native-only generation omitted qengine_rewards.py; pipeline now invokes it and callback-proof refresh before native registration so taming-native reproduces51rows. No new source facts, grant semantics or quest qualification claimed.

Collector synchronization 2026-11-08: merge canonical78294c081 as1e99701c6, retain incoming World/NPC changes and original migrations1330-1349. The previously inapplicable WORLDCHG/SRV9 equipment cohorts now ship on main and are preserved; own count60 comes from repo tooling. Native profile comparison against main shows only seven added Glider keys, zero changed existing profiles. World files match main exactly; reuse its generated World package, regenerate native/tree/central outputs and refresh/check pins, avoiding raw donor recapture or handwritten hashes. Seven PostgreSQL18.6 cases qualify source15ac only until the merged binary is tested. Error-codes build.rs watches HEAD; avoid an unnecessary rebuild by keeping HEAD stable between focused test invocations, or run the exact existing clean-source binary with isolated cases and server/duration proof. No build-source diagnostic is faked or modified.

2026-11-08 authored kill burst review correction: the suspected unchanged-gate
revision bug is already prevented by commit6738c1da7ad5cf35110145fe33dfa9ea377ab0d7 in
crates/gameplay/durability/src/durability/quest_state/writer.rs:709-710:
`if native.is_some() {changes.retain(|change|change.before!=change.after);}`.
The real three-death queued burst passed PostgreSQL in133.793s without a new
production fix. Preserve this existing guard; ordinary non-native mission/reset
writes retain their revision barrier. The original review missed this earlier
filter before the track-upsert loop. Retain the new regression; do not apply
/tmp/qengine2-apply-burst-fix.py (redundant and unnecessary).

2026-11-08 world delivery decision: reuse the qualified peer read-only per-player
world USE gift contract. Preserve shared object revisions; use owned quest guards
and complete MINT in one type2 transaction. Synthetic template provenance is
inherited from the retained Behemoth callback solely for parser coverage; no
Behemoth or Draccoon acquisition lore is authored or qualified by the fixture.
Only new migration1553 is copied/adapted; do not duplicate peer1225/1101 or the
already-owned1552 carried-input extension. Peer original world gift fields were
moved into effects by later main merges; use the current effects shape.

Main83acbdac3 integration: completion-exact migration1254 requires reward plan
completes to equal the owning Quest receipt and completed state. Preserve the
existing authored Stage.completes authority by explicitly reconciling normalized
callback completion at runtime attachment. Main also includes world item reward
support: drop duplicate shared implementations during reconciliation. The
attempted16-conflict merge was aborted under the owner's15-minute fallback;
this is active reconciliation work, not a final blocker.

Main83acbdac3 already owns world reward MINT and richer callback effects.
Decision: remove QENGINE2's duplicate1553 and helper entrypoints; retain main's
canonical mint transaction, premium/fluid behavior and admission hashes. The
world PG fixture now verifies the canonical MINT receipt and its same-transaction
completed USE receipt rather than an extra game_quest_reward_sources participant.
This preserves atomicity/replay/capacity assertions and adds transaction equality.
Authored stages use canonical primary_transition builders for NPC and world
callbacks, preserving increment/raise-to/admission-bound effects from main.

Merge integration found both main SetWithBinding and lane-only SetWhenAtLeast
using definition-encoding tag7. Focused regression RED proved left7/right7.
Preserve main's published bound-set encoding; move authored kill threshold to
unused tag11. Existing legacy/base effect hashes are unchanged. The threshold
feature has no production quest binding data in this lane; synthetic PG cases
will be refreshed against the corrected merged catalogue before qualification.

Native binding admission decision: native boot catalogues carry immutable canonical
recipe evidence; bare synthetic catalogues retain their existing low-level API.
No caller-controlled admission boolean. Compiled registry follows existing
build_npc_programs.rs. Fresh search of8active npcfix/questpc heads found no
canonical recipe admission implementation; deleted qworld/qnpcfx/wpkg are reused
through canonical main. Draccoon chosen recipe witness content/quests/definitions/quests-00200-00299.json:1 at
2ddd4f9cd; file SHA256 359681e392e0588e5f76ef80af7ff6f780550271da2bdacfa0f32f5754f84c02, exact recipe SHA256
c6bb54b0c423a165891b0ca32fb305a0963d1a06417249b0b317ec34cf65e708.
All68 current chosen recipes contain no floating point values. No readiness
promotion or whole-quest qualification is implied by registry admission.

Definition-ready claim decision2026-11-08: the shared route already exists in
main's dispatch/bundle_chest.rs and content/world_reward_claims.rs, also present
on fetched questpc7. Qualify/reuse it instead of implementing a second resolver.
The42-row derived completion queue only chooses test coverage; it never grants
runtime authority. A claim with multiple placements must bind every source
placement together. Canonical Adorned UH Rune reward11603 is the decorative
silver rune emblem, not a charged healing rune. New source-position fixtures
prove selector/item admission and ordinary dispatch behavior, not actual World
population or ownership/completion of another lane's quest.

Ready-claim admission RED: bloodherb is left out by the real native binder;
11/54rewards have no native profile although canonical base records are ready.
Pinned donor imports/canary/items-xml/items.xml SHA256 1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2
revision47dfd51f45280a59a1d3e50ba7edd573d7234446 source lines: 21982:43508, 2822:5804, 2823:5809, 2848:5904, 3068:7082, 3310:8893, 3323:9111, 3383:9899, 3579:11347, 3734:11718, 4871:14211.
All11 XML entries have no charges/duration/decayto/stopduration fields. Use the
existing exact-bound physical importer; retain its canonical/instance guardrails.
