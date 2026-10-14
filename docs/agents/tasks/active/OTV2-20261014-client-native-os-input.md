# OTV2-20261014 — native OS input on the existing owner desktop

```yaml
task_id: OTV2-20261014-client-native-os-input
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Task: OTV2-20261009-client-game-ui; continuation of six-local-steps.
Repository Oteryn/Oteryn-Game, branch feat/client-completion-20261009,
draft PR1942, sole root writer, AUTHORING. Tested published source:
ab1112366ddbc4f0441b7900db4e5996f5dbd101. No E2E, login/account action,
HUD redesign, original configuration read, deployment or paid review.

## Owner decision and exact native fixture

Owner explicitly said continue after being told the existing desktop tests may
replace clipboard contents with test text, temporarily change foreground focus
and add test paths to file-dialog history. This supersedes the earlier demand
for a separate Windows account/VM for these bounded actions. It does NOT grant
access to previous clipboard contents, personal preferences or account data.
No Windows account, VM, OS language or system focus-policy configuration is created.

Existing Windows desktop is accessible, not secure/Winlogon. Test process39140,
owned HWND8260316, release0.1.0+dev.ci.gab1112366ddb. Executable SHA256:
68cd09f1d115e92ff5586877f987913796cf58577ad301c3e29c1dbc1821d6ff.
Private fixture: C:/Users/barte/Downloads/oteryn-os-input-ab111236-20261014.
Only this child uses fixture/local-appdata as LOCALAPPDATA. Native login endpoints
are unset in its process; no automatic sign-in. Previous builds/processes and the
owner's installation/profile remain untouched. Captures use PrintWindow on this
owned client only, never the desktop or a picker's personal file listing.
All captures are1125x775 at120DPI. Every connector call uses fresh GitHub metadata
and the unchanged pinned per-call policy gate. Expired CLI GitHub401 is handled
with the independently authorized GitHub connector, not credential discovery.

Final generated report native-os-input-report.json SHA256:
7b259ee3b01020a25b9381feaaf95f0c519627e444f8ad80cca862d3e5f86ce5.
Seven key captures independently rehashed/decoded,0invalid;25 total PNGs retain
all partial/failed captures, not25 passing test cases. Owned test process39140
closes normally with exit0; controller9348 exits0. All fixtures and unrelated
processes remain. No reconstruction/restoration of the old clipboard is attempted.

## Actual consumer proof

- Windows clipboard SetText supplies controlled Unicode only. Ordinary client
  Options search receives Ctrl+V/Ctrl+A/Ctrl+C and retains Polish/CJK/emoji bytes.
  Ctrl+X clears the visible search; Ctrl+V restores it, then Copy matches exactly.
  No previous clipboard value is read, backed up or printed. This proves the
  native Options TextEdit/clipboard adapter, NOT admitted game chat.
- OS SendInput typing/copy preserves BMP Polish text. Supplementary injected
  input is a separately retained failure below, not a passing Unicode claim.
- Native rfd export and import windows are actually opened. Filename entry is
  identified by owned HWND, exact expected dialog title/class and Win32 focused
  Edit control1001. Paths are absolute owned fixtures; no existing file overwritten.
  Standard Windows COM controls/UI automation, not an alternate ZIP workflow.
- Native export produces31,460bytes, exactly one Stored oteryn-options.json
  entry checked independently with .NET ZIP. SHA256:
  0f5ddd1a86c772bd56379794194f5b8b27bb59326c59210dd4ea52334c0d6b80.
  Export picker Cancel leaves active settings/source unchanged.
- Actual advanced-options checkbox changes the visible navigation; Apply writes
  only the private profile. Advanced=true active SHA256:
  c010b00fba3ea9a4e639d704ee0b08556c672f52c5a0af61cba05e5c78b6a88a.
- Native import picker Cancel leaves it unchanged. Importing the native baseline
  ZIP switches the visible draft to Basic without changing active bytes. Escape
  closes the actual Options window; F10 reopens the retained active Advanced UI.
  A subsequent real native import + Apply switches to Basic and persists
  advanced=false, active SHA256:
  c724a769f8314a02bf8a5a55083e544b083e85d33a4075c015c189d215af290e.
  Source ZIP remains identical. Successful combined native controller27664 exits0
  after6.34s. This verifies a consumer-visible transaction, not merely serialization.
- OS SendInput wheel scrolls Basic from gameplay to lower interface/graphics/
  audio groups, while the footer remains reachable. This is WM_MOUSEWHEEL path
  proof, NOT physical precision-trackpad/pixel-wheel equivalence.

Key captures and SHA256, private fixture above:

| Capture | Evidence | SHA256 |
| --- | --- | --- |
| 07-native-export-success.png | Real native export | 5486f72089ed67dac30d8dde733c6181ac0f5282472a442716ee83d68a795c5a |
| 12-import-staged-basic.png | Draft-only import | d1bc3e9574959cebaf9301edc6bf10f2c12fdd95ba81778fac3ac02b31e06614 |
| 15-outer-escape-cancel.png | Options actually closed | 219aec5fb83a6d4fa0474f22e60c053a3be27edfcbffbf706022a128e2d26376 |
| 16-f10-restores-active-advanced.png | Active UI restored | 9d668500f1e526a1ee4e1512bb8c14da83dec44bfd5b44dfda4a7e86ed945e4d |
| 19-import-apply-basic.png | Apply consumer | 078c34df5c4adb08f4b12e68c6a3c4ddb7e66a26862403267158fe607d924502 |
| 20-unicode-cut-clears-search.png | Native Cut | 3ed942521538a638ac71de60df6c0b3e52bcade89ebc002126ff1cd5c0ead58a |
| 21-unicode-cut-paste-recovery.png | Native Paste recovery | 216c04a39319c670bb91918e9f0eba1206a28c670acbc57e792a9e423484333c |
| 24-basic-after-wheel.png | Native wheel consumer | 8571e070ec2ba4f655d2e147062b6c73bc4ceeeffcbc2a6629329a87b78baf31 |

## Supplementary input finding and remaining acceptance

SendInput KEYEVENTF_UNICODE of Zażółć + U+1F642 + -qa produces nine UTF-16 units
in the client: BMP Polish text survives, the two-unit emoji disappears. Actual
client search/copy and capture02b-native-unicode-input-failure.png establish it
(SHA256 f2b1763b401e080cd3c603cddc82a6f9e3fa4ba6a10091fdd82f6005f08db2db).
An independent blank standard WinForms/Win32 TextBox receives the EXACT same
driver and returns all11units/emoji/exact match (controller41188 exits0).
Therefore this is not waived as a producer-only typo or proved fixed by Paste.
Scope is this injected VK_PACKET path; physical keyboard, Windows emoji panel
and IME behavior must not be inferred from it.

Upstream winit0.30.13 is already the latest stable release, verified from the
upstream release API. Its Windows KeyEventBuilder groups UTF-16 WM_CHAR units
within one key event and discards an invalid standalone surrogate; the driver
emits separate key events for the surrogate units. No released newer patch or
matching VK_PACKET fix is established. Historical PR1165/1166 and macOS4596 are
NOT evidence of a fix for this current path. Route the exact finding to the
input/architecture owner before a beta migration, platform-hook change or fork.
No fake emoji insertion, silent field rewrite, new unsafe adapter or vendor fork.

Installed language IDs0415/0409/0409 provide no observed CJK IME profile. No OS
language package is installed. CJK composition, physical trackpad and admitted
game-chat acceptance remain open. Original-version matched archive/minimap
fixtures, legally usable audio bank/producers, B2/B3 and full1:1 remain separate.

## Retained harness failures and safety

Preserve every failed/partial run and capture: PowerShell ushort alias, readonly
PID variable, incorrect UIA Pane/ValuePattern selection, foreground refusal,
same-point/no-new-motion clicks, stale navigation after changing Advanced and
Options reopening, Windows ENAMETOOLONG and UIntPtr cast are harness failures,
not completed client functions or discarded passing criteria. Native dialog
Cancel is checked separately from Options Escape; wrong-page captures are NOT
consumer proof. Source filename Edit and every input target are checked before
input; foreground refusal sends no keys. Temporary topmost activation of the
owned client is restored; no system focus configuration is changed.

Remote write_file is policy-DENIED and is never called; no alternate file-write
route is used. Larger own drivers execute entirely in memory, compressed solely
for Windows command-length limits and SHA256-checked before execution. Only
fixture/data outputs are written by the authorized test process. No Remote
Desktop repository publication or CI-log fallback. No unchanged rebuild/test
rerun and no status-only candidate. Runtime source remains ab111236.
