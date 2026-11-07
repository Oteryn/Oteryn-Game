# WEVENTS contract notes

Owner rule (2026-11-06): publish each tested mechanic slice immediately, at least
once every 30 minutes; bound any individual fact investigation to ten minutes.
Unrecoverable rules use explicit conservative policy rather than deferred work.

- Flower Month follows Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
  `data-global/scripts/raids/world_changes/flower_month.lua:1-65`, SHA256
  `dc1607c195712ed166dcda95423fba91821d0ea1b22b85a249fc9aab8d74b1bb`:
  four areas, 25 Dryads per wave, 250 shared placement attempts, 15-minute
  rotation and indefinite repetition until reset. This supersedes the wiki's
  five-wave account. June admission uses the committed reset snapshot, including
  June 30's overnight tail until the next Berlin 06:00 save.
- Mummy follows the same donor revision,
  `data-global/scripts/actions/worldchanges/the_mummys_curse/horestis_jars.lua:18-122`,
  SHA256 `6e47be40083c0ff174c454940382f891f96d7c304c999457e5165ab98d24751a`.
  The owner explicitly removes the duplicate delayed boss call: one Horestis
  entry per existing World-raid firing target. Executable wall delays are
  277000 ms for 1603 and 1200000 ms for 3514, despite the announcement text.
  Current B3 walls already contain 1603; the event projection manages both wall
  appearances at those four source positions so the opening actually opens.
- Chyllfroest includes May 1: April 1 inclusive through May 2 exclusive, from
  `data-global/scripts/quests/tinder_box_quest_chyllfroest/movements_prohibit_entry.lua:12-21`,
  SHA256 `72a5b54d8759917fd75072692cef0e01e67beb70305b5839d32e1135f2322f84`
  at the same Crystal revision. UTC is the explicit synchronized calendar rule;
  the donor does not pin its process timezone.
- Native startup selectors come from `data-global/startup/tables/item.lua:3087-3126`,
  SHA256 `677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd`
  at that revision. Tooling verifies all sixteen selectors against retained
  bytes and native B3 objects; no map binary rewrite is needed.

Detailed validation and subsequent event decisions belong in
`docs/architecture/reviews/WEVENTS-Q.md` and this task's WORLDCHG-Q rows.

## Spring into Life implementation policy

Recovered donor facts: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data/items/items.xml:33011-33052`, SHA256
`c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
defines dragon nest 14757 and surprise nests 14750/14759.
`data/scripts/actions/items/surprise_bags.lua:6,17-27,31`, SHA256
`66ba74a2b736e0631776ac7674c21a6838ebd5ecd25d9804b7f9613fefe11e1e`,
uses 14759: eleven uniformly selected categories, with quantities uniformly
1–10 for candy and five coloured eggs, and quantity one for chocolate, hydra
egg, blue/red surprise bags and phoenix egg. The existing reward-bag owner
already implements this rule; reuse it rather than add another random grant.

The ten-minute investigation budget found no donor calendar, nest hatch clock
or merchant implementation. Conservative rules selected for implementation:
April 16 server save through April 23 server save, one hour to hatch an admitted
native nest, and one free participation reward per Character per season.
Reuse Rosemarie's existing source-bound merchant population during this window;
paid surprise nests cost one crystal coin each and cannot bypass participation
requirements for the free reward. These calendar/hatch/merchant choices are
Oteryn policy, not claims about donor code. April as the month is independently
corroborated by
https://github.com/s2ward/talesoftibia.com/blob/9bc90ae10ee2f0c6ea21063b4188a402e378e4bf/site/json/events.json .

## Shared lifecycle policy

The Spring map scan found six source dragon-nest objects, of which four are
inside house 3015. Only the two public objects at (32344,31604,6) and
(33977,32646,6) are eligible selectors; house objects remain excluded.

The reusable lifecycle model uses these explicitly authored fallback rules
pending the native bindings and admission proofs: Rise runs September 1 server
save through September 8 server save, with a 24-hour preparation stage, five
distinct incarnations credited within five minutes, a synchronized retry on
failure, then a final boss. These timings are Oteryn policy where retained
source bytes did not supply a complete scheduler. Kingsday continues hourly
waves after a victory during its selected day. Bank robbery checks once per
elapsed hour after reset at 10 per mille, consumes failed checks, and admits at
most one robbery per day. Insectoid restoration uses three shared fires and a
bucket holding at most three units; each successful extinguish spends one
unit atomically. Reset creates a new cycle. These are model rules, not claims
that the six remaining titles have completed native qualification.

