"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import json
from pathlib import Path
import secrets
import subprocess
import threading
import os
import re
import select
import socket
import sys
import time
from threading import Thread
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "combat-progression", "private-channel", "coin-converters", "rune-training", "sweep", "atelier", "victim", "knight-underlevel", "c17a", "c17c", "c04", "w22b-wealth", "w22b-stamina")
CREATURE_KEY = re.compile(r"[A-Za-z0-9:._/-]{1,128}")
CAPACITY = re.compile(rb"capacity [1-9][0-9]* [1-9][0-9]*\n")
SPAWNED = re.compile(rb"spawned [0-9a-f]{32} [0-9]+ [0-9]+\n")


CONTROL_EXCHANGE_SECONDS = 30
FIXTURE_RECONCILE_SECONDS = 2 * CONTROL_EXCHANGE_SECONDS


# The server bounds each accepted control request at 30 seconds.
CONTROL_REPLY_TIMEOUT = 40
CONTROL_QUEUE_TIMEOUT = 40


def _request(socket_path, line, limit, cancel=None, *, timeout=None):
    """One control exchange; returns up to `limit` answer bytes plus one overflow-detecting byte."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        queue_deadline = time.monotonic() + (CONTROL_QUEUE_TIMEOUT if timeout is None else timeout)
        control.settimeout(CONTROL_QUEUE_TIMEOUT if timeout is None else timeout)
        control.connect(socket_path)
        control.settimeout(max(0.001, queue_deadline - time.monotonic()))
        control.sendall(b"!")
        control.settimeout(max(0.001, queue_deadline - time.monotonic()))
        if control.recv(1) != b"!" or time.monotonic() >= queue_deadline:
            raise TimeoutError("qualification control was not ready in time")
        deadline = time.monotonic() + CONTROL_REPLY_TIMEOUT if timeout is None else queue_deadline
        control.settimeout(max(0.001, deadline - time.monotonic()) if timeout is not None else CONTROL_REPLY_TIMEOUT)
        control.sendall(line.encode("ascii"))
        if not line.startswith("qualify "):
            control.shutdown(socket.SHUT_WR)
        answer = bytearray()
        while len(answer) <= limit:
            control.settimeout(max(0.001, deadline - time.monotonic()))
            if time.monotonic() >= deadline:
                raise TimeoutError("qualification provisioning deadline exceeded")
            if cancel is not None:
                ready, _, _ = select.select([control, cancel], [], [], max(0, deadline - time.monotonic()))
                if cancel in ready:
                    raise RuntimeError("qualification provisioning caller disconnected")
                if not ready:
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


def provision(socket_path, session, profile, node_log=None, *, cancel=None):
    offset = node_log_offset(node_log)
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or profile not in PROFILES:
        raise ValueError("invalid qualification request")
    reconcile = profile in ("c17a", "c17c", "c04", "w22b-wealth", "w22b-stamina")
    line = f"qualify {session} {profile}{'-reconcile' if reconcile else ''}\n"
    deadline = time.monotonic() + FIXTURE_RECONCILE_SECONDS if reconcile else None
    attempts = 0
    while True:
        try:
            answer = (_request(socket_path, line, 64, timeout=max(0.001, min(CONTROL_EXCHANGE_SECONDS, deadline - time.monotonic())))
                      if reconcile else _request(socket_path, line, 64, cancel))
        except (TimeoutError, ConnectionError):
            if not reconcile:
                raise
            answer = b""
        if answer == b"qualified\n":
            return
        if attempts == 0:
            report_node_failures(node_log, offset)
        if reconcile and (not answer or b"unavailable\n".startswith(bytes(answer)) or b"qualified\n".startswith(bytes(answer))):
            if time.monotonic() < deadline:
                if attempts == 0:
                    print("C17 fixture acknowledgement unknown; reconciling the same build", file=sys.stderr)
                attempts += 1
                time.sleep(min(0.25, max(0, deadline - time.monotonic())))
                continue
            raise RuntimeError("qualification build reconciliation deadline exceeded; outcome=unknown")
        raise RuntimeError(
            "node refused qualification provisioning: "
            f"node_answer={bytes(answer[:64])!r} truncated={len(answer) > 64}"
        )


def spawn(socket_path, session, creature_key):
    """Places one creature next to the session's actor; only a qualification-mode node serves it.

    Returns the node answer `spawned <entity identity hex> <generation> <actor local id>`; a refusal raises with
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
    if creature_key == "cleanup" and answer == b"cleaned\n":
        return "cleaned"
    if not SPAWNED.fullmatch(answer):
        raise RuntimeError(
            f"node refused qualification spawn: node_answer={answer[:128]!r} "
            f"truncated={len(answer) > 128}"
        )
    return answer.decode("ascii").rstrip("\n")


def inspect_damage(socket_path, session, target):
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or not re.fullmatch(r"[0-9a-f]{32} [1-9][0-9]{0,19}", target):
        raise ValueError("invalid qualification damage inspection")
    answer = bytes(_request(socket_path, f"inspect-damage {session} {target}\n", 128))
    if not re.fullmatch(rb"damage [0-9]{1,20} [0-9a-f]{32}\n", answer):
        raise RuntimeError(f"node refused qualification damage inspection: {answer[:128]!r}")
    return answer.decode("ascii").rstrip("\n")


