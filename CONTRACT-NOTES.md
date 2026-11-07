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
