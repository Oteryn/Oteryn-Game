# PR 3037: S17b PostgreSQL comparison

Executed on 2026-11-08 with the configured real PostgreSQL harness, not the
missing-database early return. The baseline was a clean detached worktree at
main `83acbdac3720d7fe2df6c44d82623b7307806402`.

Filter: `s17b_native_observer_map_creatures_effects_and_public_speech_dispatch`.
Initial command: `cargo test --locked -p oteryn-gameplay-dispatch --lib <filter> -- --nocapture --test-threads=1`.
Repeats invoked that build's libtest binary directly with the same arguments.

| Candidate | Result | Libtest duration |
| --- | --- | --- |
| qready1 `be06eca24` | observer join failed | 67.76 s |
| qready1 `be06eca24`, repeat | observer join failed | 164.94 s |
| clean main `83acbdac3` | passed | 94.28 s |
| clean main `83acbdac3`, repeat | observer join failed | 115.03 s |
| clean main `83acbdac3`, third run | observer join failed | 95.71 s |

The passing main case printed `CLCAST S17 case 2 real-dispatch PostgreSQL: 92.185s`.
Both branch failures and the first main failure logged
`next_command_id=5 server_sequence=4`; the third main run logged
`next_command_id=5 server_sequence=5`. All failures reported
`session is unusable after an earlier failed command exchange`.
The main worktree remained clean after all three runs.

This establishes an intermittent failure of S17b on main as well as the branch.
These local failures happen before the CI failure's Stop command; they do not
establish the root cause of the exact CI Stop rejection. No cast assertion or
timeout was weakened. The cast handler, social cast implementation, and S17b
test are identical between the compared revisions.
