#!/usr/bin/env python3
from pathlib import Path

validator = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = validator.read_text(encoding="utf-8")

old_postposed = r"(?:automatically\s+(?:authori[sz]ed|approved)|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)"
new_postposed = r"(?:automatically\s+(?:authori[sz]ed|approved)|(?:authori[sz]ed|approved)\s+automatically|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)"
if text.count(old_postposed) != 1:
    raise SystemExit(f"expected one bare-host automatic authorization group, found {text.count(old_postposed)}")
text = text.replace(old_postposed, new_postposed, 1)

provider_anchor = '    re.compile(\n        r"(?=.*\\b(?:connectors?|routers?|transports?|providers?)\\b.{0,80}"'
bare_host_negative = r'''    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history)\b\s+"
        r"(?:(?:calls?|operations?|requests?|invocations?|actions?)\s+)?"
        r"(?:"
        r"(?:requires?|needs?)\s+no\s+(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)|"
        r"does\s+not\s+(?:require|need)\s+(?:a\s+)?(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)|"
        r"(?:is\s+)?exempt(?:ed)?\s+from\s+(?:a\s+)?(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)"
        r")\b",
        re.IGNORECASE,
    ),
'''
if text.count(provider_anchor) != 1:
    raise SystemExit(f"expected one active provider-enforcement anchor, found {text.count(provider_anchor)}")
text = text.replace(provider_anchor, bare_host_negative + provider_anchor, 1)

old_enforcement = r"ignore(?:s|d|ing)?|filter(?:s|ed|ing)?(?:\s+out)?"
new_enforcement = r"ignore(?:s|d|ing)?|cancel(?:s|ed|ing|led|ling)?|intercept(?:s|ed|ing)?|filter(?:s|ed|ing)?(?:\s+out)?"
if text.count(old_enforcement) != 2:
    raise SystemExit(f"expected two provider-enforcement verb groups, found {text.count(old_enforcement)}")
text = text.replace(old_enforcement, new_enforcement)

validator.write_text(text, encoding="utf-8")

Path("tools/agents/rdc_final_review_repair.py").unlink()
Path(".github/workflows/rdc-final-review-repair.yml").unlink()
