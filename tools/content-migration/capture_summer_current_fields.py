"""Capture published Summer facts; an Unknown value remains absent."""
import argparse
import hashlib
import html
import json
import re
from pathlib import Path

REVISION = "acc9994e2ad8868696bc10ef08070108e2baba92"
FILES = {
    "oteryn:creature.mimar_haffar": "mimar-haffar-87d7d",
    "oteryn:creature.phosphorus_final": "phosphorus-final-ec835",
}


def field(document, key):
    pattern = (r'data-source="' + key
               + r'"[^>]*>.*?<div class="pi-data-value[^\"]*"[^>]*>(.*?)</div>')
    match = re.search(pattern, document["html"], re.S)
    if not match:
        raise ValueError("Missing published field " + key)
    text = html.unescape(re.sub(r"<[^>]+>", "", match[1])).strip()
    if text.startswith("Unknown"):
        return None
    if not re.fullmatch(r"[0-9,]+", text):
        raise ValueError("Ambiguous published numeric field " + text)
    return int(text.replace(",", ""))


def capture(directory):
    records = []
    for key, slug in FILES.items():
        path = "quests/data/wiki/" + slug + ".json"
        raw = (directory / (slug + ".json")).read_bytes()
        document = json.loads(raw)
        facts = {}
        for name, source in [("health", "hp"), ("experience", "exp")]:
            value = field(document, source)
            if value is not None:
                facts[name] = value
        records.append({
            "creature": key,
            "source": {
                "repository": "Lavovskii/grimorio",
                "revision": REVISION,
                "path": path,
                "sha256": hashlib.sha256(raw).hexdigest(),
                "url": f"https://raw.githubusercontent.com/Lavovskii/grimorio/{REVISION}/{path}",
            },
            "facts": facts,
        })
    return {"schema": "OTERYN_CURRENT_SUMMER_PUBLISHED_FIELDS/v1", "records": records}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wiki-directory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = json.dumps(capture(args.wiki_directory), indent=2) + "\n"
    if args.check:
        if args.output.read_text() != encoded:
            raise SystemExit("Summer current published fields are stale")
    else:
        args.output.write_text(encoded)
