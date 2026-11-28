"""Keep an item authoring packet and its Rust consumer's digest in sync."""

import hashlib
import re


def sync_rust_digest(path, constant, data, *, check):
    source = path.read_text()
    digest = hashlib.sha256(data).hexdigest()
    pinned, count = re.subn(
        rf'(pub const {re.escape(constant)}: &str =\s*")[0-9a-f]{{64}}(";)',
        lambda match: f"{match[1]}{digest}{match[2]}",
        source,
    )
    if count != 1:
        raise SystemExit(f"expected one Rust digest declaration for {constant}")
    if pinned != source:
        if check:
            raise SystemExit(f"stale Rust packet digest: {constant}")
        path.write_text(pinned)
