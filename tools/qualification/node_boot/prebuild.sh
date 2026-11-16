#!/usr/bin/env bash
# Build the reusable node-boot artifacts from a clean source checkout.
set -Eeuo pipefail
[[ $# == 1 ]] || { echo 'usage: prebuild.sh <out_dir>' >&2; exit 2; }
GAME_SOURCE="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$GAME_SOURCE"
[[ -z "$(git status --porcelain)" ]] || { echo 'prebuild requires a clean tree' >&2; exit 2; }
SOURCE_SHA="$(git rev-parse HEAD)"
# Read the single literal declaration without executing the harness.
eval "$(sed -n '/^readonly NODE_BOOT_WORLD_REGIONS=/p' tools/qualification/node_boot/run.sh)"
OUT="$(realpath -m -- "$1")"
mkdir -p "$OUT/world"
rm -f "$OUT/manifest.json"
for variant in default qualification-clock; do
  features=()
  [[ "$variant" == default ]] || features=(--features qualification-clock)
  mkdir -p "$OUT/$variant"
  cargo +1.100.0 build --locked -p oteryn-game-server --bins "${features[@]}" --message-format=json > "$OUT/$variant/bins.jsonl"
  cargo +1.100.0 test --locked -p oteryn-game-server --lib --no-run "${features[@]}" --message-format=json > "$OUT/$variant/artifacts.jsonl"
  python3 - "$OUT/$variant" <<'PY_ARTIFACT'
import json, pathlib, shutil, sys
out = pathlib.Path(sys.argv[1])
artifacts = [json.loads(line) for line in (out / 'artifacts.jsonl').read_text().splitlines()]
tests = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
         and a.get('target', {}).get('name') == 'oteryn_game_server'
         and 'lib' in a['target']['kind'] and a.get('profile', {}).get('test') and a.get('executable')]
if len(tests) != 1:
    raise SystemExit(f'expected one game-server lib test artifact, got {len(tests)}')
shutil.copy2(tests[0], out / 'oteryn-game-server-lib-test')
bins = [json.loads(line) for line in (out / 'bins.jsonl').read_text().splitlines()]
for name in ('oteryn-game-server', 'oteryn-game-ops', 'oteryn-game-migrate'):
    matches = [a['executable'] for a in bins if a.get('reason') == 'compiler-artifact'
               and a.get('target', {}).get('name') == name and a.get('executable')]
    if len(matches) != 1:
        raise SystemExit(f'expected one {name} artifact, got {len(matches)}')
    shutil.copy2(matches[0], out / name)
(out / 'artifacts.jsonl').unlink()
(out / 'bins.jsonl').unlink()
PY_ARTIFACT
done
# The full-manifest preflight also needs a test harness without invoking Cargo on shards.
cargo +1.100.0 test --locked -p oteryn-gameplay-content --lib --no-run --message-format=json > "$OUT/spell-artifacts.jsonl"
python3 - "$OUT" <<'PY_SPELL'
import json, pathlib, shutil, sys
out = pathlib.Path(sys.argv[1])
artifacts = [json.loads(line) for line in (out / 'spell-artifacts.jsonl').read_text().splitlines()]
tests = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
         and a.get('target', {}).get('name') == 'oteryn_gameplay_content'
         and 'lib' in a['target']['kind'] and a.get('profile', {}).get('test') and a.get('executable')]
if len(tests) != 1:
    raise SystemExit(f'expected one content lib test artifact, got {len(tests)}')
shutil.copy2(tests[0], out / 'spell-map-lib-test')
(out / 'spell-artifacts.jsonl').unlink()
PY_SPELL
cargo +1.100.0 run --locked --release -p oteryn-world-bundle-compiler -- compile-regions "$GAME_SOURCE" "$GAME_SOURCE/content/world/pins/oteryn.identity.json" "$OUT/world/world.otwb" "$NODE_BOOT_WORLD_REGIONS" > "$OUT/world/world-compile.json"
python3 - "$OUT/manifest.json" "$SOURCE_SHA" "$NODE_BOOT_WORLD_REGIONS" <<'PY_MANIFEST'
import json, sys
with open(sys.argv[1], 'w') as out:
    json.dump(dict(source_sha=sys.argv[2], regions=sys.argv[3],
                   variants=['default', 'qualification-clock'], toolchain='1.100.0'), out, indent=2)
    out.write('\n')
PY_MANIFEST
