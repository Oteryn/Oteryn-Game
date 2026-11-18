# MAINMERGE6 delivery

Run `game-mainmerge6r3-1744` records the six PR merges delivered by
`game-mainmerge6-1627`. The standalone delivery branch starts at
`45135fdeecf2f448d0c300505d041c7397bf7731`. Each PR branch carries its own
merge commit.

## Verified PR heads

| PR | Branch suffix | Remote head | Merge result |
| --- | --- | --- | --- |
| [#3289](https://github.com/Oteryn/Oteryn-Game/pull/3289) | `npcplace` | `d6ffbc6021a78bfce84ff7a0f9236391fc25f0cd` | Merged the requested #3303 base. |
| [#3263](https://github.com/Oteryn/Oteryn-Game/pull/3263) | `overlordcr` | `83309a7c703292b7ba50dc9b25ce0bfe12046a6a` | Merged the requested #3303 base. |
| [#3277](https://github.com/Oteryn/Oteryn-Game/pull/3277) | `vocreate` | `f0200ed32f2bdb51b451f9e388615ce5e7df34c5` | Merged the requested #3303 base. |
| [#3266](https://github.com/Oteryn/Oteryn-Game/pull/3266) | `cwarlock` | `630ecc6b7483b567265b71cb9a54410c8dbdbfd0` | Merged the newer main tip. In `encounter_phases.rs`, kept the branch's fight variants and main's room and participant fields. |
| [#3269](https://github.com/Oteryn/Oteryn-Game/pull/3269) | `numkeys` | `e62a5e1ac2035a2e89f05d2ab686a3b5c8b211a2` | Merged main at `d9733eb0e7498eec48b3dfffed0e496c82c1d326`, which includes #3303. |
| [#3261](https://github.com/Oteryn/Oteryn-Game/pull/3261) | `mp09s` | `dc7865a304891a942fe0f7ea98e44f5b913cbee9` | Merged the newer main tip. In `full_world_support.rs`, consolidated the relogin helpers while preserving optional channel selection. |

## Validation

- Preflight passed on `3ffe8d05116b56d752944370a7c43341e0ca8bc8` and
  `45135fdeecf2f448d0c300505d041c7397bf7731`.
- `git ls-remote` verified the six PR head SHAs. Each contains the #3303 fix
  at `3ffe8d05116b56d752944370a7c43341e0ca8bc8`.
- `git merge-tree --write-tree origin/main <head>` exited 0 for all six
  against `45135fdeecf2f448d0c300505d041c7397bf7731`. GitHub also reported all
  six PRs as mergeable. The resubmission required no additional PR merges.
- The original run recorded matching World inputs, pins, and locks and
  successful package checks for `cwarlock` and `mp09s`. The `mp09s`
  test-target check timed out. The resubmission ran Git checks and preflight.

[Original run and remote push evidence](https://github.com/Oteryn/Oteryn-Game/issues/3302#issuecomment-6986037387)
