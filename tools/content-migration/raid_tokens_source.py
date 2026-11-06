"""The exact repeatable Jorge exchange table from the retained source corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tarfile

ROOT = Path(__file__).resolve().parents[2]
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SOURCE = 'data-global/npc/jorge.lua'
SHA = '127ff40ceb3fd0612663cbd952176840977e1ba9b9f22534ba52b2d059963db0'
TARGET = 'content/dialogues/quest-native/hunting-for-tokens.json'
ITEMS = (
    (19151, 'Abacus'), (28897, 'Assassin Doll'), (23682, 'Bag of Oriental Spices'),
    (28895, 'Bookworm Doll'), (43511, 'Citizen Doll'), (25981, 'Crimson Doll'),
    (23679, 'Doll of Durin the Almighty'), (22027, 'Dragon Eye'),
    (31265, 'Dragon Goblet'), (25979, 'Draken Doll'), (25980, 'Emblem'),
    (23678, 'Encyclopedia'), (19153, 'Friendship Amulet'), (19156, 'Frozen Heart'),
    (28896, 'Golden Falcon'), (23681, 'Golden Newspaper'), (34210, 'Goromaphone'),
    (23676, 'Hand Puppets'), (23683, 'Imortus'), (31268, 'Jade Amulet'),
    (19152, 'Key of Numerous Locks'), (37058, 'Little Adventurer Doll'),
    (31267, 'Loremaster Doll'), (37059, 'Lucky Clover Amulet'),
    (25982, 'Mathmaster Shield'), (23680, 'Medusa Skull'), (23677, 'Music Box'),
    (22028, 'Noble Sword'), (19150, 'Norseman Doll'), (28894, 'Old Radio'),
    (19155, "Orc's Jaw Shredder"), (22026, 'Phoenix Statue'), (31266, 'Pigeon Trophy'),
    (43517, 'Shield of Destiny'), (37060, 'Shield of Endless Search'),
    (19154, 'The Mexcalibur'), (43510, "Tibiora's Box"),
)


def document(root=ROOT):
    archive = root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
    if hashlib.sha256(archive.read_bytes()).hexdigest() != 'f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277':
        raise ValueError('RAID_TOKENS_CORPUS_PIN')
    with tarfile.open(archive) as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        files = [row for row in manifest['files'] if row['path'] == SOURCE
                 and row['revision'] == REVISION and row['sha256'] == SHA]
        if len(files) != 1:
            raise ValueError('RAID_TOKENS_SOURCE_CUSTODY')
        raw = corpus.extractfile(files[0]['cache_path']).read()
    if hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('RAID_TOKENS_SOURCE_PIN')
    text = raw.decode()
    items = tuple((int(item), name) for name, item in
                  re.findall(r'name = "([^"]+)", id = (\d+)', text))
    if items != ITEMS or len(items) != 37 or len({item for item, _ in items}) != 37:
        raise ValueError('RAID_TOKENS_SOURCE_TABLE')
    if ('getPlayerItemCount(creature, 19083) >= 20' not in text
            or 'doPlayerRemoveItem(creature, 19083, 20)' not in text
            or 'doPlayerAddItem(creature, items[i].id, 1)' not in text):
        raise ValueError('RAID_TOKENS_SOURCE_EXCHANGE')
    return {'schema': 'OTERYN_RAID_TOKEN_EXCHANGES/v1',
            'quest': 'oteryn:quest.hunting_for_tokens_quest', 'npc': 'oteryn:npc.jorge',
            'source': {'revision': REVISION, 'path': SOURCE, 'sha256': SHA},
            'input': {'id': 19083, 'count': 20},
            'repeat': 'unlimited',
            'offers': [{'id': item, 'name': name} for item, name in items]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    payload = json.dumps(document(), ensure_ascii=False, indent=2) + '\n'
    path = ROOT / TARGET
    if args.check:
        if path.read_text() != payload:
            raise SystemExit('RAID_TOKENS_GENERATION_DRIFT')
    else:
        path.write_text(payload)
