#!/usr/bin/env python3
from __future__ import annotations

import argparse, json, os, re, subprocess
from pathlib import Path

E_PATHS = {
    "docs/agents/tasks/active/OTV2-20260815-alpha-client-architecture.md",
    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md",
    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md",
}
F_PATHS = {
    "docs/agents/tasks/active/OTV2-20260815-analytics-integrity-architecture.md",
    "docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md",
    "docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_CONTRACT_CANDIDATE.md",
    "docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md",
    "docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_CONTRACT_CANDIDATE.md",
}


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
    need(task, "repair_cycles_for_current_gate: 4", "repair history")
    need(task, "no Codex for this continuation", "owner review constraint", ci=True)
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
    task = text("docs/agents/tasks/active/OTV2-20260815-analytics-integrity-architecture.md")
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


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-sha", required=True)
    ap.add_argument("--head-sha", required=True)
    args = ap.parse_args()
    actual = git("rev-parse", "HEAD").lower()
    if actual != args.head_sha.lower():
        fail(f"checkout SHA mismatch: actual={actual} expected={args.head_sha}")

    changed = set(filter(None, git("diff", "--name-only", f"{args.base_sha}...{args.head_sha}").splitlines()))
    if changed == E_PATHS:
        profile, checks, verdict = "ALPHA_CLIENT_01", alpha(), "PASS"
    elif changed == F_PATHS:
        profile, checks, verdict = "ANL_02_ANL_03", analytics(), "PASS"
    else:
        profile, checks, verdict = "NOT_APPLICABLE", [], "NOT_APPLICABLE"

    result = {
        "method": "dedicated deterministic independent semantic audit workflow",
        "profile": profile,
        "base_sha": args.base_sha,
        "exact_head_sha": args.head_sha,
        "changed_files": sorted(changed),
        "checks": checks,
        "verdict": verdict,
        "ai_service_used": False,
        "owner_funded_ai_used": False,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    print(f"SEMANTIC_AUDIT_{verdict}: profile={profile} exact_head={args.head_sha}")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        Path(summary).write_text(
            "## Architecture semantic audit\n\n"
            "- method: dedicated deterministic independent semantic audit workflow\n"
            f"- profile: `{profile}`\n- exact head: `{args.head_sha}`\n- verdict: **{verdict}**\n- owner-funded AI: `false`\n\n"
            + "\n".join(f"- PASS: {x}" for x in checks) + "\n",
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
