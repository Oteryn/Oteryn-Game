#!/usr/bin/env python3
from __future__ import annotations

import argparse, json, os, re, subprocess
from collections.abc import Callable
from pathlib import Path

E_PATHS = {
    "docs/agents/tasks/archive/OTV2-20260815-alpha-client-architecture.md",
    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md",
    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md",
}
F_PATHS = {
    "docs/agents/tasks/archive/OTV2-20260815-analytics-integrity-architecture.md",
    "docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md",
    "docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_CONTRACT_CANDIDATE.md",
    "docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md",
    "docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_CONTRACT_CANDIDATE.md",
}
R_PATHS = {
    "apps/game-server/src/foundation/admission_recovery_inner.rs",
    "apps/game-server/src/foundation/fnd04_verifier.rs",
    "docs/agents/tasks/archive/OTV2-20260826-impl-foundation-reconnect-durability.md",
}

PROFILE_PATHS = (
    ("ALPHA_CLIENT_01", E_PATHS),
    ("ANL_02_ANL_03", F_PATHS),
    ("FOUNDATION_RECONNECT_DURABILITY_V1", R_PATHS),
)

CURRENT_AUTHORITY_TERMS = (
    "authenticated_evidence_observed_by(record, current.observed_at)",
    "current.identity == *identity",
    "current.current_account_presence == Some(AccountPresenceClaimV1::expected_from_identity(identity)?)",
    "current.current_character_world_eligibility == Some(CharacterWorldEligibilityClaimV1::expected_from_identity(identity))",
    "current.current_candidate == Some(ReconnectCandidateBindingV1::expected_binding_from_record(record)?)",
    "current.current_runtime_scope == identity.runtime_scope()",
    "current.predecessor == record.connection().predecessor()",
    "current.authority == record.authority()",
    "current.continuity_epoch == record.continuity().control_loss_epoch()",
    "current.original_grace_deadline == record.continuity().original_grace_deadline()",
    "current.proof == *record.proof()",
    "current.fnd02 == *record.fnd02()",
    "current.protocol_major == compatibility.protocol_major()",
    "current.transport_profile == compatibility.transport_profile()",
    "current.ruleset_revision == compatibility.ruleset_revision()",
    "current.content_revision == compatibility.content_revision()",
    "current.map_revision == compatibility.map_revision()",
    "current.world_policy_revision == compatibility.world_policy_revision()",
    "current.account_security_generation == compatibility.account_security_generation()",
    "current.platform_security_evidence == *compatibility.platform_security_evidence()",
    "current.proof_trust_evidence == *compatibility.proof_trust_evidence()",
    "current.credential_expiration == compatibility.credential_expiration()",
    "current.current_candidate.is_some_and(|candidate| candidate.is_live_at(current.observed_at))",
    "current.session_state == GameSessionState::Reconnectable",
    "!current.current_controller_present",
)

CURRENT_AUTHORITY_EXPRESSION = "Ok(" + "&&".join(CURRENT_AUTHORITY_TERMS) + ")"

AUTHENTICATED_EVIDENCE_COMPARISONS = (
    "observed_at >= compatibility.platform_security_evidence().source_observed_at()",
    "observed_at >= compatibility.proof_trust_evidence().source_observed_at()",
)

AUTHENTICATED_EVIDENCE_EXPRESSION = "&&".join(
    AUTHENTICATED_EVIDENCE_COMPARISONS
)

AUTHORIZE_AUTHORITY_GUARD = (
    "if !current_authority_matches_record(&self.record, &current)? "
    "|| !authenticated_evidence_observed_by(&self.record, now) "
    "{ self.phase = ReconnectDurabilityPhaseV1::Terminal; "
    "return Err(ReconnectDurabilityErrorV1::StaleAuthority); }"
)

AUTHORIZE_DEADLINE_GUARD = (
    "if now > deadline || current.observed_at > deadline "
    "{ self.phase = ReconnectDurabilityPhaseV1::Terminal; "
    "return Err(ReconnectDurabilityErrorV1::DeadlineExpired); }"
)

