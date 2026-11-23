# Black-box result integrity: BB-02 and SB-03

Authoring base: `245ba7838e58318a2d851857b6c2b2d7c757aa09`, PR #3430.
This packet records two implemented caller-oracle repairs. It does not claim
that the complete Game, Platform, PostgreSQL or client suite has passed.
`validation.json` binds the observed commands to the four edited source files.

## BB-02: every selected indexed row must produce its own verdict

**PROVEN:** the original aggregate accepted an unrelated `I01.full PASS` for
the shard selecting index 206 (`S09.client`). The existing standalone regression
driver ran 11 methods on the unchanged base: eight failures and three errors
(the new expected-identity API did not exist).

The implementation now puts `{index, scenario}` identities in the shard plan,
validates them against the actual dispatcher, and requires exactly one terminal
verdict for every selected identity. The closed dispatcher convention covers
all 302 active indices, including the four older direct reporters. Unknown
dispatcher forms or selectors outside that convention fail planning rather
than disappearing. `indexed_row_identities.json` is an identity inventory, not
semantic approval of 302 gameplay assertions.

A different row, a bounded slice, a duplicate, a nonterminal status or a capture
cannot replace the expected result. `--missing-informational` still concerns
explicit `MISSING-*` verdicts; it cannot waive an absent result. Existing
FAIL/baseline semantics are unchanged. A recorded terminal verdict is an
execution fact, not necessarily a PASS.

**Observed checks:** `test_matrix.py`: 33/33 PASS. The actual summary CLI returns
1 for capture-only `S09.client`, including with `--missing-informational`.
`prebuilt.plan` accepts the reference, PR and full plans: respectively 14, 14
and 302 indexed identities; their shard counts remain 1, 7 and 37.

**Historical log compatibility:** GitHub run `45075887150`, job `137384086699`,
head `bd9a8d384164b4b78da975b761de1a87de81b053`, is a real failed reference run.
Its actual evidence lines were replayed through the new extractor/summary.
All 14 required reference identities occur exactly once; all 14 are FAIL.
The node result remains failed. The current and historical dispatcher identity
maps are equal (302 identities), checked from the historical dispatcher and
four direct emitters. This is compatibility evidence, not a new runtime run or
a green qualification. See `historical_reference_replay.json`.

The summary remains one part of the existing gate. The workflows additionally
require the shard jobs to succeed; an empty completeness-problem list does not
override a failed shard. No workflow or known-failure baseline was relaxed.

## SB-03: zero selected Rust tests cannot qualify S3B

**PROVEN:** the actual `wp5_s3b/run.sh` execution/cleanup tail previously emitted
PASS after a zero-test child returned zero. The repair retains the child exit
status through `pipefail`, captures that command's output, and requires one
successful libtest summary, the exact selected case header, and its existing
terminal body evidence. It adds no production Rust marker.

| Mode | Selected case | Existing final body evidence |
| --- | --- | --- |
| `s3b` | `real_owners_compose_fresh_admission_and_fence_replacement` | `S3B_EVIDENCE composition=complete` |
| `seam` | `qualification::server_seam_real_owners_over_tcp_tls` | `SEAM_EVIDENCE shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED` |
| `spell-seam` | `qualification::server_seam_real_owners_over_tcp_tls` | `SEAM_EVIDENCE spell_seam shutdown=drained scope=real_owner_tcp_tls gameplay_complete=false` |

The markers are produced after the real final assertions in
`apps/game-server/tests/wp5_s3b_composition.rs::composition_flow` and the two
branches of `apps/game-server/src/qualification.rs::seam_flow`.

Regression tests use the actual mode selection, execution tail and cleanup,
substituting only the child process. All three modes reject zero tests, another
case, missing/wrong body evidence, no summary, duplicate execution, ignored-only
output, and an expected case name mentioned only inside body text. A nonzero
child remains failed even with apparently complete stdout. These 30 mode/case
probes are three unittest methods in the existing CI-selected `test_prebuilt.py`.
They are not a live Platform simulation. The final module result is 9/9 PASS.

Before the repair the new caller tests had 21 failing negative subcases. A later
name-only-in-body negative reproduced three failures in the initial unanchored
header guard; anchoring the case header made those cases fail closed too.
The real historical node-boot log also confirms the libtest success summary
and exact-case header syntax used by this guard. A successful live S3B run on
the final candidate remains required; it was not executed in this packet.

## Test value and a concrete next black-box journey

**PROVEN:** `client_row_s09.rs::run_packet` returns before performing logout or
relogin when `E2E_ALL_CAPTURE_DIR` is absent. When capture is enabled its success
is `CAPTURED`; `qualification.rs::e2e_result_line` correctly places that event
under `e2e_capture`. `indexed_verdict.rs` does not fabricate success for the
empty path. The new aggregate exposes the omission. It deliberately does not
turn capture creation into a gameplay or replay PASS.

The strongest existing public entry point is
`tests/security/auth/tests/native_ticket_and_gateway_fail_closed.rs::admitted_journey`:
real OAuth, native ticket, real Gateway and TLS Game admission, then independent
Platform/Game receipt and session checks. It currently drops that session; it
does not by itself prove a move, native logout or a second public admission.

**RECOMMENDATION:** extend that existing journey with one independently expected
fixture movement, a refused move with unchanged state, native logout, a fresh
Gateway-issued admission and retained position/inventory checks. Preserve
account/Character/World/Channel/session binding and exact per-admission receipt
deltas. A harness-signed reconnect is not a substitute for the second public
Gateway admission. Keep deterministic PostgreSQL transaction, generation,
replay and rollback tests: the single public happy path does not replace their
negative-state coverage. This is a concrete proposal; no such journey was
implemented or claimed executed in this packet.

## Remaining limits

- Indexed identity completeness is implemented. Group, basic, spells and
  game-platform execution completeness still need their own declarations;
  dynamic creature selection cannot be proven by an arbitrary row count.
- Capture-only producers still need an explicit capture-to-replay contract
  binding artifacts, producer revision and the actual consumer test. Full
  qualification now reveals their missing terminal results rather than hiding
  them behind another row's PASS.
- Local node-boot discovery after BB-02 ran 106 methods: 99 PASS, seven ERROR
  because this container denies AF_UNIX socket creation in unchanged provision
  tests. This is an environment limitation, not PASS. The final SB-03 methods
  were then tested separately; the final combined suite needs the authorized
  environment that permits those sockets.
- No Rust compilation or live PostgreSQL/Game/Platform/Windows/GPU execution is
  covered by these Python/shell checks. No source test was skipped or loosened.
