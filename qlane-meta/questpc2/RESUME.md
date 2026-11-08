# questpc2b main integration checkpoint

Active branch claude/happy-davinci-ijz3wy-questpc2b; last pushed3df0d0c43.
Parent questpc2 remains frozen at0a6d6e208. Current merge target6f61ac15d
includes QUESTPC4 and physical NPC hand-in lane key33. All44 conflicts resolved,
retaining both lanes. Own migrations1240-1244; incomingmain1250-1258 and0999-1008.
No1200-1205duplicate. No force/rebase/mainpush/PR.

Integration fixes: union selector/storage/callback fields and party context;
retain main server timer persistence; preserve own party transfer and completion;
compose both ladder USE implementations; NPC consumption now uses main's actual
SpellLanePermit while still rejecting Ground. Exact1948 profile composition keeps
LADDER and main nonmaterializable; exact1791 overlap keeps main nonmaterializable.
All other record differences reject. Remove duplicate Python charged-claim
functions which silently shadowed main's five-claim map with Desert-only map.
Retain exact source-charge matching. Equipment source cohort62 remains asserted;
five keys also counted by main are excluded only from the sum. Repo tooling
confirms3820 distinct unionprofiles. Main has3725base profiles; lane adds57basekeys.

First current workspace alltargets pass4m22s. Native45outputs regenerated.
Two exact composition cases PASS2.061s, source-gate5PASS11.354s. Other focused producer checks passed: callback16, key2, exact charge4,
source rewards7 and variants13. All applicable tree/central/pin generators
finished. Postpin workspace check PASSED3m29s; fresh PG cases are pending. Do not claim prior
Oldrak PG78.620s applies to merged Rust. Full runtime suite is CI pending.

Tools/logs /tmp/questpc2b-main-6f-*. Fullregeneration uses repo --resolve
--skip-checks; targeted checks run separately. Mainmerge fingerprint tool updates
outside-six digest only with exact activeMERGE_HEAD; no manual pins. Freshness,
dupmigrations, fmt, latestworkspace and scopedunit/PG evidence required for final
merge publication; owner cadence exception permits explicit pendingtests after
steps1/2/4. Record exact remaining checks in receipt, never silently skip.

Packet unchanged:35entries,29owned,6excluded;3wholequestsQUALIFIED,
4IN_PROGRESS(Soul War,Rotten Blood,Hero of Rathleton,Demon Oak),22notstarted.
Oldrak NPC slice already pushed; full Demon Oak needs engine waves/counters.
Big generic requests belongQENGINE2; move to supported owned work. No agents.

Environment source /workspace/oteryn-env/env.sh opaquely. CARGO_INCREMENTAL=0,
CARGO_BUILD_JOBS=2; retire only inactive this-lane caches if diskfills.
Fetch main and explicitlane ref beforeeverypush; merge movement, neverforce.
Author/committer blakinio <75369544+blakinio@users.noreply.github.com>;
trailers Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
and Claude-Session: https://claude.ai/code/session_01D1SN2sEo2yY1k19f6kjEHf.

Publication checkpoint: all generation/pins PASS, postpin workspace3m29sPASS,
clippy7m18sPASS and fmtPASS. Cadence exception used only for explicitly
pending changed-crate/dispatch PG tests. Run /tmp/questpc2b-run-main-6f-focused.py
after rebuilding current dispatch test binary, without mutating runtime content.
