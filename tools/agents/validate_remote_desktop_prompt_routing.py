#!/usr/bin/env python3
"""Validate Game prompt binding to the canonical META Remote Desktop gate."""
from __future__ import annotations

import html
import json
from pathlib import Path
import re
import sys
import unicodedata
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[2]
LIFECYCLE_PATH = ROOT / "docs/agents/PROMPT_LIFECYCLE.json"
META_SHA = "e002fc7532188e73a0f495da3e20710541ed50e0"
SECTION = "## Remote Desktop execution routing"

CANONICAL_PROMPT_SECTION = f"""## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. This prompt cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work."""

CANONICAL_SURFACE_SECTIONS = {
    "AGENTS.md": f"""## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve this Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. Game cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.

This provider binding is repository/prompt enforcement only. It MUST NOT be described as connector/router physical enforcement unless the actual Remote Desktop transport has a verified fail-closed hook consuming the same per-action semantics.""",
    "docs/agents/GITHUB_ONLY_EXECUTION.md": f"""## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. Game cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.

This policy does not claim connector/router physical enforcement; such a claim requires a verified fail-closed transport hook.""",
    "docs/agents/PROMPTING_STANDARD.md": f"""## Remote Desktop execution routing

Every reusable prompt must contain exactly one `## Remote Desktop execution routing` section. Before any Remote Desktop/Desktop Commander use, the prompt must resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

The reusable prompt section must state that `list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions; unknown or undeclared tools fail closed; a prior ALLOW never authorizes a different action or tool; Game cannot broaden META exception reasons; and Remote Desktop cannot become a routine fallback for repository tests, Git inspection, CI/log polling or convenience. It must also state that a Remote Desktop DENY is not automatically a blocker and useful authorized work continues through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when possible.

Prompt self-containment does not copy the META machine-readable policy into Game and does not claim connector/router physical enforcement.""",
    "docs/agents/PROMPT_EVAL_STANDARD.md": f"""## Remote Desktop execution routing

A reusable prompt fails evaluation if `list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations or another direct connector call can be treated as ordinary capability discovery. Unknown or undeclared tools must fail closed, a prior ALLOW must not authorize a different action/tool, and Game cannot broaden META exception reasons. Remote Desktop must not become a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: the prompt must continue useful authorized work through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when possible.

The evaluated prompt may restate the routing boundary for self-containment, but must not copy/fork META machine-readable policy or claim connector/router physical enforcement without a verified transport hook.""",
}

CANONICAL_SURFACES = tuple(CANONICAL_SURFACE_SECTIONS)

PROMPT_EVAL_GATES_SECTION = f"""## Gates

- **Authority:** exact writable repositories and protected/live exclusions are explicit.
- **Resolution:** task can be located from repository state without relying on chat.
- **Ownership:** paths/contracts do not overlap ambiguously.
- **Architecture:** accepted ADRs and product boundaries are preserved.
- **Completeness:** observable outcome and all required layers are named.
- **Evidence:** source order, truth labels and exact-head requirements are explicit.
- **Validation:** focused/component/integration/E2E/audit/CI expectations are proportional and executable.
- **Autonomy:** agent continues through lifecycle but has real bounded stop conditions.
- **Handover:** durable checkpoint fields and one next action are required.
- **Safety:** secrets, production, assets, destructive data and cross-repository operations are protected.
- **Codex review routing:** reusable prompts defer to `CODEX_REVIEW_POLICY.json`; covered review never requires per-run owner relay, non-covered owner-funded AI remains exact-owner-authorized, and standing authorization does not create candidate/control-plane/auditor authority.
- **Remote Desktop routing:** every reusable prompt contains exactly one `## Remote Desktop execution routing` section bound to `Oteryn/Oteryn@{META_SHA}`; out-of-band local connector/tool registration and argument-schema inspection is distinct from every direct `Remote_Desktop_Commander.*` invocation, which requires a fresh valid host-exception context and positive per-action authorization for the exact semantic host action and exact connector tool."""

CANONICAL_ROUTING_ADJACENT_SECTIONS = {
    "docs/agents/PROMPT_EVAL_STANDARD.md": {
        "## Gates": PROMPT_EVAL_GATES_SECTION,
    },
}

