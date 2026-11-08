# Phase A reward-source qualification, 2026-11-06

PROVEN:12 real dispatch PostgreSQL cohorts passed:56 reward sets,104 distinct claims,106 source placements. 88 claims newly qualify;16 retain earlier locators. Global inventory127qualified/207supported-unqualified/2Dawnport-excluded; A..M106qualified/44unqualified/2excluded. Source reward proofs do not certify access puzzles, NPC stories, outfits, quest-log integration or deployed World admission.

The first11 cohorts and shared scenario are byte-for-byte unchanged after adding cohort12. Exact first-run source is retained at `.questpc/restart2-phase-a-source-11.rs`, matching its saved hash. Original manifest11 and extra manifest12 remain immutable; default full rerun membership is `restart-phase-a-cohorts-all.json`.

Command: `/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib quest_catalogue_chests_phase_a_ -- --test-threads=2 --nocapture`.
Wall duration:1003.407s. Full log:`.questpc/restart-phase-a-pg-2.log`; all per-source positive durations:`restart-phase-a-pg-2.qualified.json`.

Command: `/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- quest_catalogue_chests_phase_a_12_use_replay_and_refusal --test-threads=2 --nocapture`.
Wall duration:189.941s. Full log:`.questpc/restart-phase-a-pg-3.log`; all per-source positive durations:`restart-phase-a-pg-3.qualified.json`.

- `quest_catalogue_chests_phase_a_01_use_replay_and_refusal`: 5 reward sets/10 claims/11 source placements; 5.229527441s, 5.780197785s, 5.332525108s, 5.157659106s, 5.733159112s, 5.235168702s, 5.111554086s, 5.679262224s, 5.13471564s, 5.451962929s, 5.850229369s.
- `quest_catalogue_chests_phase_a_02_use_replay_and_refusal`: 5 reward sets/10 claims/11 source placements; 5.599474055s, 5.335603901s, 5.814005859s, 5.305262555s, 5.644183853s, 5.87351754s, 5.231065849s, 5.660814371s, 5.172239565s, 5.139687402s, 5.158577349s.
- `quest_catalogue_chests_phase_a_03_use_replay_and_refusal`: 5 reward sets/7 claims/7 source placements; 6.342526994s, 6.026677998s, 6.369084569s, 5.541228271s, 6.075695099s, 6.349147641s, 6.365772291s.
- `quest_catalogue_chests_phase_a_04_use_replay_and_refusal`: 5 reward sets/13 claims/13 source placements; 6.114598345s, 5.966200069s, 6.944473597s, 7.035204391s, 5.832414246s, 6.129997217s, 5.902479287s, 6.398477599s, 7.119916497s, 6.063694363s, 5.556689097s, 6.536066313s, 6.319267002s.
- `quest_catalogue_chests_phase_a_05_use_replay_and_refusal`: 5 reward sets/14 claims/14 source placements; 6.202522984s, 6.702224553s, 6.186385516s, 5.377341324s, 6.312224194s, 6.785404258s, 7.082307069s, 6.197611601s, 5.503727053s, 5.557267111s, 6.540011492s, 7.205554247s, 6.339128669s, 5.486535136s.
- `quest_catalogue_chests_phase_a_06_use_replay_and_refusal`: 5 reward sets/7 claims/7 source placements; 6.701999362s, 6.124515976s, 6.546595103s, 5.782856995s, 6.571287132s, 6.250700842s, 6.419897464s.
- `quest_catalogue_chests_phase_a_07_use_replay_and_refusal`: 5 reward sets/9 claims/9 source placements; 5.698148299s, 6.012994372s, 6.720864567s, 5.931585731s, 6.073326969s, 7.317808125s, 6.37141911s, 5.774253018s, 5.929695349s.
- `quest_catalogue_chests_phase_a_08_use_replay_and_refusal`: 5 reward sets/8 claims/8 source placements; 6.438191314s, 6.188842547s, 5.924731629s, 6.388034372s, 6.479861735s, 6.709017055s, 6.032892645s, 7.233485349s.
- `quest_catalogue_chests_phase_a_09_use_replay_and_refusal`: 5 reward sets/7 claims/7 source placements; 5.465519588s, 5.948269949s, 5.9791066s, 7.149989024s, 6.102147913s, 5.527568528s, 5.687258318s.
- `quest_catalogue_chests_phase_a_10_use_replay_and_refusal`: 5 reward sets/11 claims/11 source placements; 5.633759518s, 5.787892526s, 5.587284049s, 5.763457718s, 6.947656126s, 5.91855409s, 5.292973688s, 7.238010509s, 5.53366988s, 6.188668073s, 5.555852997s.
- `quest_catalogue_chests_phase_a_11_use_replay_and_refusal`: 4 reward sets/4 claims/4 source placements; 7.688441264s, 6.896141822s, 7.816478106s, 8.075536734s.
- `quest_catalogue_chests_phase_a_12_use_replay_and_refusal`: 2 reward sets/4 claims/4 source placements; 3.860197354s, 3.83457468s, 3.86239078s, 3.871521756s.
