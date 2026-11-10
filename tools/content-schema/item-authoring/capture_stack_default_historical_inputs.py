"""Package hash-exact historical inputs for offline, shallow-checkout replay.

This captures existing qualification evidence, never current Native authority.
Only the explicitly pinned merge base can supply a missing historical byte.
"""
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile

import check_stack_default_historical_context as context
import historical_world_owner_context as world


def main():
    root = context.ROOT
    receipt = context.read_context(root)
    packet_path = str(context.base.OUTPUT.relative_to(root))
    packet = json.loads(context.base.checked(root, packet_path, receipt['immutable_artifacts'][packet_path]))
    expected = dict(packet['sources']['map_owner_inputs'])
    expected[context.base.COMPILER] = packet['compiler']['sha256']
    expected[context.base.BINDINGS] = packet['sources']['bindings_sha256']
    expected[context.base.WIKI] = '24e3e57fddc6c14f00aa5c4976e91117d31bc2102bd3835b3d7891d9e41fbeae'
    captured = {}

    def historical(path):
        return subprocess.check_output(['git', 'show', f'{context.FROZEN_INPUT_PARENT}:{path}'], cwd=root)

    for path, digest in expected.items():
        raw = historical(path)
        if hashlib.sha256(raw).hexdigest() != digest and path == world.MARKER:
            raw = world.original_marker(root)
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError('historical source differs from qualification: ' + path)
        captured[path] = raw
    # Reproduce the separately retained pre-Sugar Hit receipt using its existing
    # owner, with the exact main-era marker and catalogues it originally saw.
    with tempfile.TemporaryDirectory() as directory:
        view = Path(directory)
        for path in (world.MARKER, world.HIT_PACKET, *(
                'content/world/terrain/' + name
                for name in (world.GROUND, world.SOULPIT, world.SUGAR_GROUND))):
            target = view / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(historical(path))
        captured[world.HIT_PACKET] = world.original_hit_packet(view)
    if hashlib.sha256(captured[world.HIT_PACKET]).hexdigest() != world.HIT_SHA:
        raise ValueError('historical Hit receipt changed')
    archive = io.BytesIO()
    with gzip.GzipFile(fileobj=archive, mode='wb', filename='', mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode='w', format=tarfile.USTAR_FORMAT) as stream:
            for name, raw in sorted(captured.items()):
                member = tarfile.TarInfo(name)
                member.size = len(raw)
                member.mode = 0o644
                stream.addfile(member, io.BytesIO(raw))
    raw = archive.getvalue()
    (root / context.FROZEN_INPUT_ARCHIVE).write_bytes(raw)
    source = Path(context.__file__)
    text, count = re.subn(r'(FROZEN_INPUT_ARCHIVE_SHA256 = ")[0-9a-f]{64}(")',
                         lambda match: match[1] + hashlib.sha256(raw).hexdigest() + match[2],
                         source.read_text())
    if count != 1:
        raise ValueError('historical archive pin declaration changed')
    source.write_text(text)
    print(f'Historical inputs: {len(captured)} files, {len(raw)} compressed bytes')


if __name__ == '__main__':
    main()
