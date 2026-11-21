"""Run a complete migration check inventory in bounded, isolated subprocesses."""
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
import subprocess


@dataclass(frozen=True)
class Check:
    name: str
    command: tuple[str, ...]
    cwd: Path | None = None


def run_checks(checks: list[Check], *, jobs: int) -> None:
    if not checks or type(jobs) is not int or not 1 <= jobs <= 4:
        raise ValueError("migration checks require an inventory and 1..4 workers")
    names = [check.name for check in checks]
    if (len(set(names)) != len(names) or any(not name for name in names)
            or any(not check.command for check in checks)):
        raise ValueError("migration check names must be unique and commands nonempty")

    def run(check):
        print(f"START migration-check={check.name}", flush=True)
        try:
            result = subprocess.run(check.command, cwd=check.cwd, check=False)
            code = result.returncode
        except OSError as error:
            print(f"Cannot launch migration-check={check.name}: {error}", flush=True)
            code = 127
        print(f"{'PASS' if code == 0 else 'FAIL'} migration-check={check.name} exit={code}", flush=True)
        return code

    failures = {}
    # Threads schedule OS processes only. Python module state, mocks and fixture
    # mutations never cross check boundaries, and a failure does not skip siblings.
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        pending = {pool.submit(run, check): check.name for check in checks}
        for future in as_completed(pending):
            code = future.result()
            if code != 0:
                failures[pending[future]] = code
    if failures:
        details = ", ".join(f"{name}={code}" for name, code in sorted(failures.items()))
        raise RuntimeError(f"migration checks failed: {details}")
    print(f"PASS migration-checks={len(checks)} workers={jobs}", flush=True)
