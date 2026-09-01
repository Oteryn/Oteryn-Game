#!/usr/bin/env python3
from pathlib import Path
import subprocess

validator = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = validator.read_text(encoding="utf-8")

anchor = '''    re.compile(
        r"(?=.*\\b(?:connectors?|routers?|transports?|providers?)\\b.{0,80}"
        r"\\b(?:stop|stops|stopped|stopping|refuse|refuses|refused|refusing|drop|drops|dropped|dropping|discard|discards|discarded|discarding|skip(?:s|ped|ping)?|suppress(?:es|ed|ing)?|ignore(?:s|d|ing)?|cancel(?:s|ed|ing|led|ling)?|intercept(?:s|ed|ing)?|quarantin(?:e|es|ed|ing)|filter(?:s|ed|ing)?(?:\\s+out)?)\\b.{0,80}"
'''
insert = '''    re.compile(
        r"\\b(?:filesystem|search|process|session|terminal|history|ping)\\b.{0,80}"
        r"\\b(?:may\\s+be\\s+used|can\\s+run)\\s+without\\s+(?:a\\s+)?(?:per[- ]action\\s+)?"
        r"(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\\b",
        re.IGNORECASE,
    ),
''' + anchor
if text.count(anchor) != 1:
    raise SystemExit(f"expected one provider enforcement anchor, found {text.count(anchor)}")
text = text.replace(anchor, insert, 1)

old_enforcement_tail = r"|intercept(?:s|ed|ing)?|quarantin(?:e|es|ed|ing)|filter(?:s|ed|ing)?(?:\s+out)?"
new_enforcement_tail = r"|intercept(?:s|ed|ing)?|quarantin(?:e|es|ed|ing)|declin(?:e|es|ed|ing)|filter(?:s|ed|ing)?(?:\s+out)?"
if text.count(old_enforcement_tail) != 2:
    raise SystemExit(f"expected two provider enforcement tails, found {text.count(old_enforcement_tail)}")
text = text.replace(old_enforcement_tail, new_enforcement_tail)

old_url_return = '    return _remove_default_ignorables(value)\n'
new_url_return = '    return _remove_default_ignorables(value).replace("\\\\", "/")\n'
if text.count(old_url_return) != 1:
    raise SystemExit(f"expected one URL normalization return, found {text.count(old_url_return)}")
text = text.replace(old_url_return, new_url_return, 1)

validator.write_text(text, encoding="utf-8")

canonical_workflow = subprocess.check_output(
    [
        "git",
        "show",
        "3fee35ac1a3b8d80f3290fef67da2154a9209e1b:.github/workflows/agent-governance.yml",
    ],
    text=True,
)
Path(".github/workflows/agent-governance.yml").write_text(canonical_workflow, encoding="utf-8")
Path("tools/agents/rdc_final_review_repair.py").unlink()
