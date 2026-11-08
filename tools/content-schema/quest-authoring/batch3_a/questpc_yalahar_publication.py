"""Release original mechanism callbacks only when native startup UIDs are present."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import quest_access_bindings as access


def restored(quest, row):
    if quest != 'in_service_of_yalahar':
        return False
    uid = row['selector'].get('unique')
    if uid not in {1018, 1019, *range(3091, 3103)}:
        return False
    capture = json.loads((ROOT / access.CAPTURE).read_bytes())
    matches = [b for b in capture['bindings'] if b['attribute'] == 'unique'
               and b['value'] == uid and b['source'] == 'data-global/startup/tables/item.lua']
    if len(matches) != 1:
        return False
    binding = matches[0]
    x, y, z = binding['position']
    path = ROOT / f'content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3'
    raw = path.read_bytes()
    index = json.loads((ROOT / access.INDEX).read_bytes())
    after, imported = access.overlay_region(raw, index['palette'], [binding])
    return after == raw and len(imported) == 1
