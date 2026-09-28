# tibia.com capture snapshots

Owner-run command (tibia.com blocks the build container and GitHub-hosted runners, #1077):

```
python3 tools/official-capture/tibiacom_capture.py fetch --out imports/official/tibia-com/<YYYY-MM-DD>
```

Each dated directory is committed immutable; a newer capture adds a new date directory instead of overwriting one.
