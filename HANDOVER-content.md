# Content session handover (2026-11-16)

Session: Content, session_01H5hdvQ699QV6YhBMxJyAF6. Coordinator: session_01D1SN2sEo2yY1k19f6kjEHf.
Server session (owner of shared files, #3056, quests): session_01H5hdvRcH9cRdJH2RatxpTE.
Laptop client session (Global/test-server measurements): session_01H5hdvNTF6gnjUDuZEAT6P3.

## What this session does

- Parity areas, one file per area in `docs/reference/parity/<area>.md` and one PR per area:
  healing (done), damage (in progress), defence, progression, regeneration/conditions.
- Owner rule 2026-11-16 (in `docs/reference/parity/README.md`): the Global import and Crystal
  are the baseline; a client measurement decides; a Wiki value alone never changes content.
- Charms follow-up gaps (#3183 follow-up, branch `charmgaps`), then #3181 (Wheel).
- Codex Cloud runs the code work: `node .claude/skills/oteryn-waves/scripts/codex-cloud.mjs`.

## Finished

- #3218 healing parity: merged.
- #3210 charm reset fee above level 100: merged.
- #3230 merged 2026-11-16 14:22Z (head e71da3d2249222964c27187d76dd7e87e641b739).
  - Contents:
    - Global client evidence from 2026-11-15.
    - Great Spirit Potion (i7642) mana restored to 100-200, everywhere it is defined: the
      canonical item, `register_spell_families.py`, `spell-native-profiles.json` and the dispatch
      parity test.
    - The owner rule.
  - Codex review P1 (native provider) fixed and resolved.
- Test-server measurements (laptop session, 2026-11-16).
  - The files are on the laptop, not yet in the repo:
    `/home/mole/oteryn-client-observation/docs/reference/parity/evidence/2026-11-16-testserver/`.
  - Monk cooldowns:
    - Base values, from a level-8 character without the Wheel, all match our data. The Crystal
      values are wrong: Devastating Knockout 8 s, Forceful Uppercut 60 s, Greater Flurry 16 s,
      Thousand Fist Blows 8 s, Mystic Repulse 8 s (base power 85), Spiritual Outburst 24 s.
    - The level-757 character has a Thousand Fist Blows Wheel augment of -4 s.
  - Runes on Hydra, from Invalid Monk (level 757, magic level 54):

    | rune | measured | n | our range | result |
    |---|---|---|---|---|
    | Sudden Death | 426-589 | 23 | 419-587 | MATCH |
    | Great Fireball | 227-317 | 25 | 178-346 | MISMATCH |
    | Avalanche (raw, Hydra ice 50%) | 224-314 | 29 | 178-346 | MISMATCH |

    For Great Fireball and Avalanche our spread is too wide. The measurements fit the classic
    formula level/5 + c*ML + k.
  - Spell level requirements: a level-8 character only gets "You must learn this spell first.".
    Swift Jab and Tiger Clash have no level restriction.

## Remaining

1. **Damage parity, branch `claude/happy-davinci-ijz3wy-dmgpar` (head 0598e00307ae).**
   - `docs/reference/parity/damage.md` has 981 rows: 626 MATCH, 33 MISMATCH, 322 UNKNOWN.
   - No PR yet.
   - To do:
     - add the 2026-11-16 test-server evidence;
     - mark the monk cooldown rows as MEASURED (ours correct);
     - record the Great Fireball / Avalanche mismatch.
   - Fitting the area-rune coefficients needs a second measurement point: another character
     with a different level and magic level. The owner is to log one in.
   - Not measured yet:
     - Stalagmite (Hydra is earth-immune; needs another target);
     - Lightest Missile, Thunderstorm and Stone Shower (not sold in Thais).
2. **Charms, branch `claude/happy-davinci-ijz3wy-charmgaps` (head 28dc0fe639d3, WIP).**
   - Codex Cloud thread 6afa3f2d-4798-8191-aaa9-3b599214e06f (ct-charmgaps3), last turn
     completed.
   - Open item: the flaky delayed-chain test.
     - Root cause: a cast pass whose deadline expires retires the holder connection, and the
       retry helper `cast_ready` never calls `await_ready()` to recover.
     - Baseline: 7 of 8 runs green (chain Fatal Hold failed).
   - Next:
     1. Implement the holder recovery with the assertions unchanged.
     2. Get 10 green runs.
     3. Merge main, push and open the PR.
3. **Healing-rune cooldowns:** UNKNOWN rows in healing.md, waiting for a client measurement.
4. **Supreme Health and Great Mana Potion:** Wiki values only, not measured. Needs a knight and
   a mage character.
5. **Spirit Mend formula fix:** handed to the Server session.
6. **Quests:** wait for #3056 (QMECH) to merge.
   - Donor extraction already exists: `tools/content-schema/quest-authoring/samples/`, including
     `completion-matrix/all373.json`.
   - Mechanics index: `qlane-meta/qmech/INDEX.md`.
7. **Later:** #3181 Wheel; drafts #3146 and #3154 (handed to Server).

## Open PRs owned by this session

None.

- #3230 and #3210 are merged.
- Do not touch: #3056 #3111 #3042 #3115 #3120 #3102 #3100 #3124 #3145 #3104 #3190, Client PRs,
  other sessions' lanes.
