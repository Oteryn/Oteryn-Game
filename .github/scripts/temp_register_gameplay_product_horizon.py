from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if new in text:
        return
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


register = "docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md"
backlog = "docs/architecture/FOUNDATION_DECISION_BACKLOG.md"
programme = "docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md"

replace_once(
    register,
    "This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages.\n",
    "This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages. The detailed open gameplay and product horizon is retained in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.\n",
)

register_section = """## Registered gameplay and product decision horizon

Detailed scope, dependencies and non-decisions are canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

### Blocks durable gameplay

- `GAME-CHAR-01` — Character Lifecycle and Progression. Must precede the final `DUR-02` character schema.
- `GAME-ITEM-01` — Item Model and Equipment Rules. Must precede the final `DUR-03` item transaction model.

### Required for Playable Alpha completeness

- `GAME-ABILITY-01` — Ability, Spell and Condition Architecture.
- `GAME-AI-01` — Creature AI, Spawn and Pathfinding Architecture.
- `GAME-INTERACTION-01` — World Interaction and Environmental Mechanics.
- `PROD-LIVEOPS-01` — Live Operations and Runtime Configuration.
- `PROD-COMPAT-01` — Release Compatibility and Version Train.
- `SEC-CLIENT-01` — Client Integrity and Anti-Cheat Boundary.
- `DATA-PRIVACY-01` — Product Privacy and Data Lifecycle.
- `UX-I18N-A11Y-01` — Localization, Input, Onboarding and Accessibility.
- `OPS-GM-01` — Support, Moderation and GM Operations.

### Expansion or deferred

- `GAME-META-01` — Collections, Achievements and Recurring Progression (`EXPANSION`).
- `GAME-INSTANCES-01` — Dungeons, Arenas, Matchmaking and Spectating (`EXPANSION`).
- `GAME-WORLD-LIFECYCLE-01` — World Lifecycle, Transfer and Merge (`EXPANSION`).
- `INTEGRATION-API-01` — External APIs, Notifications and Integrations (`EXPANSION`).
- `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce Boundary (`DEFERRED`).
- `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem (`DEFERRED`).

Registration prevents omission; it does not accept technologies, formulas, schemas, service topology or implementation.

"""
replace_once(register, "## Stage C — blocks the foundation vertical slice\n", register_section + "## Stage C — blocks the foundation vertical slice\n")

replace_once(
    register,
    "12. VSL-02 uses one atomic Oteryn-v2 destination PR; the later otclient PR is source-marker closeout only.\n",
    "12. VSL-02 uses one atomic Oteryn-v2 destination PR; the later otclient PR is source-marker closeout only.\n13. Every gameplay/product package must reconcile `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`; an unnamed domain may not be silently absorbed into an unrelated gate.\n",
)

replace_once(
    register,
    "Draft and accept `FND-01` — the **Workspace, Dependency and Existing-Rust Migration Contract**. Its terminal next action is `VSL-02`, followed by one atomic destination migration/workspace PR, the source-only cutover marker and only then layer-specific contracts.\n",
    "Draft and accept `FND-01` — the **Workspace, Dependency and Existing-Rust Migration Contract**. Its terminal next action is `VSL-02`, followed by one atomic destination migration/workspace PR, the source-only cutover marker and only then layer-specific contracts. The registered gameplay/product gates remain ordered future work and do not replace this immediate action.\n",
)

