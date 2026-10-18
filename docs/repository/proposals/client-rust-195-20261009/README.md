# Rust 1.95 CI migration packet

The native client requires Rust 1.95.0 while the protected-base workflows and
invoked helpers select 1.94.0. The owner authorized the two separate CI PRs and
required independent review: "Tak, przygotuj PR-y CI i przegląd". Root alone
publishes and dispatches review. Merge, protected activation and the audit
self-edit exception remain separate decisions.

- [#1943](https://github.com/Oteryn/Oteryn-Game/pull/1943), frozen at
  `4b535efc116da937bff3fa8a1fe7e064f29c2278`, rotates the exact future PR/MQ gate pins and
  four toolchain-dependent audit assertions. Its exact-head independent review
  is clean, as reported by root.
- [#1944](https://github.com/Oteryn/Oteryn-Game/pull/1944), frozen at
  `eba78bf8868322d25e9a9fb1327cd3257587aae7`, aligns sixteen workflows, four existing
  policy/regression paths and six directly reachable helpers with Rust 1.95.
  It depends on #1943 reaching protected main and then needs its normal audit.
  The earlier P1 helper-selection finding is repaired; the
  [material-repair deep review](https://github.com/Oteryn/Oteryn-Game/pull/1944#issuecomment-6081296506)
  on this head [completed clean](https://github.com/Oteryn/Oteryn-Game/pull/1944#issuecomment-6081391569).

Both PRs target main. These patches are proposal evidence in the product PR;
they do not apply the migration to its active workflow or helper source.
`manifest.json` now records **27 source paths**: the original 21 plus the six
necessary helpers authorized by root after P1 finding `4230087890`. The existing
regression path also traces actual workflow/helper references and rejects stale
or implicit toolchain selection. No permissions, action pins, triggers, producer
or artifact pins, topology, source fences, cleanup or failure guards are weakened.

## Reconstruction and source identity

The patch pair uses zero context (`git apply --unidiff-zero`) against captured
base `1e77ca22b4fcb7869f8f1d169f4e12bc749b295d`. Audit and migration source come from the
exact frozen Git objects above, excluding their task records. Reconstructing
both patches in a temporary directory outside Git reproduces all 27 complete
source files and their Git modes; every SHA-256 matches the manifest. Apply
source changes only through the owning authorized CI lifecycle.

The PR/MQ/audit workflow bytes are unchanged by the six-helper review repair:

- PR gate blob: `4abf4feb48ab406eb09b7fa7c142c8edaa7d82a5`.
- MQ gate blob: `2fe13184cae1982f7d9d605794bb98ecde760061`.
- Audit blob: `39e13fdf3d86086e815710a38e4689983542faad`.
- Post-merge Rust workflow SHA-256:
  `942ddf0ec317633a20113e89a91139dc56aa75630ffce28d2203437b7443039b`.

The earlier full policy wrapper required the two Linux/Windows evidence-job hash
rotations recorded in `qualification_repair`. Those job blocks differ only by
1.94-to-1.95 selection. The later P1 repair explicitly selects 1.95 in installer,
node, S3-A/S3-B, native-room and nested spell helpers, including commands that
previously inherited the unchanged repository 1.94 default.

## Qualification and limits

Both isolated candidates passed governance, 59 governance tests, full repository
policy and staged/base-to-head/commit whitespace checks. Migration passed 32
canonical regressions plus queue/routing checks and 13 helper-selection negative
mutations. The unchanged workflow migration passed official actionlint on all
17 changed workflow files. Official tool archive hashes were verified. Paired
future audit/gate fixtures passed six fail-closed canaries; their gate bytes
remain unchanged by the helper repair.

The final helper evidence binds `eba78bf8868322d25e9a9fb1327cd3257587aae7` and source hashes:
two PowerShell AST parses, original installer native-failure propagation through
its caller, five shell parses, seventeen real preflight refusals and four spell
modes selecting 1.95 and propagating native failure passed. No services were
started. Real Rust 1.95 typechecking passed engine lib/bins, the three composed
and native-room targets, and all synthetic-harness targets. Full engine no-run
code generation received SIGKILL; successful linking, Windows/Inno installation
and live composed qualification are not claimed by these local checks.

The existing spell Python suite remains **13 PASS / 1 FAIL**. Its staging-equality
assertion compares node support for `loot_tables` with an older deployment
stager field set. The same mismatch is proven from the captured base. Node boot
uses its own embedded stager and Server Seam reads the original manifest;
neither depends on deployment `FIELDS` or that static test. The deployment/test
staging gap is retained and no passing result is claimed for this suite.

Two standalone retained qualifications also remain unqualified: G4 requests an
unchanged original artifact that returns HTTP 410; S3-A retains its original
exact-six-path task envelope, which excludes this migration's 26 source paths
plus archive. Neither feeds canonical PR/MQ `game-gate`. Preserve their pins and
ownership guards; address artifact custody or applicability through the owning
route before claiming those qualifications. Neither failure receives an audit
exception. `validation.json` records these boundaries and the completed-clean
review state for both exact heads; it does not claim hosted CI is green.

## Protected integration boundary

`docs/repository/GITHUB_GOVERNANCE.md:39` requires explicit owner authorization,
exact-head validation, a green protected audit and post-merge configuration
readback for control-plane changes. Existing protected-base self-edit rejection
is preserved. Only a separately authorized exact frozen #1943 decision can
address that intentional rejection; it grants no exception to #1944 or another
failure. Local paired fixtures and typechecks are not hosted protected audit or
Merge Queue qualification. No merge or activation is authorized here.

The product world input pin was separately refreshed with identical payload
builds, and the font notices plus exact epaint_default_fonts 0.36.2 license
exception were separately qualified. Windows/Inno installer execution remains
pending and is outside this proposal packet's passing checks.
