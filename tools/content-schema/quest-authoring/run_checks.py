"""Run all offline Quest data checks without regenerating committed files."""
import argparse
import os
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    root = parser.parse_args().root.resolve()
    here = root / 'tools/content-schema/quest-authoring'
    packet = here / 'samples/migration/quest-source-packet.json'
    scripts = ['verify_quest_schema.py', 'validate_quest_content.py',
               'refresh_quest_source_checks.py', 'quest_catalogue_authoring.py',
               'quest_tree_authoring.py', 'bundle_authoring.py', 'source_text_authoring.py',
               'build_wiki_source_schema.py', 'wiki_source_inventory.py', 'wiki_source_supplements.py', 'wiki_all_source_inventory.py', 'quest_rollout_authoring.py', 'wiki_requirement_interpretations.py', 'source_facts_authoring.py']
    for name in ('source_text_capture.json', 'samples/source_texts/source_texts.json'):
        if not (here / name).is_file():
            parser.error('required source prose check input missing: ' + str(here / name))
    missing = [str(here / name) for name in scripts if not (here / name).is_file()]
    bundle = here / 'samples/source_migration/bundle.json'
    if not bundle.is_file():
        missing.append(str(bundle))
    if not packet.is_file():
        missing.append(str(packet))
    if missing:
        parser.error('required Quest check inputs missing: ' + ', '.join(missing))
    commands = [
        ['verify_quest_schema.py'],
        ['../reward-claim-authoring/test_reward_claim_variant_authoring.py'],
        ['../reward-claim-authoring/reward_claim_authoring.py', 'content', '--check'],
        ['../reward-claim-authoring/reward_claim_variant_migration.py', '--check'],
        ['-m', 'unittest', 'discover', '-s', '.', '-p', 'test_*.py'],
        ['validate_quest_content.py', 'samples/chests/claims.json', 'samples/questlog/quests.json',
         '--catalog', 'samples/chests/catalog.json', '--manifest', 'samples/chests/manifest.json',
         '--gates', 'samples/doors/gates.json', '--gates-manifest', 'samples/doors/manifest.json',
         '--progress', 'samples/questlog/progress.json', '--interactions', 'samples/interactions/interactions.json',
         '--interactions-manifest', 'samples/interactions/manifest.json'],
        ['refresh_quest_source_checks.py', '--check'],
        ['quest_catalogue_authoring.py', '--check'],
        ['source_text_authoring.py', '--check'],
        ['build_wiki_source_schema.py', '--check'],
        ['wiki_source_inventory.py', '--check'],
        ['wiki_source_supplements.py', '--check'],
        ['wiki_all_source_inventory.py', '--check'],
        ['wiki_all_source_inventory.py', '--schema-out', 'wiki_all_source_specs.schema.json', '--check'],
        ['wiki_requirement_interpretations.py', '--check'],
        ['source_facts_authoring.py', '--specifications', 'samples/wiki-source-all373/source-specs-373.json', '--samples', 'samples/unbound-source-facts', '--check'],
        ['quest_rollout_authoring.py', '--check'],
        ['bundle_authoring.py', '--check'],
        ['quest_tree_authoring.py', 'content', '--check', '--source-packet', str(packet)],
    ]
    env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
    for arguments in commands:
        print('Quest check: ' + ' '.join(arguments), flush=True)
        result = subprocess.run([sys.executable, *arguments], cwd=here, env=env)
        if result.returncode:
            return result.returncode
    return 0


if __name__ == '__main__':
    sys.exit(main())