AUTHORIZE_COMMIT_ORDERED_INVARIANTS = (
    "if self.phase != ReconnectDurabilityPhaseV1::AwaitFinalRevalidation",
    "if !current_authority_matches_record(&self.record, &current)? || !authenticated_evidence_observed_by(&self.record, now)",
    "return Err(ReconnectDurabilityErrorV1::StaleAuthority);",
    "let deadline = self.record.authorization_deadline()?;",
    "if now > deadline || current.observed_at > deadline",
    "return Err(ReconnectDurabilityErrorV1::DeadlineExpired);",
    "let request = ReconnectCommitRequestV1",
)

RECONCILIATION_V1_ORDERED_INVARIANTS = (
    "if self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired",
    "if snapshot.record != self.record { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }",
    "DurableReconnectStateV1::Committed",
    "snapshot.current_generation != Some(self.record.connection().candidate())",
    "snapshot.current_transport_ref != Some(self.record.connection().transport_ref())",
    "current.observed_at > self.record.authorization_deadline()?",
    "!current_authority_matches_record(&self.record, &current)? { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }",
    "self.phase = ReconnectDurabilityPhaseV1::Completed;",
    "Ok(ReconnectProjectionDecisionV1::InstallController",
    "generation: self.record.connection().candidate()",
    "transport_ref: self.record.connection().transport_ref()",
)

RECONCILIATION_V2_ORDERED_INVARIANTS = (
    "if self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired",
    "if snapshot.record != self.record { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }",
    "ReconnectDurableOutcomeV2::Committed",
    "current_generation != self.record.connection().candidate()",
    "current_transport_ref != self.record.connection().transport_ref()",
    "current.observed_at > self.record.authorization_deadline()?",
    "!current_authority_matches_record(&self.record, &current)? { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }",
    "self.phase = ReconnectDurabilityPhaseV1::Completed;",
    "Ok(ReconnectProjectionDecisionV2::InstallController",
    "generation: current_generation",
    "transport_ref: current_transport_ref",
)

RECONCILIATION_V1_COMMITTED_GUARD = (
    "if snapshot.current_generation != Some(self.record.connection().candidate()) "
    "|| snapshot.current_transport_ref != Some(self.record.connection().transport_ref()) "
    "|| current.observed_at > self.record.authorization_deadline()? "
    "|| !current_authority_matches_record(&self.record, &current)? "
    "{ return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }"
)

RECONCILIATION_V2_COMMITTED_GUARD = (
    "if current_generation != self.record.connection().candidate() "
    "|| current_transport_ref != self.record.connection().transport_ref() "
    "|| current.observed_at > self.record.authorization_deadline()? "
    "|| !current_authority_matches_record(&self.record, &current)? "
    "{ return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }"
)


def select_profiles(changed: set[str]) -> list[str]:
    return [name for name, paths in PROFILE_PATHS if changed & paths]


def run_profiles(
    selected_profiles: list[str],
    profile_checks: dict[str, Callable[[], list[str]]],
) -> tuple[list[dict[str, object]], list[str]]:
    profiles: list[dict[str, object]] = []
    failures: list[str] = []
    for profile in selected_profiles:
        try:
            checks = profile_checks[profile]()
            profiles.append({"profile": profile, "checks": checks, "verdict": "PASS"})
        except SystemExit as error:
            message = str(error)
            failures.append(f"{profile}: {message}")
            profiles.append(
                {"profile": profile, "checks": [], "verdict": "FAIL", "error": message}
            )
    return profiles, failures


def fail(msg: str) -> None:
    raise SystemExit(f"SEMANTIC_AUDIT_FAIL: {msg}")


def git(*args: str) -> str:
    p = subprocess.run(["git", *args], text=True, capture_output=True, check=False)
    if p.returncode:
        fail(f"git {' '.join(args)} failed: {p.stderr.strip()}")
    return p.stdout.strip()


def text(path: str) -> str:
    p = Path(path)
    if not p.is_file():
        fail(f"required file missing: {path}")
    return p.read_text(encoding="utf-8")


def rust_braced_block(doc: str, marker: str, label: str) -> str:
    start = doc.find(marker)
    if start < 0:
        fail(f"{label}: missing {marker!r}")
    marker_opening = marker.find("{")
    opening = (
        start + marker_opening
        if marker_opening >= 0
        else doc.find("{", start + len(marker))
    )
    if opening < 0:
        fail(f"{label}: missing opening brace")
    depth = 0
    for index in range(opening, len(doc)):
        if doc[index] == "{":
            depth += 1
        elif doc[index] == "}":
            depth -= 1
            if depth == 0:
                return doc[start : index + 1]
    fail(f"{label}: missing closing brace")


