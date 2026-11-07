# R23 candidate evidence — qualification remains open

This directory records a new local candidate. The historical r22 audit stays unchanged.
Neither these data nor passing library tests establish a final production candidate.
The current test manifest has SHA256
`f3f1fac9cb6dd473824938c4c7a24f8de01e86e3160403a78bf3c5e7a653d2d6`.
Its magnitude policy is explicitly `baseline_test`, with training revision
`s3b-content-1`; this is test admission, not complete production numerical ownership.

The normalized provider contains 162 creatures. Derivation enables one approximate
physical melee slot for 125 and disables 37 without compatible source closure.
Armor/shield mitigation is absent and player health stays at least one. Ranged
attacks, defenses, custom callbacks, complete spawn/chase and full monster spell
scheduling are not qualified by this adapter. The full per-record derivation and
original approximations remain in `monster-melee-derivation.json`.

Soul War has 16 source-transcribed rules, 17 unique participants, 23 positive zone
boxes and five excluded boxes. The 22 participant references include duplicates
across nine roles. These are candidate encounter/admission data; real progression,
access, rewards and live participant/callback composition remain open.

Retained evidence records 1586 library tests passed and four ignored, six encounter
admission tests passed, and successful PostgreSQL guard cases. The first composed
TCP/TLS spell attempt failed at `exori vis` after seven reported successful self
casts. Its full log was overwritten when the coordinator began a second attempt;
`evidence/first-physical-attempt-failure.json` preserves the previously observed
failure excerpt and explicitly has no full-log hash. The second-attempt log is
only an in-progress snapshot. No final physical result is claimed here.

Current web access did not provide fresh wiki verification: Tibiopedia redirected
to a setup page despite HTTP200, TibiaWiki BR returned403 and Fandom402. The
coordinator reported unauthorized Tavily access and offline Remote Desktop devices.
Existing source derivations and approximations must not be relabeled as fresh wiki
truth. `research/source-access.json` separates the HTTP records from coordinator
observations.

To reproduce the normalized manifest inputs from this checkout, choose a new output
directory and run:

```sh
python docs/reference/spells/r23-candidate/materialize_manifest.py --out /tmp/oteryn-r23-inputs
```

The recipe uses only relative repository paths, verifies every exact payload hash,
and refuses to overwrite an existing populated candidate directory. It reuses pinned
normalized r21 provider data, overrides the melee creature table and training input,
and emits the exact r23 test manifest. It does not load host-only paths, redistribute
raw source assets, stage gameplay, publish Git or activate deployment.

`candidate-status.json` names the remaining runtime, quest, Premium/house and
content/deployment blockers. Premium/Aleta core tests do not prove live Platform
entitlement, HouseInstance permission/session behavior or production admission.
The coordinator must append current final receipts after resolving the physical
failure; later successful attempts must preserve this failed attempt as history.
