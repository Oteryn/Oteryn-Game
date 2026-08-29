#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = path.read_text(encoding="utf-8")

# Expand only the ping-specific blanket authority vocabulary.
old = "approval|permission|authori[sz]ation"
new = "approval|permission|authori[sz]ation|authority"
start = text.index('r"(?=.*\\bping\\b)"')
end = text.index('    re.compile(\n        r"(?=.*\\bping\\b)"', start + 1)
prefix = text[:start]
block = text[start:end]
suffix = text[end:]
count = block.count(old)
if count < 4:
    raise SystemExit(f"ping matcher authority anchors too few: {count}")
block = block.replace(old, new)
text = prefix + block + suffix

# URL scan is already bounded to exact GitHub hosts. Percent-decoding can turn
# encoded userinfo into strings containing '/', so allow any non-space/non-@ chars.
old_userinfo = r'(?:[^\s/@]+@)?'
new_userinfo = r'(?:[^\s@]+@)?'
count = text.count(old_userinfo)
if count != 2:
    raise SystemExit(f"userinfo anchors expected 2, got {count}")
text = text.replace(old_userinfo, new_userinfo)

path.write_text(text, encoding="utf-8")
