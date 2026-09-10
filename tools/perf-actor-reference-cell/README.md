# PERF-01 actor reference-cell evidence processor

This standalone Python-stdlib package validates retained physical evidence for
`PERF-01-ACTOR-CARRIER-CELL-V1`. It does not launch the benchmark, configure a
runner, choose populations, or produce an accepted/production capacity.

```bash
python3 tools/perf-actor-reference-cell/reference_cell.py INPUT.json
python3 tools/perf-actor-reference-cell/reference_cell.py INPUT.json --output RESULT.json
python3 tools/perf-actor-reference-cell/reference_cell.py INPUT.json --output RESULT.json --check
python3 -m unittest discover -s tools/perf-actor-reference-cell -p 'test_*.py'
```

Output is canonical JSON: ASCII escaped, keys sorted recursively, compact
separators, and one terminal newline. `--check` exits nonzero unless the output
file exactly matches those bytes.

The processor deliberately accepts the repository placeholder without any
capacity, returning `PLACEHOLDER_UNBOUND`; even that shortcut first validates
the exact schema version, contract, state, candidate and PERF-01 status. A
non-placeholder packet must be `MEASUREMENT_INCOMPLETE`, carry no pre-populated
capacity, and supply
every contract input, exact frozen and observed fingerprints, read-back process
enforcement, retained samples, progressive repetitions, soak, coupled limits,
and atomic `M`/`M+1` evidence. Validation errors are machine-readable on stderr
and exit status is `2`.

Executable validation is independent of JSON Schema invocation. It enforces the
named runner constants, physical-evidence identity on both sides of comparison,
lowest effective allowed CPU selection and singleton readback, matching
fingerprint/enforcement `RLIMIT_AS`, native-process proof, and a strictly
increasing positive-u64 population plan.

The fingerprint digest covers only the schema's sanitized fields. Never add
hostnames other than the approved runner name, serial numbers, addresses,
credentials, secret environment data, identifying mount paths, or raw `/proc`
and runner-registration contents.
