# Local completion import

`quest_completion_import.py` produces an **unactivated** candidate in the existing
QuestState loader format. It preserves the production `quest-state.json` and
all 352 canonical definitions. No second progress store or runtime is added.

The base is the exact `main@ad7a08f96caa4e7bd0e7fa90c67b39229d637277`
catalogue stored in `../state-effect-refinements/main-state.json`. Five finite
Source-proved effects are refined, then 68 explicitly chosen progress profiles
are appended. Existing native Rust loader and pure evaluator have qualified the
actual result; this does not prove dialogue dispatch, rewards or persistence.

From the `quest-authoring` directory:

```sh
python quest_completion_import.py
python quest_completion_import.py --check
python -m unittest test_quest_completion_import
python samples/server-completion/chosen-progress/test_builder.py
python samples/state-effect-refinements/test_refinements.py
python samples/server-completion/readiness242/test_audit.py
python samples/server-completion/npc-dialogue/test_builder.py
```

The generator and `--check` use only included pinned packets and the current
local canonical definitions; they require no Git, network, donor download or
external snapshot. Exact-source requalification of the events/rewards builder
requires the specified Git revision and verified NPC snapshot. Its additional
local test uses the original research-workspace paths, so that test is not added
to repository-wide `run_checks.py`. The portable wrapper tests and final import
`--check` are included there.

Outputs:

- `content/quests/missions/quest-state-completion-candidate.json`: 164 owners,
  1746 tracks, 3679 transitions, 382 unsupported transitions.
- `content/quests/missions/completion-candidate.json`: hashes, provenance and
  explicit activation/consumer limits.
- `content/quests/missions/completion-binding-plan.json`: all 416 selected
  stages joined to their real generated transition keys and available target,
  reward and NPC branch candidates. No branch is silently selected.

Nine daily recipes retain an explicit missing cycle-reset binding. Stage counters
are chosen occurrence counters; eligibility, per-target quantities, level,
premium and prerequisite predicates need the owning callers. Reward identities
and outfit/addon associations are not grants. The accepted JSON loader cannot
import XP effects even though the runtime has an XP writer.

The 242 readiness report separates CHOSEN data from preserved Source holds.
Its local Item-hold assessment is superseded, for current-main Item admission,
by `docs/agents/evidence/OTV2-20261004-quest-current-item-holds.json`: 19 owners
still have Item admission dependencies; two i3081 dependencies are already
resolved as Item definitions, with distinct instance/Source-charge holds.

Worker handoffs describe sealed producer snapshots. Root repaired the adopted
chosen builder's nonempty authored-Quest owner fence after review; `ADOPTION.json`
records that successor. They are local byte pins, not remote FREEZE_SHA.