# Reusable prompts have one canonical Remote Desktop authority block. Any additional
# Remote Desktop policy vocabulary outside that block is rejected so another paragraph
# cannot silently broaden, override, or claim physical enforcement of the contract.
OUTSIDE_ROUTING_PATTERNS = (
    re.compile(r"Remote_Desktop_Commander", re.IGNORECASE),
    re.compile(r"\bRemote(?:\s+|\s*[-\u2010-\u2015]\s*)Desktop\b", re.IGNORECASE),
    re.compile(r"\bDesktop(?:\s+|\s*[-\u2010-\u2015]\s*)Commander\b", re.IGNORECASE),
    re.compile(r"\bRDC\b", re.IGNORECASE),
    re.compile(r"\blist_devices\b", re.IGNORECASE),
    re.compile(r"\bwho_am_i\b", re.IGNORECASE),
    re.compile(r"\bget_config\b", re.IGNORECASE),
    re.compile(
        r"\bdirect\s+(?:connectors?|tools?)\b.{0,160}"
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b.{0,160}"
        r"\bdirect\s+(?:connectors?|tools?)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\bdirect(?:ly)?\b)"
        r"(?=.*\b(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b.{0,160}"
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b.{0,160}"
        r"\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b)"
        r"(?=.*(?:\b(?:(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)|preapproval|(?:approval|permission|authority)\s+by\s+default)\b|\bby\s+default\b.{0,80}\b(?:approval|permission|authority)\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?|actions?)\s+to\s+(?:the\s+)?(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)|preapproval|(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority))\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?|actions?)\b.{0,100}\b(?:through|via)\b.{0,100}\b(?:host\s+)?(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)|preapproval|(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority))\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b.{0,160}"
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)|preapproval|(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority))\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)|preapproval|(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority))\b.{0,160}"
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:(?:direct\s+(?:connectors?|tools?))|"
        r"(?:(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?))|"
        r"(?:(?:filesystem|search|process|session|terminal|history|ping)(?:\s+(?:calls?|operations?|requests?|invocations?|actions?))?))\b)"
        r"(?=.*\b(?:approval|permission|authority)\s+automatically\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\b.{0,40}"
        r"\b(?:has|have)\s+(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\b.{0,80}"
        r"\b(?:is|are|was|were|be|been|being)\s+(?:automatically\s+(?:authori[sz]ed|approved)|(?:authori[sz]ed|approved)\s+automatically|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\b.{0,80}"
        r"\b(?:is|are|was|were|be|been|being)\s+(?:granted|given)\s+"
        r"(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)\b.{0,80}"
        r"\b(?:is|are|was|were|be|been|being)\s+(?:granted|given)\s+to\s+"
        r"(?:filesystem|search|process|session|terminal|history|ping)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\bdirect\s+(?:connectors?|tools?)\b)"
        r"(?=.*(?:\b(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)\b|"
        r"\b(?:approval|permission|authority)\s+by\s+default\b|\bby\s+default\b.{0,160}\b(?:approval|permission|authority)\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\bdirect\s+(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:approval|permission|authority)\b)"
        r"(?=.*(?:\b(?:granted|given)\b.{0,80}\bby\s+default\b|"
        r"\bautomatically\b.{0,80}\b(?:granted|given)\b|"
        r"\b(?:granted|given)\b.{0,80}\bautomatically\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)|"
        r"(?:filesystem|search|process|session|terminal|history|ping)\s+(?:calls?|operations?|requests?|invocations?|actions?))\b)"
        r"(?=.*\b(?:approval|permission|authority)\b)"
        r"(?=.*(?:\b(?:granted|given)\b.{0,80}\bby\s+default\b|"
        r"\bautomatically\b.{0,80}\b(?:granted|given)\b|"
        r"\b(?:granted|given)\b.{0,80}\bautomatically\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\bping\b)"
        r"(?=.*(?:\bautomatically\s+(?:authori[sz]ed|approved)\b|\b(?:authori[sz]ed|approved)\s+automatically\b|\bpreauthori[sz]ed\b|\bpreapproved\b|"
        r"\b(?:blanket|standing|automatic|default)\s+(?:approval|permission|authori[sz]ation|authority)\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?(?:granted|given)\s+by\s+default\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+by\s+default\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?automatically\s+(?:granted|given)\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?(?:granted|given)\s+automatically\b|"
        r"\bautomatically\s+(?:granted|given)\s+(?:approval|permission|authori[sz]ation|authority)\b|"
        r"\bping\b.{0,80}\b(?:is\s+)?(?:granted|given)\s+(?:approval|permission|authori[sz]ation|authority)\s+automatically\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?(?:granted|given)\s+to\s+ping\s+automatically\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?automatic\b|"
        r"\b(?:authori[sz]ed|approved)\s+by\s+default\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\bping\b)"
        r"(?=.*(?:\b(?:requires?|needs?)\s+no\s+(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\b|"
        r"\b(?:does\s+not\s+(?:require|need)|without|exempt(?:ed)?\s+from)\b.{0,80}"
        r"\b(?:per[- ]action|decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\b))",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history)\b\s+"
        r"(?:(?:calls?|operations?|requests?|invocations?|actions?)\s+)?"
        r"(?:"
        r"(?:requires?|needs?)\s+no\s+(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)|"
        r"does\s+not\s+(?:require|need)\s+(?:a\s+)?(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)|"
        r"(?:is\s+)?exempt(?:ed)?\s+from\s+(?:a\s+)?(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)"
        r")\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:connectors?|routers?|transports?|providers?)\b.{0,80}"
        r"\b(?:stop|stops|stopped|stopping|refuse|refuses|refused|refusing|drop|drops|dropped|dropping|discard|discards|discarded|discarding|skip(?:s|ped|ping)?|suppress(?:es|ed|ing)?|ignore(?:s|d|ing)?|cancel(?:s|ed|ing|led|ling)?|intercept(?:s|ed|ing)?|filter(?:s|ed|ing)?(?:\s+out)?)\b.{0,80}"
        r"\b(?:calls?|requests?|invocations?|actions?)\b)"
        r"(?=.*\b(?:per[- ]action|decision|gate|routing|authori[sz]\w*)\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:calls?|requests?|invocations?|actions?)\b.{0,120}"
        r"(?:\b(?:is|are|be|being|been|get|gets|got|getting)\b.{0,30})?"
        r"\b(?:stop|stops|stopped|stopping|refuse|refuses|refused|refusing|drop|drops|dropped|dropping|discard|discards|discarded|discarding|skip(?:s|ped|ping)?|suppress(?:es|ed|ing)?|ignore(?:s|d|ing)?|cancel(?:s|ed|ing|led|ling)?|intercept(?:s|ed|ing)?|filter(?:s|ed|ing)?(?:\s+out)?)\s+(?:by|at|within|inside)\s+(?:the\s+)?"
        r"(?:connectors?|routers?|transports?|providers?)\b)"
        r"(?=.*\b(?:per[- ]action|decision|gate|routing|authori[sz]\w*)\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:connectors?|routers?|transports?|providers?)\b)"
        r"(?=.*\b(?:fail(?:s|ed|ing)?\s+closed|deny|denies|denied|denying)\b)"
        r"(?=.*\b(?:per[- ]action|decision|gate|routing|authori[sz]\w*)\b)",
        re.IGNORECASE,
    ),
    re.compile(r"\bping\b.{0,100}\b(?:capability|discover|connector|tool|host)\b", re.IGNORECASE),
    re.compile(r"\b(?:capability|discover|connector|tool|host)\b.{0,100}\bping\b", re.IGNORECASE),
    re.compile(
        r"(?=.*\b(?:connectors?|routers?|transports?|providers?)\b)"
        r"(?=.*\b(?:guarantee(?:s|d|ing)?|implement(?:s|ed|ing)?)\b)"
        r"(?=.*\b(?:per[- ]action|decision|gate|routing|authori[sz]\w*)\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:connectors?|routers?|transports?|providers?)\b)"
        r"(?=.*\b(?:enforce|enforces|enforced|enforcing|enforcement|block|blocks|blocked|blocking|reject|rejects|rejected|rejecting|prevent|prevents|prevented|preventing)\b)"
        r"(?=.*\b(?:per[- ]action|decision|gate|routing|authori[sz]\w*)\b)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?=.*\b(?:connectors?|routers?|transports?|providers?)\b)"
        r"(?=.*\bphysical(?:ly)?\b)"
        r"(?=.*\benforc\w*\b)",
        re.IGNORECASE,
    ),
    re.compile(r"\b(?:connectors?|routers?|transports?|providers?)\b.{0,100}\bphysical(?:ly)?\b.{0,100}\benforc", re.IGNORECASE),
    re.compile(r"\bphysical(?:ly)?\b.{0,100}\b(?:connectors?|routers?|transports?|providers?)\b.{0,100}\benforc", re.IGNORECASE),
)

