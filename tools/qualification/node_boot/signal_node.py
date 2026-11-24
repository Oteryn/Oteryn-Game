#!/usr/bin/env python3
"""Signal a tracked node without following a recycled PID."""
from pathlib import Path
import os
import signal
import sys


def signal_node(record, requested):
    pid = int(record.name)
    if pid <= 0:
        return 1
    try:
        birth = record.read_text().strip()
        fd = os.pidfd_open(pid)
        try:
            fields = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
            if fields[0] == "Z" or fields[19] != birth:
                return 1
            signal.pidfd_send_signal(fd, requested)
        finally:
            os.close(fd)
    except (FileNotFoundError, ProcessLookupError):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(signal_node(Path(sys.argv[1]), {"-0": 0, "-TERM": signal.SIGTERM}[sys.argv[2]]))
