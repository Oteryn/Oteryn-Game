# FND-02 terminal-outcome retention evidence

This directory contains the deterministic evidence-only harness for
FND02_TERMINAL_OUTCOME_RETENTION_RESOURCE_EVIDENCE_663.

It does not implement Foundation runtime storage, mutate the resource registry,
choose a Rust layout, register protocol payloads, or grant CW4 resume authority.

## What the harness proves

- The accepted first-playable A/1 -> B/1 -> replay A/1 witness spans two valid
  GameSessions. Its demonstrated retention lower bound is therefore one terminal
  record per GameSession, not two.
- Zero retained records cannot satisfy that exact replay witness.
- One retained record per GameSession is sufficient for that exact witness.
- Same normalized intent plus original binding replays the retained outcome
  without another modeled gameplay mutation.
- Changed normalized intent or binding conflicts.
- Terminal-only eviction is deterministic. Evicted CommandRefs become expired
  and remain non-reservable because the ingress high-water is unchanged.
- Pending commands are never evicted to make room for terminal retention.
- Count and byte boundary arithmetic is checked before modeled gameplay mutation.
- A later CommandId cannot terminalize ahead of an earlier pending CommandId.

## Byte-accounting boundary

The model uses explicit synthetic logical widths only to exercise arithmetic:

- fixed store charge: 32 bytes;
- fixed record charge: 40 bytes;
- variable charge: normalized intent + binding + outcome bytes.

Those widths are not Python object size, process RSS, Rust ABI size, wire size,
or a proposed production charging rule.

The protected 65,536-byte ClientCommand and CommandResult payload limits do not
bound the Foundation-owned normalized intent plus original binding evidence plus
terminal semantic outcome resident record. Issue #663 deliberately leaves the
physical representation unfrozen.

Therefore the evidence packet proposes count=1 for the current first-playable
slice but returns BLOCKED_ON_EVIDENCE for aggregate charged resident bytes.

The smallest missing discriminator is:

FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND

That evidence must define or measure one accepted complete retained semantic
record charge, including all simultaneously retained owned copies and metadata.
It must not copy fixture 8, FND02-OUTSTANDING-COMMANDS=64, NET03 budgets, either
65,536-byte wire limit, or arbitrary headroom.

## Run

From repository root:

    python -m py_compile tools/fnd02-terminal-outcome-retention-evidence/model.py tools/fnd02-terminal-outcome-retention-evidence/generate.py tools/fnd02-terminal-outcome-retention-evidence/self_test.py
    python tools/fnd02-terminal-outcome-retention-evidence/self_test.py
    python tools/fnd02-terminal-outcome-retention-evidence/generate.py --check-json docs/agents/evidence/OTV2-20260918-fnd02-terminal-outcome-retention-resource-evidence.json --check-md docs/agents/evidence/OTV2-20260918-fnd02-terminal-outcome-retention-resource-evidence.md

Expected focused result:

    PASS 14 tests
    PASS evidence JSON matches deterministic generator
    PASS evidence Markdown matches deterministic generator