META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[^\s`<>:]+:ecosystem/agent-execution-routing-policy\.json",
    re.IGNORECASE,
)
ANGLE_BRACKET_META_ROUTING_COORDINATE_RE = re.compile(
    r"<(Oteryn/Oteryn@[^\s`<>:]+:ecosystem/agent-execution-routing-policy\.json)>",
    re.IGNORECASE,
)
GITHUB_META_ROUTING_URL_RE = re.compile(
    r"(?:(?:https?:)?//)(?:(?:(?!(?:https?:)?//)[^\x00])*@)?(?:(?:www\.)?github\.com\.?(?::[0-9]{1,5})?/Oteryn/Oteryn/(?:blob|tree|raw)|raw\.githubusercontent\.com\.?(?::[0-9]{1,5})?/Oteryn/Oteryn)/([^\s`<>)]+?)/ecosystem/agent-execution-routing-policy\.json",
    re.IGNORECASE,
)
GITHUB_CONTENTS_META_ROUTING_URL_RE = re.compile(
    r"(?:(?:https?:)?//)(?:(?:(?!(?:https?:)?//)[^\x00])*@)?api\.github\.com\.?(?::[0-9]{1,5})?/repos/Oteryn/Oteryn/contents/"
    r"ecosystem/agent-execution-routing-policy\.json(?:\?([^\s`<>)#]*))?",
    re.IGNORECASE,
)
EXPECTED_META_ROUTING_COORDINATE = (
    f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
)

