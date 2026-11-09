"""Reuse the committed World owner's admission keys for imported exploration."""
import json
from pathlib import Path

SOURCE = 'content/encounters/world-changes/owner-catalogue.json'


def catalogue(root):
    result = {}
    for entry in json.loads((Path(root) / SOURCE).read_text())['entries']:
        if isinstance(entry.get('quest'), str):
            result.setdefault(entry['quest'], []).append(entry)
    return result


def requirement(quest, owners):
    matches = owners.get(quest, [])
    if not matches:
        if (quest.endswith(('_world_change', '_worldchange'))
                or quest in ('oteryn:quest.bewitched', 'oteryn:quest.authored.bewitched')):
            raise ValueError('World-change EXPLORE needs an explicit current-World owner: ' + quest)
        return None
    if len(matches) != 1 or matches[0]['status'] not in ('READY', 'QUALIFIED'):
        raise ValueError('World-change EXPLORE needs one admitted World owner: ' + quest)
    return matches[0]['key']
