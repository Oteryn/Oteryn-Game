# spell-rules — REVIEWED

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. All 24 tracked package files, including manifests, property tests and embedded tests, substantively reviewed. Source was not changed.

## Entrypoints and authority

The package builds pure cast, geometry, native actor, companion and item plans. It does not apply SQL or runtime mutations. A native plan requires CompiledNativeSpell; the actual content constructor has private fields and accepts only a complete spell/dependency payload equal to one of 67 embedded profiles. Exact source identity, current actor facts, lease generation, payment and mutation remain the owning adapters' duties. Relevant dispatch/spell callers were traced, not assumed from planner names.

## Invariants, failures and retries

- Cast cooldown keys distinguish spell and group namespaces and are canonicalized before use. Rearm computes all checked semantic deadlines before mutation, so one overflowing group cannot leave a partially updated plan. Focus reset preserves unknown individual keys and the separately owned stairhop pacification. Percentage mana uses u64 arithmetic sufficient for u32 endpoints and saturates only at the declared output boundary. Current Premium, rather than NotActivated, satisfies ordinary premium spells.
- Geometry uses checked coordinates, widened differences, explicit matrix centers and qualified tile/sight data. The path search is bounded and forbids diagonal corner cutting through missing or solid neighbors. Rune legality rejects unknown occupancy/solidity as appropriate; only a known occupied solid tile obtains the explicit exception. PZ, floor, sight and range order were compared with the actual preparation path.
- Direct-target selection assumes a qualified target. Actual ordinary-combat preparation intersects candidates with current legal actors before resolving target/master/position and applies live PvP legality for player targets, PvE floor/solid/alive/master/attackable/PZ gates otherwise. Later application planning binds the content, command occurrence, world digest, current equipment and selected variation; effect count is bounded.
- Harmony uses exact bounded rational arithmetic for base multipliers. Native healing validates closed formula ASTs, rejects nonfinite/division-by-zero results and samples through a bounded deterministic normal-draw procedure. Focus, forced Serene and stance replacement use checked time/charge limits; current eligible vocation, equipment skill, exact target percentage health and nonstacking party-effect keys were examined. Stance damage origin exclusions prevent applying the monk spell modifier indiscriminately to item damage or healing.
- Familiar and summon plans carry actions but grant no direct authority. Placement and creation precede success costs; source cap, corpse validity, master ownership, current mana and privileged group flags are required by their corresponding branch. The actual summon adapter reads privileged flags from the current durable group, not client facts. Familiar casts bind the current actor/character/content/occurrence and derive Premium/settings from that owner.
- Generic donor familiar login timestamps are adapted by the production lifecycle owner to the accepted durable online-remaining-time contract. Current owned IDs, writer state and return restrictions are checked, stored remaining duration is supplied, and exact millisecond deadlines override rounded planner values. This closes the observed donor-versus-runtime timing concern without claiming the whole lifecycle/durability package was audited.
- Food validates its closed pool before draws and uses checked indices; mimic treats an existing container as authoritative even when empty. Disintegration caps visited entries at 500, including protected objects, validates unique instance IDs and excludes scripted/protected corpse items. Remove-field selects the first field and requires the exact supported source identity. Barrier placement requires current tile existence, no floor change or occupant and a bounded positive duration.
- The actual world-item owner validates current fence/command/content, samples and retains one pending plan, compares reconnect and before-state facts, checks current access after awaited work, and applies through a guarded durable request. Known refusal rolls back uncommitted state; unknown commit retains recovery state. Committed receipts are marked before fallible follow-up and historical results reconcile rather than granting a second mutation. Only the recorded dependency range is reviewed.

## Hypotheses resolved

1. A direct target bypass through TargetSelection is refuted by the actual prepare adapter's current legal-actor filter and target qualification.
2. Untrusted native matrix, AST or role parameters are not accepted through the compiled native path: private construction pins the complete embedded spell and dependency payload. Generic pure-helper assumptions are not a demonstrated production endpoint.
3. Client-supplied summon/convince privilege is refuted by the actual current durable group lookup; absent policy facts refuse.
4. Familiar donor wall-time logic does not establish loss of the accepted offline pause: the runtime adapter supplies stored remaining time and retains the exact durable deadline.
5. Generic signed subtraction in normal_draw was examined. Production uses found are positive bounded healing, 0..100 wheel roll and 1..100 drunk movement; no reachable overflow input was identified. This is not reported as a confirmed finding.

## Findings

No new confirmed finding in this package. Existing GP-001 and GR-SOUL-001 remain separately owned and are not duplicated. Public-seed RNG concerns were already recorded by the prior audit; deterministic sampling alone is not a new finding.

## Evidence and remaining scope

All package tests were analyzed as source, not executed. No native server, SQL or integration run is claimed. Dependency excerpts establish the specific counterguards above and do not mark larger spell/dispatch/durability modules complete. No package files remain. Ledger entries carry exact analyzed ranges, actual line counts, source SHA256 and the immutable base commit.