def rust_braced_blocks(doc: str, marker: str, label: str) -> list[str]:
    blocks = []
    search_from = 0
    while (start := doc.find(marker, search_from)) >= 0:
        block = rust_braced_block(doc[start:], marker, label)
        blocks.append(block)
        search_from = start + len(block)
    if not blocks:
        fail(f"{label}: missing {marker!r}")
    return blocks


def rust_impl_method(
    doc: str, impl_marker: str, method_marker: str, label: str
) -> str:
    methods = [
        rust_braced_block(block, method_marker, label)
        for block in rust_braced_blocks(doc, impl_marker, label)
        if method_marker in block
    ]
    if len(methods) != 1:
        fail(f"{label}: expected exactly one method, found {len(methods)}")
    return methods[0]


def compact(doc: str) -> str:
    return " ".join(doc.split())


def compact_rust(doc: str) -> str:
    return re.sub(r"\s+", "", doc)


def need_ordered(doc: str, fragments: tuple[str, ...], label: str) -> None:
    offset = 0
    for fragment in fragments:
        found = doc.find(fragment, offset)
        if found < 0:
            fail(f"{label}: missing or out of order: {fragment!r}")
        offset = found + len(fragment)


def need_exactly_once(doc: str, fragment: str, label: str) -> None:
    count = doc.count(fragment)
    if count != 1:
        fail(f"{label}: expected exactly one {fragment!r}, found {count}")


def need(doc: str, fragment: str, label: str, *, ci: bool = False) -> None:
    hay, needle = (doc.casefold(), fragment.casefold()) if ci else (doc, fragment)
    if needle not in hay:
        fail(f"{label}: missing {fragment!r}")


def need_re(doc: str, pattern: str, label: str) -> None:
    if re.search(pattern, doc, re.IGNORECASE | re.DOTALL) is None:
        fail(f"{label}: pattern not satisfied: {pattern}")


def forbid_re(doc: str, pattern: str, label: str) -> None:
    if re.search(pattern, doc, re.IGNORECASE | re.DOTALL) is not None:
        fail(f"{label}: forbidden pattern present: {pattern}")


def common(task: str) -> None:
    need_re(task, r"(?m)^status:\s*completed\s*$", "terminal task lifecycle")
    declared = re.search(r"(?m)^repair_cycles_for_current_gate:\s*([0-9]+)\s*$", task)
    if declared is None:
        fail("repair history: missing repair_cycles_for_current_gate")
    cycles = int(declared.group(1))
    if cycles < 4:
        fail(f"repair history: expected owner-overridden stable gate at cycle >= 4, got {cycles}")
    need(task, "repair_cycle_4_owner_override:", "owner repair override")
    need_re(
        task,
        r"owner_review_constraint:\s*no Codex for (?:this|final) continuation",
        "owner review constraint",
    )
    need(task, "owned_paths: []", "terminal path ownership")
    need(task, "implementation_authority: NONE", "terminal implementation authority", ci=True)
    need(task, "MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY", "merge authority")


