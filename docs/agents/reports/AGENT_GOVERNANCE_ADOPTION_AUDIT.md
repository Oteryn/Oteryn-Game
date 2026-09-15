# Agent governance adoption audit

Date: 2026-08-05  
Target: `blakinio/Oteryn-v2`  
Mode: policy adoption and adaptation audit  
Final verdict: `PASS`

## Sources reviewed

The audit compared the live default-branch policies of:

- `blakinio/Oteryn-Platform`;
- `blakinio/Otheryn`;
- `blakinio/otclient`;
- `blakinio/freqtrade`.

Pinned policy evidence included:

- Oteryn Platform root `AGENTS.md` blob `64af76058e810eb1a535cc1505aa12cc4a158bd7`;
- Oteryn Platform bootstrap blob `6f7712bc5932afbda1a7abbcf04515df8dadbf8b`;
- Oteryn Platform anti-stall blob `6aa0f127827c968400390334d29017220fd92f14`;
- Oteryn Platform GitHub-only blob `240bf242a3ac025ad2b00f64972893d62d46562f`;
- Otheryn root `AGENTS.md` blob `a112b60139af020ea137a55b86ae4d669447212e`;
- Otheryn bootstrap blob `48a26648117125a029c29f5a554509da45a702b6`;
- otclient root `AGENTS.md` blob `d991b8da2169f498ada06d5775e78f97cacf2c36`;
- otclient bootstrap blob `378187381769d36a3ebf3ba92449c93415dc6bca`;
- freqtrade root `AGENTS.md` blob `d876abd90599639fcf0208553ad9850959ce44fc`.

## Adopted common principles

- repository write allowlist and cross-repository separation;
- trusted-base authority freeze;
- dedicated task branches/PRs and durable checkpoints;
- bounded autonomous execution and terminal-CI limits;
- session recovery without reliance on chat history;
- GitHub-only execution when local tooling is unavailable;
- exact-head validation, independent audit, E2E classification and full closeout;
- prompt construction, evaluation and handover standards;
- explicit truth labels and context minimization;
- squash merge after all gates pass.

## Adapted for Oteryn v2

- repository allowlist changed to `blakinio/Oteryn-v2`;
- task prefix changed to `OTV2`;
- Laravel/PHP and C++ build rules replaced by greenfield Rust/workspace rules;
- Canary-compatible architecture replaced by `protocol-oteryn`-only target runtime;
- multichannel world/channel/instance ownership made mandatory;
- Oteryn Platform, Otheryn and otclient classified as separately authorized external repositories;
- build matrix made bootstrap-aware so agents do not invent a Cargo workspace before it exists;
- high-risk gates added for leases, stale writers, item transfers, channel hopping, PvP, houses and protocol reconciliation;
- asset provenance and server-authoritative gameplay made explicit;
- a dedicated context-handoff contract was added after independent review identified the need for stronger durable continuation.

## Intentionally not copied

- Laravel, Composer, Blade, payment-provider and PHP-specific policy;
- CMake, MSVC, PCH, C++ Lua userdata and Canary Docker quickstart rules;
- legacy OTClient modules, OTUI, C++ protocol adapters and proprietary asset installation specifics;
- Freqtrade/live-capital, exchange, model and trading-operation controls;
- repository-specific active indexes, historic task state and completed programme records;
- any policy that treats Canary as the target game server or `protocol-canary` as required runtime compatibility.

## Consistency review

The adopted set distinguishes:

- current repository state from planned workspace paths;
- repository mutation from production/live operation;
- world-shared from channel-local authority;
- game rulesets from wire protocol versions;
- local task authority from external repository authority;
- implementation evidence from plans and historical results.

## Validation and review

- implementation PR: #2;
- exact final head: `7ca8ab13c584d16436360dea66663054ad52194f`;
- governance workflow run: `30981501550`;
- workflow conclusion: `success`;
- automated review: one P2 durable-checkpoint finding;
- repair: checkpoint now uses live PR head authority, records immutable prior validation evidence and provides a current exact next action;
- unresolved review threads at merge: zero;
- implementation merge: `7ed8c6826e1fe04d259d4268049ec9fdfcdf3bf1`.

## Residual follow-ups

- The Rust workspace and actual build commands do not yet exist; `BUILD_TEST_MATRIX.md` must be updated during workspace bootstrap.
- Branch protection should require the proven `Agent governance` check for relevant future changes.
- Nearer path-specific `AGENTS.md` files are required as protocol, persistence, server, client, content and release areas are created.
- Common policy evolution in older repositories does not automatically propagate; synchronization requires a reviewed governance task, not blind copying.

These are future repository-hardening actions, not open material findings against this completed adoption task.
