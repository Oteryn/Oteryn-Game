#!/usr/bin/env python3
from pathlib import Path
import subprocess

validator = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = validator.read_text(encoding="utf-8")

old_url_prefix = 'r"(?:(?:https?:)?//)(?:(?:(?!(?:https?:)?//)[^\\x00])*@)?'
new_url_prefix = 'r"(?:(?:(?:https?:)?//)(?:(?:(?!(?:https?:)?//)[^\\x00])*@)?|(?<![A-Za-z0-9_.-]))'
if text.count(old_url_prefix) != 2:
    raise SystemExit(f"expected two GitHub URL prefixes, found {text.count(old_url_prefix)}")
text = text.replace(old_url_prefix, new_url_prefix)

old_bare_host_auth = 'r"(?:automatically\\s+(?:authori[sz]ed|approved)|(?:authori[sz]ed|approved)\\s+automatically|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\\s+by\\s+default)\\b",'
new_bare_host_auth = 'r"(?:automatically\\s+(?:authori[sz]ed|approved)|(?:authori[sz]ed|approved)\\s+automatically|always\\s+(?:authori[sz]ed|approved)|preauthori[sz]ed|preapproved|(?:authori[sz]ed|approved)\\s+by\\s+default)\\b",'
if text.count(old_bare_host_auth) != 1:
    raise SystemExit(f"expected one bare-host authorization matcher, found {text.count(old_bare_host_auth)}")
text = text.replace(old_bare_host_auth, new_bare_host_auth, 1)

old_enforcement_tail = r"|intercept(?:s|ed|ing)?|filter(?:s|ed|ing)?(?:\s+out)?"
new_enforcement_tail = r"|intercept(?:s|ed|ing)?|quarantin(?:e|es|ed|ing)|filter(?:s|ed|ing)?(?:\s+out)?"
if text.count(old_enforcement_tail) != 2:
    raise SystemExit(f"expected two provider enforcement tails, found {text.count(old_enforcement_tail)}")
text = text.replace(old_enforcement_tail, new_enforcement_tail)

validator.write_text(text, encoding="utf-8")

original_agent_governance = subprocess.check_output(
    [
        "git",
        "show",
        "6e65c1d46975877059d7bcf7fcff47f126d16496:.github/workflows/agent-governance.yml",
    ],
    text=True,
)
Path(".github/workflows/agent-governance.yml").write_text(
    original_agent_governance,
    encoding="utf-8",
)
Path("tools/agents/rdc_final_review_repair.py").unlink()
Path(".github/workflows/rdc-final-review-repair.yml").unlink()
