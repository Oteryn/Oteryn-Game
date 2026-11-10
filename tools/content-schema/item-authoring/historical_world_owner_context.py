"""Reconstruct pinned pre-W15c owner receipts without changing live ownership."""

import hashlib
import importlib.util
import json
from functools import lru_cache
from pathlib import Path

MARKER = "content/world/terrain/index.json"
GROUND = "terrain-08613-08613.json"
BASE_SHA = "abffe2dd83dfec1ce1afd657bd0fcaae3760a10a509f8e1034e35834a714ba79"
MARKER_SHA = "360e65beb939450179cb6c99b5c19a0e2842051fd9c3beaf21e6366b9c9245a4"
GROUND_SHA = "aefdac30e8f0a33bbb52acd1d4e160b33c7f40ce9d8d447496ba95d525b182aa"
NOTE = (
    " Additionally one decision-21 appearance-only Grizzly ground;"
    " total 8614 Terrain records."
)
SOULPIT = "terrain-08614-08668.json"
SOULPIT_SHA = "dfd54e670a7de3eb8b5a444df9c8efc15faf8ddafadab761347338d80141f820"
SOULPIT_MARKER_SHA = "22cb600cca441ac68526501c8e3576ec9c1dd07f77b276db470c70b996028fec"
SOULPIT_NOTE = (
    " Additionally 55 W07 Soulpit static appearance records;"
    " total 8669 Terrain records."
)
SUGAR_GROUND = "terrain-08614-08692.json"
FEROXA_STATIC = "terrain-08748-08748.json"