def alpha() -> list[str]:
    task = text(next(iter([p for p in E_PATHS if "/tasks/" in p])))
    analysis = text("docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md")
    c = text("docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md")
    common(task)

    required = {
        "implementation truth": "ImplementationStatus: `NOT_STARTED`",
        "runtime authority": "Runtime authorization: **NONE**",
        "ticket": "one-time Game Login Ticket",
        "gateway": "Platform-owned Game Gateway",
        "protocol": "FND-02 `protocol-oteryn` transport/bootstrap",
        "final admission": "final game-owned FND-04 admission",
        "no gateway bypass": "MUST NOT bypass Game Gateway ticket redemption/route selection",
        "production codec path": "same accepted **production protocol schemas, production codecs, sequencing and admission contracts**",
        "independent wire oracle": "shared production code MUST NOT be the only oracle",
        "scene non-authority": "visual scene is a **presentation projection**, not a second gameplay/world model",
        "audio non-authority": "Audio is a client-side **presentation-only** subsystem",
        "Studio heading": "### 14.1 Oteryn Studio low-level sharing boundary",
        "Studio allowlist": "low-level, representation-neutral, non-authoritative components",
        "Studio exclusions": "The following MUST remain product-specific",
        "Studio acyclic": "Dependency direction MUST remain acyclic",
        "Studio export": "authoring-only state MUST be projected/exported through an accepted revisioned content schema",
        "Studio negative evidence": "negative tests proving authoring-only/server-only fields cannot enter the runtime client-safe projection",
        "settings schema scope": "Every durable setting MUST declare a semantic scope",
        "account fail closed": "the client MUST treat the account layer as absent rather than inventing local account authority",
        "device hardware": "including selected audio output",
        "privacy restrictive wins": "the **most restrictive valid privacy choice wins**",
        "privacy cannot re-enable": "MUST NOT re-enable diagnostics disabled at OS-user/installation policy scope",
        "versioned scope migration": "requires an explicit versioned migration",
        "migration fields": "source scope, destination scope, conflict resolution and rollback/recovery",
        "diagnostic persistence": "MUST NOT silently re-enable diagnostics",
    }
    for label, fragment in required.items():
        need(c, fragment, label)
    for scope in ("`ACCOUNT`", "`OS_USER`", "`INSTALLATION`", "`DEVICE`"):
        need(c, scope, "settings scope")

    for fragment in (
        "Platform Identity -> one-time Game Login Ticket -> Platform-owned Game Gateway",
        "independent FND-02 wire evidence",
        "audio application-owned, bounded and presentation-only",
        "diagnostics opted out -> no automatic upload/retry, no gameplay impact",
    ):
        need(analysis, fragment, "analysis consistency")

    need_re(c, r"DEVICE\s*\n\s*>\s*OS_USER\s*\n\s*>\s*ACCOUNT\s*\n\s*>\s*product default", "settings precedence")
    need_re(c, r"shared low-level components MUST NOT depend on `apps/client`, a Studio application root, live-session state or product UI", "shared dependency prohibition")
    forbid_re(c, r"(?:Gateway|Platform)\s+(?:owns|creates|mints)\s+(?:canonical\s+)?(?:GameSessionId|CharacterLease)", "final authority transfer")
    return [
        "admission/Gateway/final-game authority",
        "pre-native fail-closed readiness",
        "production codecs + independent wire oracle",
        "scene/audio presentation-only authority",
        "settings scope/precedence/privacy/migration",
        "Studio sharing/dependency/export boundary",
    ]


