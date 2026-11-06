import subprocess
from pathlib import Path

def run(*command):
    print("RUN " + " ".join(command), flush=True)
    subprocess.run(command, check=True)

run("python3", "tools/content-schema/quest-authoring/quest_tree_authoring.py", "content")
run("/home/mole/oteryn-cache/venv-ci312/bin/python", "tools/content-schema/quest-authoring/quest_donor_source_authoring.py")
run("python3", "tools/content-schema/quest-authoring/quest_tree_authoring.py", "content")
commands = Path(".questpc/generation-order.txt").read_text().splitlines()
start = next(i for i, c in enumerate(commands) if c.endswith("quest_tree_authoring.py content"))
for command in commands[start + 1:-1]:
    run(*command.split())

# Binding packets record canonical quest digests; refresh their donor
# crosswalk receipts after downstream authoring, without changing SOURCE cores.
run("/home/mole/oteryn-cache/venv-ci312/bin/python", "tools/content-schema/quest-authoring/quest_donor_source_authoring.py")
