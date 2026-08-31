#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = path.read_text(encoding="utf-8")

old_ping = r'''        r"(?=.*(?:\b(?:requires?|needs?)\s+no\s+(?:per[- ]action\s+)?(?:authori[sz]ation|approval|permission|host[- ]exception|exception)\b|"
        r"\b(?:does\s+not\s+(?:require|need)|without|exempt(?:ed)?\s+from)\b.{0,80}"
        r"\b(?:per[- ]action|authori[sz]ation|approval|permission|host[- ]exception|exception)\b))",'''
new_ping = r'''        r"(?=.*(?:\b(?:requires?|needs?)\s+no\s+(?:per[- ]action\s+)?(?:decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\b|"
        r"\b(?:does\s+not\s+(?:require|need)|without|exempt(?:ed)?\s+from)\b.{0,80}"
        r"\b(?:per[- ]action|decision|authori[sz]ation|approval|permission|host[- ]exception|exception)\b))",'''
if text.count(old_ping) != 1:
    raise SystemExit(f"expected one ping negative matcher, found {text.count(old_ping)}")
text = text.replace(old_ping, new_ping, 1)

anchor = r'''    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\b.{0,40}"
        r"\b(?:has|have)\s+(?:blanket|standing|automatic|default)\s+(?:approval|permission|authority)\b",
        re.IGNORECASE,
    ),
'''
addition = anchor + r'''    re.compile(
        r"\b(?:filesystem|search|process|session|terminal|history|ping)\b.{0,80}"
        r"\b(?:is|are|was|were|be|been|being)\s+(?:automatically\s+(?:authori[sz]ed|approved)|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\s+by\s+default)\b",
        re.IGNORECASE,
    ),
'''
if text.count(anchor) != 1:
    raise SystemExit(f"expected one bare-host authority anchor, found {text.count(anchor)}")
text = text.replace(anchor, addition, 1)
path.write_text(text, encoding="utf-8")

Path("tools/agents/rdc_latest_review_repair.py").unlink()
Path(".github/workflows/rdc-latest-review-repair.yml").unlink()
