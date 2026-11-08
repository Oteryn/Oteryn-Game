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