@lru_cache(maxsize=1)
def sugar_owner():
    """Replay the existing pinned official/Crystal witness, never bless a live hash."""
    path = Path(__file__).resolve().parents[1] / "world-object-authoring/sugar_room.py"
    spec = importlib.util.spec_from_file_location("historical_sugar_room", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    # sugar_room.outputs verifies the complete official artifact, membership and
    # Crystal00ce source flags before producing these exact appearance-only rows.
    return module.MARKER_NOTE, module.outputs()[module.SHARD]


def without_sugar(root, raw):
    marker = json.loads(raw)
    note, expected = sugar_owner()
    if not marker["notes"].endswith(note):
        raise ValueError("source digest drift: Sugar terrain marker note")
    shard = root / "content/world/terrain" / SUGAR_GROUND
    if not shard.is_file() or shard.read_bytes() != expected:
        raise ValueError("source digest drift: Sugar terrain witness")
    if (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode() != raw:
        raise ValueError("source digest drift: Sugar terrain marker bytes")
    marker["notes"] = marker["notes"].removesuffix(note)
    return (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def original_soulpit(root):
    raw = (Path(root) / 'content/world/terrain' / SOULPIT).read_bytes()
    if sha(raw) == SOULPIT_SHA:
        return raw
    document = json.loads(raw)
    if (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode() != raw:
        raise ValueError('source digest drift: Soulpit serialization')
    shared = {43516, 46629, 46630, 46631, 46632, 46633, 46634, 48236}
    changed = set()
    for row in document['records']:
        iid = row['provenance']['source_item_id']
        if iid not in shared:
            continue
        for field in ('blocks_projectile', 'blocks_sight'):
            if (row[field] != {'state': 'KNOWN', 'value': iid == 43516}
                    or row['provenance']['fields'].get(field) != ['appearance:flags.unsight']):
                raise ValueError('source digest drift: joint Soulpit sight witness')
            row[field] = {'state': 'UNKNOWN'}
            del row['provenance']['fields'][field]
        changed.add(iid)
    predecessor = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    if changed != shared or sha(predecessor) != SOULPIT_SHA:
        raise ValueError('source digest drift: historical Soulpit witness')
    return predecessor


def original_marker(root):
    """Recover the exact old marker after independently qualified owner additions."""
    root = Path(root)
    raw = (root / MARKER).read_bytes()
    # Later publishers compose their annotations after Sugar and move Soulpit
    # to the end. Recover only the frozen receipt's annotation order; the hash
    # checks below still require every byte of the original marker.
    marker = json.loads(raw)
    if (json.dumps(marker, indent=2, ensure_ascii=False) + '\n').encode() != raw:
        raise ValueError('source digest drift: terrain marker serialization')
    for note in (
        ' Additionally M49 Earth altar ground7748 retains its canonical Item identity.',
        ' Additionally one source-qualified Rafzan elevator ground; total 8694 Terrain records.',
        ' Additionally one source-qualified Reliable Ram static appearance; total 8694 Terrain records.',
        ' Additionally SRX-3 pinned activity static geometry: 61 Terrain and 177 WorldObject records; no Item aliases.',
    ):
        marker['notes'] = marker['notes'].replace(note, '')
    if SOULPIT_NOTE in marker['notes']:
        marker['notes'] = marker['notes'].replace(SOULPIT_NOTE, '')
        marker['notes'] = marker['notes'].replace(NOTE, NOTE + SOULPIT_NOTE)
    raw = (json.dumps(marker, indent=2, ensure_ascii=False) + '\n').encode()
    feroxa = root / "content/world/terrain" / FEROXA_STATIC
    if feroxa.exists():
        path = Path(__file__).resolve().parents[1] / "world-object-authoring/feroxa_static.py"
        spec = importlib.util.spec_from_file_location("historical_feroxa_static", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        expected = (json.dumps({'family': 'Terrain', 'records': module.records(root)},
                               sort_keys=True, separators=(',', ':')) + '\n').encode()
        marker = json.loads(raw)
        if (feroxa.read_bytes() != expected or not marker["notes"].endswith(module.NOTE)
                or (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode() != raw):
            raise ValueError("source digest drift: Feroxa static World witness")
        marker["notes"] = marker["notes"].removesuffix(module.NOTE)
        raw = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    sugar = root / "content/world/terrain" / SUGAR_GROUND
    if sugar.exists():
        raw = without_sugar(root, raw)
    soulpit = root / "content/world/terrain" / SOULPIT
    if soulpit.exists():
        # Sugar is independently stripped first. The remaining marker and shard
        # must still be byte-exact W07 witnesses, never a newly blessed digest.
        try:
            soulpit_digest = sha(original_soulpit(root))
        except ValueError as error:
            raise ValueError('source digest drift: unrecognized Soulpit World owner inputs') from error
        if sha(raw) != SOULPIT_MARKER_SHA or soulpit_digest != SOULPIT_SHA:
            raise ValueError(
                "source digest drift: unrecognized Soulpit World owner inputs"
            )
        marker = json.loads(raw)
        if not marker["notes"].endswith(SOULPIT_NOTE):
            raise ValueError("source digest drift: Soulpit terrain marker note")
        marker["notes"] = marker["notes"].removesuffix(SOULPIT_NOTE)
        raw = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    ground = root / "content/world/terrain" / GROUND
    if sha(raw) == BASE_SHA and not ground.exists():
        return raw
    if (
        sha(raw) != MARKER_SHA
        or not ground.is_file()
        or sha(ground.read_bytes()) != GROUND_SHA
    ):
        raise ValueError("source digest drift: unrecognized Grizzly World owner inputs")
    marker = json.loads(raw)
    if not marker["notes"].endswith(NOTE):
        raise ValueError("source digest drift: Grizzly terrain marker note")
    marker["notes"] = marker["notes"].removesuffix(NOTE)
    original = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    if sha(original) != BASE_SHA:
        raise ValueError("source digest drift: original terrain marker")
    return original


def link_world(root, target, detached=()):
    """Replay the closed World input set; current-owner validation stays separate."""
    from check_stack_default_historical_context import frozen_archive
    original = original_marker(root)
    archived = frozen_archive()
    if archived[MARKER] != original:
        raise ValueError('source digest drift: historical World marker')
    target.mkdir(parents=True)
    for source in (root / "content/world").iterdir():
        if source.name in detached:
            continue
        destination = target / source.name
        if source.name not in {'terrain', 'objects'}:
            destination.symlink_to(source.resolve(), target_is_directory=source.is_dir())
            continue
        destination.mkdir()
        prefix = f'content/world/{source.name}/'
        for name, raw in archived.items():
            if name.startswith(prefix):
                (destination / Path(name).name).write_bytes(raw)


HIT_PACKET = "docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json"
HIT_SHA = "5f7665e304d31b5b9efff5e940e02c1355fef972d830766fa5035a2c558359a9"


def original_hit_packet(root):
    """Recover only the hash-exact historical receipt, including every Hit fact."""
    original_marker(root)
    raw = (root / HIT_PACKET).read_bytes()
    # Nested weapon replays already carry the exact original receipt. Terrain
    # remains independently checked above; never reinterpret its BASE marker as
    # a live Sugar footprint or accept an unpinned historical receipt.
    if sha(raw) == HIT_SHA:
        return raw
    packet = json.loads(raw)
    # Later map owners may change only the receipt's input footprint. Reprove
    # its complete live footprint and every frozen Hit fact before replaying
    # the independently hash-pinned historical receipt.
    current_inputs = {str(path.relative_to(root)): sha(path.read_bytes())
                      for family in ('objects', 'terrain')
                      for path in (root / 'content/world' / family).glob('*.json')}
    if packet['sources']['world_owner_inputs'] == current_inputs:
        from check_stack_default_historical_context import frozen_archive
        frozen_raw = frozen_archive()[HIT_PACKET]
        if sha(frozen_raw) != HIT_SHA:
            raise ValueError('source digest drift: frozen historical Hit packet')
        frozen = json.loads(frozen_raw)
        packet['sources']['world_owner_inputs'] = frozen['sources']['world_owner_inputs']
        candidate = (json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
        if sha(candidate) != HIT_SHA:
            raise ValueError('source digest drift: historical Hit facts')
        return frozen_raw
    inputs = packet["sources"]["world_owner_inputs"]
    ground = "content/world/terrain/" + GROUND
    soulpit = "content/world/terrain/" + SOULPIT
    sugar = "content/world/terrain/" + SUGAR_GROUND
    if sugar in inputs and inputs.get(MARKER) == sha((root / MARKER).read_bytes()):
        if inputs[sugar] != sha((root / sugar).read_bytes()):
            raise ValueError("source digest drift: historical Hit Sugar footprint")
        del inputs[sugar]
        if (root / soulpit).exists():
            if inputs.get(soulpit) != SOULPIT_SHA:
                raise ValueError(
                    "source digest drift: historical Hit Soulpit footprint"
                )
            del inputs[soulpit]
        inputs[MARKER] = MARKER_SHA
    elif (
        inputs.get(MARKER) == SOULPIT_MARKER_SHA and inputs.get(soulpit) == SOULPIT_SHA
    ):
        inputs[MARKER] = MARKER_SHA
        del inputs[soulpit]
    if inputs.get(MARKER) == MARKER_SHA and inputs.get(ground) == GROUND_SHA:
        inputs[MARKER] = BASE_SHA
        del inputs[ground]
    raw = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if sha(raw) != HIT_SHA:
        raise ValueError("source digest drift: historical Hit packet")
    return raw


def link_weapon_evidence(root, target):
    """Detach the historical Hit receipt; all other evidence stays live."""
    current = target
    source = root / "docs"
    for component in ("agents", "evidence", Path(HIT_PACKET).name):
        current.mkdir()
        for child in source.iterdir():
            if child.name != component:
                (current / child.name).symlink_to(
                    child.resolve(), target_is_directory=child.is_dir()
                )
        current = current / component
        source = source / component
    current.write_bytes(original_hit_packet(root))
