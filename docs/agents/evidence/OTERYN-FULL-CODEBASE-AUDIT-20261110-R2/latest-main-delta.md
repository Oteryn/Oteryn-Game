# Latest-main delta: startup diagnostics

Audit base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Reviewed delta: `af33fd3baeac15d9991bd24c91bcacc8733492e2` (#3092). The immutable checkout and primary ledger stay on the audit base.

## Coverage and behavior

All four changed files were read in full, including their unchanged context and tests; exact SHA-256 and ranges are in `latest-main-delta.json`. The change adds local best-effort diagnostic files to hidden Windows startup failures and prints them only on installer relaunch timeout. It does not remove release-pointer validation, install mutex checks, package verification, or the 60-second client-mutex condition.

- `apps/client/installer/ci-installer.ps1`: Full latest357lines read: clears previous startup/exit diagnostic files under LOCALAPPDATA/Oteryn before relaunch;60second mutex polling preserved; only timeout prints two new diagnostic files and exact current setup log, then fails. Does not treat code0 as proof of living child.
- `apps/client/src/bin/oteryn-launcher.rs`: Full latest382lines read: success records launcher exit0; error records existing localized launcher failure and exit2 before prior dialog/exit behavior. Launcher status means spawn/start operation success, not child lifetime health; CI still waits actual client mutex.
- `apps/client/src/crash_report.rs`: Full latest209lines read: added startup.log append and launcher-exit-code.txt in settings directory; append helper creates parent; exit recorder writes0/2 best effort. Existing minimal crash/consent path unchanged. New test reads append behavior but was not run. Log is local, no automatic support upload wired by this change; no log rotation in added utility.
- `apps/client/src/main.rs`: Full latest101lines read: on windows_shell::run Err append fixed ShellError class under component client, then retain existing stderr/FAILURE path. No successful flow or package/mutex guard removed.

## Result and limits

No new actionable finding in this delta after tracing the fixed ShellError caller and launcher failure variants. This is static REVIEWED coverage, not independent VERIFIED status. No Windows/PowerShell/build/relaunch tests ran. The new append helper has no rotation; this is a bounded-scope observation, not a claimed exploitation or operational incident. Existing package/updater dependencies remain under the ongoing complete apps/client audit.
