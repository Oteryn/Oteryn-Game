import json, os, pathlib, subprocess, sys, time
name, *command = sys.argv[1:]
started = time.monotonic()
env = dict(os.environ, CARGO_BUILD_JOBS="3")
with pathlib.Path(f".questpc7/{name}.log").open("w") as out:
    result = subprocess.run(command, env=env, stdout=out, stderr=subprocess.STDOUT)
row = {"command":command,"exit":result.returncode,"duration_seconds":round(time.monotonic()-started,3)}
pathlib.Path(f".questpc7/{name}.result.json").write_text(json.dumps(row,indent=2)+"\n")
print(json.dumps(row))
sys.exit(result.returncode)
