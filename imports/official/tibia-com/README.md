# tibia.com capture snapshots

Owner-run command (tibia.com blocks the build container and GitHub-hosted runners, #1077):

```
python3 tools/official-capture/tibiacom_capture.py fetch --out imports/official/tibia-com
```

The command creates a uniquely named UTC run directory (`YYYY-MM-DD-HHMMSSZ`). Legacy
`YYYY-MM-DD` directories remain valid. Every snapshot directory is immutable once committed;
the tool refuses to overwrite an existing output directory.

Directory names and every date field are checked against the literal `YYYY-MM-DD` shape (ASCII
digits, zero-padded, a real calendar date). `fetch` checks `--out` before any request and refuses a
name that is not that shape or does not match the run's own UTC stamp; `verify` and `verify-root`
reject the same names and dates in committed snapshots.

The capture covers all 19 sections linked from the manual's Contents page. It stores page URLs,
fetch times, HTTP status, SHA-256 digests and bounded factual excerpts. It does not mirror the
manual's full text or artwork.

## Adding the spell library

The `2026-09-28` snapshot (#1125) captured the manual only (`"spells": "PENDING_1077"`). The spell
library is captured by the same command: when `tools/content-schema/spell-authoring/tibiacom_spells.py`
is on the checkout, `fetch` also requests `https://www.tibia.com/library/?subtopic=spells` and
sets `"spells": "captured"`. The list table is parsed by #1077's own `list_facts` through an explicit
adapter in the tool (`spell_facts_from_library_html`), so each spell becomes one `spells` fact
(`spells.list.<slug>`, a JSON object with `name`, `words`, `subclass`, `type`, `premium` and, when the
list states them, `levelrequired` and `mana`). The manifest keeps only the page URL, fetch time,
HTTP status, SHA-256 and visible-text length, never page text.

Owner command, from a checkout of `main` on a machine tibia.com serves (about 20 throttled requests):

```
python3 tools/official-capture/tibiacom_capture.py fetch --out imports/official/tibia-com
```

Then commit the new `imports/official/tibia-com/<UTC run>/` directory (`manifest.json` and
`facts.json` only) in a PR. This is a deliberate new snapshot version: it does not amend
`2026-09-28` or `2026-09-28-160207Z`, which stay immutable, and `check-immutability` rejects any
edit to them. Before committing, `python3 tools/official-capture/tibiacom_capture.py verify
imports/official/tibia-com/<UTC run>` must pass; `fetch` already runs it and removes the new
directory if it fails.

`fetch` aborts without writing output when the spell page is not HTTP 200, is redirected or blocked
by a Cloudflare challenge, or has no list table with the header `Name, Group, Type, Exp Lvl, Mana,
Premium` (for example a page that only renders its table with JavaScript). Report that case instead of
working around it.

The original six-section snapshot uses schema v1. The complete 19-section capture uses schema
v2; the verifier accepts both and checks each against its own required section set.
