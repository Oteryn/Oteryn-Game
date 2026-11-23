"""Replay retained real CI result.json values through the selected matrix source."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--fixtures", type=Path, default=Path(__file__).parent / "real_artifact_results.json")
args = parser.parse_args()
source = args.repo.resolve() / "tools/qualification/node_boot/matrix.py"
spec = importlib.util.spec_from_file_location("matrix_replayed", source)
matrix = importlib.util.module_from_spec(spec)
spec.loader.exec_module(matrix)

observations = []
valid = True
for fixture in json.loads(args.fixtures.read_text()):
    name = fixture["name"]
    _, first, last = name.split("-")
    plan = matrix.plan("custom", rows=f"{int(first)}:{int(last)}")
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        (directory / "result.json").write_text(json.dumps(fixture["result"]))
        _, problems = matrix.summarize(plan, directory, "", missing_fatal=False)
    if name == "rows-000-010":
        accepted = not problems
    elif name == "rows-080-090":
        accepted = any("missing terminal verdict for C01.client" in p for p in problems)
    elif name == "rows-220-230":
        accepted = any("missing terminal verdict for C20a.client" in p for p in problems)
    else:
        accepted = bool(problems)
    valid = valid and accepted
    observations.append({"shard": name, "job_id": fixture["job_id"],
                         "problems": problems, "meets_expected_completeness": accepted})
print(json.dumps(observations, indent=2))
raise SystemExit(0 if valid else 1)
