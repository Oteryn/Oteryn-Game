# Oracle Game operations

The owner-authorized Oracle migration is tracked in #2217 and Platform #1639.
Use `Oracle Operations` on protected main: `status` proves the exact Oracle
runner, ARM64, uid1001, absence of a Docker socket and host readiness; `deploy`
takes a full current protected-main SHA. The host independently requires that SHA
and its latest successful `game-gate`, loads the CI-built ARM64 image, backs up,
runs the owner-role migrator and restarts through the separately installed
durable Game incarnation controller. The workflow cannot replace host
controllers or secrets.

Host implementation and runner Compose are maintained in Oteryn-Platform's
`deploy/oracle/`. The private component-bound SSH key is constrained by a root-owned
forced command; it grants no shell, PTY, forwarding or access to another component.
The runner has no Docker socket or host runtime mount. Its existing game-runners
group remains repository-restricted; labels include oteryn-game, oteryn-oracle and
ARM64. Docker restart keeps the agent available after reboot.

## ARM64 image handoff

For `deploy`, a GitHub-hosted ARM64 runner builds from the exact source SHA
and uploads a one-day artifact named `oteryn-game-image-<sha>`. Its only
member is `image.tar` (up to 3 GiB), tagged `oteryn-game:<sha>-arm64`
with the matching OCI revision label.

The Oracle runner downloads the original artifact ZIP (up to 1 GiB) from
GitHub and sends its bytes on standard input to the host's forced command
`deploy-image <sha> <run_id> <run_attempt> <artifact_id>`. The GitHub token
stays on the runner. The host checks the artifact digest before loading the
image and uses its existing content mounts. `status` skips the image build.

Disable the retired `synology-game-deploy.yml` and
`synology-game-runner-acceptance.yml` in GitHub Actions before enabling the Oracle
runner. They retain historical x86_64/NAS contracts and are not Oracle deployers.
NAS writer and runner containers stay stopped with restart disabled.

World/content, canonical IDs, trust, independent floors and fencing are preserved.
No content activation, authority bootstrap, floor reset or schema rollback is part
of this workflow. Uncertain outcomes stay closed for explicit reconciliation.
Real native gameplay admission is a separate acceptance result from readiness.
