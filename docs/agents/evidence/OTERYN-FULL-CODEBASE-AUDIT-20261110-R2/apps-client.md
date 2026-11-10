# Native client application — complete source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **REVIEWED**, not runtime VERIFIED. All **1,180 tracked files / 143,488 text lines** were substantively reviewed. The frozen coverage ledger contains exact hashes and full ranges; no tracked package file remains outside the review. The immutable source checkout is clean.

## Scope and integration

The package contains 1,056 Rust source/build/test files, 115 JSON files, four Markdown files, one Cargo manifest, two PowerShell scripts, one Inno Setup file and one development environment template. The JSON scope includes 112 play registrations, two wire fixtures and one names-only preference catalogue. All 28 top-level integration test files were read, alongside embedded and feature tests. Fixture or mocked Stepper tests are not native gameplay execution.

The complete delegated 422-file allocation was integrated through exact hash/range checks. Historical child checkpoint groups are not additive coverage; the consolidated per-file ledger is authoritative. All four files changed by the separately checked newer main commit have their own full review in `latest-main-delta.md` and do not silently replace the immutable input.

## Entrypoints and state

The review traced executable startup, installed slot selection, package integrity, launcher/update handoff, error propagation and crash reporting. Development asset roots remain distinct from the installed lifecycle. Native login, callback/attempt state, device handling, character selection, account-service entry and fresh game authorization were followed through their mounted modules. Installer and concurrent lifecycle questions remain private candidates, without a production execution claim.

Client commands pass from the real native input and Outbox/Stepper path through runtime validation and Session. Bounded observations feed reducers and presentation. The registration check covers the 112 play JSON files, 296 module edges and 20 routed command groups. Old unmounted copies are tracked review scope but cannot alone prove a production defect.

Snapshot replacement, current generations, pending-command resolution, malformed/refused results, closed queues and logout were reviewed across features. Map replacement installs fresh geometry. The explicit death event counter and command-drain barrier clear queued world actions even when respawn precedes the next frame; logout remains possible. Repeated vitals presentation does not itself create a continuing protection-query loop. Protocol handoff dependencies retain separate independent evidence.

## Native controls and rendering

The review covered actual Windows chord dispatch, window lifetime/open flags, callbacks, weak mailbox references and shutdown. Touch owns its contact, filters promoted mouse input and reuses existing action adapters. Held movement waits for an idle action path and cancels on focus loss, occlusion, death and relevant input blocking.

BrowseField retains the displayed item handles. The archive resolves a unique current spell from retained display identity and does not silently accept reused or ambiguous indices. Server-provided eligibility, UI cooldown estimates and actual cast authority remain distinct. Native server modals use the Session-owned bounded answer route, exact server IDs, refusal retry and closure handling. Inline image data is decoded as bounded pixels.

World/map appearance, inventory/container references, sprites and cache keys were inspected with their actual consumers. The separate Watch path uses its own public picture observations. Effect expiry is time-driven and terminal failure clears its view. Settings, preferences, keybindings, audio, screenshot/support consent and native layout were followed to their real persistence and ownership boundaries. A names-only preference catalogue is not a feature authorization schema.

## Store and account-service boundaries

The Store review follows retained purchase identity, catalogue refresh, confirmation and retry. The confirmed bounded modal/request discrepancy has separate independent evidence and does not establish duplicate debit. Account-service forms generally use an immutable displayed View before subsequent HTTP work mutates state; that sequencing is a material guard when assessing stale-row hypotheses.

Bazaar operations retain their pending request identity across uncertain outcomes. Receipt resolution remains operation-specific. Transfer uses the server's quote with character, destination and expiry checks. Coin purchase binds package, quote and order state, does not credit coins merely by opening checkout, and rejects conflicting or regressing observations. Test-only local payment fixtures do not prove a production provider's behavior. Premium overviews are read-only observations and do not grant entitlement.

## Findings, checks and limits

CL-001 through CL-006 have separate independent lead verification; unresolved client candidates stay outside confirmed counts. Native control omissions, presentation/state races and installer questions require their own bounded adjudication. Counterguards include immutable ready trade offers, current identity/revision checks and actual module mounting. A latent unsafe parser in an unmounted old window was not promoted into a reachable production crash. The earlier isolated Highscores test inconsistency remains a test observation, not a full-suite or production failure claim.

Executed checks include exact source/ledger consolidation, registration and reference validation, the 213-name preference catalogue check, child wire-fixture semantic inspection and parent validation of all 422 child files. The package's Rust tests were read, not run. No full Cargo/workspace build, Windows event loop, installer/PowerShell run, GPU/audio device test, live OAuth/payment service, SQL migration or native gameplay session was executed. Full source coverage at the stated base is complete; runtime validation and remaining cross-package work remain explicit.