def carry_capacity(socket_path, session):
    """Read the current Character's level and carried-weight limit in centi-ounces."""
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session:
        raise ValueError("invalid qualification capacity request")
    answer = bytes(_request(socket_path, f"capacity {session}\n", 64))
    if not CAPACITY.fullmatch(answer):
        raise RuntimeError(
            f"node refused qualification capacity: node_answer={answer[:64]!r} "
            f"truncated={len(answer) > 64}"
        )
    return answer.decode("ascii").rstrip("\n")


# Container exec is outside the helper's process group. A per-operation lease
# fences delayed starts and lets cancellation kill and observe the actual PHP
# writer, rather than merely killing the local Docker client.
def writer_clock():
    # Docker shares the host kernel's boot clock; wall-clock corrections cannot
    # extend a writer lease or remove a still-live cancellation fence.
    return time.clock_gettime(time.CLOCK_BOOTTIME)


PARENT_CANCELLED = threading.Event()
WRITER_LOCK = threading.Lock()
REMOTE_WRITER = None

REMOTE_INTENT = r'''op=$1; expires=$2; shift 2
[ $(cut -d. -f1 /proc/uptime) -lt "$expires" ] || exit 124
for old in /tmp/oteryn-c17-intent-*; do
  [ -f "$old/expires" ] || continue
  old_expiry=$(cat "$old/expires")
  case "$old_expiry" in ''|*[!0-9]*) continue ;; esac
  [ "$old_expiry" -ge "$(cut -d. -f1 /proc/uptime)" ] || rm -rf -- "$old"
done
lease=/tmp/oteryn-c17-intent-$op
mkdir -p "$lease"
printf "%s\n" "$expires" > "$lease/expires"
[ ! -f "$lease/cancelled" ] || exit 124
sh -c 'lease=$1; op=$2; expires=$3; shift 3
printf "%s\n" "$$" > "$lease/pid"
[ ! -f "$lease/cancelled" ] || exit 124
[ $(cut -d. -f1 /proc/uptime) -lt "$expires" ] || exit 124
exec "$@"' c17-writer "$lease" "$op" "$expires" "$@" &
writer=$!
( remaining=$((expires - $(cut -d. -f1 /proc/uptime)))
  [ "$remaining" -le 0 ] || sleep "$remaining"
  if tr "\000" " " < "/proc/$writer/cmdline" 2>/dev/null | grep -Fq -- "$op"; then
    kill -KILL "$writer" 2>/dev/null || true
  fi
) </dev/null >/dev/null 2>&1 &
watchdog=$!
wait "$writer"; result=$?
kill "$watchdog" 2>/dev/null || true
wait "$watchdog" 2>/dev/null || true
[ -f "$lease/cancelled" ] || rm -rf -- "$lease"
exit "$result"
'''

CANCEL_INTENT = r'''op=$1; expires=$2
lease=/tmp/oteryn-c17-intent-$op
mkdir -p "$lease"
printf "%s\n" "$expires" > "$lease/expires"
: > "$lease/cancelled"
[ -f "$lease/pid" ] || exit 0
writer=$(cat "$lease/pid")
case "$writer" in ''|*[!0-9]*) exit 0 ;; esac
owned() { tr "\000" " " < "/proc/$writer/cmdline" 2>/dev/null | grep -Fq -- "$op"; }
if owned; then kill -KILL "$writer" 2>/dev/null || true; fi
for attempt in 1 2 3 4 5 6 7 8 9 10; do
  owned || exit 0
  sleep 0.1
done
exit 1
'''