def analytics() -> list[str]:
    task = text("docs/agents/tasks/archive/OTV2-20260815-analytics-integrity-architecture.md")
    a2 = text("docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md")
    c2 = text("docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_CONTRACT_CANDIDATE.md")
    a3 = text("docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md")
    c3 = text("docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_CONTRACT_CANDIDATE.md")
    common(task)

    for label, fragment in {
        "ANL-02 authority": "Runtime/client/Platform/PostgreSQL/production authority: **NONE**",
        "fail-closed no-regression": "NO_MATERIAL_REGRESSION_SUPPORTED` is a **fail-closed disposition**",
        "insufficient evidence": "REGRESSION_EVIDENCE_INSUFFICIENT",
        "quality prerequisite": "**quality/completeness**",
        "sample prerequisite": "**sample/exposure**",
        "comparability prerequisite": "**comparability**",
        "reconciliation prerequisite": "**reconciliation/finality**",
        "privacy prerequisite": "**privacy/suppression sufficiency**",
        "provenance prerequisite": "**method/provenance**",
        "warning not green": "warning-only green acceptance is forbidden",
        "read-only negative evidence": "proof no analytical/dashboard path can mutate gameplay",
    }.items():
        need(c2, fragment, label, ci=True)
    need_re(c2, r"If a material regression evaluation is attempted.*?any applicable precondition.*?not affirmatively satisfied.*?REGRESSION_EVIDENCE_INSUFFICIENT", "attempted evaluation fail closed")
    forbid_re(c2, r"PARTIAL[^\n]{0,180}NO_MATERIAL_REGRESSION_SUPPORTED[^\n]{0,120}(?:allowed|permitted|may)", "partial evidence green acceptance")

    for label, fragment in {
        "ANL-03 authority": "Runtime/client/Platform/PostgreSQL/production/enforcement authority: **NONE**",
        "read-only evidence": "read-only evidence + triage input",
        "disposition list": "Allowed **substantive evidentiary dispositions**",
        "integrity disposition": "SUPPORTED_INTEGRITY_OR_DEFECT_FINDING",
        "security disposition": "SUPPORTED_SECURITY_FINDING",
        "false positive": "NOT_SUPPORTED_FALSE_POSITIVE",
        "inconclusive": "INCONCLUSIVE_INSUFFICIENT_EVIDENCE",
        "pipeline failure": "DATA_QUALITY_OR_PIPELINE_FAILURE",
        "duplicate": "DUPLICATE_OR_ALREADY_COVERED",
        "referral not evidence": "`REFERRED_TO_SECURITY_GM_PRODUCT_OR_ENGINE_OWNER` is **not an evidentiary disposition**",
        "no naked referral": "MUST NOT be the sole terminal analytical classification",
        "referral prerequisite": "preceding substantive disposition",
        "routing not classification": "referral is never a substitute for evidentiary classification",
        "target acceptance": "does not imply the target owner accepted",
        "no sanction": "does not authorize ban/mute/kick/confiscation/rollback/account action",
        "immutable lifecycle": "immutable audit record",
    }.items():
        need(c3, fragment, label, ci=True)

    need(a3, "ANL-03 first records its substantive evidentiary disposition and then may emit a separate referral/evidence reference", "ANL-03 analysis ordering")
    need(a3, "referral does not imply acceptance or authority transfer", "ANL-03 analysis authority")
    need(a2, "REGRESSION_EVIDENCE_INSUFFICIENT", "ANL-02 analysis consistency")
    need_re(c3, r"referral.*?require.*?preceding substantive disposition.*?same review generation", "same-generation routing")

    disposition_section = re.search(
        r"Allowed \*\*substantive evidentiary dispositions\*\*.*?(?=\n`REFERRED_TO_SECURITY_GM_PRODUCT_OR_ENGINE_OWNER` is)",
        c3,
        re.IGNORECASE | re.DOTALL,
    )
    if disposition_section is None:
        fail("cannot isolate substantive evidentiary disposition list")
    if "REFERRED_TO_SECURITY_GM_PRODUCT_OR_ENGINE_OWNER" in disposition_section.group(0):
        fail("referral appears in substantive evidentiary disposition list")

    return [
        "ANL-02 read-only evidence authority",
        "fail-closed no-regression evidence prerequisites",
        "REGRESSION_EVIDENCE_INSUFFICIENT on attempted insufficient evaluation",
        "ANL-03 immutable evidence lifecycle",
        "substantive disposition before referral",
        "no sanction/enforcement/mutation authority",
    ]


def foundation_reconnect() -> list[str]:
    return foundation_reconnect_documents(
        text("docs/agents/tasks/archive/OTV2-20260826-impl-foundation-reconnect-durability.md"),
        text("apps/game-server/src/foundation/admission_recovery_inner.rs"),
        text("apps/game-server/src/foundation/fnd04_verifier.rs"),
    )