APPROVED_SURFACE_OUTSIDE_ROUTING_PARAGRAPHS = {
    "AGENTS.md": {
        "Before any local/remote repository mutation, including work through Remote Desktop/Desktop Commander, Synology, WSL, Docker or a local worktree, the agent MUST first resolve from GitHub the exact repository, current `main` SHA, governing Issue/task (or explicit `NOT_APPLICABLE` for bounded trivial/read-only work), active PR/task branch, exact base/head SHAs and material overlapping work.",
        "Remote Desktop/Desktop Commander remains exception-only under the organization execution-routing policy and is not the routine fallback for repository work. Tool availability never grants or broadens authorization.",
        "For project work, use GitHub state, GitHub Actions or an approved runner, and an isolated worktree first. Remote Desktop/Desktop Commander is default-deny. A host exception must record one closed reason, the least-privilege semantic host action and the exact requested connector tool; it is never justification for routine builds, tests, Git inspection or polling. When equivalent CI exists, agents MUST NOT use RDC to poll process output, Docker logs, workflow state or Git state.",
    },
    "docs/agents/GITHUB_ONLY_EXECUTION.md": set(),
    "docs/agents/PROMPTING_STANDARD.md": set(),
    "docs/agents/PROMPT_EVAL_STANDARD.md": {
        f"- **Remote Desktop routing:** every reusable prompt contains exactly one `## Remote Desktop execution routing` section bound to `Oteryn/Oteryn@{META_SHA}`; out-of-band local connector/tool registration and argument-schema inspection is distinct from every direct `Remote_Desktop_Commander.*` invocation, which requires a fresh valid host-exception context and positive per-action authorization for the exact semantic host action and exact connector tool."
    },
}

FENCE_OPEN_RE = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})(?P<rest>.*)$")
LEVEL2_RE = re.compile(r"^ {0,3}##(?!#)(?:[ \t]+|$)")
RAW_HTML_CONTAINER_OPEN_RE = re.compile(
    r"^ {0,3}<(?P<tag>pre|script|style|textarea)(?:[ \t>]|$)",
    re.IGNORECASE,
)
COMMONMARK_BLOCK_TAGS = (
    "address", "article", "aside", "base", "basefont", "blockquote", "body",
    "caption", "center", "col", "colgroup", "dd", "details", "dialog", "dir",
    "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form",
    "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
    "hr", "html", "iframe", "legend", "li", "link", "main", "menu", "menuitem",
    "nav", "noframes", "ol", "optgroup", "option", "p", "param", "search",
    "section", "summary", "table", "tbody", "td", "tfoot", "th", "thead", "title",
    "tr", "track", "ul",
)
RAW_HTML_BLOCK_TAG_OPEN_RE = re.compile(
    r"^ {0,3}</?(?:" + "|".join(COMMONMARK_BLOCK_TAGS) + r")(?:[ \t/>]|$)",
    re.IGNORECASE,
)
RAW_HTML_COMPLETE_TAG_RE = re.compile(
    r"^ {0,3}</?[A-Za-z][A-Za-z0-9-]*(?:[ \t]+[^>]*)?/?>[ \t]*$"
)


