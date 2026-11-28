"""Select G10's native client test executable for a live row batch."""
import json
import re
import sys


def selected(batch):
    """Match the Rust row registry's batch semantics for index 242."""
    if batch == "all":
        return True
    if re.fullmatch(r"\+?[0-9]+:\+?[0-9]+", batch) is None:
        return False
    start, length = map(int, batch.split(":"))
    return start <= 242 and 242 - start < length <= sys.maxsize * 2 + 1


def executable():
    paths = []
    for line in sys.stdin:
        artifact = json.loads(line)
        if (artifact.get("reason") == "compiler-artifact"
                and artifact.get("target", {}).get("name") == "oteryn_client"
                and artifact.get("profile", {}).get("test")
                and artifact.get("executable")):
            paths.append(artifact["executable"])
    if len(paths) != 1:
        raise SystemExit("G10 replay build must produce exactly one client test executable")
    print(paths[0])


if __name__ == "__main__":
    if sys.argv[1] == "selected":
        raise SystemExit(0 if selected(sys.argv[2]) else 1)
    executable()
