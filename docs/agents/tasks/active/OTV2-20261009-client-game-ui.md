# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

AUTHORING on draft #1942. Published anchor:
34e9206fd98cd1ba784340c518330b9c6fadc3fa. Not frozen or merged.
Root is sole publisher. Workers author disjoint client files only.
Owner scope: complete native client/UI/settings plus necessary Platform/engine
work. Preserve missing server values as unknown; prepare future controls without
claiming their absent mechanics work. No production deployment, database reset,
password rotation, merge or third-party proprietary asset redistribution.

## Current authoring increment

Nine independent action rows (three bottom/left/right), twelve local slots each.
Extra rows default hidden/unbound. Global 108-shortcut validation, per-row locks,
assignment clearing, context/focus/held-key quarantine and session reset retain
legacy preferences. Three directly observed row-count settings now have actual
consumers; old future count choices migrate preserving shortcuts/locks.
The full and quick settings use one typed row/shortcut editor. Complete saved
settings fit the unchanged 32KiB bound, including maximum escaped text.

Keyboard commands, row clicks and assignment menus must obey the same modal
boundary. Independent engineering review found simultaneous held action keys
could suppress a second shortcut, modal row clicks could bypass keyboard
suppression, and tiny windows could hide settings/footer recovery. Repairs and
actual egui regression tests passed on the final source. Map rendering/pointer picking use
one count/scale transform; a too-small scene still renders usable settings UI.

Quest/Cyclopedia tracker navigation, Wheel local presets and separate perk/vessel
views, achievement filters/grade summaries/pagination, Gem/Fragment composition
are prepared. Private original saved NAS captures verify Gem's left
vessels/three revelation tiers and right detail/filter/collection areas, plus
Fragment's four linked grades and right searchable tile region. Wheel topology
is still unverified; no fabricated allocation geometry or server records.
Session inventory/container/corpse handles and item counts are actual projections.
Spell entries/cooldowns/consumable commands need their owning content protocol.

Baseline contains 28 shortcuts, 22 specialised panel compositions, 20 Cyclopedia
pages, six reference dialogs and 27 settings sections/276 catalog entries. Their
presence is not full behavior or visual parity. Future selections are validated
intent, with consumer/provenance status retained; names-only options are not
promoted to directly observed UI. Original RCC/QM/binary content stays private.

## Runtime and native qualification

Windows checkout:
C:/Users/barte/Downloads/oteryn-client-main-20261007.
Native target: x86_64-pc-windows-msvc. Before source replacement verify all
baseline/staged SHA-256 values, back up exact files and refresh LastWriteTimeUtc
because archive timezone interpretation previously let Cargo reuse stale builds.
Capture only the exact own executable's HWND via PrintWindow. Global click
scripts require foreground HWND checks; never click unrelated owner windows.

Baseline 0c3: native 111 library+4 application+5 launcher tests, strict all-targets
Clippy and release build pass; normal browser PKCE/consent, authoritative
character selection and world admission visibly verified. Current pre-repair
Linux 121 library+5 launcher tests and Windows GNU all-target strict Clippy pass.
Final successor: native 122 library+19 application+5 launcher tests (146 total),
MSVC all-targets Clippy and release PASS. Linux 123 library+5 launcher tests,
Linux strict Clippy, GNU Windows strict all-target Clippy and format PASS.
Normal PKCE/selection entered the local world; native settings/footer verified.
Caption/shortcut rows and per-category scrolling were corrected after actual
screenshots and qualified again. Extra rows default hidden/unbound; final
all-nine-row world HUD was visually verified after Apply saved [3,3,3].
Final bounded two-file repair uses actual scroll viewport width and current
body height for columns/separators. Ten actual-source headless egui tests pass,
including PL/EN default/tall/high-scale overflow and tiny recovery checks.
Native final tests/Clippy/release pass after rustfmt on the same staged bytes;
final native responsive screen visually verified: no redundant outer scrollbar,
visible Defaults/account actions, notices and Apply/Cancel footer.

Local Docker portal/directory return HTTP 200 and one ready world. Runtime is
Ubuntu-24.04 at /home/mole/oteryn-native-20261008 and preserved local deployment
/srv/oteryn-login-local-desktop. ProgramData repair was child-process-only.
Renewed 13 qualification certificates for 30 days preserving existing keys,
issuer/subject/SAN/EKU; validated chains and backed up 23 matching public copies.
Reloaded local PostgreSQL/nginx and gateway only. No production trust change.
Character Oteryn Tester was created through fenced Game operator bootstrap;
normal local registration created its DPAPI-protected private test credential.
No raw character injection, database reset or credential readout.

## Secure recurring-login candidate

Owner wants repeated game login in the client without weakening security.
Existing accepted GA-006/ADR-0009/OTCLIENT_GAME_AUTH_CONTRACT requires external
system-browser PKCE and forbids embedded credential webviews. Preserve it while
an owning replacement is reviewed. No OAuth password grant, client secret,
plaintext password/token preferences or removal of ticket-time revocation.

Preferred candidate: first-use interactive PKCE plus explicit remembered-device
opt-in. It does not imply owner acceptance of the first-use browser exception
to the earlier entirely in-window request. Native password forms require a
separate accepted contract and independent security review.