def _raw_html_block_start(raw: str) -> tuple[str, str] | None:
    stripped = raw.lstrip(" ")
    if len(raw) - len(stripped) > 3:
        return None

    container = RAW_HTML_CONTAINER_OPEN_RE.match(raw)
    if container is not None:
        return "tag", container.group("tag").lower()
    if stripped.startswith("<?"):
        return "delimiter", "?>"
    if stripped.startswith("<![CDATA["):
        return "delimiter", "]] >".replace(" ", "")
    if re.match(r"<![A-Z]", stripped) is not None:
        return "delimiter", ">"
    if RAW_HTML_BLOCK_TAG_OPEN_RE.match(raw) is not None:
        return "blank", ""
    if RAW_HTML_COMPLETE_TAG_RE.fullmatch(raw) is not None:
        return "blank", ""
    return None


def _raw_html_tag_closed(raw: str, tag: str) -> bool:
    return re.search(rf"</{re.escape(tag)}[ \t]*>", raw, re.IGNORECASE) is not None


VISIBLE_RAW_HTML_POLICY_TAGS = {"pre", "textarea"}


def load_lifecycle(errors: list[str]) -> dict:
    try:
        value = json.loads(LIFECYCLE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        errors.append(f"unable to load prompt lifecycle: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append("prompt lifecycle root must be an object")
        return {}
    return value


def reusable_prompt_paths(lifecycle: dict, errors: list[str]) -> list[str]:
    entries = lifecycle.get("prompts")
    if not isinstance(entries, list):
        errors.append("prompt lifecycle prompts must be a list")
        return []
    paths: list[str] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"prompt lifecycle entry {index} must be an object")
            continue
        status = entry.get("status")
        reusable = entry.get("reusable")
        if status == "reusable":
            if reusable is not True:
                errors.append(
                    f"prompt lifecycle entry {index} has inconsistent reusable status/flag"
                )
        elif reusable is True:
            errors.append(
                f"prompt lifecycle entry {index} has inconsistent reusable status/flag"
            )
            continue
        else:
            continue
        path = entry.get("path")
        if not isinstance(path, str) or not path.startswith("docs/agents/prompts/") or not path.endswith(".md"):
            errors.append(f"reusable prompt entry {index} has invalid prompt path")
            continue
        paths.append(path)
    if not paths:
        errors.append("prompt lifecycle contains no reusable prompts")
    if len(paths) != len(set(paths)):
        errors.append("reusable prompt paths must be unique")
    return sorted(set(paths))


def _markdown_line_records(text: str) -> list[tuple[int, int, str, bool]]:
    """Return line spans and whether each line is inert Markdown code/comment/raw-HTML content."""
    records: list[tuple[int, int, str, bool]] = []
    offset = 0
    fence_char: str | None = None
    fence_len = 0
    in_html_comment = False
    raw_html_tag: str | None = None
    raw_html_delimiter: str | None = None
    raw_html_until_blank = False

    for segment in text.splitlines(keepends=True):
        raw = segment.rstrip("\r\n")
        inert = False

        if fence_char is not None:
            inert = True
            close = re.fullmatch(
                rf" {{0,3}}{re.escape(fence_char)}{{{fence_len},}}[ \t]*",
                raw,
            )
            if close is not None:
                fence_char = None
                fence_len = 0
        elif raw_html_tag is not None:
            inert = True
            if _raw_html_tag_closed(raw, raw_html_tag):
                raw_html_tag = None
        elif raw_html_delimiter is not None:
            inert = True
            if raw_html_delimiter in raw:
                raw_html_delimiter = None
        elif raw_html_until_blank:
            inert = True
            if not raw.strip():
                raw_html_until_blank = False
        elif in_html_comment:
            inert = True
            if "-->" in raw:
                in_html_comment = False
        else:
            stripped = raw.lstrip(" ")
            indent = len(raw) - len(stripped)
            if indent <= 3 and stripped.startswith("<!--"):
                inert = True
                if "-->" not in stripped[4:]:
                    in_html_comment = True
            else:
                html_block = _raw_html_block_start(raw)
                if html_block is not None:
                    inert = True
                    kind, value = html_block
                    if kind == "tag" and not _raw_html_tag_closed(raw, value):
                        raw_html_tag = value
                    elif kind == "delimiter" and value not in raw[2:]:
                        raw_html_delimiter = value
                    elif kind == "blank":
                        raw_html_until_blank = True
                else:
                    opening = FENCE_OPEN_RE.fullmatch(raw)
                    if opening is not None and not (
                        opening.group("fence").startswith("`") and "`" in opening.group("rest")
                    ):
                        inert = True
                        fence = opening.group("fence")
                        fence_char = fence[0]
                        fence_len = len(fence)
                    elif raw.rfind("<!--") > raw.rfind("-->"):
                        in_html_comment = True

        end = offset + len(segment)
        records.append((offset, end, raw, inert))
        offset = end

    return records


def _level2_sections(text: str) -> list[tuple[str, int, int, str]]:
    records = _markdown_line_records(text)
    headings: list[tuple[str, int]] = []
    for start, _end, raw, inert in records:
        if inert or LEVEL2_RE.match(raw) is None:
            continue
        headings.append((raw.strip(), start))

    sections: list[tuple[str, int, int, str]] = []
    for index, (heading, start) in enumerate(headings):
        end = headings[index + 1][1] if index + 1 < len(headings) else len(text)
        sections.append((heading, start, end, text[start:end].strip()))
    return sections


def _visible_markdown_line(raw: str, in_html_comment: bool) -> tuple[str, bool]:
    """Remove HTML comments while preserving visible prefixes and suffixes."""
    visible: list[str] = []
    cursor = 0
    while cursor < len(raw):
        if in_html_comment:
            close = raw.find("-->", cursor)
            if close == -1:
                return "".join(visible), True
            cursor = close + 3
            in_html_comment = False
            continue

        opening = raw.find("<!--", cursor)
        if opening == -1:
            visible.append(raw[cursor:])
            break
        visible.append(raw[cursor:opening])
        cursor = opening + 4
        in_html_comment = True
    return "".join(visible), in_html_comment


def _operative_text(text: str) -> str:
    lines: list[str] = []
    fence_char: str | None = None
    fence_len = 0
    in_html_comment = False
    raw_html_tag: str | None = None
    raw_html_delimiter: str | None = None
    raw_html_until_blank = False

    for segment in text.splitlines():
        raw = segment.rstrip("\r\n")

        if fence_char is not None:
            close = re.fullmatch(
                rf" {{0,3}}{re.escape(fence_char)}{{{fence_len},}}[ \t]*",
                raw,
            )
            if close is not None:
                fence_char = None
                fence_len = 0
            lines.append("")
            continue

        if raw_html_tag is not None:
            active_raw_html_tag = raw_html_tag
            if _raw_html_tag_closed(raw, active_raw_html_tag):
                raw_html_tag = None
            if active_raw_html_tag in VISIBLE_RAW_HTML_POLICY_TAGS:
                visible, in_html_comment = _visible_markdown_line(raw, in_html_comment)
                lines.append(visible)
            else:
                lines.append("")
            continue

        if raw_html_delimiter is not None:
            if raw_html_delimiter in raw:
                raw_html_delimiter = None
            lines.append("")
            continue

        if raw_html_until_blank:
            if not raw.strip():
                raw_html_until_blank = False
                lines.append("")
            else:
                visible, in_html_comment = _visible_markdown_line(raw, in_html_comment)
                lines.append(visible)
            continue

        if not in_html_comment:
            html_block = _raw_html_block_start(raw)
            if html_block is not None:
                kind, value = html_block
                if kind == "tag":
                    if not _raw_html_tag_closed(raw, value):
                        raw_html_tag = value
                    if value in VISIBLE_RAW_HTML_POLICY_TAGS:
                        visible, in_html_comment = _visible_markdown_line(raw, in_html_comment)
                        lines.append(visible)
                    else:
                        lines.append("")
                elif kind == "delimiter" and value not in raw[2:]:
                    raw_html_delimiter = value
                    lines.append("")
                elif kind == "blank":
                    raw_html_until_blank = True
                    visible, in_html_comment = _visible_markdown_line(raw, in_html_comment)
                    lines.append(visible)
                else:
                    lines.append("")
                continue

            opening = FENCE_OPEN_RE.fullmatch(raw)
            if opening is not None and not (
                opening.group("fence").startswith("`") and "`" in opening.group("rest")
            ):
                fence = opening.group("fence")
                fence_char = fence[0]
                fence_len = len(fence)
                lines.append("")
                continue

        visible, in_html_comment = _visible_markdown_line(raw, in_html_comment)
        lines.append(visible)

    operative = "\n".join(lines)
    for canonical_example in (CANONICAL_PROMPT_SECTION, *CANONICAL_SURFACE_SECTIONS.values()):
        operative = operative.replace(canonical_example, "")
    return operative


def _extract_canonical_section(path: str, text: str, errors: list[str]) -> tuple[str, str] | None:
    matches = [section for section in _level2_sections(text) if section[0] == SECTION]
    if len(matches) != 1:
        errors.append(f"{path}: must contain exactly one operative {SECTION!r} section")
        return None

    _heading, start, end, section_text = matches[0]
    outside_text = (text[:start] + "\n" + text[end:]).strip()
    return section_text, outside_text


MARKDOWN_LINK_RE = re.compile(r"!?\[([^]\n]*)\]\([^\n)]*\)")
MARKDOWN_REFERENCE_LINK_RE = re.compile(r"!?\[([^]\n]*)\]\[[^]\n]*\]")
HTML_COMMENT_INLINE_RE = re.compile(r"<!--.*?-->", re.DOTALL)
HTML_TAG_RE = re.compile(
    r"</?[A-Za-z][A-Za-z0-9-]*(?:\s+[A-Za-z_:][A-Za-z0-9_.:-]*"
    r"(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s\"'=<>`]+))?)*\s*/?>"
)
MARKDOWN_ESCAPE_RE = re.compile(r"\\([\\`*_{}\[\]()#+.!~>-])")
MARKDOWN_UNDERSCORE_EMPHASIS_RE = re.compile(r"(?<!\w)_{1,3}(?=\w)|(?<=\w)_{1,3}(?!\w)")
DEFAULT_IGNORABLE_RANGES = (
    (0x00AD, 0x00AD),
    (0x034F, 0x034F),
    (0x061C, 0x061C),
    (0x115F, 0x1160),
    (0x17B4, 0x17B5),
    (0x180B, 0x180F),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x206F),
    (0x3164, 0x3164),
    (0xFE00, 0xFE0F),
    (0xFEFF, 0xFEFF),
    (0xFFA0, 0xFFA0),
    (0xFFF0, 0xFFF8),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE0FFF),
)