## Devovorga's Essence price tiers

The retained recipe identifies Tentacle Piece `oteryn:item.tibia.i11666`,
Anmothra's two-piece offering and the selectable 2/5/8/12/16 tier counts:
`content/quests/definitions/quests-00200-00299.json:1`, SHA256
`422ff275d4b416d2a77acf960bf61fb88ae68b044ca738b45198b1ac9d803a8b`.
It cites the pinned article
https://tibia.fandom.com/index.php?curid=76216&oldid=845419
(body SHA256 `4f0767fb999b622c67eaef1d49a978c8a7502d122504176c51c71b3fd59d033e`)
and spoiler
https://tibia.fandom.com/index.php?curid=76217&oldid=944253
(body SHA256 `0e5d9ea11a467ee2aed7b1baf384992f9f76253729affa594d07973372388cd0`).
The engine's five incarnation slots use those counts and that item. Spring's
independently authored merchant price remains one crystal coin.

## Seasonal expiry and isolated test time

Expired active/dormant cycles transition to failure when the qualified event's
scheduler next runs; a bounded indexed page prevents an unbounded catch-up
transaction. Victory remains final. Current-cycle jar actions do not acquire
locks for older annual cycles. Fixture-only time overrides are scoped to a
Root and its clones, reject production Roots, and never change PostgreSQL time.

Spring public nest selectors are authored IDs 52601/52602, on the two verified
source objects (item 14098, donor items.xml:32647-32650, same SHA above).
Each successful native hatch rearms its public nest for one hour. A stale or
replayed placement ACK cannot rearm it twice. If every target Channel reports
failed placement, retry after an hour without awarding a hatch. The latest
successful hatch time preserves a participant's earned claim when joining
another nest. Superseded persistent hatchlings use the raid runner's LOST
cleanup; they do not accumulate across hourly nest generations. The merchant
reward/purchase tests seed completed-hatch prerequisites separately from the
scheduler/placement and actual-actor cases.

Rosemarie's native conversation reuses her retained greeting, farewell and
walk-away lines, rather than a placeholder: Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/npc/rosemarie.lua:51-53`, SHA256
`e96bd8db17c9fa763a6e38a7dffa1df0f3e02f09110f4b13b32accfecd58aa84`.
The shared quest-reward greeting registry supplies her conversation; the World
binding still admits or refuses each seasonal purchase and free reward.

Completed raid firing rows remain immutable. Migration 1125 records retirement
separately; current physical-effect reads and timed kill admission reject a
retired firing. Running firings may transition to LOST, while completed firings
retain their terminal state and append normal LOST cleanup effects. This also
prevents a prior successful hatch's retained actors from accumulating when its
next object revision fires.

## Kingsday implementation policy

Royal greeting words and reply follow Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/npc/king_tibianus.lua:482-491`, SHA256
`e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd`:
"hail king" / "salutations king", the loyal-subject greeting and "How rude!"
walk-away. The default farewell follows the retained npc_handler.lua:82,
SHA256 `8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a`.
The retained normalized placement is (32311,32172,6):
`tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1`,
SHA256 `1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a`.

The pinned recipe explicitly ties Loyal Subject to respectful attendance,
not arena combat:
https://tibia.fandom.com/index.php?curid=76245&oldid=994341
(body SHA256 `0b67e7e6c7047759d08c7efec385a5d1ab30586518258f185cec9466cfa0cef3`).
A respectful greeting opens the native conversation; asking about the
celebration records attendance and atomically reserves its achievement claim.
The existing achievement definition is `oteryn:achievement/loyal_subject`.

No complete donor celebration scheduler or exhibition population was found
within the investigation budget. The conservative authored policy retains the
model's 24-per-mille daily activation, hourly exhibitions after the first hour,
and two Orc Warlords per exhibition. The Oteryn exhibition ring is the verified
public courtyard (32304..32308,32184..32188,7), all house 0, flags 0 and simple
ground 103, in `content/world/placements/region-z07-x126-y125.b3`, SHA256
`b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037`.
These location/population/chance choices are explicitly Oteryn policy, not
claims that the retained royal dialogue supplies a raid. Previous exhibition
actors retire on the next firing or day reset. Arena waves continue after
attendance victory; a daily claim cannot repeat the account achievement.

## Bank Robbery implementation policy

