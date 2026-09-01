#!/usr/bin/env python3
"""Validate Game prompt binding to the canonical META Remote Desktop gate."""
from __future__ import annotations

import html
import json
from pathlib import Path
import re
import sys
import unicodedata
from urllib.parse import parse_qs, unquote, urlsplit

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
META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[^\s`<>:]+:ecosystem/agent-execution-routing-policy\.json",
    re.IGNORECASE,
)
ANGLE_BRACKET_META_ROUTING_COORDINATE_RE = re.compile(
    r"<(Oteryn/Oteryn@[^\s`<>:]+:ecosystem/agent-execution-routing-policy\.json)>",
    re.IGNORECASE,
)
EXPECTED_META_ROUTING_COORDINATE = (
    f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
)

# Bounded lexical recognizers feed semantic policy classes. They intentionally
# recognize concepts and relationships rather than complete reviewed sentences.
REMOTE_POLICY_MARKER_RE = re.compile(
    r"Remote_Desktop_Commander"
    r"|\bRemote(?:\s+|\s*[-\u2010-\u2015]\s*)Desktop\b"
    r"|\bDesktop(?:\s+|\s*[-\u2010-\u2015]\s*)Commander\b"
    r"|\bRDC\b|\blist_devices\b|\bwho_am_i\b|\bget_config\b",
    re.IGNORECASE,
)
PROTECTED_HOST_ACTION_RE = re.compile(
    r"\b(?:filesystem|search|process|session|terminal|history|ping)\b",
    re.IGNORECASE,
)
DIRECT_CONNECTOR_RE = re.compile(
    r"(?:\bdirect(?:ly)?\b.{0,64}\b(?:connectors?|tools?)\b)"
    r"|(?:\b(?:connectors?|tools?)\b.{0,64}\bdirect(?:ly)?\b)",
    re.IGNORECASE,
)
CONNECTOR_ACTION_RE = re.compile(
    r"\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b"
    r"|\b(?:calls?|operations?|requests?|invocations?|actions?)\s+(?:to|through|via)\s+(?:the\s+)?(?:connectors?|tools?)\b",
    re.IGNORECASE,
)
CAPABILITY_CONCEPT_RE = re.compile(
    r"\b(?:capabilit(?:y|ies)|discover(?:y|able|ed|ing)?|probe[sd]?|probing|metadata|read[- ]?only|inspection)\b",
    re.IGNORECASE,
)
DURABLE_AUTHORITY_RE = re.compile(
    r"\b(?:blanket|standing|automatic(?:ally)?|default|durable|reusable|persistent|ongoing|continuing|permanent|indefinite|perpetual|always)\b"
    r"|\bby\s+default\b|\bpre[- ]?(?:approved?|authorized?|authorised?|approval|authorization|authorisation)\b",
    re.IGNORECASE,
)
AUTHORITY_CONCEPT_RE = re.compile(
    r"\b(?:authority|authorization|authorisation|authorized|authorised|approval|approved|permission|permitted|permit|allowed|allowance|exception|entitlement)\b"
    r"|\bpre[- ]?(?:approv(?:e[sd]?|al)|authori[sz](?:e[sd]?|ation))\b",
    re.IGNORECASE,
)
DECISION_CONCEPT_RE = re.compile(
    r"\b(?:decision|authorization|authorisation|approval|permission|exception|check|gate)\b",
    re.IGNORECASE,
)
PROVIDER_NOUN_RE = re.compile(
    r"\b(?:connectors?|routers?|providers?|transports?)\b",
    re.IGNORECASE,
)
GATE_CONTEXT_RE = re.compile(
    r"\b(?:per[- ]?action|fresh|authorization|authorisation|approval|permission|decision|host[- ]?exception|gate|routing|unauthori[sz]ed|missing|lacking|physical(?:ly)?)\b",
    re.IGNORECASE,
)
ENFORCEMENT_CONCEPT_RE = re.compile(
    r"\b(?:enforc(?:e[sd]?|ing|ement)|block(?:s|ed|ing)?|reject(?:s|ed|ing|ion)?|den(?:y|ies|ied|ying|ial)|stop(?:s|ped|ping)?|refus(?:e[sd]?|ing|al)|drop(?:s|ped|ping)?|discard(?:s|ed|ing)?|filter(?:s|ed|ing)?|suppress(?:es|ed|ing|ion)?|ignor(?:e[sd]?|ing)|skip(?:s|ped|ping)?|cancel(?:s|ed|led|ing|ling|ation)?|intercept(?:s|ed|ing|ion)?|quarantine(?:s|d|ing)?|declin(?:e[sd]?|ing)|prevent(?:s|ed|ing|ion)?|guarantee(?:s|d|ing)?|implement(?:s|ed|ing|ation)?|fail(?:s|ed|ing)?[- ]closed)\b",
    re.IGNORECASE,
)
GITHUB_URL_CANDIDATE_RE = re.compile(
    r"(?:https?:)?//[^\s`<>\[\]()]+"
    r"|(?<![A-Za-z0-9_.-])(?:www\.)?(?:github\.com\.?|raw\.githubusercontent\.com\.?|raw\.github\.com\.?|api\.github\.com\.?)(?::\d{1,5})?/[^\s`<>\[\]()]+",
    re.IGNORECASE,
)
ROUTING_POLICY_TAIL = ("ecosystem", "agent-execution-routing-policy.json")

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


