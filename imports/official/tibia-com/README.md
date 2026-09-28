# tibia.com capture snapshots

Owner-run command (tibia.com blocks the build container and GitHub-hosted runners, #1077):

```
python3 tools/official-capture/tibiacom_capture.py fetch --out imports/official/tibia-com/<YYYY-MM-DD>
```

Each dated directory is committed immutable; a newer capture adds a new date directory instead of overwriting one.

The capture covers all 19 sections linked from the manual's Contents page. It stores page URLs,
fetch times, HTTP status, SHA-256 digests and bounded factual excerpts. It does not mirror the
manual's full text or artwork. The spell library is captured separately when its parser is
available.
