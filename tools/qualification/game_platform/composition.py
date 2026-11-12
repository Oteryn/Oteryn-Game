#!/usr/bin/env python3
"""Wire real purpose-separated Game channels from the upstream harness report."""
import json
import subprocess
import sys
from pathlib import Path


def node_documents(report, text, base):
    """Encode the two real Game configs from validated harness discovery."""
    q = json.dumps
    text = text.replace('peer_name = "source.test"', 'peer_name = "localhost"')
    text = text.replace('source_authority = "platform"', 'source_authority = "oteryn-platform-e2e"')
    text = text.replace('route_revision = "route-s3b-1"', f'route_revision = {q(report["seed"]["route"]["routeRevision"])}')
    text = text.replace('assignment_wait_ms = 120000', 'assignment_wait_ms = 600000')
    for section, role, extra in [
        ("runtime_status", "runtime-status", 'assignment_epoch = 1\n'),
        ("bootstrap_intents", "bootstrap-intents", ''),
        ("account_characters", "lcfa", f'source_authority = "oteryn-game-e2e"\nepoch_fence_file = {q(str(base / "run/lcfa-epoch-fence"))}\n'),
    ]:
        text += f'\n[platform.{section}]\nclient_certificate_file = {q(str(base / "node/secrets" / (role + ".crt")))}\nclient_key_file = {q(str(base / "node/secrets" / (role + ".key")))}\n{extra}'
    route = report["seed"]["route"]
    identity = report["clients"]["runtime-status"]["san"]
    # The node identity comes from the actual runtime-status certificate's one SAN.
    config = f'''endpoint = "127.0.0.1:{int(report['baseUrls']['internal'].rsplit(':', 1)[1])}"
peer_name = "localhost"
trust_roots_file = {q(str(base / 'ops/platform-ca.pem'))}
client_certificate_file = {q(str(base / 'ops/scope-assignment.crt'))}
client_key_file = {q(str(base / 'ops/scope-assignment.key'))}
assignment_epoch = 1
other_producer_certificate_files = [{q(str(base / 'ops/lcfa.crt'))}, {q(str(base / 'ops/bootstrap-intents.crt'))}]
[node_certificate_files]
{q(identity)} = {q(str(base / 'ops/runtime-status.crt'))}
[[scope]]
world_id = {q(route['worldId'])}
channel_id = {q(route['channelId'])}
node_identities = [{q(identity)}]
'''
    return text, config


def configure(work, node, base, uid):
    report = json.loads((work / "portal-report.json").read_text())
    def install(source, destination, owner, mode):
        subprocess.run(["sudo", "install", "-o", owner, "-m", mode, str(source), str(destination)],
                       check=True, capture_output=True)
    for role in ["runtime-status", "lcfa", "bootstrap-intents"]:
        for key, ext in [("cert", "crt"), ("key", "key")]:
            install(report["clients"][role][key], base / "node/secrets" / f"{role}.{ext}", uid, "0600")
    for role in ["runtime-status", "lcfa", "bootstrap-intents", "scope-assignment"]:
        install(report["clients"][role]["cert"], base / "ops" / f"{role}.crt", "root", "0644")
    install(report["clients"]["scope-assignment"]["key"], base / "ops/scope-assignment.key", "root", "0600")
    install(report["caFile"], base / "ops/platform-ca.pem", "root", "0644")
    # Fresh deployment only: never reset an existing external epoch fence.
    fence = base / "run/lcfa-epoch-fence"
    subprocess.run(["sudo", "test", "!", "-e", str(fence)], check=True, capture_output=True)
    subprocess.run(["sudo", "test", "!", "-L", str(fence)], check=True, capture_output=True)
    initial_fence = work / "lcfa-epoch-fence"
    initial_fence.write_text("0\n")
    install(initial_fence, fence, uid, "0600")
    text, config = node_documents(report, node.read_text(), base)
    node.write_text(text)
    temporary = work / "assignment-report.toml"
    temporary.write_text(config)
    install(temporary, base / "ops/assignment-report.toml", "root", "0644")


def bootstrap(work, account, world, operation, revisions):
    env = json.loads((work / "portal/secrets/env.json").read_text())
    binary = json.loads((work / "identity.json").read_text())["portal_binary"]
    if len(revisions) != 4:
        raise ValueError("intent_context_missing")
    args = dict(zip(["profileRevision", "rulesetRevision", "contentRevision", "starterTemplateRevision"], revisions))
    args.update(accountId=account, targetWorldId=world, operationId=operation,
                requestedName="Qualification Two", requestedSex="Female")
    result = subprocess.run([binary, "seed", "bootstrap-intent", json.dumps(args)],
                            env=env, capture_output=True, timeout=20, check=True)
    value = json.loads(result.stdout)
    if value.get("operationId") != operation or value.get("accountId") != account or value.get("targetWorldId") != world:
        raise ValueError("intent_binding_mismatch")


if __name__ == "__main__":
    try:
        if sys.argv[1] == "configure":
            configure(Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4]), sys.argv[5])
        elif sys.argv[1] == "bootstrap":
            bootstrap(Path(sys.argv[2]), *sys.argv[3:6], sys.argv[6:])
        else:
            raise ValueError("unknown_step")
    except (OSError, ValueError, KeyError, IndexError, subprocess.SubprocessError):
        print("game_platform_composition_failed", file=sys.stderr)
        sys.exit(1)
