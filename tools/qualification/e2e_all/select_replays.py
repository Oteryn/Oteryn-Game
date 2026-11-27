"""Select real capture replays using evidence from this invocation of node_boot."""
from pathlib import Path
import re
import sys


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "node_boot"))
from row_selection import selected_rows


def capture_writers(log):
    return set(re.findall(
        r"(?m)^SEAM_EVIDENCE (?:e2e_all|e2e_capture) scenario=(\S+) "
        r"status=(?:PASS|CAPTURED)(?: |$)",
        log.read_text(),
    ))


def registrations():
    registered = {}
    for line in Path(__file__).with_name("capture_replays.txt").read_text().splitlines():
        if line and not line.startswith("#"):
            writer, name = line.split()
            registered[name] = writer
    return registered


def expected_writers(registered):
    return set(selected_rows()) & set(registered.values())


def select(log, captures, ignored, listing):
    registered = registrations()
    writers = capture_writers(log)
    missing = expected_writers(registered) - writers
    if missing:
        raise SystemExit("\n".join(f"{row}: missing success verdict" for row in sorted(missing)))
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


def verify_passes(logs):
    registered = registrations()
    expected = expected_writers(registered)
    passed = {
        registered.get(name.rsplit("::", 1)[-1])
        for log in logs if log.is_file()
        for name in re.findall(r"(?m)^test (\S+) \.\.\. ok$", log.read_text())
    }
    if missing := expected - passed:
        raise SystemExit("\n".join(f"{row}: no passing capture replay" for row in sorted(missing)))


if __name__ == "__main__":
    if sys.argv[1] == "--verify-passes":
        verify_passes(map(Path, sys.argv[2:]))
    else:
        for test in select(*map(Path, sys.argv[1:])):
            print(test)
