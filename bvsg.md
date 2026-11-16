Single agent work in this cloud session. Use the repository checkout you are in; build with CARGO_BUILD_JOBS=4. Load the repo skills oteryn-dev, oteryn-coding and oteryn-verify. Do not open a PR, do not label, do not merge. Final report at most 20 lines.

Repository Oteryn/Oteryn-Game. Start from `origin/claude/happy-davinci-ijz3wy-bvint` at exact commit `b97bf3aa2c2ee9bfef6a047817f898c087b230e2` (git fetch origin claude/happy-davinci-ijz3wy-bvint; git checkout -b claude/happy-davinci-ijz3wy-bvsg b97bf3aa2c2ee9bfef6a047817f898c087b230e2). This is the Bevy client package (apps/client/src/bevy_shell/**, feature `bevy-shell`); the Win32 UI is being removed. Parity reference is Tibia Global (current official client); OTClient-only extras are out of scope.

A Codex review found the findings below. Fix EVERY finding in this list. For each one add a regression test that goes through the REAL caller (the Bevy input route / observer / system that a player triggers, not a direct call into the helper the finding names) and fails before the fix and passes after; name it in the report. If a finding is wrong (already correct or unreachable), change nothing for it and give the traced reason. Touch only the files the findings need (plus their tests and the six-language client text catalogs if you add UI text). No stubs, no todo!/unimplemented!, no #[ignore], no empty match arms, no placeholder data; no test skipped, ignored, loosened or deleted. Do not touch .github/**, tools/repository/**, docs/agents/**.

Checks (iterate until green):
1. cargo fmt --all --check
2. cargo clippy --locked -p oteryn-client --features bevy-shell --all-targets -- -D warnings
3. cargo clippy --locked -p oteryn-client --all-targets -- -D warnings
4. cargo test --locked -p oteryn-client --features bevy-shell --lib -- bevy_shell::
5. cargo test --locked -p oteryn-client --lib
6. the client text catalog check if you touched UI text (find it with `rg -l "six-language" apps/client crates/client-ui | head`).

Commit on `claude/happy-davinci-ijz3wy-bvsg` with author `blakinio <75369544+blakinio@users.noreply.github.com>`, then `git push origin claude/happy-davinci-ijz3wy-bvsg`. No PR, no force, no other branch. Never end a turn with only a status: end only after a push or a verbatim hard blocker (disk full/OOM are not blockers: clean your own target, CARGO_BUILD_JOBS=2).
Final reply: branch, pushed SHA, base SHA, per finding fixed (file:line, test name) or declined (reason), per check its result line, push output.

FINDINGS:

1. **P1 — Unreachable object selection in multiaction editor** — `apps/client/src/bevy_shell/game_ui/actions/editor.rs:500`. Open an empty action slot, press Multiaction, then any object action: Equip, Use item, Use position, Use on self, Use on target, or Use at cursor. Each requires `values[1]`, but that field is hidden and item drops are rejected while the editor is active. These buttons therefore refuse the input; players cannot select objects for the draft. The test masks this by directly setting the hidden field at `editor/tests.rs:463`. Add a visible item picker or draft drop target, and test that input path.

2. **P2 — Nonhermetic maintenance test** — `apps/client/src/bevy_shell/maintenance_tests.rs:89`. When built with a valid `OTERYN_CLIENT_TUF_ROOT_STABLE`, the test’s Verify click reaches `activate → start → maintain → UpdateClient::check`, contacting `account.oteryn.com` and potentially downloading the release package. Its “no signing root” assumption is unenforced; transport requests can take 300 seconds while the test fails after 30 seconds, leaving its detached worker running. Inject a deterministic operation result while retaining the real button/system path.
