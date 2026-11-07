#!/usr/bin/env python3
"""Stage the pinned NPC catalogue the node loads with --npc-data-project/--npc-data-sha256.

usage: stage_npc_data.py <pin> <repository-root> <destination>

The destination is a fresh directory holding exactly what the node reads (NPC-LOAD-1,
crates/gameplay/content/src/content/npc_catalogue.rs and crates/world-bundle/src/npc.rs):

  content/world/{project.json,manifest.json,content.lock.json}  the WorldProject control files
  content/world/<locator>                                       the manifest's documents only
  content/{npcs/definitions,dialogues/definitions,services}/    the catalogue the bundle was compiled against
  PROJECT_SHA256                                                the --npc-data-sha256 value (pin)

--npc-data-project is DEST/content/world; the node hashes DEST/content as the compiler does. Every
file is checked before it is written: the control files and documents against the sha256 and byte
length of project.json and manifest.json, the project revision against the pin, and the catalogue
files against the pinned catalogue_sha256 (npc.rs catalogue_digest). Any difference raises.
"""
import hashlib
import json
import pathlib
import re
import struct
import sys

PIN_SCHEMA = "OTERYN_NPC_DATA_PIN/v1"
PROJECT_SCHEMA = "OTERYN_WORLD_PROJECT_ROOT/v2"
MANIFEST_SCHEMA = "OTERYN_WORLD_PROJECT_MANIFEST/v2"
# crates/world-bundle/src/npc.rs CATALOGUE_DIRS, relative to the content root.
CATALOGUE_DIRS = ("npcs/definitions", "dialogues/definitions", "services")
# crates/gameplay/content/src/content/npc_catalogue.rs: max_documents 11 = 3 control files + 8 documents.
MAX_DOCUMENTS = 11
HEX64 = re.compile(r"^[0-9a-f]{64}$")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def safe_locator(raw):
    """A relative, normalised, slash-separated path without dot segments or backslashes."""
    if not isinstance(raw, str):
        raise ValueError(f"unsafe locator {raw!r}")
    locator = pathlib.PurePosixPath(raw)
    if (
        locator.is_absolute()
        or "\\" in raw
        or not locator.parts
        or any(part in ("", ".", "..") for part in locator.parts)
        or locator.as_posix() != raw
    ):
        raise ValueError(f"unsafe locator {raw!r}")
    return raw


def read_regular(root, relative):
    """Bytes of a regular file under root, never through a symlink."""
    path = root.joinpath(*pathlib.PurePosixPath(relative).parts)
    current = root
    for part in pathlib.PurePosixPath(relative).parts:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"symlink in {relative}")
    if not path.is_file():
        raise ValueError(f"missing regular file {relative}")
    return path.read_bytes()


def catalogue_digest(files):
    """npc.rs catalogue_digest: path-sorted (u64 BE length, path, u64 BE length, bytes) records."""
    hasher = hashlib.sha256()
    for path, data in sorted(files, key=lambda item: item[0].encode()):
        encoded = path.encode()
        hasher.update(struct.pack(">Q", len(encoded)) + encoded)
        hasher.update(struct.pack(">Q", len(data)) + data)
    return hasher.hexdigest()


def collect_catalogue(content):
    """Every regular file under CATALOGUE_DIRS as (path relative to content, bytes)."""
    files = []
    for directory in CATALOGUE_DIRS:
        base = content / directory
        if base.is_symlink():
            raise ValueError(f"symlink {directory}")
        if not base.is_dir():
            continue  # npc.rs: a missing directory contributes nothing
        for path in sorted(base.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"symlink {path.relative_to(content).as_posix()}")
            if path.is_file():
                files.append((path.relative_to(content).as_posix(), path.read_bytes()))
    return files


def load_pin(path):
    pin = json.loads(pathlib.Path(path).read_bytes())
    if pin.get("schema") != PIN_SCHEMA:
        raise ValueError("pin schema")
    for key in ("project_sha256", "catalogue_sha256"):
        if not isinstance(pin.get(key), str) or not HEX64.match(pin[key]):
            raise ValueError(f"pin {key} must be 64 lowercase hex digits")
    if not isinstance(pin.get("project_revision"), str) or not pin["project_revision"]:
        raise ValueError("pin project_revision")
    return pin


def project_files(world):
    """The control files and manifest documents as (locator, bytes), each digest-checked."""
    project_bytes = read_regular(world, "project.json")
    project = json.loads(project_bytes)
    if project.get("schema") != PROJECT_SCHEMA:
        raise ValueError("project.json schema")
    selected = {"project.json": project_bytes}
    manifest_locator = safe_locator(project["manifest_locator"])
    lock_locator = safe_locator(project["content_lock_locator"])
    manifest_bytes = read_regular(world, manifest_locator)
    lock_bytes = read_regular(world, lock_locator)
    if sha256(manifest_bytes) != project["manifest_sha256"]:
        raise ValueError("manifest digest mismatch")
    if sha256(lock_bytes) != project["content_lock_sha256"]:
        raise ValueError("content lock digest mismatch")
    selected[manifest_locator] = manifest_bytes
    selected[lock_locator] = lock_bytes
    manifest = json.loads(manifest_bytes)
    if manifest.get("schema") != MANIFEST_SCHEMA:
        raise ValueError("manifest schema")
    for document in manifest["documents"]:
        locator = safe_locator(document["locator"])
        data = read_regular(world, locator)
        if len(data) != document["byte_length"] or sha256(data) != document["sha256"]:
            raise ValueError(f"document {locator} does not match the manifest")
        if selected.setdefault(locator, data) != data:
            raise ValueError(f"conflicting document {locator}")
    if len(selected) > MAX_DOCUMENTS:
        raise ValueError("project has more documents than the NPC catalogue limit")
    return project["project_revision"], selected


def stage(pin_path, repo_root, destination):
    pin = load_pin(pin_path)
    repo = pathlib.Path(repo_root).resolve(strict=True)
    content = repo / "content"
    revision, project = project_files(content / "world")
    if revision != pin["project_revision"]:
        raise ValueError(f"project revision {revision} is not the pinned {pin['project_revision']}")
    catalogue = collect_catalogue(content)
    if not catalogue:
        raise ValueError("no catalogue files")
    got = catalogue_digest(catalogue)
    if got != pin["catalogue_sha256"]:
        raise ValueError(f"catalogue_sha256 {got} is not the pinned {pin['catalogue_sha256']}")
    staged = pathlib.Path(destination)
    staged.mkdir(mode=0o755, parents=True)
    for locator, data in project.items():
        write(staged / "content" / "world" / locator, data)
    for locator, data in catalogue:
        write(staged / "content" / locator, data)
    # The value supervisor.sh passes as --npc-data-sha256; the node recomputes it at boot.
    write(staged / "PROJECT_SHA256", (pin["project_sha256"] + "\n").encode())
    return pin


def write(path, data):
    path.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    path.write_bytes(data)
    path.chmod(0o644)


def main(argv):
    if len(argv) != 4:
        raise SystemExit("usage: stage_npc_data.py <pin> <repository-root> <destination>")
    pin = stage(argv[1], argv[2], argv[3])
    print(
        f"npc-data staged revision={pin['project_revision']} "
        f"project_sha256={pin['project_sha256']} catalogue_sha256={pin['catalogue_sha256']}"
    )


if __name__ == "__main__":
    main(sys.argv)
