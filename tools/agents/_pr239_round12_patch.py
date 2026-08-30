#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = path.read_text(encoding="utf-8")

old = r'(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority))\b)'
new = r'(?:approval|permission|authority)\s+by\s+default|by\s+default.{0,80}(?:approval|permission|authority)|(?:approval|permission|authority)\s+automatically)\b)'
count = text.count(old)
if count != 3:
    raise SystemExit(f"expected exactly 3 grant matcher sites, found {count}")
text = text.replace(old, new)

old_userinfo = r'(?:[^\s]+@)?'
new_userinfo = r'(?:[^#?<>\(\)\r\n]*@)?'
count = text.count(old_userinfo)
if count != 2:
    raise SystemExit(f"expected exactly 2 GitHub userinfo matcher sites, found {count}")
text = text.replace(old_userinfo, new_userinfo)

path.write_text(text, encoding="utf-8")