The conservative native scope is Thais, Naji and Jesse the Wicked. The existing
hourly 10-per-mille draw is consumed once per hour after the first hour; one
successful robbery closes Naji's bank service until recovery or the next save.
This is explicit Oteryn policy where the retained bank dialogue supplies no
complete robbery scheduler. Other cities are not selected implicitly.

Jesse's existing native creature and guaranteed stolen-gold bag loot follow
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`data-otservbr-global/monster/bosses/jesse_the_wicked.lua:21-22,77`, SHA256
`08d8ca5871bf4b1bbd3123d940dc26193796bc70dce003f63c21bf0eec2825c2`:
https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/bosses/jesse_the_wicked.lua#L77
Recovery must consume item 13429 with a native MINT cause matching the actual
current-World/current-day robbery kill. Ordinary bags, previous-day bags and
foreign-World bags do not prove recovery. The bag debit, shared reopening and
quest reward are atomic. The pinned quest reward is ten platinum coins and
Honest Finder, per https://tibia.fandom.com/index.php?curid=76139&oldid=1082154
(body SHA256 `a865de3250f6abd2e432be0b265a88cb14e6bc202b07661af28f2f9cd35d5cc1`).

Naji's retained normalized placement is (32342,32229,7), in
`tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1`,
SHA256 `1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a`.
The authored escape alley (32336..32339,32233..32235,7) has simple ground 506,
house 0 and flags 0 in region-z07-x126-y125.b3, SHA256
`b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037`.
The position is an explicit Oteryn fallback rather than an inferred donor raid.

The historical canonical bag weight is 5000, but retained Crystal
`data/items/items.xml:31463-31466` @ `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
SHA256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
and pinned Canary XML:31281-31284 both explicitly declare **500**. Per the owner
donor-wins rule, the active native Item profile uses 500. The historical
canonical import remains intact; the source override is explicit in the
generated qengine reward profile evidence. No other known physical fact is
overridden. Naji's registered `parseBank` callback is retained Crystal
`data-global/npc/naji.lua:61-80`, SHA256
`7752587c14e87d611edc01ee3723afb75fa08c2f29aded246ad458353ed7c1ac`.
A published event-owned bank service admits this existing native Bank owner;
profession labels and arbitrary donor callback text alone still grant no service.

Jesse's corpse chain also needed native admission. Retained Crystal
`data/items/items.xml:37349-37370` @ `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
SHA256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
agrees with pinned Canary XML:37154-37176: 18153 (20 slots, 10 s) → 18150
(20 slots, 300 s) → 18151 (10 slots, 300 s) → 18152 (60 s) → removal.
All four native profiles are Ground-only and reuse the existing durable
absolute-deadline/decay owner. Their legacy reference-only identities remain
unchanged. Native loot admission now derives the corpse capacities from those
exact profiles, alongside the stolen-gold bag's portable admission.

### WEVENTS: Greenshore restoration policy

Insectoid Invasion uses the existing 100-permille daily World selection. Its two
Scouts are reserved from ordinary activation and projected through the timed raid
owner at their retained positions (32208,32058,7), (32209,32060,7).
Evidence: `content/world/spawns/spawns-10000-11999.json:1`, SHA256
`a74b7da322bba5ebea20c772dae368d1337ec4caf9cd146f89b1bb9b8419b051`;
the source respawn interval is 90 seconds. Oteryn policy replaces the preceding
population at each synchronized 90-second slot, bounding the event to two Scouts.

No complete donor restoration script was retained. The conservative authored
rule is one shared bucket with capacity three and three shared fires: filling
adds one water unit, extinguishing consumes one, and extinguishing all three
ends the invasion. Full buckets and repeated/completed actions are no-ops.
The four native ground USE selectors and decorative, nonpickupable projections
are explicitly authored in `content/quests/world-event-object-policy.json`.
The selected public tiles are verified against
`content/world/placements/region-z07-x125-y125.b3:1`, SHA256
`268c6b75fad682c6d9baeadc6563c2cace27d8414caa66ca538b49687a6dc262`.
Victory removes the event population and projections; reset starts fresh state.
These restoration details are Oteryn policy, not claimed donor behavior.

### Insectoid Invasion implementation policy

