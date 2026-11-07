"""Use the registry inverse to correct noncanonical fragment filenames."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path("tools/contracts").resolve()))
import registry_fragments as rf

text = rf.load_text(rf.PROTOCOL)
_, fragments = rf.split(rf.PROTOCOL, text)
for collection, entries in fragments.items():
    directory = Path(rf.collection_dir(rf.PROTOCOL, collection))
    for name, content in entries:
        target = directory / name
        if target.exists():
            assert target.read_text() == content, target
            continue
        prefix = name.split("-", 1)[0]
        old, = directory.glob(prefix + "-*.json")
        assert old.read_text() == content, old
        old.rename(target)
        print(f"{old} -> {target}")
assert rf.load_text(rf.PROTOCOL) == text, "aggregate contents must remain identical"
