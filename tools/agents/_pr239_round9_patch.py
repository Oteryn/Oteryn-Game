#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = path.read_text(encoding="utf-8")

old_ping = '''    re.compile(
        r"(?=.*\\bping\\b)"
        r"(?=.*(?:\\bautomatically\\s+(?:authori[sz]ed|approved)\\b|\\bpreauthori[sz]ed\\b|\\bpreapproved\\b|"
        r"\\b(?:blanket|standing|automatic|default)\\s+(?:approval|permission|authori[sz]ation)\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?(?:granted|given)\\s+by\\s+default\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+by\\s+default\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?automatically\\s+(?:granted|given)\\b|"
        r"\\b(?:authori[sz]ed|approved)\\s+by\\s+default\\b))",
        re.IGNORECASE,
    ),
'''
new_ping = '''    re.compile(
        r"(?=.*\\bping\\b)"
        r"(?=.*(?:\\bautomatically\\s+(?:authori[sz]ed|approved)\\b|\\bpreauthori[sz]ed\\b|\\bpreapproved\\b|"
        r"\\b(?:blanket|standing|automatic|default)\\s+(?:approval|permission|authori[sz]ation)\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?(?:granted|given)\\s+by\\s+default\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+by\\s+default\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?automatically\\s+(?:granted|given)\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?(?:granted|given)\\s+automatically\\b|"
        r"\\bautomatically\\s+(?:granted|given)\\s+(?:approval|permission|authori[sz]ation)\\b|"
        r"\\b(?:approval|permission|authori[sz]ation)\\s+(?:is\\s+)?automatic\\b|"
        r"\\b(?:authori[sz]ed|approved)\\s+by\\s+default\\b))",
        re.IGNORECASE,
    ),
'''
if text.count(old_ping) != 1:
    raise SystemExit(f"ping matcher anchor count={text.count(old_ping)}")
text = text.replace(old_ping, new_ping, 1)

old_location = 'r"\\b(?:stop|stops|stopped|stopping|refuse|refuses|refused|refusing)\\s+(?:by|at)\\s+(?:the\\s+)?"'
new_location = 'r"\\b(?:stop|stops|stopped|stopping|refuse|refuses|refused|refusing)\\s+(?:by|at|within|inside)\\s+(?:the\\s+)?"'
if text.count(old_location) != 1:
    raise SystemExit(f"provider location anchor count={text.count(old_location)}")
text = text.replace(old_location, new_location, 1)

old_web = 'r"(?:(?:https?:)?//)(?:(?:www\\.)?github\\.com\\.?(?::[0-9]{1,5})?/Oteryn/Oteryn/(?:blob|tree|raw)|raw\\.githubusercontent\\.com\\.?(?::[0-9]{1,5})?/Oteryn/Oteryn)/([^\\s`<>)]+?)/ecosystem/agent-execution-routing-policy\\.json",'
new_web = 'r"(?:(?:https?:)?//)(?:[^\\s/@]+@)?(?:(?:www\\.)?github\\.com\\.?(?::[0-9]{1,5})?/Oteryn/Oteryn/(?:blob|tree|raw)|raw\\.githubusercontent\\.com\\.?(?::[0-9]{1,5})?/Oteryn/Oteryn)/([^\\s`<>)]+?)/ecosystem/agent-execution-routing-policy\\.json",'
if text.count(old_web) != 1:
    raise SystemExit(f"GitHub web URL anchor count={text.count(old_web)}")
text = text.replace(old_web, new_web, 1)

old_api = 'r"(?:(?:https?:)?//)api\\.github\\.com\\.?(?::[0-9]{1,5})?/repos/Oteryn/Oteryn/contents/"'
new_api = 'r"(?:(?:https?:)?//)(?:[^\\s/@]+@)?api\\.github\\.com\\.?(?::[0-9]{1,5})?/repos/Oteryn/Oteryn/contents/"'
if text.count(old_api) != 1:
    raise SystemExit(f"GitHub Contents URL anchor count={text.count(old_api)}")
text = text.replace(old_api, new_api, 1)

path.write_text(text, encoding="utf-8")