def _semantic_clauses(text: str) -> list[str]:
    return [
        part.strip()
        for part in re.split(r"(?:[;\n]+|(?<=[.!?])\s+)", text)
        if part.strip()
    ]


def _has_protected_action(text: str) -> bool:
    return bool(
        PROTECTED_HOST_ACTION_RE.search(text)
        or DIRECT_CONNECTOR_RE.search(text)
        or CONNECTOR_ACTION_RE.search(text)
    )


PROTECTED_SUBJECT_TEXT_PATTERN = (
    r"(?:\b(?:filesystem|search|process|session|terminal|history|ping)\b"
    r"|\bdirect(?:ly)?\s+(?:connectors?|tools?)(?:\s+(?:calls?|operations?|requests?|invocations?|actions?))?\b"
    r"|\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?|actions?)\b"
    r"|\b(?:calls?|operations?|requests?|invocations?|actions?)\s+(?:to|through|via)\s+(?:the\s+)?(?:connectors?|tools?)\b)"
)
PROTECTED_SUBJECT_RE = re.compile(PROTECTED_SUBJECT_TEXT_PATTERN, re.IGNORECASE)
AUTHORITY_NOUN_PATTERN = r"(?:authority|authorization|authorisation|approval|permission|entitlement)"
DURABLE_AUTHORITY_PHRASE_RE = re.compile(
    rf"\b(?:blanket|standing|automatic|default|durable|reusable|persistent|ongoing|continuing|permanent|indefinite|perpetual)\s+{AUTHORITY_NOUN_PATTERN}\b"
    rf"|\b{AUTHORITY_NOUN_PATTERN}\b(?:\s+(?:is|are|was|were|be|been|being))?\s+(?:automatic(?:ally)?|by\s+default|always)\b"
    rf"|\b{AUTHORITY_NOUN_PATTERN}\b.{{0,24}}\b(?:granted|given|retained|attached)\b.{{0,32}}\b(?:automatic(?:ally)?|by\s+default|always)\b"
    rf"|\b{AUTHORITY_NOUN_PATTERN}\b.{{0,24}}\b(?:automatic(?:ally)?|by\s+default|always)\b.{{0,24}}\b(?:granted|given|retained)\b"
    rf"|\b(?:automatic(?:ally)?|by\s+default|always)\b.{{0,24}}\b(?:granted|given|retained)\b.{{0,24}}\b{AUTHORITY_NOUN_PATTERN}\b"
    rf"|\b(?:automatically|always)\s+(?:authorized|authorised|approved|permitted|allowed)\b"
    rf"|\b(?:authorized|authorised|approved|permitted|allowed)\s+(?:automatically|by\s+default|always)\b"
    rf"|\bpre[- ]?(?:approved|authorized|authorised)\b",
    re.IGNORECASE,
)
DEFAULT_AUTHORITY_RELATION_RE = re.compile(
    rf"\bby\s+default\b.{{0,72}}\b(?:remains?|is|are|has|have|retains?|carries?|keeps?)\b.{{0,20}}\b(?:permitted|allowed|authorized|authorised|approved|{AUTHORITY_NOUN_PATTERN})\b",
    re.IGNORECASE,
)
DECISION_REQUIREMENT_RE = re.compile(
    r"\b(?:fresh(?:\s+exact)?(?:\s+per[- ]?action)?\s+(?:decision|authorization|authorisation|approval|permission|exception|check)"
    r"|per[- ]?action\s+(?:decision|authorization|authorisation|approval|permission|exception|check)"
    r"|host[- ]?exception|authorization|authorisation|approval|permission|exception)\b",
    re.IGNORECASE,
)
EXEMPTION_RELATION_RE = re.compile(
    r"\b(?:without|absent)\b"
    r"|\b(?:requires?|needs?)\s+no\b"
    r"|\b(?:does|do)\s+not\s+(?:require|need)\b"
    r"|\b(?:exempt(?:ed|ion)?\s+from|free\s+(?:from|of))\b"
    r"|\bno\b.{0,56}\b(?:is|are|was|were)?\s*(?:required|needed|necessary)\b"
    r"|\b(?:not\s+required|not\s+needed|unnecessary|optional|waived|bypassed)\b"
    r"|\b(?:waive[sd]?|waiving|bypass(?:es|ed|ing)?)\b"
    r"|\bwith\s+no\b",
    re.IGNORECASE,
)
CAPABILITY_RELATION_RE = re.compile(
    rf"\b(?:treat(?:s|ed|ing)?|consider(?:s|ed|ing)?|regard(?:s|ed|ing)?|classif(?:y|ies|ied|ying)|count(?:s|ed|ing)?)\b.{{0,48}}{PROTECTED_SUBJECT_TEXT_PATTERN}.{{0,28}}\b(?:as\s+)?(?:ordinary\s+)?(?:capabilit(?:y|ies)(?:\s+discovery)?|discovery|probe|probing|metadata|read[- ]?only(?:\s+inspection)?|inspection)\b"
    rf"|{PROTECTED_SUBJECT_TEXT_PATTERN}.{{0,36}}\b(?:is|are|counts?|qualifies?|serves?)\b.{{0,24}}\b(?:as\s+)?(?:ordinary\s+)?(?:capabilit(?:y|ies)(?:\s+discovery)?|discovery|probe|probing|metadata|read[- ]?only(?:\s+inspection)?|inspection)\b"
    rf"|\b(?:capabilit(?:y|ies)(?:\s+discovery)?|discovery|probe|probing|metadata|read[- ]?only(?:\s+inspection)?|inspection)\b.{{0,40}}\b(?:includes?|covers?|applies?\s+to)\b.{{0,48}}{PROTECTED_SUBJECT_TEXT_PATTERN}",
    re.IGNORECASE,
)