Platform draft #1484, published 787028f8dbc7bd07ae58f33819c342f9a3ccd85f:
OFF-by-default opaque device-session candidate, hashed DB credential, account/
public-client/security-generation binding, absolute≤30d and inactivity≤7d
expiry, serialized rotation/replay family revocation, strict JSON/rate/cache and
TLS boundaries. HTTP/MFA/catalogue tests 75/737 assertions, PHPStan 10 and Pint
pass; hosted runtime-tests/test/platform-gate pass. Unrelated LCFA checkpoint
failure remains visible. Deployment-DB race qualification is outstanding.

Client remembered-device module is OFF/unwired. Pure durable-block→delete-old→
one request→validate successor→vault save→unblock logic passed eight adversarial
groups. Concrete OS locks/journal/HTTPS/native-ticket adapters and live vault/
cross-process qualification remain absent. Ambiguous rotation forces fresh
interactive auth; do not promise outage preserves a valid stored credential.
No complete recurring-login or entirely native credential-flow claim.

## Owner-authorized separate CI prerequisites

Owner answered "Tak, przygotuj PR-y CI i przegląd": publication of two CI PRs
and required independent deep review in this task is authorized. Root is their
sole publisher/review dispatcher. Merge remains a separate decision.

- #1943: main-targeted audit-pin rotation, frozen 4b535efc116da937bff3fa8a1fe7e064f29c2278.
- #1944: main-targeted active Rust 1.95 workflow migration, frozen eba78bf8868322d25e9a9fb1327cd3257587aae7; depends on #1943.

Protected audit self-edit rejection stays intact; no owner exception or merge
is self-granted. Both exact frozen heads received completed-clean independent
deep reviews. Migration's original P1 stale/implicit helper overrides were
repaired in six actual/nested helpers, with one material-repair re-review on
eba78bf. The existing-owned regression rejects thirteen helper mutations.
The full policy wrapper also required two omitted evidence-job hash rotations.
Gate/audit bytes were unchanged by the helper repair. The durable proposal
packet reconstructs all27 exact source files, full hashes and Git modes.
Hosted migration game-gate and changed-crate Clippy/tests pass at eba78bf;
its protected audit still depends on #1943 integration. Neither draft is merged.

Real Rust1.95 engine/harness typechecks, helper parses and failure canaries pass.
Full engine no-run code generation received SIGKILL; linking and live composed
qualification are not claimed. The inherited spell staging suite remains13/1,
proven on trusted main and outside normal canonical node/seam runtime. Retained
G4 artifact HTTP410 and original S3-A exact-six-path applicability rejection are
still visible; no custody/pin/ownership guard is weakened or owner-exempted.
Product active workflows remain unchanged.

License exception for exact epaint_default_fonts 0.36.2/full font notices and its
negative canary pass. Windows/Inno installer execution remains unqualified.
World pin-check passed two identical builds with unchanged payload f8b11ebc…;
compiler inputs 88be0150… are stable. Original proprietary assets are excluded.

## Reference fidelity correction

Owner rejected generic UI as visually/behaviorally inaccurate. Basic now has
four compact groups; advanced navigation is nested. Reference draft confirmed
all three independent All masters preserve raw row choices, and Mouse/AA/Loot
choices exactly. Typed masters and Boolean legacy migration are active; hidden
hotkeys, locks/chords/assignments remain unchanged. Clear is admitted-session-only.
Own grey grain/bevels, smaller HUD/icons, minimap/paperdoll/item/chat proportions
share viewport constants; original assets stay private. Volume is an actual
horizontal intent slider; audio and other pending consumers remain absent.
Linux core125 and actual-module headless21 tests pass. Final Windows124lib +
25app +5launcher (154 total), strict MSVC Clippy/release pass on staged final
bytes. Basic clip tests cover PL/EN; native PL is visible. OK/Apply/Cancel are distinct. All off/on
visibly changes viewport and preserves raw rows; saved original layout restored.
Own-window captures verify compact HUD/options without original UI assets.
One admission attempt failed; a normal retry admitted the same test character.
Its cause was not diagnosed or fixed by this UI batch.

Remaining fidelity: rotation/stack modifiers, cursor/link-copy/expiry controls,
shortcut draft ordering and other advanced page arrangements. Prepared panels
still require content/mechanics; this draft is not a finished client. Frozen
CI #1943/#1944 stay unchanged, unmerged. Secure remembered sign-in stays unwired.

## Validation commands for this product increment

- `cargo test --locked -p oteryn-client --all-targets`: pass, 123+5 Linux tests.
- `cargo clippy --locked -p oteryn-client --lib --tests -- -D warnings`: pass.
- `cargo clippy --locked -p oteryn-client --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: pass.
- Native Windows same package/all-targets test, Clippy and release: pass,154 tests.
- `cargo fmt --all --check`: pass.
- Scoped source whitespace: pass.
- `python tools/agents/validate_governance.py`: validate in clean publication clone;
  root worktree has older unrelated untracked task records, excluded from commit.
- `python -m unittest discover -s tools/agents/tests`: pass,59 tests; rerun with
  clean publication-clone governance before guarded push.
