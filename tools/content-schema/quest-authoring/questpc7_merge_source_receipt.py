"""Reconcile the questpc7 merge using the existing frozen-SOURCE verifier."""
import hashlib
import subprocess
from pathlib import Path

import amend_itemhold_source_receipt as amendment

ROOT = Path(__file__).resolve().parents[3]
PARENT = '5c39e6be3c934499b27ff01f6c3cc63b8af46581'


def main():
    # Only the pushed lane parent is an accepted predecessor. The existing
    # amendment still verifies every other descriptor and every frozen core.
    previous = subprocess.check_output(
        ['git', 'show', f'{PARENT}:{amendment.COMPILER}'], cwd=ROOT)
    amendment.PREVIOUS = hashlib.sha256(previous).hexdigest()
    for relative, text in amendment.outputs(ROOT).items():
        (ROOT / relative).write_text(text, encoding='utf-8')
    amendment.guard.verified_receipt(ROOT)
    print('PASS questpc7 merge SOURCE compiler; frozen SOURCE cores unchanged')


if __name__ == '__main__':
    main()
