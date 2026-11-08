"""Replay six named Soul War entry configurations; source facts grant no admission."""
import hashlib, json, re, sys, tarfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'encounters'))
from import_boss_levers import POSITION, number, position, table

DIRECTORY = 'tools/content-schema/quest-authoring/samples/donor-source'
SOURCE = 'data-global/lib/quests/soul_war.lua'
NAMES = ('Spite', 'Malice', 'Greed', 'Hatred', 'Cruelty', 'Megalomania')
OUTPUT = 'tools/content-schema/quest-authoring/samples/questpc2-soul-entries.json'


def capture(root):
    declaration = json.loads((root / DIRECTORY / 'corpus-input.json').read_bytes())
    archive = root / DIRECTORY / declaration['path']
    if hashlib.sha256(archive.read_bytes()).hexdigest() != declaration['sha256']:
        raise ValueError('donor archive digest differs')
    with tarfile.open(archive) as stream:
        manifest = json.load(stream.extractfile('corpus-manifest.json'))
        row = next(r for r in manifest['files'] if r['source'] == 'crystalserver' and r['path'] == SOURCE)
        raw = stream.extractfile(row['cache_path']).read()
    if row['revision'] != '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d' or hashlib.sha256(raw).hexdigest() != row['sha256']:
        raise ValueError('Soul War source witness differs')
    text, records = raw.decode(), []
    for suffix in NAMES:
        name = 'goshnars' + suffix
        body = table(text, rf'\b{name}\b')
        if body is None:
            raise ValueError('missing named entry configuration: ' + name)
        lever = re.search(rf'\b{name}Position\s*=\s*\{{\s*x\s*=\s*(\d+),\s*y\s*=\s*(\d+),\s*z\s*=\s*(\d+)', text)
        pairs = re.findall(r'\{\s*pos\s*=\s*' + POSITION + r'\s*,\s*teleport\s*=\s*' + POSITION, table(body, r'\bplayerPositions'))
        cooldowns = [number(v) for v in re.findall(r'\btimeToFightAgain\s*=\s*([^,\n]+)', body)]
        records.append(dict(name=name, line=text[:text.index(body)].count('\n') + 1,
            lever=[int(v) for v in lever.groups()], boss=position(table(body, r'\bboss'), r'\bposition'),
            players=[dict(from_=[int(v) for v in p[:3]], to=[int(v) for v in p[3:]]) for p in pairs],
            zone_from=position(table(body, r'\bspecPos'), r'\bfrom'), zone_to=position(table(body, r'\bspecPos'), r'\bto'),
            exit=position(body, r'\bexit'), minimum_level=int(re.search(r'\brequiredLevel\s*=\s*(\d+)', body)[1]),
            cooldown_assignments=cooldowns, cooldown_seconds=cooldowns[-1], custom_on_use='onUseExtra' in body))
    for record in records:
        for player in record['players']:
            player['from'] = player.pop('from_')
    return dict(schema='OTERYN_SOUL_WAR_ENTRY_SOURCE_FACTS/v1', native_admission=False,
                source={k: row[k] for k in ('repository', 'revision', 'path', 'sha256', 'git_blob_sha1')}, entries=records)


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    (root / OUTPUT).write_text(json.dumps(capture(root), sort_keys=True, indent=2) + '\n')