backlog_section = """## Registered gameplay and product decision horizon

The complete open-decision scope is canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

- `GAME-CHAR-01` and `GAME-ITEM-01` are new durable-gameplay gates: character semantics must precede final `DUR-02`, and item semantics must precede final `DUR-03`.
- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` are required before Playable Alpha gameplay breadth is claimed; bounded vertical-slice contracts may precede them.
- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` are required before Playable Alpha operational completeness is claimed.
- `GAME-META-01`, `GAME-INSTANCES-01`, `GAME-WORLD-LIFECYCLE-01` and `INTEGRATION-API-01` are expansion gates.
- `PROD-ENTITLEMENTS-01` and `MOD-ECOSYSTEM-01` remain explicitly deferred until an owner decision activates them.

Registering a gate does not accept its implementation choice.

"""
replace_once(backlog, "## Explicitly deferred\n", backlog_section + "## Explicitly deferred\n")
replace_once(backlog, "- complete dungeon/arena instance programme;\n", "- `GAME-INSTANCES-01` complete dungeon/arena/matchmaking/spectator programme;\n")
replace_once(backlog, "- public mod ecosystem;\n", "- `MOD-ECOSYSTEM-01` public mod ecosystem;\n")

old_order = """12. Accept DUR-01 full Identifier Contract for database and durable-state representation
13. Accept ANL-01 Game Event and Audit Foundation Contract
14. Accept DUR-02 Persistence v1 Contract with transactional outbox/audit recovery
15. Accept DUR-03 Item Transaction and Anti-Duplication Contract, if not complete in DUR-02
16. Draft ANL-02 and ANL-03 on the accepted event/persistence/item foundations
17. Run the bounded world-format spike and complete DUR-04 under ADR-0005
18. Accept VSL-01 Foundation Vertical-Slice Programme with correlated event/audit evidence
19. Execute the separately authorized vertical-slice implementation programme
20. Complete ANL-02/ANL-03 before production-grade alpha analytics claims; defer ANL-04 until read-only investigation is authorized
"""
new_order = """12. Accept DUR-01 full Identifier Contract for database and durable-state representation
13. Accept GAME-CHAR-01 before DUR-02 freezes the durable character schema
14. Accept GAME-ITEM-01 before DUR-03 freezes item behavior and transfer semantics
15. Accept ANL-01 Game Event and Audit Foundation Contract
16. Accept DUR-02 Persistence v1 Contract with transactional outbox/audit recovery
17. Accept DUR-03 Item Transaction and Anti-Duplication Contract, if not complete in DUR-02
18. Draft ANL-02 and ANL-03 on the accepted event/persistence/item foundations
19. Run the bounded world-format spike and complete DUR-04 under ADR-0005
20. Accept VSL-01 Foundation Vertical-Slice Programme with correlated event/audit evidence
21. Execute the separately authorized vertical-slice implementation programme
22. Accept GAME-ABILITY-01, GAME-AI-01 and GAME-INTERACTION-01 before Playable Alpha gameplay breadth is claimed
23. Accept PROD-LIVEOPS-01, PROD-COMPAT-01, SEC-CLIENT-01, DATA-PRIVACY-01, UX-I18N-A11Y-01 and OPS-GM-01 before Playable Alpha operational completeness is claimed
24. Complete ANL-02/ANL-03 before production-grade alpha analytics claims; defer ANL-04 until read-only investigation is authorized
25. Activate expansion/deferred gameplay-product gates only when their milestone or explicit owner decision requires them
"""
replace_once(backlog, old_order, new_order)

replace_once(
    backlog,
    "- `DUR-01`, `DUR-02` and `DUR-03` must be accepted before authoritative durable character, item or currency mutation.\n",
    "- `DUR-01`, `DUR-02` and `DUR-03` must be accepted before authoritative durable character, item or currency mutation.\n- `GAME-CHAR-01` must be accepted before `DUR-02` finalizes the character schema; `GAME-ITEM-01` must be accepted before `DUR-03` finalizes item semantics.\n- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` are required before Playable Alpha gameplay breadth is claimed; bounded vertical-slice contracts may precede them.\n- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` are required before Playable Alpha operational completeness is claimed.\n",
)

replace_once(
    programme,
    "Canonical evidence: ADR-0001 through ADR-0006.\n",
    "Canonical evidence: ADR-0001 through ADR-0006.\n\nRegistered open-decision coverage: `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`. Registration prevents omission but does not accept solutions.\n",
)

