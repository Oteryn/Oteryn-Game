import json, re
from pathlib import Path

rows=[]
for path in sorted(Path(".questpc").glob("r3-*.result.json")):
    result=json.loads(path.read_text())
    log=path.with_name(path.name.replace(".result.json", ".log"))
    text=log.read_text(errors="replace") if log.exists() else ""
    summaries=re.findall(r"test result: .+|Ran \d+ tests? in .+|Quest unit tests failed: .+|ValueError: stale donor Source capture: .+", text)
    rows.append({**result,"log":str(log),"summaries":summaries,"postgres_completed": "POSTGRES_CASE_COMPLETED" in text})
Path(".questpc/r3-validation.json").write_text(json.dumps(rows,indent=2)+"\n")
lines=["| Command | Exit | Seconds including queue/build | Evidence |", "| --- | ---: | ---: | --- |"]
for row in rows:
    evidence="; ".join(row["summaries"]) or "See log"
    lines.append(f"| `{row['command']}` | {row['exit']} | {row['duration_seconds']} | {evidence} (`{row['log']}`) |")
Path(".questpc/r3-validation.md").write_text("\n".join(lines)+"\n")
