"""Compose regenerated QENGINE rows with the other shared-document owners."""
import json


def compose(root, generated):
    path = 'content/quests/shared-reward-callbacks.json'
    existing = json.loads((root / path).read_bytes())
    owned = {row['key'] for row in json.loads(generated[path])['callbacks']}
    foreign_npcs = {row['npc'] for row in existing['callbacks'] if row['key'] not in owned}
    for path, fields in (
        (path, ('callbacks', 'tracks')),
        ('content/quests/shared-reward-offers.json', ('offers',)),
        ('content/dialogues/quests/shared-reward-greetings.json', ('npcs',)),
        ('content/quests/shared-outfit-callbacks.json', ('callbacks',)),
    ):
        current = json.loads((root / path).read_bytes())
        fresh = json.loads(generated[path])
        for field in fields:
            def identity(row):
                if field == 'offers':
                    return json.dumps({k: v for k, v in row.items()
                                       if k not in ('reply', 'next_topic')}, sort_keys=True)
                return row['npc'] if field == 'npcs' else row['key']

            replacements = {identity(row): row for row in fresh.get(field, [])}
            if len(replacements) != len(fresh.get(field, [])):
                raise ValueError('duplicate shared authoring identity: ' + path)
            rows = []
            for row in current.get(field, []):
                replacement = replacements.pop(identity(row), row)
                # Shared NPC greetings belong to the retained quest owner overlay.
                rows.append(row if field == 'npcs' and row['npc'] in foreign_npcs
                            else replacement)
            current[field] = rows + list(replacements.values())
        generated[path] = (json.dumps(current, ensure_ascii=False, indent=2) + '\n').encode()