stable_ids = """- `GAME-CHAR-01` — Character Lifecycle and Progression.
- `GAME-ITEM-01` — Item Model and Equipment Rules.
- `GAME-ABILITY-01` — Ability, Spell and Condition Architecture.
- `GAME-AI-01` — Creature AI, Spawn and Pathfinding Architecture.
- `GAME-INTERACTION-01` — World Interaction and Environmental Mechanics.
- `PROD-LIVEOPS-01` — Live Operations and Runtime Configuration.
- `PROD-COMPAT-01` — Release Compatibility and Version Train.
- `SEC-CLIENT-01` — Client Integrity and Anti-Cheat Boundary.
- `DATA-PRIVACY-01` — Product Privacy and Data Lifecycle.
- `UX-I18N-A11Y-01` — Localization, Input, Onboarding and Accessibility.
- `OPS-GM-01` — Support, Moderation and GM Operations.
- `GAME-META-01` — Collections, Achievements and Recurring Progression.
- `GAME-INSTANCES-01` — Dungeons, Arenas, Matchmaking and Spectating.
- `GAME-WORLD-LIFECYCLE-01` — World Lifecycle, Transfer and Merge.
- `INTEGRATION-API-01` — External APIs, Notifications and Integrations.
- `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce Boundary.
- `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem.
"""
replace_once(programme, "- `ANL-04` — Read-Only Investigation and AI Contract.\n", "- `ANL-04` — Read-Only Investigation and AI Contract.\n" + stable_ids)

replace_once(
    programme,
    "- `ANL-02`/`ANL-03` gate production-grade analytics claims; `ANL-04` gates later read-only AI investigation.\n",
    "- `ANL-02`/`ANL-03` gate production-grade analytics claims; `ANL-04` gates later read-only AI investigation.\n- `GAME-CHAR-01` and `GAME-ITEM-01` gate final character/item durable models before `DUR-02`/`DUR-03`.\n- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` gate Playable Alpha gameplay breadth.\n- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` gate Playable Alpha operational completeness.\n- Expansion/deferred gameplay-product gates remain inactive until their milestone or explicit owner decision requires them.\n",
)

replace_once(programme, "- complete instance and market programmes;\n", "- `GAME-INSTANCES-01` complete instance/matchmaking/spectator programme and full market programme;\n")
replace_once(programme, "- public mod ecosystem;\n", "- `MOD-ECOSYSTEM-01` public mod ecosystem;\n")

replace_once(
    programme,
    "- [ ] `DUR-03` accepted before durable item/currency mutation.\n",
    "- [ ] `DUR-03` accepted before durable item/currency mutation.\n- [ ] `GAME-CHAR-01` accepted before `DUR-02` freezes the durable character schema.\n- [ ] `GAME-ITEM-01` accepted before `DUR-03` freezes item behavior and transaction semantics.\n- [ ] Playable Alpha gameplay breadth is not claimed before `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01`.\n- [ ] Playable Alpha operational completeness is not claimed before live-ops, compatibility, client-integrity, privacy, localization/accessibility and GM-operation gates are accepted.\n",
)

replace_once(
    programme,
    "last_progress: ADR-0001 through ADR-0006 are accepted; ADR-0002 requires VSL-02 immediately after FND-01 and one atomic destination migration/workspace PR before a source-only cutover marker and shared-contract freeze.\n",
    "last_progress: ADR-0001 through ADR-0006 are accepted; ADR-0002 requires VSL-02 immediately after FND-01 and one atomic destination migration/workspace PR before a source-only cutover marker and shared-contract freeze; missing gameplay and product domains are registered as open future gates.\n",
)

replace_once(
    programme,
    "  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md\n",
    "  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md\n  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md\n",
)

for path, marker in [
    (register, "GAME-CHAR-01"),
    (backlog, "PROD-LIVEOPS-01"),
    (programme, "UX-I18N-A11Y-01"),
]:
    if marker not in Path(path).read_text(encoding="utf-8"):
        raise SystemExit(f"{path}: missing marker {marker}")
