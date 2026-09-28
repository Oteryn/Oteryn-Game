# tibia.com capture snapshots

Owner-run command (tibia.com blocks the build container and GitHub-hosted runners, #1077):

```
python3 tools/official-capture/tibiacom_capture.py fetch --out imports/official/tibia-com
```

The command creates a uniquely named UTC run directory (`YYYY-MM-DD-HHMMSSZ`). Legacy
`YYYY-MM-DD` directories remain valid. Every snapshot directory is immutable once committed;
the tool refuses to overwrite an existing output directory.

The capture covers all 19 sections linked from the manual's Contents page. It stores page URLs,
fetch times, HTTP status, SHA-256 digests and bounded factual excerpts. It does not mirror the
manual's full text or artwork. The spell library is captured separately when its parser is
available.

The original six-section snapshot uses schema v1. The complete 19-section capture uses schema
v2; the verifier accepts both and checks each against its own required section set.