The Greenshore shared restoration rule, native coordinates and source hashes are
specified above under [WEVENTS: Greenshore restoration policy](#wevents-greenshore-restoration-policy).

### Devovorga world event policy

Retained Crystal `data-global/npc/a_beautiful_girl.lua:1-58` at
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, SHA256
`2a4dd3dc8afca0ee526d223b827d773b2974ffd0aeb198ce5da7a0b583f33e61`,
contains NPC setup and a greeting, with no calendar, encounter fee or reward
implementation. The retained corpus and inspected public tree contain no complete
Devovorga World controller. The following calendar, mutation and location choices
are explicit conservative Oteryn rules, rather than claimed donor script facts.

Rise retains the existing September 1 Berlin 06:00 through September 8 Berlin
06:00 window and one-day preparation. Five distinct incarnations must fall within
five minutes of the first credited defeat. Bloom of Doom and Spawn of Devovorga
then precede a second synchronized incarnation round, Vulnerable Cocoon, and final
Devovorga. Expired incarnation attempts restart together; seasonal expiry fails
an unfinished event and preserves a completed victory. Daily reset readmits
uncredited members. Global stage completion retires superseded projections.

The authored public Vengoth clearing and separate paid encounter room are verified
against `content/world/placements/region-z07-x128-y123.b3:1`, SHA256
`865ee3aa9e0c4d592571b40b72c137a8b05a19f850e0b570974276ebbeb9a22d`.
The existing 16-permille daily Essence selection gates five entrances. Admission
consumes tentacle pieces 11666 in tiers 2/5/8/12/16 for
Anmothra/Chikhaton/Irahsae/Phrodomo/Teneshpar. Native encounters first bind the admitted generation's complete creature policy
table, including in a Channel with no previous creature spawn. Native encounter
preflight and a bounded room lease precede the fenced debit. Active calls are serialized against
other native room admissions. These authored USE entrances admit level-1 and
higher characters without an upper level bracket; this is an explicit Oteryn
admission choice in the absence of a complete donor encounter controller. The source recipe and pinned wiki citations for
these tiers remain in `content/quests/definitions/quests-00200-00299.json:1`.

Normalized actor facts and exact public source URLs/file lines/SHA256 are in
`content/quests/world-event-creature-policy.json`. Public Fencore source is
`fifth-empearl/fencore` @ `00484ac96d3379fd7f18a3a410e53c2ae31fc981`:
`data-otservbr-global/monster/devovorga.lua:5-25,83-90` SHA256
`32f3e3a4854a0e30c223bc58beb1a1d307128afcae17267e32bcf815ff3c1361`,
`phrodomo.lua:5-25,77-86` SHA256
`78ed2e396f95cb2d516f453bdf7d5b205f24c5a174355140e6eaaf3066481830`,
and `teneshpar.lua:5-24,83-92` SHA256
`d1faede5a81ca84fc06f0c3f6d37e61d8e0bc8d05bfe5d191ed8f61edc012b07`.
Anmothra and Irahsae literal health/default defensive facts come from
`soul4soul/converted-tibiawiki-monster-data` @
`a2cccf99fccc547e13720704f99cd0125f22f82f`,
`tfs revscriptsys/Anmothra.lua:6-22,87-88` SHA256
`f2fbfc4e6bd51d5c338d96abfd9c1a9bb24f7ea8a329ed79fe71f2a8f0f94711`
and `Irahsae.lua:6-22,87-88` SHA256
`a0ef65c05bf48aa5ead6f0fb8bd451e42ff2e4fad482d53f924ddac4867b2160`.
Their zero outfit IDs are incomplete conversion fields: the authored fallback
uses Chikhaton's presentation and Cave Rat physical attack. Mutation actors use
1000 HP, with Bloom/Cocoon passive. Other physical attacks use donor ranges or the
existing native nonlethal attack/skill approximation. These actors claim primary
physical combat, not complete donor spell parity. Null loot bindings explicitly
admit no ordinary corpse/loot/XP; fenced event rewards remain separate.

Rise's chosen level-160 reward is one Demon Backpack 9601 containing five
Tentacle Pieces, three blue giant shimmering pearls 281 and five Gold Ingots
9058, plus I Did My Part. This retains the selected high-level source package;
the spoiler's blue pearls win its base-page green-pearl conflict. Admission is
once per character/season, requires global victory and a native final-boss defeat
receipt, and uses the existing container MINT and account achievement owners.


The existing NPC reward proof in migration 0999 admitted only `command`, despite
its Rust owner accepting native selector rewards. Migration 1127 uses the same
closed `command`/`use`/`step` set without relaxing same-transaction receipt,
consumption, MINT or outfit evidence. Native reward USE remains a fenced Quest
and World transaction; it does not bypass the reward owner.