def _protected_subject_spans(text: str) -> list[tuple[int, int]]:
    return [match.span() for match in PROTECTED_SUBJECT_RE.finditer(text)]


def _span_gap(left: tuple[int, int], right: tuple[int, int]) -> int:
    if left[1] < right[0]:
        return right[0] - left[1]
    if right[1] < left[0]:
        return left[0] - right[1]
    return 0


def _any_spans_related(
    left: list[tuple[int, int]],
    right: list[tuple[int, int]],
    max_gap: int,
) -> bool:
    return any(_span_gap(a, b) <= max_gap for a in left for b in right)


def _authority_is_explicitly_denied(text: str) -> bool:
    if re.search(
        r"\bno\b.{0,56}\b(?:authority|authorization|authorisation|approval|permission|exception|entitlement)\b",
        text,
        re.IGNORECASE,
    ):
        return True
    if re.search(
        r"\b(?:not|never)\b.{0,36}\b(?:authorized|authorised|approved|permitted|allowed|granted|given|retained|automatic(?:ally)?|standing|default)\b",
        text,
        re.IGNORECASE,
    ):
        return True
    return bool(
        re.search(
            r"\b(?:authority|authorization|authorisation|approval|permission|exception|entitlement)\b.{0,48}\b(?:is|are|was|were|be|been|being|remains?)\s+not\b",
            text,
            re.IGNORECASE,
        )
    )


