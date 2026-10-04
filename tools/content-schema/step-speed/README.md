# Step-speed table generator (SPEED-1)

Generates `content/movement/step_speed_v1.json`, the u16 table of CONDITIONS-0 §4.2: for every
clamped effective speed from 10 to 65,535 it holds Canary's step speed
`floor(857.36 × ln(speed + 261.29) − 4795.01 + 0.5)` (Canary `04b83b51`, `creature.hpp:76-78,
1061-1067`, `OTS_HYPOTHESIS_ONLY`). The game server (`apps/game-server/src/movement/speed.rs`)
and the client (`apps/client/src/input.rs`) read the table; neither evaluates `ln`.

`sha256_u16le` is the SHA-256 of the table as consecutive little-endian u16 values. The game
server checks it when it loads the table, and both test suites pin it.

| File | Purpose |
|---|---|
| `generate_step_speed_table.py` | Writes the table; `--check` verifies the committed file. Run once, offline. |
| `test_generate_step_speed_table.py` | The committed file equals the generated one; range, digest, Canary samples, monotonicity, no value on a rounding boundary. |

```sh
python3 tools/content-schema/step-speed/generate_step_speed_table.py --check
python3 -m unittest tools/content-schema/step-speed/test_generate_step_speed_table.py
```
