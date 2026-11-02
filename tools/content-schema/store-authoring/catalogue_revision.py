"""D2(b) catalogue revision: compact, recursively sorted JSON in raw UTF-8.

Only integer numbers are admitted, including in source metadata.
"""

import argparse
import hashlib
import json
from pathlib import Path


def revision(obj):
    def validate(value):
        if isinstance(value, float):
            raise TypeError("catalogue numbers must be integers")
        if isinstance(value, dict):
            for child in value.values():
                validate(child)
        elif isinstance(value, list):
            for child in value:
                validate(child)

    validate(obj)
    canonical = json.dumps(
        obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("catalogue", type=Path)
    args = parser.parse_args()
    print(revision(json.loads(args.catalogue.read_text(encoding="utf-8"))))