def _is_blanket_authority_claim(text: str) -> bool:
    subjects = _protected_subject_spans(text)
    if not subjects or _authority_is_explicitly_denied(text):
        return False
    authority_spans = [match.span() for match in DURABLE_AUTHORITY_PHRASE_RE.finditer(text)]
    if _any_spans_related(subjects, authority_spans, 72):
        return True
    for subject in subjects:
        window = text[max(0, subject[0] - 72):min(len(text), subject[1] + 72)]
        if DEFAULT_AUTHORITY_RELATION_RE.search(window):
            return True
    return False


def _is_restrictive_without_requirement(text: str) -> bool:
    restrictive = re.compile(
        r"\b(?:no|never|cannot|can't|do\s+not|does\s+not|must\s+not|may\s+not|shall\s+not|will\s+not|is\s+not\s+allowed|are\s+not\s+allowed|is\s+not\s+permitted|are\s+not\s+permitted|prohibited|forbidden)\b",
        re.IGNORECASE,
    )
    for match in re.finditer(r"\bwithout\b", text, re.IGNORECASE):
        full_prefix = text[:match.start()]
        suffix = text[match.end():min(len(text), match.end() + 112)]
        stripped_prefix = re.sub(r"^\s*(?:[-*+]\s+)?", "", full_prefix)
        if re.match(r"^(?:no|never)\b", stripped_prefix, re.IGNORECASE):
            prefix_subjects = _protected_subject_spans(full_prefix)
            if prefix_subjects:
                first_subject_start = min(span[0] for span in prefix_subjects)
                if not DECISION_REQUIREMENT_RE.search(full_prefix[:first_subject_start]):
                    return True
        local_prefix = full_prefix[max(0, len(full_prefix) - 112):]
        if restrictive.search(local_prefix) or restrictive.search(suffix):
            return True
    return False


def _is_fresh_decision_exemption(text: str) -> bool:
    subjects = _protected_subject_spans(text)
    decisions = [match.span() for match in DECISION_REQUIREMENT_RE.finditer(text)]
    relations = [match.span() for match in EXEMPTION_RELATION_RE.finditer(text)]
    if not subjects or not decisions or not relations:
        return False
    if _is_restrictive_without_requirement(text):
        return False
    for subject in subjects:
        for decision in decisions:
            if _span_gap(subject, decision) > 112:
                continue
            if any(
                _span_gap(relation, subject) <= 112
                and _span_gap(relation, decision) <= 72
                for relation in relations
            ):
                return True
    return False


def _is_capability_exemption_claim(text: str) -> bool:
    return bool(CAPABILITY_RELATION_RE.search(text))


