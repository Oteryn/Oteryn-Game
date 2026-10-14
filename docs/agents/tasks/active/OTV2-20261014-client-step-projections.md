# OTV2-20261014 — refresh received HUD projections on completed steps

```yaml
task_id: OTV2-20261014-client-step-projections
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Parent OTV2-20261009-client-game-ui; sole root writer, draft AUTHORING. Latest
owner scope excludes point6/HUD-graphics redesign. Owned runtime is play.rs;
this repairs existing admission consumers, not a new gameplay activation.
Five read-only lanes mapped existing audio/actions/chat/NPC/item producers and
live gates; they performed no writes, builds, paid review or operator actions.

## Defect and evidence

Session applies pushed domain stores while awaiting a step result. run_session
refreshed GameState after action/chat/use/combat/idle, but not after a successful
step exchange. Continuous movement could leave inventory/vitals/chat and open
corpse entries stale; stale-menu cleanup reads that same GameState.

Refresh the admitted snapshot before publishing step completion. Keep existing
receipt validation, single-step reservation, pacing and pushed-event ordering.
No protocol/store/authority/capability changes or optimistic item mutations.

The actual play_channel/run_session regression holds the exchange with oneshot,
then changes scripted projections and keeps idle service permanently pending.
It checks the initial state before receipt and updated inventory/vitals/chat,
removed entries and capability4's closed Some(OpenContainer::default()) after
completion. Copy mutation cannot resurrect an entry. Reverting the four-line
production repair made this regression fail on stale GameState; restoring it
passed. The first fixture compile failed for missing main_backpack fields;
those initializers were corrected without suppressing lints or tests.

Read-only peer audit found no blocking correctness issue and corrected the
closed-store fixture to Session's actual selected-domain representation. This
is a client projection-routing proof, not native/live wire gameplay.

## Source and qualification

Normal merge3295351abfb1a9939e702b6257bf68e1aa9d2d13 preserves earlier client
work and includes accepted main15c6cab775b33c6ae6058975b56d5bd211a8a59a.
Readiness refresh32ee396e additionally incorporates accepted main71c014d4
(OPS-SAVE-2/MECHANICS-PLAN-1), with no client/Session/protocol/Cargo/asset changes.
Remote source00198640 has19/19 successful checks; no status transfers to a new
head. Post-merge Rust1.95 locked all-target Linux client229/203/5+Session82,
strict all-target Clippy, fmt, governance60 and diff checks pass. Clone-on-Copy
fixture lint findings were repaired and the full selected checks rerun. Windows
qualification below uses a new client-owned worktree/audit directory and prior
owned build cache; no existing binaries/checkouts/processes are removed.

PC access is confirmed. A bounded read reverified source001986403edb21731bad44d83c7f56b1a59c7004
and executable f1ff20379929f35bf73bb9e9646897deae5e66cfd4855908282fb952bbfda8b0
under its target/x86_64-pc-windows-msvc/release directory. Former loginPID20744
is absent; it was not terminated by root. The initial lookup incorrectly used
target/release, then the target-qualified path resolved successfully.

### Exact published-source native proof

Source6f217f1632eeead19a0f5c799c6843c440e0ecdf: Rust1.95 MSVC client228/209/5
and Session82, strict all-target client/Session Clippy, fmt and release pass.
Named projection regression, all three item-menu regressions and Windows-only
toolbar rollback execute and pass. Release0.1.0+dev.ci.g6f217f1632ee;
executable b87ef4e040fdbe89f5a14d788db9baa2bc5954b0436ba042cdca14f641a7b500.
DX12 at125%DPI:20pages/78 independently rehashed captures/0invalid; manifest
fa864d07e2e8f8f06e659c265554ce8f82aa73221e8d87c05d5e90952b3c1426.
Build C:\Users\barte\Downloads\oteryn-client-6f217f16-20261014-native-attempt2;
audit C:\Users\barte\Downloads\oteryn-settings-audit-6f217f16-20261014-attempt2.

Attempt1/PID32272 refused before compilation: --no-checkout left an empty index,
so sparse configuration alone did not materialize tracked source. Its directory
is retained. Attempt2/PID19356 initializes only the fresh owned checkout with
read-tree -mu, then passes exact clean-source fences; nothing is reset/deleted
in an existing checkout. Qualification marker reports143.184seconds. Its native
launcher child may keep the wrapper pipe open; no outer-exit0 claim is made.
An independent bounded read verifies new loginPID11608/window461552, exact
executable/hash, responsiveness and source-correct U+2014 title. Existing public
launcher/CA and explicit asset root are reused without automatic sign-in, trust,
personal preferences, installation, starter, account or server changes.

Guarded6f publication/readback passes; recovery
/workspace/oteryn-client-publication-6f217f16.bundle SHA256
37e63e1d1804a243d4067900d505a5d55a8981805d5817978a4160ca98b56698.
Bounded same-6f CI read:13success/4running/0failed; final game-gate not yet
available. Do not inherit parent's19/19. This evidence-only successor changes
no runtime bytes and waits locally to avoid cancelling useful running CI.
Same-head final review/CI/MQ/main-channel artifact and full live E2E remain open.
Native captures qualify compiled page reachability/integrity, not fresh manual
comparison of every pixel/control, OS picker/clipboard/trackpad, original1:1
or native gameplay. HUD/graphics redesign remains explicitly outside this task.

## Reuse map and blockers (2026-10-14)

- NPC capability3/talk7/domains7–8 and new trade8 producers exist; bundle-only
  offer and typed Session/UI consumers are required, after first gameplay proof.
- Item cap15 direct-backpack ingress exists, but15 is not offered. Do not enable
  unrelated assignment, use-with, by-definition, equipment or loot behavior.
- Received corpse/container handles could supply a future Object assignment
  slice; it is not activated before B2. Spell catalogue remains absent.
- Official manual documents Shift+Up/Down recall, currently missing. Future
  consumer must preserve ordered text/key events, Unicode caret, scratch draft,
  real mailbox acceptance and native configured-chord precedence. Private peer
  tabs cannot be inferred from own echoes containing speaker/text alone.
- Audio has neither mature runtime backend nor a rights-cleared recording bank;
  recovered844-record metadata is reference-only. No fake playback is enabled.
- Original ZIP/minimap, verified Help URLs and17 actual screenshot triggers
  remain unresolved. Existing Oteryn ZIP and renderer screenshots are retained.

Fresh #1622 read:293comments; root6149042654 remains operationally unanswered.
Architect6146789689 still requires dedicated QA/character/item, confirmed
deployed bundle node, authorized temporary ON/OFF/restart/rollback and harness,
3/3 same-build passes within5 recorded attempts, and a retirement decision.
Acceptance-only2084/2085 remain PROPOSED, NOT EFFECTIVE; no child activation or
operator authority follows from them. The owner's online PC is not this setup.
No account/deploy/switch/personal clipboard/profile/trust action is performed.
