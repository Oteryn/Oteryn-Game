#!/usr/bin/env python3
"""Private per-run input for the upstream harness, never an identity SQL writer."""
import json
import os
import secrets
import shlex
import sys
from pathlib import Path


def main():
    work = Path(sys.argv[1])
    account_a, account_b, world, channel, fresh, recovery, fresh_id, recovery_id = sys.argv[2:]
    binary = Path(json.loads((work / "portal-meta.json").read_text())["target_directory"]) / "debug/oteryn-portal"
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("real_portal_binary_missing")
    callback_port = int(os.environ.get("GAME_PLATFORM_CALLBACK_PORT", "18099"))
    if not 1024 <= callback_port <= 65535:
        raise ValueError("invalid_callback_port")
    callback = f"http://127.0.0.1:{callback_port}/callback"
    native_port = int(os.environ.get("GAME_PLATFORM_NATIVE_PORT", "18100"))
    if not 1024 <= native_port <= 65535:
        raise ValueError("invalid_native_port")
    client = "019a0000-0000-7000-8000-0000000c11e7"
    suffix = secrets.token_hex(8)
    accounts = [
        {"accountId": account_a, "email": f"gpe2e-a-{suffix}@example.invalid", "password": secrets.token_urlsafe(48)},
        {"accountId": account_b, "email": f"gpe2e-b-{suffix}@example.invalid", "password": secrets.token_urlsafe(48)},
    ]
    steps = [{"name": name, "entity": "account", "args": args} for name, args in zip(["accountA", "accountB"], accounts)]
    steps += [
        {"name": "world", "entity": "world", "args": {"worldId": world, "name": "Entry", "region": "local"}},
        {"name": "nativeClient", "entity": "oauth-native-client", "args": {"clientId": client}},
        {"name": "freshTrust", "entity": "signing-trust", "args": {"purpose": "fresh_admission", "keyId": fresh_id, "publicKeyHex": fresh}},
        {"name": "recoveryTrust", "entity": "signing-trust", "args": {"issuer": "urn:oteryn:platform:game-recovery", "purpose": "existing_actor_recovery", "keyId": recovery_id, "publicKeyHex": recovery}},
        {"name": "channel", "entity": "channel", "args": {"worldId": world, "channelId": channel}},
        {"name": "route", "entity": "route", "args": {"worldId": world, "channelId": channel, "host": "127.0.0.1", "port": int(os.environ["NODE_BOOT_GAME_PORT"]), "tlsServerName": "localhost", "version": 1}},
    ]
    steps.append({"name": "routeReplay", "entity": "route", "args": steps[-1]["args"].copy()})
    (work / "route-input.json").write_text(json.dumps(steps[-1]["args"]))
    (work / "seed-plan.json").write_text(json.dumps({"steps": steps}))
    (work / "identity.json").write_text(json.dumps({"accounts": accounts, "callback": callback, "client": client,
        "native_origin": f"https://127.0.0.1:{native_port}", "portal_binary": str(binary)}))
    (work / "oauth.json").write_text(json.dumps({"enabled": True, "clients": [{"client_id": client, "redirect_uris": [callback]}]}))
    custody = work / "portal" / "secrets" / "admission-custody"
    custody_args = shlex.quote(json.dumps({"directory": str(custody), "purpose": "fresh_admission"}))
    launcher = work / "portal-exec"
    launcher.write_text("#!/usr/bin/env bash\nset -euo pipefail\nset +x\n"
                        "export OTERYN_ACCOUNT_LOGIN=true OTERYN_ACCOUNT_MFA=true APP_ENV=testing\n"
                        "export OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED=true\n"
                        "export OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY=oteryn-game-e2e\n"
                        f"export OTERYN_NATIVE_TLS_BIND=127.0.0.1:{native_port}\n"
                        'export OTERYN_NATIVE_TLS_CERT_FILE="$OTERYN_INTERNAL_TLS_CERT_FILE"\n'
                        'export OTERYN_NATIVE_TLS_KEY_FILE="$OTERYN_INTERNAL_TLS_KEY_FILE"\n'
                        f"export PORTAL_OAUTH_CONFIG=\"$(cat {shlex.quote(str(work / 'oauth.json'))})\"\n"
                        'if [[ "${1:-}" == serve ]]; then\n'
                        f"  {shlex.quote(str(binary))} seed signing-custody {custody_args} > {shlex.quote(str(work / 'custody-result.json'))}\n"
                        f"  export PORTAL_NATIVE_ADMISSION_KEY_CUSTODY_FILE={shlex.quote(str(custody / 'admission-custody.bin'))}\n"
                        f"  export PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE={shlex.quote(str(custody / 'admission-wrapping.key'))}\n"
                        '  [[ "$(stat -c %s "$PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE")" == 32 ]]\n'
                        '  [[ "$(stat -c %a "$PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE")" == 600 ]]\n'
                        '  export PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED=true\n'
                        'fi\n'
                        f"exec {shlex.quote(str(binary))} \"$@\"\n")
    launcher.chmod(0o700)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        # Only a fixed internal error may escape; never render input values.
        if isinstance(error, ValueError) and str(error) == "real_portal_binary_missing":
            print("real_portal_binary_missing", file=sys.stderr)
            sys.exit(3)
        print("game_platform_prepare_failed", file=sys.stderr)
        sys.exit(1)