def _enforcement_match_is_negated(text: str, start: int, end: int) -> bool:
    prefix = text[max(0, start - 56):start]
    suffix = text[end:min(len(text), end + 48)]
    if re.search(
        r"\b(?:no|not|never)\b(?:\s+[A-Za-z_-]+){0,4}\s*$"
        r"|\b(?:doesn't|don't|isn't|aren't|wasn't|weren't|won't|can't|cannot)\b(?:\s+[A-Za-z_-]+){0,4}\s*$",
        prefix,
        re.IGNORECASE,
    ):
        return True
    return bool(
        re.match(
            r"\s+(?:is|are|was|were|will|would|can|could|must|may|should)\s+not\b",
            suffix,
            re.IGNORECASE,
        )
    )


ENFORCEMENT_TARGET_RE = re.compile(
    r"\b(?:calls?|requests?|invocations?|actions?|operations?)\b",
    re.IGNORECASE,
)
STRONG_GATE_CONTEXT_RE = re.compile(
    r"\b(?:per[- ]?action|host[- ]?exception|authorization|authorisation|approval|permission|decision|gate|routing|unauthori[sz]ed|missing|lacking)\b",
    re.IGNORECASE,
)
PHYSICAL_ENFORCEMENT_RE = re.compile(
    r"\bphysical(?:ly)?\b",
    re.IGNORECASE,
)
PROVIDER_LOCATION_RELATION_RE = re.compile(
    r"\b(?:by|at|within|inside)\b",
    re.IGNORECASE,
)


def _is_provider_enforcement_claim(text: str) -> bool:
    providers = list(PROVIDER_NOUN_RE.finditer(text))
    enforcements = [
        match
        for match in ENFORCEMENT_CONCEPT_RE.finditer(text)
        if not _enforcement_match_is_negated(text, match.start(), match.end())
    ]
    if not providers or not enforcements:
        return False

    for provider in providers:
        provider_span = provider.span()
        for enforcement in enforcements:
            enforcement_span = enforcement.span()
            if _span_gap(provider_span, enforcement_span) > 120:
                continue
            around_start = max(0, min(provider.start(), enforcement.start()) - 104)
            around_end = min(len(text), max(provider.end(), enforcement.end()) + 136)
            around = text[around_start:around_end]
            has_gate = bool(STRONG_GATE_CONTEXT_RE.search(around))
            has_physical = bool(PHYSICAL_ENFORCEMENT_RE.search(around))
            if not (has_gate or has_physical):
                continue

            if provider.start() <= enforcement.start():
                between = text[provider.end():enforcement.start()]
                if len(between) > 72 or re.search(r"[.;,:]", between):
                    continue
                if has_physical or has_gate:
                    return True
                continue

            between = text[enforcement.end():provider.start()]
            if re.search(r"[.;]", between):
                continue
            if not PROVIDER_LOCATION_RELATION_RE.search(between):
                continue
            target_window_start = max(0, enforcement.start() - 96)
            target_window_end = min(len(text), provider.end() + 40)
            target_window = text[target_window_start:target_window_end]
            if has_physical or (
                has_gate and ENFORCEMENT_TARGET_RE.search(target_window)
            ):
                return True
    return False


def _is_outside_routing_policy(text: str) -> bool:
    if REMOTE_POLICY_MARKER_RE.search(text):
        return True
    for clause in _semantic_clauses(text):
        if _is_capability_exemption_claim(clause):
            return True
        if _is_blanket_authority_claim(clause):
            return True
        if _is_fresh_decision_exemption(clause):
            return True
        if _is_provider_enforcement_claim(clause):
            return True
    return False


def _outside_routing_paragraphs(text: str) -> list[str]:
    operative = _operative_text(text)
    paragraphs = [
        paragraph.strip()
        for paragraph in re.split(r"\n\s*\n", operative)
        if paragraph.strip()
    ]
    units: list[str] = []
    for paragraph in paragraphs:
        lines = [line.strip() for line in paragraph.splitlines() if line.strip()]
        if len(lines) > 1 and all(line.startswith("- ") for line in lines):
            units.extend(
                line
                for line in lines
                if _is_outside_routing_policy(_normalize_policy_text(line))
            )
            continue
        if _is_outside_routing_policy(_normalize_policy_text(paragraph)):
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
    return _remove_default_ignorables(value).replace("\\", "/")


