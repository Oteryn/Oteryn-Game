"""Check package registrations against the immutable current-main baseline."""
from collections import defaultdict
import json
from pathlib import Path
import re
import subprocess
import sys

base = "origin/main"
paths = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", base], text=True).splitlines()
categories = {
    "capability": ("docs/contracts/protocol-oteryn/v1/registry/capabilities", {384: "mechanics-status"}),
    "command": ("docs/contracts/protocol-oteryn/v1/registry/command_types", {669: "mechanics-status-query", 641: "proficiency-browse"}),
    "mechanic": ("crates/gameplay/dispatch/mechanics", {642: "mechanics_status", 638: "weapon_proficiency_browse"}),
}
for kind, (directory, wanted) in categories.items():
    occupied = defaultdict(list)
    for path in paths:
        if path.startswith(directory + "/"):
            match = re.match(r"(\d+)-", Path(path).name)
            if match:
                occupied[int(match[1])].append(path)
    for number, slug in wanted.items():
        assert number not in occupied, (kind, number, occupied[number])
        print(f"PASS {kind} {number} {slug}: free on {base}")
    current = defaultdict(list)
    for path in Path(directory).iterdir():
        match = re.match(r"(\d+)-", path.name)
        if match:
            current[int(match[1])].append(str(path))
    assert all(len(matches) == 1 for matches in current.values()), current
offers = defaultdict(list)
for path in Path("crates/gameplay/dispatch/mechanics").glob("*.mechanic"):
    for number in re.findall(r"^offer=(\d+),", path.read_text(), re.M):
        offers[int(number)].append(str(path))
assert all(len(matches) == 1 for matches in offers.values()), offers
assert offers[639] == ["crates/gameplay/dispatch/mechanics/0642-mechanics_status.mechanic"]
print("PASS offer 639 uniquely belongs to mechanics_status; CL7 adds no offer or capability")
for directory in ("capabilities", "command_types"):
    for path in Path(f"docs/contracts/protocol-oteryn/v1/registry/{directory}").glob("*.json"):
        assert int(path.name.split("-", 1)[0]) == json.loads(path.read_text())["id"], path
changed = subprocess.check_output(["git", "diff", "--name-only", base], text=True).splitlines()
for name in changed:
    path = Path(name)
    if path.is_file():
        assert not re.search(r"^(<<<<<<<|=======|>>>>>>>|\|\|\|\|\|\|\|)(?: |$)", path.read_text(), re.M), name
print("PASS fragment IDs match filenames, no duplicate registrations or merge markers")
sys.path.insert(0, str(Path("tools/contracts").resolve()))
import registry_fragments

errors = registry_fragments.validate(registry_fragments.PROTOCOL)
assert not errors, errors
print("PASS repository protocol fragment validation")
