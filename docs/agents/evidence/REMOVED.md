# Removed evidence

Owner decision 2026-10-29: evidence that no code or CI reads was removed. Recover any file with `git show 329f78f8:<path>`.

- `monster-data-completion-20261002/` (about 40 MB, including the 34 MB `monster-data-completion-review.tar.gz`). The task card `docs/agents/tasks/**/OTV2-20261001-monster-data-completion.md` and `CARDS-AUDIT-1.md` still cite it as history.
- 53 non-Markdown evidence units (about 2.6 MB of JSON, plus `.py`/`.rs` helpers inside `OTV2-20261005-shards-of-a-broken-moon-reconstruction/`), none named in code, CI, content or imports at `329f78f8`: the `*.json` receipts, audits and reviews at the top level of this directory, the non-Markdown files in `OTV2-20261009-house-client-source-audit/`, `OTV2-20261005-shards-of-a-broken-moon-reconstruction/` and `OTV2-20261005-soul-war-reconstruction/`, and `OTV2-20260907-meta-agent-policy-v3/evaluation.json`. Their Markdown companions were left in place.
