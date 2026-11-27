"""Select real capture replays using evidence from this invocation of node_boot."""
from pathlib import Path
import re
import sys


def select(log, captures, ignored, listing):
    writers = set(re.findall(
        r"(?m)^SEAM_EVIDENCE (?:e2e_all|e2e_capture) scenario=(\S+) "
        r"status=(?:PASS|CAPTURED|FAIL) ",
        log.read_text(),
    ))
    registered = {}
    for line in Path(__file__).with_name("capture_replays.txt").read_text().splitlines():
        if line and not line.startswith("#"):
            writer, name = line.split()
            registered[name] = writer
    excluded = set(ignored.read_text().splitlines())
    for line in listing.read_text().splitlines():
        if not line.endswith(": test") or line in excluded:
            continue
        name = line.removesuffix(": test")
        writer = registered.get(name.rsplit("::", 1)[-1])
        if writer is None or writer in writers:
            yield name
        elif writer.startswith("file:") and (captures / writer[5:]).is_file():
            yield name


if __name__ == "__main__":
    for test in select(*map(Path, sys.argv[1:])):
        print(test)
