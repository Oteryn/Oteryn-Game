"""Closed, source-only movement proof for one exact Canary revision pair.

This transfers an existing authoring descriptor, not engine/API qualification.
The only global.lua difference is the exact PvP function replacement below;
all other bytes, including rope tables and startup effects, stay identical.
There is no generic drift acceptance, regex extraction or Lua execution.
"""
import hashlib
import os
from pathlib import Path
import subprocess

import native_house_movement as native

OLD_REVISION = native.REVISIONS['canary']
REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
GLOBAL_PATH = 'data/global.lua'
CURRENT_GLOBAL_SHA = '449b703a6ba45b8ff2daa26ad05d1553454e4707552648308fb4eb6063bdb3c5'
OLD_PVP = b'function IsRetroPVP()\n\treturn configManager.getBoolean(configKeys.TOGGLE_SERVER_IS_RETRO)\nend\n'
NEW_PVP = b'''local function getWorldType()
\treturn string.lower(configManager.getString(configKeys.WORLD_TYPE) or "")
end

function IsRetroPVP()
\tlocal worldType = getWorldType()
\treturn worldType == "retro-pvp" or worldType == "pvp"
end

function IsExpertPVP()
\treturn getWorldType() == "expert-pvp"
end
'''
FILES = tuple(native.SUPPORT_FILES['vertical_move'])


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(repo, revision, path):
    return subprocess.check_output(['git', '-C', str(repo), 'show', revision + ':' + path],
                                   env=dict(os.environ, GIT_NO_LAZY_FETCH='1'))


def qualify(name, current, qualified, revision=REVISION):
    """Require full pinned casts/support bytes and prove the single exact delta."""
    name = name.casefold()
    if name not in ('levitate', 'magic rope') or revision != REVISION:
        raise ValueError('movement closure is restricted to two exact Canary registrations')
    spell_path = native.FILES[name]
    paths = {spell_path, *FILES}
    if set(current) != paths or set(qualified) != paths:
        raise ValueError('movement closure file set mismatch')
    if (current[spell_path] != qualified[spell_path]
            or sha(current[spell_path]) != native.DIGESTS[name]):
        raise ValueError('movement full cast bytes changed')
    proofs = []
    for path in FILES:
        old, new = qualified[path], current[path]
        expected = native.SUPPORT_DIGESTS['canary', path]
        if sha(old) != expected:
            raise ValueError('movement qualified support bytes changed: ' + path)
        if path == GLOBAL_PATH:
            if (sha(new) != CURRENT_GLOBAL_SHA or old.count(OLD_PVP) != 1
                    or new.count(NEW_PVP) != 1 or old.replace(OLD_PVP, NEW_PVP) != new):
                raise ValueError('movement global delta outside exact PvP replacement')
        elif old != new:
            raise ValueError('movement helper bytes changed: ' + path)
        proofs.append({'path': path, 'qualified_sha256': expected, 'current_sha256': sha(new),
                       'full_bytes_equal': old == new})
    # Exact pinned files already established; this makes the reviewed separation explicit.
    for path in paths - {GLOBAL_PATH}:
        if any(symbol in current[path] for symbol in (b'IsRetroPVP', b'IsExpertPVP', b'getWorldType')):
            raise ValueError('movement helper refers to changed PvP functions')
    return {'source': 'canary', 'qualified_template_revision': OLD_REVISION,
            'current_source_revision': revision, 'full_cast_path': spell_path,
            'full_cast_sha256': sha(current[spell_path]), 'full_cast_bytes_equal': True,
            'support_files': proofs, 'global_delta': {'only_change': 'exact_PvP_function_replacement',
            'old_block_sha256': sha(OLD_PVP), 'new_block_sha256': sha(NEW_PVP),
            'remaining_global_bytes_equal': True,
            'changed_symbols_absent_from_cast_tile_position': True},
            'scope': 'existing_movement_descriptor_source_transfer_only',
            'engine_and_transitive_API_execution_qualified': False,
            'runtime_activation': False, 'external_sources_used': False}


def build(repo, name):
    paths = {native.FILES[name.casefold()], *FILES}
    current = {path: read(Path(repo), REVISION, path) for path in paths}
    old = {path: read(Path(repo), OLD_REVISION, path) for path in paths}
    return qualify(name, current, old), current, old
