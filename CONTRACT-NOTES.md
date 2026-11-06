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