def _decode_url_component(value: str) -> str:
    decoded = value
    for _ in range(4):
        next_value = unquote(decoded)
        if next_value == decoded:
            break
        decoded = next_value
    return _remove_default_ignorables(unicodedata.normalize("NFKC", decoded))


def _normalize_github_host(value: str) -> str:
    host = _decode_url_component(value).lower().rstrip(".")
    aliases = {
        "www.github.com": "github.com",
        "raw.github.com": "raw.githubusercontent.com",
        "www.raw.githubusercontent.com": "raw.githubusercontent.com",
    }
    return aliases.get(host, host)


def _normalize_url_path(value: str) -> list[str]:
    decoded = _decode_url_component(value).replace("\\", "/")
    segments: list[str] = []
    for raw_segment in decoded.split("/"):
        segment = _decode_url_component(raw_segment)
        if not segment or segment == ".":
            continue
        if segment == "..":
            if segments:
                segments.pop()
            continue
        segments.append(segment)
    return segments


def _github_url_candidates(text: str) -> list[str]:
    normalized = _normalize_url_scan_text(text)
    candidates: list[str] = []
    for match in GITHUB_URL_CANDIDATE_RE.finditer(normalized):
        candidate = match.group(0).rstrip(".,;:!?\"'}>")
        if candidate:
            candidates.append(candidate)
    return candidates


def _github_routing_reference(candidate: str) -> tuple[str, str] | None:
    parse_target = candidate
    if not re.match(r"^[A-Za-z][A-Za-z0-9+.-]*://", parse_target):
        if not parse_target.startswith("//"):
            parse_target = "//" + parse_target
    try:
        parts = urlsplit(parse_target)
        host = _normalize_github_host(parts.hostname or "")
    except (TypeError, ValueError):
        return None
    segments = _normalize_url_path(parts.path)
    lowered = [segment.lower() for segment in segments]
    tail = [part.lower() for part in ROUTING_POLICY_TAIL]

    if host == "github.com":
        if len(segments) >= 5 and lowered[:2] == ["oteryn", "oteryn"] and lowered[2] in {"blob", "tree", "raw"} and lowered[-2:] == tail:
            return ("GitHub routing-policy URL", "/".join(segments[3:-2]))
        return None

    if host == "raw.githubusercontent.com":
        if len(segments) >= 4 and lowered[:2] == ["oteryn", "oteryn"] and lowered[-2:] == tail:
            return ("GitHub raw routing-policy URL", "/".join(segments[2:-2]))
        return None

    if host == "api.github.com":
        expected = ["repos", "oteryn", "oteryn", "contents", *tail]
        if lowered == expected:
            query_text = _decode_url_component(parts.query)
            query = parse_qs(query_text, keep_blank_values=True)
            refs = [
                _decode_url_component(value)
                for key, values in query.items()
                if _decode_url_component(key).lower() == "ref"
                for value in values
            ]
            selector = refs[0] if len(refs) == 1 else ""
            return ("GitHub Contents routing-policy URL", selector)
        return None

    return None


def _validate_meta_routing_coordinates(path: Path, text: str, errors: list[str]) -> None:
    operative = _normalize_policy_text(_operative_text(text))
    for coordinate in META_ROUTING_COORDINATE_RE.findall(operative):
        if coordinate != EXPECTED_META_ROUTING_COORDINATE:
            errors.append(
                f"{path}: stale META execution-routing coordinate {coordinate!r}; expected {EXPECTED_META_ROUTING_COORDINATE!r}"
            )

    for candidate in _github_url_candidates(text):
        reference = _github_routing_reference(candidate)
        if reference is None:
            continue
        label, selector = reference
        if selector != META_SHA:
            errors.append(
                f"{path}: stale META execution-routing coordinate in {label} {candidate!r}; expected selector {META_SHA!r}"
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