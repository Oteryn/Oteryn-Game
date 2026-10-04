# R23 canonical-r4 candidate — bounded physical pass, gameplay prerequisites open

The current normalized provider contains **1508 Creature and 1508 Presentation**
records. Source-attested derivation enables one approximate nonlethal physical melee
slot for 936 creatures and disables 572. Its appearance provider contains 268 records,
including 138 additional actual source-qualified illusion bindings. The generated test
manifest has SHA256
`272466bd8965a4a71566212f68fb61c2a3f35885b1adf9dab3434e31509a5fd5`.
The magnitude policy remains explicitly `baseline_test`, with training revision
`s3b-content-1`. Portable generation is not content admission or runtime qualification.

`canonical-profile-union-proof.json` records the accepted canonical inputs, exact
output hashes and counts. The union preserves 162 existing active profiles and adds
1346 from the 1503 canonical Creature records. Two complete canonical records, Rat
and Skeleton, are retained in the proof as source-only exclusions: their native
display names duplicate the preserved existing Canary identities. Neither identity
allocation nor a name/revision substitution is claimed. The proof retains source-only
encounter references and the reasons they remain disabled.

Melee remains approximate: armor/shield mitigation is absent, player health stays
at least one, and only one compatible source slot is selected. Full ranged attacks,
defenses, custom callbacks, source scheduling, spawn and chase remain unqualified.
`monster-melee-derivation.json` preserves the limitations and complete per-record
source evidence; expanding a provider does not implement full wild monster AI.

The earlier 162-creature overlay, 125/37 derivation, training input, recipe, exact
native manifest and status are preserved under `historical/initial-162/`. Their
manifest `f3f1fac9...` belongs to the earlier physical attempts. The first attempt
failed at `exori vis` after seven reported successful self-casts. Its full log was
overwritten by the next attempt; the retained failure excerpt explicitly has no
full-log hash. The partial second-attempt log also stays historical. Neither is a
validation result for the expanded r4 manifest.

Current retained output records 1590 library tests passed and four ignored, plus
one expanded canonical profile admission/staging test passed with 1593 filtered.
Physical attempt 4 passed on the exact r4 manifest in 342 seconds; the coordinator
observed process exit 0 in session 56156. Normal owner movement crossed the source
stairs, kept `exori vis` refused in the temple protection zone without mana change,
and accepted it outside protection for 20 mana. Durable training recorded 1870 mana
paid and 2738 saved training after ordinary disconnect.

The 246 untargeted catalog probes have complete unique indices: 241 selected and
five unselected. Their outcomes were 11 Cast, 23 CoolingDown, 83 NotAvailable,
128 Rejected and one TargetRequired. They do not establish 246 successful spell
executions. The log explicitly leaves CreatureDamage, condition visibility and
wounded healing magnitude unevaluated. See the retained log and machine-readable
`evidence/physical-attempt-4-receipt.json` and `evidence/current-validation-receipt.json`.

Game-owned wild spawn qualification is now underway. Actual CreatureDamage and
complete wild AI remain unproved. Required Clippy still fails; the coordinator owns
the required diagnostics and gates. These bounded passes do not qualify full gameplay,
production activation or all client/quest/entitlement paths. Historical 1586/four-ignored
and failed f3f attempts remain scoped historical evidence.

Soul War retains 16 source-transcribed rules, 17 unique participants, 23 positive zone
boxes and five exclusions. Its 22 participant references include duplicates across
nine roles. Live quest progression, access, rewards, participant placement and
callback composition remain separate blockers. Premium/Aleta core tests likewise
do not prove live Platform entitlement, HouseInstance ACL/session behavior or
production house admission.

No fresh wiki verification is available: Tibiopedia returned a setup redirect
with HTTP200, TibiaWiki BR 403 and Fandom402. The coordinator reported unauthorized
Tavily access and offline Remote Desktop devices. Existing derivations and source
approximations remain labeled as such in `research/source-access.json`.

Choose a new output directory and reproduce the pinned normalized inputs from this
checkout with:

```sh
python docs/reference/spells/r23-candidate/materialize_manifest.py --out /tmp/oteryn-r23-canonical-r4-inputs
```

The bounded helper remains unchanged by this data/report update. The recipe uses
repository-relative pinned paths, overrides Creature, Presentation, appearances
and training, and emits the exact 11-provider manifest. It refuses an existing output
directory and does not stage gameplay, publish Git, consume host-only pointers or
activate deployment. Historical r22 remains untouched.

The repository profile search inspected both current checkouts, hidden/untracked
and ignored files, target certificate/profile filenames, known live CLI interfaces
and bounded current GitHub searches. It found no actual long-lived address/CA/grant
profile. The known live environment keys were absent. Loopback examples and mktemp
qualification nodes remain identified as disposable, rather than inferred as a
persistent target. Methods and exact pointers are retained in
`evidence/repository-connection-profile-search.json`; no secrets were emitted and
no existing long-lived server deployment/test is claimed.