def foundation_reconnect_documents(
    task: str, implementation: str, verifier: str
) -> list[str]:

    for label, fragment in {
        "authority decision": "DUR-RECONNECT-AUTHORITY-V1",
        "transport uniqueness decision": "DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1",
        "terminal Foundation write authority": "write_authority: none",
        "attempt bound provenance": "FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8",
        "no SQLx scope": "No SQLx/query/migration/schema work",
        "Foundation authority retained": "Foundation retains admission/security/controller authority",
    }.items():
        need(task, fragment, label, ci=True)
    need(task, "status: COMPLETED_ARCHIVED", "terminal Foundation lifecycle", ci=True)
    need(task, "owned_paths: []", "terminal Foundation path ownership")
    need(
        task,
        "This record is immutable historical evidence, owns no path, and grants no dispatch, review, validation, or runtime-write authority.",
        "terminal Foundation non-authority",
    )

    for label, fragment in {
        "stable transport ref": "pub struct AuthenticatedTransportRefV1([u8; 16]);",
        "zero ref rejection": "if bytes.iter().all(|byte| *byte == 0)",
        "attempt cap": "const RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1: usize = 8;",
        "full durability record": "pub struct ReconnectDurabilityRecordV1",
        "identity evidence": "identity: ReconnectIdentityV1",
        "connection evidence": "connection: ReconnectConnectionFenceV1",
        "authority evidence": "authority: ReconnectAuthorityFenceV1",
        "continuity evidence": "continuity: ReconnectContinuityV1",
        "proof evidence": "proof: ReconnectProofV1",
        "FND-02 evidence": "fnd02: Fnd02ReconciliationFenceV1",
        "compatibility evidence": "compatibility: ReconnectCompatibilityEvidenceV1",
        "one-live PREPARED": "entry.state == ReconnectAttemptStateV1::Prepared",
        "collision terminal": "ReconnectAttemptStateV1::CollisionTerminal",
        "prepare split phase": "ReconnectDurabilityPhaseV1::AwaitFinalRevalidation",
        "commit split phase": "ReconnectDurabilityPhaseV1::PendingCommit",
        "reconciliation phase": "ReconnectDurabilityPhaseV1::ReconciliationRequired",
        "legacy journal retained": "ReconnectAttemptJournal<T>",
    }.items():
        need(implementation, fragment, label)

    need_re(
        implementation,
        r"if let Some\(entry\) = self\.entries\.iter\(\)\.find\(\|entry\| entry\.attempt == attempt\).*?entry\.transport_ref == transport_ref.*?ReconnectAttemptReservationV1::Existing.*?IdempotencyConflict",
        "one attempt binds one immutable transport ref",
    )
    need_re(
        implementation,
        r"if self\.entries\.len\(\) >= RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1\s*\{\s*return Err\(ReconnectDurabilityErrorV1::AttemptCapacityExceeded\);\s*\}\s*self\.entries\.push",
        "attempt 9 rejected before allocation",
    )
    need_re(
        implementation,
        r"Prepared \| ReconnectPrepareDispositionV1::ExistingPrepared.*?any\(\|\(other, entry\)\| other != index && entry\.state == ReconnectAttemptStateV1::Prepared\).*?ConcurrentPrepared",
        "second live PREPARED fails closed",
    )
    need_re(
        implementation,
        r"replacement_allowed_after_collision.*?entries\.len\(\) < RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1.*?!self\.entries\.iter\(\)\.any\(\|entry\| entry\.state == ReconnectAttemptStateV1::Prepared\).*?CollisionTerminal",
        "collision replacement requires capacity, no PREPARED and terminal collision",
    )

    for disposition in (
        "Prepared",
        "ExistingPrepared",
        "RejectedTransportRefCollision",
        "RejectedConcurrentPrepared",
        "RejectedStaleAuthority",
        "AttemptCapacityExceeded",
        "ExistingTerminal",
        "Unavailable",
        "Ambiguous",
        "IdempotencyConflict",
    ):
        need(implementation, disposition, f"typed PREPARE disposition {disposition}")

    need_re(
        implementation,
        r"ReconnectPrepareDispositionV1::Unavailable\s*=>\s*Ok\(ReconnectPrepareActionV1::RetrySameRequest\(self\.prepare_request\.clone\(\)\)\)",
        "PREPARE unavailable retries the same request",
    )
    need_re(
        implementation,
        r"ReconnectPrepareDispositionV1::Ambiguous.*?ReconciliationRequired.*?ReconcileSameAttempt",
        "PREPARE ambiguous reconciles the same attempt",
    )
    need_re(
        implementation,
        r"ReconnectCommitDispositionV1::Unavailable\s*=>\s*Ok\(ReconnectCommitActionV1::RetrySameRequest\(completion\.request\)\)",
        "COMMIT unavailable retries the same request",
    )
    need_re(
        implementation,
        r"ReconnectCommitDispositionV1::Committed \| ReconnectCommitDispositionV1::Ambiguous.*?ReconciliationRequired.*?ReconcileSameAttempt",
        "COMMIT committed/ambiguous requires reconciliation",
    )

    authority_matcher = compact_rust(
        rust_braced_block(
            implementation,
            "fn current_authority_matches_record(",
            "complete current authority matcher",
        )
    )
    need(
        authority_matcher,
        compact_rust(CURRENT_AUTHORITY_EXPRESSION) + "}",
        "complete conjunctive current authority matcher",
    )
    for term in CURRENT_AUTHORITY_TERMS:
        need(
            authority_matcher,
            compact_rust(term),
            "complete current authority matcher",
        )

    evidence_matcher = compact_rust(
        rust_braced_block(
            implementation,
            "fn authenticated_evidence_observed_by(",
            "authenticated evidence observation matcher",
        )
    )
    for comparison in AUTHENTICATED_EVIDENCE_COMPARISONS:
        need(
            evidence_matcher,
            compact_rust(comparison),
            "authenticated evidence observation matcher",
        )
    need(
        evidence_matcher,
        compact_rust(AUTHENTICATED_EVIDENCE_EXPRESSION) + "}",
        "conjunctive authenticated evidence observation matcher",
    )

    authorize_commit_v1 = compact(
        rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn authorize_commit(",
            "ReconnectDurabilityFlowV1 authorize_commit",
        )
    )
    authorize_commit_v2 = compact(
        rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV2 {",
            "pub fn authorize_commit(",
            "ReconnectDurabilityFlowV2 authorize_commit",
        )
    )
    need_ordered(
        authorize_commit_v1,
        AUTHORIZE_COMMIT_ORDERED_INVARIANTS,
        "ReconnectDurabilityFlowV1 authorize_commit",
    )
    need_ordered(
        authorize_commit_v2,
        AUTHORIZE_COMMIT_ORDERED_INVARIANTS,
        "ReconnectDurabilityFlowV2 authorize_commit",
    )
    for label, authorize_commit in (
        ("ReconnectDurabilityFlowV1 authorize_commit", authorize_commit_v1),
        ("ReconnectDurabilityFlowV2 authorize_commit", authorize_commit_v2),
    ):
        normalized = compact_rust(authorize_commit)
        need(normalized, compact_rust(AUTHORIZE_AUTHORITY_GUARD), label)
        need(normalized, compact_rust(AUTHORIZE_DEADLINE_GUARD), label)
        need_exactly_once(
            normalized,
            "ReconnectCommitRequestV1{",
            f"{label} commit request construction",
        )
    need_re(
        implementation,
        r"authorization_deadline.*?prepared_deadline.*?original_grace_deadline.*?platform_deadline.*?trust_deadline.*?credential_expiration",
        "authorization deadline is bounded by grace, prepared, evidence and credential expiry",
    )
    accept_reconciliation_v1 = compact(
        rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn accept_reconciliation(",
            "ReconnectDurabilityFlowV1 accept_reconciliation",
        )
    )
    accept_reconciliation_v2 = compact(
        rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV2 {",
            "pub fn accept_reconciliation(",
            "ReconnectDurabilityFlowV2 accept_reconciliation",
        )
    )
    need_ordered(
        accept_reconciliation_v1,
        RECONCILIATION_V1_ORDERED_INVARIANTS,
        "ReconnectDurabilityFlowV1 accept_reconciliation",
    )
    need_ordered(
        accept_reconciliation_v2,
        RECONCILIATION_V2_ORDERED_INVARIANTS,
        "ReconnectDurabilityFlowV2 accept_reconciliation",
    )
    for label, reconciliation, guard in (
        (
            "ReconnectDurabilityFlowV1 accept_reconciliation",
            accept_reconciliation_v1,
            RECONCILIATION_V1_COMMITTED_GUARD,
        ),
        (
            "ReconnectDurabilityFlowV2 accept_reconciliation",
            accept_reconciliation_v2,
            RECONCILIATION_V2_COMMITTED_GUARD,
        ),
    ):
        normalized = compact_rust(reconciliation)
        need(normalized, compact_rust(guard), label)
        need_exactly_once(
            normalized,
            "InstallController{",
            f"{label} controller installation",
        )

    need(verifier, "pub struct VerifiedRecoveryDurabilityFactsV1", "rich recovery verifier result")
    need_re(
        verifier,
        r"verify_recovery_grant_durability_v1.*?let verified = verify_recovery_grant\(token, now, trust, current\)\?;.*?parse_compact_jws\(token\)",
        "legacy verifier decision happens before signed-field preservation",
    )
    need_re(
        verifier,
        r"claims\.nonce != verified\.grant_nonce\(\).*?claims\.account_id\.as_str\(\) != verified\.account_id\(\).*?character != verified\.character_id\(\).*?world != verified\.world_id\(\).*?binding_mismatch",
        "rich verifier rebinds parsed claims to the legacy verified identity",
    )
    for field in (
        "account_security_generation",
        "protocol_major",
        "transport_profile",
        "ruleset_revision",
        "content_revision",
        "map_revision",
        "world_policy_revision",
        "credential_expiration",
    ):
        need(verifier, field, f"signed recovery field preserved: {field}")
    need(verifier, "it does not invent source revisions or decision identities", "no fabricated source evidence")

    joined = implementation + "\n" + verifier
    forbid_re(
        joined,
        r"\b(?:sqlx|reqwest|hyper|TcpStream|TcpListener|UdpSocket|tokio::net|std::net|std::fs|OpenOptions)\b|File::open",
        "Foundation reconnect boundary must not perform database/network/filesystem I/O",
    )
    forbid_re(joined, r"\basync\s+fn\b|\.await\b", "Foundation logical writer must remain split-phase and non-blocking")

    return [
        "Foundation-only reconnect durability authority and scope",
        "exact non-zero 16-byte transport reference",
        "one-attempt/one-ref idempotency and 8-attempt cap-before-allocation",
        "one-live-PREPARED and collision replacement fencing",
        "typed PREPARE/COMMIT split-phase retry and ambiguity semantics",
        "fresh final authority/security/deadline revalidation before COMMIT",
        "exact durable reconciliation before controller installation",
        "FND-02, proof, continuity and compatibility evidence preservation",
        "legacy FND-04 recovery verification retained before rich signed-field extraction",
        "no fabricated source evidence and no DB/network/filesystem I/O",
    ]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-sha", required=True)
    ap.add_argument("--head-sha", required=True)
    args = ap.parse_args()
    actual = git("rev-parse", "HEAD").lower()
    if actual != args.head_sha.lower():
        fail(f"checkout SHA mismatch: actual={actual} expected={args.head_sha}")

    changed = set(filter(None, git("diff", "--name-only", f"{args.base_sha}...{args.head_sha}").splitlines()))
    selected_profiles = select_profiles(changed)
    profile_checks = {
        "ALPHA_CLIENT_01": alpha,
        "ANL_02_ANL_03": analytics,
        "FOUNDATION_RECONNECT_DURABILITY_V1": foundation_reconnect,
    }
    profiles, failures = run_profiles(selected_profiles, profile_checks)
    verdict = "FAIL" if failures else "PASS" if profiles else "NOT_APPLICABLE"
    profile_names = ",".join(selected_profiles) if selected_profiles else "NOT_APPLICABLE"
    checks = [check for profile in profiles for check in profile["checks"]]

    result = {
        "method": "dedicated deterministic independent semantic audit workflow",
        "profile": profile_names,
        "profiles": profiles,
        "base_sha": args.base_sha,
        "exact_head_sha": args.head_sha,
        "changed_files": sorted(changed),
        "checks": checks,
        "verdict": verdict,
        "ai_service_used": False,
        "owner_funded_ai_used": False,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    print(f"SEMANTIC_AUDIT_{verdict}: profile={profile_names} exact_head={args.head_sha}")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        profile_summary = []
        for profile in profiles:
            profile_summary.append(
                f"- {profile['profile']} verdict: **{profile['verdict']}**"
            )
            profile_summary.extend(
                f"  - PASS: {check}" for check in profile["checks"]
            )
            if "error" in profile:
                profile_summary.append(f"  - error: `{profile['error']}`")
        Path(summary).write_text(
            "## Architecture semantic audit\n\n"
            "- method: dedicated deterministic independent semantic audit workflow\n"
            f"- profiles: `{profile_names}`\n- exact head: `{args.head_sha}`\n- verdict: **{verdict}**\n- owner-funded AI: `false`\n\n"
            + "\n".join(profile_summary)
            + "\n",
            encoding="utf-8",
        )
    if failures:
        raise SystemExit("\n".join(failures))


if __name__ == "__main__":
    main()
