# R25 spell mechanics and presentation completion

This batch fixes source extraction and prepares the current test-content input.
It does not claim complete gameplay execution, client rendering or audio playback.

## Player spells

All 246 definitions and their gameplay/dependency/identity fields are retained.
67 sound presentations are corrected: 61 missing engine-default cast cues and six
source-precedence cases. The pinned engines default to SPELL_OR_RUNE (ID 10) and
silent impact; explicit or unregistered Canary silence wins over Crystal audio.
The source-selection choices remain unchanged, with the new catalog digest.

The 67 closed native profiles are synchronized with that catalog: 13 headers
change presentation only. The exact native equality guard remains unchanged.
Current tool samples use the corrected input; historical r21-r24 files stay intact
and must be interpreted with their original Git version and qualification receipts.

The per-definition supplement retains 96 additional source cue references across
69 definitions. Conditional/direct/delayed calls remain reference evidence until
their corresponding owner outcomes and presentation producers are integrated.
The supplement is not silently attached as an executable native manifest section.

## Monster spells

The source sidecar retains all 20,640 slots from 5,188 donor profiles. It records
actual helper setters and registered spell branches, visual/projectile IDs,
sound defaults/random alternatives and condition parameters. In particular,
the helper's impact-default setter overwrites cast sound; the output preserves
this behavior rather than replacing it with the presumed intent.

Eleven unresolved/invalid source visual values have explicit reasons. These are
source donor IDs, not native asset bindings or activation authority. The native
population remains the r24 1,508 profiles with 1,253 approximate melee selections.

## Verification and research

Two new tests prove actual healing/paralysis owner-batch atomicity, refusal and
replay. Existing runtime already implements the dispel; these are not new Exura
gameplay implementations or an end-to-end cast claim.

The cue builder now checks the owning typed enum and real Lua registration; IDs
from foreign enums or comments are refused. Existing 213 cue records retain
their values: 51 effects, 14 projectiles and 148 sound aliases.

Current wiki facts and disagreements are in research/source-access-and-effect-findings.json.
Eight public pages were read through existing Chrome/CDP after ordinary HTTP
blocks. No owner computer files or settings were read or modified. Canary/Crystal
code was read from pinned Git captures. Death Echo, Grenade and Spiritual Outburst
differences remain explicit; no uncertain formula is silently overwritten.

## Prepare and qualify

```sh
python docs/reference/spells/r25-candidate/materialize_manifest.py --out /tmp/oteryn-r25-inputs
bash tools/qualification/spells/run.sh map /tmp/oteryn-r25-inputs/manifest.json
```

The exact input and outer-artifact validation receipt is recorded separately.
The regenerated scenario matrix describes 101 fixture-dependent selected paths,
140 integration-dependent selected definitions and five inactive aliases; these
are not successful gameplay counts.

## Remaining integration

Visual/audio delivery still lacks accepted wire allocation, node egress and a
Session/client consumer, renderer/audio playback and qualified asset closure.
The checked 15.30 asset manifest contains 6,249 files and no audio files; this is
not an assertion about files on the owner's computer. Negative-floor presentation
projection and Words/Value producers also remain separate requirements.

Ten effect dispatchers and four House owners, live Premium composition and full
monster special/ranged/defensive behavior remain integration work. Candidate codec
and source facts are not permission to allocate protocol IDs or to bypass owners.
No long-lived deployment, production activation or merge qualification is claimed.

Final validation: Game library **1595 passed/four ignored**, player tools **236
passed/zero skips**, monster presentation **11 passed**, cue provenance **four
passed**. Full manifest admission passed with exact outer pins. Required Game
Clippy still fails with **859** library-test diagnostics, without added suppression.
Independent review passed. This remains a preservation draft.