def _remove_default_ignorables(value: str) -> str:
    return "".join(
        character
        for character in value
        if not any(start <= ord(character) <= end for start, end in DEFAULT_IGNORABLE_RANGES)
    )


def _normalize_policy_text(text: str) -> str:
    """Approximate rendered Markdown text for fail-closed policy detection."""
    value = text
    for _ in range(4):
        decoded = html.unescape(value)
        if decoded == value:
            break
        value = decoded
    value = unicodedata.normalize("NFKC", value)
    value = MARKDOWN_LINK_RE.sub(lambda match: match.group(1), value)
    value = MARKDOWN_REFERENCE_LINK_RE.sub(lambda match: match.group(1), value)
    value = HTML_COMMENT_INLINE_RE.sub(" ", value)
    value = ANGLE_BRACKET_META_ROUTING_COORDINATE_RE.sub(r"\1", value)
    value = HTML_TAG_RE.sub(" ", value)
    value = MARKDOWN_ESCAPE_RE.sub(r"\1", value)
    value = _remove_default_ignorables(value)
    value = re.sub(r"[*~`]+", "", value)
    value = MARKDOWN_UNDERSCORE_EMPHASIS_RE.sub("", value)
    value = re.sub(r"[\s\u00a0]+", " ", value)
    return value


