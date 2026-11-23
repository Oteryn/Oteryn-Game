#!/usr/bin/env python3
"""Run the original StageDumpTests oracle and retain its ignored process result."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--ref", default="8e9f0e16631aab1beb8416f8d5ca707aa3031083")
    args = parser.parse_args()
    repo = args.repo.resolve()
    prefix = "tools/qualification/node_boot/"

    def source(name):
        return subprocess.check_output(
            ["git", "-C", str(repo), "show", f"{args.ref}:{prefix}{name}"], text=True
        )

    with tempfile.TemporaryDirectory() as tmp:
        run_sh = Path(tmp, "run.sh")
        run_sh.write_text(source("run.sh"))
        old_value = os.environ.get("NODE_BOOT_RUN_SH")
        os.environ["NODE_BOOT_RUN_SH"] = str(run_sh)
        module = {"__file__": str(repo / prefix / "test_node_boot_tools.py"),
                  "__name__": "audit_original_stage_dump"}
        try:
            exec(compile(source("test_node_boot_tools.py"), module["__file__"], "exec"), module)
        finally:
            if old_value is None:
                del os.environ["NODE_BOOT_RUN_SH"]
            else:
                os.environ["NODE_BOOT_RUN_SH"] = old_value
        calls = []
        original_bash = module["bash"]

        def observe(*positional, **keyword):
            result = original_bash(*positional, **keyword)
            calls.append({"returncode": result.returncode, "stdout": result.stdout,
                          "stderr": result.stderr})
            return result

        module["bash"] = observe
        case = module["StageDumpTests"](
            "test_each_stage_end_dumps_postgres_and_a_failed_stage_is_verbose"
        )
        result = unittest.TestResult()
        case.run(result)
        print(json.dumps({"ref": args.ref, "tests_run": result.testsRun,
                          "test_passes": result.wasSuccessful(),
                          "errors": result.errors, "failures": result.failures,
                          "observed_stage_processes": calls}, indent=2))


if __name__ == "__main__":
    main()
