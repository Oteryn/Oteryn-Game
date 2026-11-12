"""Select the explicit file closure consumed by native cosmetics provisioning."""
import hashlib
import json
from pathlib import PurePosixPath

# Match native_cosmetics/provisioning.rs and cosmetics_authoring.py. QENGINE2's
# source-backed catalogue (804,946 bytes) outgrew the historical NX-5 sample cap.
MAX_AUTHORING_BYTES = 8 * 1024 * 1024


def select_cosmetics_inputs(document, manifest, repo_root, selected):
    pin = document.get("cosmetics")
    if pin is None:
        return
    locator = (manifest.parent / pin["path"]).resolve(strict=True)
    descriptor = json.loads(selected[locator.relative_to(repo_root).as_posix()])
    # Historical inline sections have no filesystem dependencies. The descriptor
    # closure and bounds mirror native_cosmetics/provisioning.rs:44-106.
    if descriptor.get("schema") != "OTERYN_NATIVE_COSMETICS_INPUT/v1":
        return

    def select(directory, file_pin, limit):
        raw = file_pin["path"]
        path = PurePosixPath(raw)
        if not raw or path.is_absolute() or "\\" in raw:
            raise ValueError("unsafe cosmetics locator")
        source = directory.joinpath(*path.parts).resolve(strict=True)
        if not source.is_relative_to(repo_root):
            raise ValueError("cosmetics input escapes repository root")
        with source.open("rb") as handle:
            data = handle.read(limit + 1)
        if not data or len(data) > limit:
            raise ValueError("cosmetics input exceeds provisioning bound")
        if hashlib.sha256(data).hexdigest() != file_pin["sha256"]:
            raise ValueError("cosmetics input digest mismatch")
        name = source.relative_to(repo_root).as_posix()
        if name in selected and selected[name] != data:
            raise ValueError("conflicting cosmetics input")
        selected[name] = data
        return source, data

    select(locator.parent, descriptor["authoring"], MAX_AUTHORING_BYTES)
    source, data = select(locator.parent, descriptor["source_manifest"], 16 * 1024)
    total = len(data)
    for file_pin in json.loads(data)["files"]:
        name = file_pin["path"]
        if not name.endswith(".json") or not all(
            c.isascii() and (c.isalnum() or c in "_.-") for c in name
        ):
            raise ValueError("unsafe cosmetics source shard locator")
        _, shard = select(source.parent, file_pin, 8 * 1024 * 1024)
        total += len(shard)
        if total > 32 * 1024 * 1024:
            raise ValueError("cosmetics source aggregate bounds")