def _outside_routing_paragraphs(text: str) -> list[str]:
    operative = _operative_text(text)
    paragraphs = [paragraph.strip() for paragraph in re.split(r"\n\s*\n", operative) if paragraph.strip()]
    units: list[str] = []
    for paragraph in paragraphs:
        lines = [line.strip() for line in paragraph.splitlines() if line.strip()]
        if len(lines) > 1 and all(line.startswith("- ") for line in lines):
            units.extend(
                line
                for line in lines
                if any(pattern.search(_normalize_policy_text(line)) is not None for pattern in OUTSIDE_ROUTING_PATTERNS)
            )
            continue
        if any(pattern.search(_normalize_policy_text(paragraph)) is not None for pattern in OUTSIDE_ROUTING_PATTERNS):
            units.append(paragraph)
    return units


def _validate_outside_routing_text(
    path: str,
    outside_text: str,
    approved: set[str],
    errors: list[str],
) -> None:
    for paragraph in _outside_routing_paragraphs(outside_text):
        if paragraph not in approved:
            snippet = paragraph.replace("\n", " ")[:160]
            errors.append(
                f"{path}: Remote Desktop policy text outside canonical section: {snippet!r}"
            )


def _validate_routing_adjacent_sections(path: str, text: str, errors: list[str]) -> None:
    expected_sections = CANONICAL_ROUTING_ADJACENT_SECTIONS.get(path, {})
    if not expected_sections:
        return

    sections = _level2_sections(text)
    for heading, expected in expected_sections.items():
        matches = [section_text for section_heading, _start, _end, section_text in sections if section_heading == heading]
        if len(matches) != 1 or matches[0] != expected:
            errors.append(
                f"{path}: routing list section {heading!r} must match exactly"
            )


