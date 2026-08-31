#!/usr/bin/env python3
from pathlib import Path

validator = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = validator.read_text(encoding="utf-8")

old_postposed = r"(?:automatically\s+(?:authori[sz]ed|approved)|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)"
new_postposed = r"(?:automatically\s+(?:authori[sz]ed|approved)|(?:authori[sz]ed|approved)\s+automatically|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)"
if text.count(old_postposed) != 1:
    raise SystemExit(f"expected one bare-host automatic authorization group, found {text.count(old_postposed)}")
text = text.replace(old_postposed, new_postposed, 1)

old_negative_subject = '        r"(?=.*\\bping\\b)"\n        r"(?=.*(?:\\b(?:requires?|needs?)\\s+no\\s+(?:per[- ]action\\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\\b|"'
new_negative_subject = '        r"(?=.*\\b(?:filesystem|search|process|session|terminal|history|ping)\\b)"\n        r"(?=.*(?:\\b(?:requires?|needs?)\\s+no\\s+(?:per[- ]action\\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\\b|"'
if text.count(old_negative_subject) != 1:
    raise SystemExit(f"expected one ping-only negative matcher prefix, found {text.count(old_negative_subject)}")
text = text.replace(old_negative_subject, new_negative_subject, 1)

old_enforcement = r"ignore(?:s|d|ing)?|filter(?:s|ed|ing)?(?:\s+out)?"
new_enforcement = r"ignore(?:s|d|ing)?|cancel(?:s|ed|ing|led|ling)?|intercept(?:s|ed|ing)?|filter(?:s|ed|ing)?(?:\s+out)?"
if text.count(old_enforcement) != 2:
    raise SystemExit(f"expected two provider-enforcement verb groups, found {text.count(old_enforcement)}")
text = text.replace(old_enforcement, new_enforcement)

validator.write_text(text, encoding="utf-8")

Path("tools/agents/rdc_final_review_repair.py").unlink()
Path(".github/workflows/rdc-final-review-repair.yml").unlink()