class PlatformIntentWriter:
    def __init__(self, container, operation):
        self.container = container
        self.operation = str(operation)
        self.quiesced = threading.Event()
        self.expires = int(writer_clock()) + 30
        self.cancel_lock = threading.Lock()

    def cancel(self):
        result = subprocess.run(["docker", "exec", "--user", "www-data", self.container,
            "sh", "-c", CANCEL_INTENT, "c17-cancel", self.operation, str(self.expires)],
            capture_output=True, timeout=5)
        if result.returncode:
            raise RuntimeError("C17 Platform writer cancellation was not acknowledged")
        self.quiesced.set()

    def quiesce(self):
        warned = False
        while not self.quiesced.is_set():
            with self.cancel_lock:
                if self.quiesced.is_set():
                    return
                try:
                    self.cancel()
                except (OSError, subprocess.TimeoutExpired, RuntimeError):
                    if not warned:
                        print("C17 cancellation awaiting remote acknowledgement or absolute writer expiry", file=sys.stderr)
                        warned = True
                    if writer_clock() > self.expires + 1:
                        # Remote shell and PHP both reject this absolute expiry;
                        # delayed Docker exec starts cannot issue a new intent.
                        self.quiesced.set()
                    else:
                        time.sleep(0.1)

    def run(self, php, request):
        global REMOTE_WRITER
        with WRITER_LOCK:
            if PARENT_CANCELLED.is_set():
                raise RuntimeError("C17 parent cancelled before intent issuance")
            REMOTE_WRITER = self
        try:
            result = run_capture(["docker", "exec", "--user", "www-data", self.container,
                "sh", "-c", REMOTE_INTENT, "c17-intent", self.operation, str(self.expires),
                "php", "-r", php, json.dumps(dict(request, writer_deadline=self.expires))], "Platform bootstrap intent", timeout=35)
            self.quiesced.set()  # The remote wrapper waited for PHP and reaped its watchdog.
            if PARENT_CANCELLED.is_set():
                raise RuntimeError("C17 parent cancelled during intent issuance")
            return result
        except Exception:
            if not self.quiesced.is_set():
                self.quiesce()
            if PARENT_CANCELLED.is_set():
                raise SystemExit(124) from None
            raise
        finally:
            with WRITER_LOCK:
                if REMOTE_WRITER is self and self.quiesced.is_set():
                    REMOTE_WRITER = None


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
$clock=file_get_contents("/proc/uptime");
if (!is_string($clock) || !preg_match('/^([0-9]+(?:\.[0-9]+)?) /',$clock,$clockMatch)) { fwrite(STDERR,"qualification writer clock unavailable\n"); exit(124); }
if ((float)$clockMatch[1] >= $r["writer_deadline"]) { exit(124); }
$code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",[
"--identity-id"=>(string)$id,"--operation-id"=>$r["operation"],
"--target-world-id"=>$r["world"],"--requested-name"=>$r["name"],"--requested-sex"=>"Female",
"--profile-revision"=>$r["revisions"][0],"--ruleset-revision"=>$r["revisions"][1],
"--content-revision"=>$r["revisions"][2],"--starter-template-revision"=>$r["revisions"][3]]);
if ($code !== 0) { fwrite(STDERR,Illuminate\Support\Facades\Artisan::output()); }
exit($code);'''
    PlatformIntentWriter(containers[0], operation).run(php, request)
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
        sys.stdin.buffer.read(1)
        with WRITER_LOCK:
            PARENT_CANCELLED.set()
            writer = REMOTE_WRITER
        if writer is not None:
            # Do not report cancellation complete while the remote writer can
            # still issue the intent. Its own 30s watchdog also bounds PHP.
            writer.quiesce()
        os._exit(124)
    threading.Thread(target=stopped, daemon=True).start()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session")
    parser.add_argument("--node-log")
    parser.add_argument("--project")
    parser.add_argument("--watch-parent-stdin", action="store_true")
    parser.add_argument("--watch-stdin", action="store_true")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--profile", choices=PROFILES)
    group.add_argument("--fresh-character")
    group.add_argument("--spawn", metavar="CREATURE_KEY")
    group.add_argument("--item-handle", type=int)
    group.add_argument("--recovery-epoch", type=int)
    group.add_argument("--capacity", action="store_true")
    parser.add_argument("--cancel-on-stdin", action="store_true")
    group.add_argument("--inspect-damage", metavar="TARGET")
    args = parser.parse_args()
    if args.watch_parent_stdin or (args.watch_stdin and args.fresh_character is not None):
        watch_parent_stdin()
    elif args.watch_stdin:
        def stop_when_parent_drops_pipe():
            # The parent deliberately sends no bytes. EOF closes the control request
            # even when sudo's outer process and Python have different credentials.
            os.read(0, 1)
            os._exit(125)

        Thread(target=stop_when_parent_drops_pipe, daemon=True).start()
    if args.fresh_character is not None:
        if not args.project:
            parser.error("--fresh-character requires --project")
        print(fresh_character(args.socket, args.project, json.loads(args.fresh_character), args.node_log))
        sys.exit(0)
    if not args.session:
        parser.error("current-session control operations require --session")
    if args.recovery_epoch is not None:
        identifier = uuid.UUID(args.session)
        if identifier.version != 7 or str(identifier) != args.session or not 0 < args.recovery_epoch < 2**64:
            raise ValueError("invalid recovery replay observation")
        if _request(args.socket, f"recovery-replay {args.session} {args.recovery_epoch}\n", 64) != b"recovery-grant-consumed\n":
            raise RuntimeError("recovery owner did not observe a spent grant")
    elif args.item_handle is not None:
        identifier = uuid.UUID(args.session)
        if identifier.version != 7 or str(identifier) != args.session or not 0 < args.item_handle < 2**64:
            raise ValueError("invalid item-instance request")
        answer = bytes(_request(args.socket, f"item-instance {args.session} {args.item_handle}\n", 64))
        if not re.fullmatch(rb"instance [0-9a-f-]{36}\n", answer):
            raise RuntimeError("item handle has no live durable instance")
        print(answer.decode("ascii").strip())
    elif args.capacity:
        print(carry_capacity(args.socket, args.session))
    elif args.spawn is not None:
        print(spawn(args.socket, args.session, args.spawn))
    elif args.inspect_damage is not None:
        print(inspect_damage(args.socket, args.session, args.inspect_damage))
    else:
        provision(args.socket, args.session, args.profile, args.node_log, cancel=sys.stdin if args.cancel_on_stdin else None)