def _normalize_url_scan_text(text: str) -> str:
    value = _operative_text(text)
    for _ in range(4):
        decoded = html.unescape(value)
        if decoded == value:
            break
        value = decoded
    value = unicodedata.normalize("NFKC", value)
    for _ in range(4):
        decoded = unquote(value)
        if decoded == value:
            break
        value = decoded
    return _remove_default_ignorables(value)


def _validate_meta_routing_coordinates(path: str, text: str, errors: list[str]) -> None:
    normalized = _normalize_policy_text(_operative_text(text))
    coordinates = META_ROUTING_COORDINATE_RE.findall(normalized)
    stale = sorted(
        {coordinate for coordinate in coordinates if coordinate != EXPECTED_META_ROUTING_COORDINATE}
    )
    for coordinate in stale:
        errors.append(f"{path}: stale META execution-routing coordinate: {coordinate}")

    url_selectors = GITHUB_META_ROUTING_URL_RE.findall(_normalize_url_scan_text(text))
    for selector in sorted({selector for selector in url_selectors if selector != META_SHA}):
        errors.append(
            f"{path}: stale META execution-routing coordinate: GitHub routing-policy URL selector {selector}"
        )

    contents_queries = GITHUB_CONTENTS_META_ROUTING_URL_RE.findall(_normalize_url_scan_text(text))
    stale_contents_selectors: set[str] = set()
    for query in contents_queries:
        ref_match = re.search(r"(?:^|&)ref=([^&]+)", query, re.IGNORECASE)
        selector = ref_match.group(1) if ref_match is not None else ""
        if selector != META_SHA:
            stale_contents_selectors.add(selector or "<default-branch>")
    for selector in sorted(stale_contents_selectors):
        errors.append(
            f"{path}: stale META execution-routing coordinate: GitHub Contents routing-policy URL selector {selector}"
        )


def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:
    _validate_meta_routing_coordinates(path, text, errors)
    extracted = _extract_canonical_section(path, text, errors)
    if extracted is None:
        return
    section_text, outside_text = extracted

    if section_text != CANONICAL_PROMPT_SECTION:
        errors.append(f"{path}: canonical Remote Desktop routing section must match exactly")

    _validate_outside_routing_text(path, outside_text, set(), errors)


def validate_surface_text(path: str, text: str, errors: list[str]) -> None:
    _validate_meta_routing_coordinates(path, text, errors)
    extracted = _extract_canonical_section(path, text, errors)
    if extracted is None:
        return
    section_text, outside_text = extracted

    expected = CANONICAL_SURFACE_SECTIONS.get(path)
    if expected is None:
        errors.append(f"{path}: canonical surface is not registered")
        return
    if section_text != expected:
        errors.append(f"{path}: canonical Remote Desktop routing section must match exactly")

    approved = APPROVED_SURFACE_OUTSIDE_ROUTING_PARAGRAPHS.get(path, set())
    _validate_outside_routing_text(path, outside_text, approved, errors)
    _validate_routing_adjacent_sections(path, text, errors)


def validate() -> list[str]:
    errors: list[str] = []
    lifecycle = load_lifecycle(errors)
    prompt_paths = reusable_prompt_paths(lifecycle, errors)

    for relative in prompt_paths:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read reusable prompt: {exc}")
            continue
        validate_reusable_prompt_text(relative, text, errors)

    for relative in CANONICAL_SURFACES:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read canonical surface: {exc}")
            continue
        validate_surface_text(relative, text, errors)

    root_agents = ROOT / "AGENTS.md"
    if root_agents.is_file():
        root_text = root_agents.read_text(encoding="utf-8")
        coordinates = META_ROUTING_COORDINATE_RE.findall(_normalize_policy_text(_operative_text(root_text)))
        expected = f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
        if expected not in root_text:
            errors.append("AGENTS.md: canonical META execution-routing coordinate is missing")
        stale = sorted({coordinate for coordinate in coordinates if coordinate != expected})
        for coordinate in stale:
            errors.append(f"AGENTS.md: stale META execution-routing coordinate: {coordinate}")

    if not errors:
        print(
            f"Validated Remote Desktop per-action routing for {len(prompt_paths)} reusable prompts "
            f"and {len(CANONICAL_SURFACES)} canonical surfaces against META {META_SHA}."
        )
    return errors


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(f"Remote Desktop prompt routing validation failed with {len(errors)} error(s).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())