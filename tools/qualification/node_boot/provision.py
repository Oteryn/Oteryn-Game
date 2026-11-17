"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import json
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
import sys
import time
import threading
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "c17a", "c17c")


CREATURE_KEY = re.compile(r"[A-Za-z0-9:._/-]{1,128}")
SPAWNED = re.compile(rb"spawned [0-9a-f]{32} [0-9]+\n")


def _request(socket_path, line, limit, timeout=20):
    """One control exchange; returns up to `limit` answer bytes plus one overflow-detecting byte."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        deadline = time.monotonic() + timeout
        control.settimeout(timeout)
        control.connect(socket_path)
        control.sendall(line.encode("ascii"))
        control.shutdown(socket.SHUT_WR)
        answer = bytearray()
        while len(answer) <= limit:
            control.settimeout(max(0.001, deadline - time.monotonic()))
            if time.monotonic() >= deadline:
                raise TimeoutError("qualification provisioning deadline exceeded")
            chunk = control.recv(limit + 1 - len(answer))
            if not chunk:
                break
            answer.extend(chunk)
        return answer


SENSITIVE_LINE = re.compile(r"password|postgres://|mysql://|secret|token|key=", re.IGNORECASE)


def sanitized_text(output):
    if isinstance(output, bytes):
        output = output.decode("utf-8", errors="replace")
    return "\n".join(line for line in (output or "").splitlines()
                     if not SENSITIVE_LINE.search(line))


def report_child_output(stage, stdout, stderr):
    for stream, output in (("stdout", stdout), ("stderr", stderr)):
        safe = sanitized_text(output)
        if safe:
            print(f"{stage} {stream}:\n{safe}", file=sys.stderr)


def run_capture(command, stage, timeout):
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        report_child_output(stage, error.stdout, error.stderr)
        raise RuntimeError(f"{stage} timed out after {timeout}s") from None
    if result.returncode:
        report_child_output(stage, result.stdout, result.stderr)
        raise RuntimeError(f"{stage} exit status {result.returncode}")
    return result.stdout


def node_log_offset(node_log):
    try:
        return Path(node_log).stat().st_size if node_log else 0
    except OSError:
        return 0


def report_node_failures(node_log, offset):
    if not node_log:
        return
    try:
        with Path(node_log).open("rb") as log:
            # Bound the read, exclude boot/previous-operation failures, and
            # tolerate log truncation on node replacement.
            end = log.seek(0, 2)
            log.seek(max(offset if offset <= end else 0, end - 65536))
            lines = log.read(65536).decode("utf-8", errors="replace").splitlines()
    except OSError:
        print("C17 node failure diagnostics unavailable", file=sys.stderr)
        return
    events = []
    for line in lines:
        header, separator, detail = line.partition(" detail=")
        event = re.search(r"(?:^| )event=(character_bootstrap_failure|qualification_fixture_failure)(?= |$)", header)
        if not event:
            if re.search(r"(?:^| )event=qualification_fixture_refused(?= |$)", header):
                reason = re.search(r"(?:^| )reason=(fixture_disabled|native_unavailable|malformed_request)(?= |$)", header)
                if reason:
                    safe = f"event=qualification_fixture_refused reason={reason[1]}"
                    if safe not in events:
                        events.append(safe)
            continue
        # The structured renderer separates event from quoted detail (and may
        # put a trace between them). Never copy that trace or the whole detail.
        tokens = re.search(
            r"stage=[a-z_]+ reason=[a-z_]+"
            r'(?: step=[a-z_]+ cause=[a-z_]+(?: replay=[a-z_]+)?)?(?= |"|$)',
            detail if separator else header[event.end():])
        if tokens:
            safe = f"event={event[1]} {tokens[0]}"
            if safe not in events:
                events.append(safe)
    for event in events[-10:]:
        print(event, file=sys.stderr)


def fresh_character_name(operation):
    # Both pinned naming contracts allow 2..29 ASCII letters with single spaces;
    # neither imposes a per-word limit. This name is 26 characters long.
    return "Spell " + "".join(chr(ord("a") + int(c, 16)) for c in operation.hex[-20:])


def provision(socket_path, session, profile, node_log=None):
    offset = node_log_offset(node_log)
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or profile not in PROFILES:
        raise ValueError("invalid qualification request")
    reconcile = profile in ("c17a", "c17c")
    line = f"qualify {session} {profile}{'-reconcile' if reconcile else ''}\n"
    deadline = time.monotonic() + 20
    while True:
        try:
            answer = (_request(socket_path, line, 64, timeout=max(0.001, deadline - time.monotonic()))
                      if reconcile else _request(socket_path, line, 64))
        except (TimeoutError, ConnectionError):
            if not reconcile:
                raise
            answer = b""
        if answer == b"qualified\n":
            return
        report_node_failures(node_log, offset)
        if reconcile and (not answer or b"qualified\n".startswith(bytes(answer))) and time.monotonic() < deadline:
            print("C17 fixture acknowledgement unknown; reconciling the same build", file=sys.stderr)
            continue
        raise RuntimeError(
            "node refused qualification provisioning: "
            f"node_answer={bytes(answer[:64])!r} truncated={len(answer) > 64}"
        )


def spawn(socket_path, session, creature_key):
    """Places one creature next to the session's actor; only a qualification-mode node serves it.

    Returns the node answer `spawned <entity identity hex> <generation>`; a refusal raises with
    the exact `rejected reason=<token>` answer.
    """
    identifier = uuid.UUID(session)
    if (
        identifier.version != 7
        or str(identifier) != session
        or not CREATURE_KEY.fullmatch(creature_key)
    ):
        raise ValueError("invalid qualification spawn request")
    answer = bytes(_request(socket_path, f"spawn {session} {creature_key}\n", 128))
    if not SPAWNED.fullmatch(answer):
        raise RuntimeError(
            f"node refused qualification spawn: node_answer={answer[:128]!r} "
            f"truncated={len(answer) > 128}"
        )
    return answer.decode("ascii").rstrip("\n")



def fresh_character(socket_path, project, request, node_log=None):
    """Issue a real Platform intent, then ask the existing node bootstrap owner."""
    operation = uuid.UUID(int=(int(time.time() * 1000) << 80)
                          | (7 << 76) | (secrets.randbits(12) << 64)
                          | (2 << 62) | secrets.randbits(62))
    name = fresh_character_name(operation)
    request.update(operation=str(operation), name=name)
    containers = run_capture([
        "docker", "ps", "--filter", f"label=com.docker.compose.project={project}",
        "--filter", "label=com.docker.compose.service=platform", "--format", "{{.ID}}",
    ], "Platform container discovery", timeout=20).split()
    if len(containers) != 1:
        raise RuntimeError(f"qualification requires one Platform container; found={len(containers)}")
    php = r'''require "vendor/autoload.php";
$app=require "bootstrap/app.php";
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
$r=json_decode($argv[1],true,512,JSON_THROW_ON_ERROR);
$id=Illuminate\Support\Facades\DB::table("identities")->where("account_id",$r["account"])->value("id");
if (!$id) { fwrite(STDERR,"C17 bootstrap: no Platform identity for requested AccountId\n"); exit(1); }
$code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",[
"--identity-id"=>(string)$id,"--operation-id"=>$r["operation"],
"--target-world-id"=>$r["world"],"--requested-name"=>$r["name"],"--requested-sex"=>"Female",
"--profile-revision"=>$r["revisions"][0],"--ruleset-revision"=>$r["revisions"][1],
"--content-revision"=>$r["revisions"][2],"--starter-template-revision"=>$r["revisions"][3]]);
if ($code !== 0) { fwrite(STDERR,Illuminate\Support\Facades\Artisan::output()); }
exit($code);'''
    run_capture(["docker", "exec", "--user", "www-data", containers[0],
                    "php", "-r", php, json.dumps(request)],
                   "Platform bootstrap intent", timeout=30)
    commit_character_bootstrap(socket_path, operation, node_log)
    return name


def commit_character_bootstrap(socket_path, operation, node_log=None):
    # D2's unavailable result is ambiguous, not a rejection. As game-ops instructs,
    # reconcile the SAME operation: a committed receipt wins before any external
    # reads, and the Platform intent must never be reissued on this path.
    offset = node_log_offset(node_log)
    deadline = time.monotonic() + 60
    last_answer = "unavailable"
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            report_node_failures(node_log, offset)
            raise RuntimeError(f"fresh spell character bootstrap deadline exceeded; answer={last_answer!r}")
        # serve.rs CONTROL_DEADLINE is 30s. Leave part of the original 60s
        # budget to reconcile a commit whose acknowledgement was lost.
        exchange_deadline = min(deadline, time.monotonic() + 30)
        answer = bytearray()
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
                control.settimeout(min(30, remaining))
                control.connect(socket_path)
                control.sendall(f"{operation}\n".encode("ascii"))
                control.shutdown(socket.SHUT_WR)
                while len(answer) <= 64:
                    remaining = exchange_deadline - time.monotonic()
                    if remaining <= 0:
                        raise TimeoutError("bootstrap acknowledgement deadline exceeded")
                    control.settimeout(remaining)
                    chunk = control.recv(65 - len(answer))
                    if not chunk:
                        break
                    answer.extend(chunk)
        except (TimeoutError, ConnectionError):
            # The node may have committed already. No new intent or operation.
            answer = bytearray()
        if answer == b"committed\n":
            return
        if answer == b"unavailable\n":
            last_answer = "unavailable"
        elif any(len(answer) < len(reply) and reply.startswith(answer)
                 for reply in (b"committed\n", b"unavailable\n", b"rejected\n")):
            last_answer = "acknowledgement_unknown"
        else:
            raise RuntimeError(f"node refused fresh spell character bootstrap: answer={sanitized_text(answer.decode(errors='replace')) or '<empty or filtered>'!r}")
        print(f"C17 bootstrap {last_answer}; reconciling the same operation id", file=sys.stderr)
        report_node_failures(node_log, offset)
        time.sleep(min(0.5, max(0, deadline - time.monotonic())))


def watch_parent_stdin():
    """The parent owns the pipe's write end; EOF cancels even a root-owned helper.

    No stdin data is part of this CLI protocol. The watcher never issues a node
    operation; a request already delivered remains subject to native reconciliation.
    """
    def stopped():
        try:
            sys.stdin.buffer.read(1)
        finally:
            os._exit(124)
    threading.Thread(target=stopped, daemon=True).start()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--watch-parent-stdin", action="store_true")
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session")
    parser.add_argument("--node-log")
    parser.add_argument("--project")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--profile", choices=PROFILES)
    group.add_argument("--spawn", metavar="CREATURE_KEY")
    group.add_argument("--fresh-character")
    args = parser.parse_args()
    if args.watch_parent_stdin:
        watch_parent_stdin()
    if args.fresh_character:
        if not args.project:
            parser.error("--fresh-character requires --project")
        print(fresh_character(args.socket, args.project, json.loads(args.fresh_character), args.node_log))
    else:
        if not args.session:
            parser.error("provisioning and spawning require --session")
        if args.spawn is not None:
            print(spawn(args.socket, args.session, args.spawn))
        else:
            provision(args.socket, args.session, args.profile, args.node_log)
