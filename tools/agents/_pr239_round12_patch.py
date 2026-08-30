#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = path.read_text(encoding="utf-8")

anchor = '''    re.compile(\n        r"(?=.*\\bdirect\\s+(?:connectors?|tools?)\\b)"\n        r"(?=.*(?:\\b(?:blanket|standing|automatic|default)\\s+(?:approval|permission|authority)\\b|"\n'''
addition = '''    re.compile(\n        r"(?=.*\\b(?:(?:direct\\s+(?:connectors?|tools?))|"\n        r"(?:(?:connectors?|tools?)\\s+(?:calls?|operations?|requests?|invocations?|actions?))|"\n        r"(?:(?:filesystem|search|process|session|terminal|history|ping)\\s+(?:calls?|operations?|requests?|invocations?|actions?)))\\b)"\n        r"(?=.*\\b(?:approval|permission|authority)\\s+automatically\\b)",\n        re.IGNORECASE,\n    ),\n'''
count = text.count(anchor)
if count != 1:
    raise SystemExit(f"expected exactly 1 direct-authority insertion anchor, found {count}")
text = text.replace(anchor, addition + anchor, 1)

old_userinfo = r'(?:[^\s]+@)?'
new_userinfo = r'(?:[^#?<>()\r\n]*@)?'
count = text.count(old_userinfo)
if count != 2:
    raise SystemExit(f"expected exactly 2 GitHub userinfo matcher sites, found {count}")
text = text.replace(old_userinfo, new_userinfo)

path.write_text(text, encoding="utf-8")
