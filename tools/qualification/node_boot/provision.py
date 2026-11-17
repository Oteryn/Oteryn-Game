"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import json
import re
import secrets
import socket
import subprocess
import sys
import time
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "c17a", "c17c")


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


def fresh_character_name(operation):
    # Both pinned naming contracts allow 2..29 ASCII letters with single spaces;
    # neither imposes a per-word limit. This name is 26 characters long.
    return "Spell " + "".join(chr(ord("a") + int(c, 16)) for c in operation.hex[-20:])


def provision(socket_path, session, profile):
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or profile not in PROFILES:
        raise ValueError("invalid qualification request")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        deadline = time.monotonic() + 20
        control.settimeout(20)
        control.connect(socket_path)
        control.sendall(f"qualify {session} {profile}\n".encode("ascii"))
        control.shutdown(socket.SHUT_WR)
        answer = bytearray()
        while len(answer) <= 64:
            control.settimeout(max(0.001, deadline - time.monotonic()))
            if time.monotonic() >= deadline:
                raise TimeoutError("qualification provisioning deadline exceeded")
            chunk = control.recv(65 - len(answer))
            if not chunk:
                break
            answer.extend(chunk)
        if answer != b"qualified\n":
            raise RuntimeError(f"node refused qualification provisioning: answer={sanitized_text(answer.decode(errors='replace')) or '<empty or filtered>'!r}")


def fresh_character(socket_path, project, request):
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
    commit_character_bootstrap(socket_path, operation)
    return name


def commit_character_bootstrap(socket_path, operation):
    # D2's unavailable result is ambiguous, not a rejection. As game-ops instructs,
    # reconcile the SAME operation: a committed receipt wins before any external
    # reads, and the Platform intent must never be reissued on this path.
    deadline = time.monotonic() + 60
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("fresh spell character bootstrap deadline exceeded; answer='unavailable'")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
            control.settimeout(remaining)
            control.connect(socket_path)
            control.sendall(f"{operation}\n".encode("ascii"))
            control.shutdown(socket.SHUT_WR)
            answer = bytearray()
            while len(answer) <= 64:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("fresh spell character bootstrap deadline exceeded")
                control.settimeout(remaining)
                chunk = control.recv(65 - len(answer))
                if not chunk:
                    break
                answer.extend(chunk)
        if answer == b"committed\n":
            return
        if answer != b"unavailable\n":
            raise RuntimeError(f"node refused fresh spell character bootstrap: answer={sanitized_text(answer.decode(errors='replace')) or '<empty or filtered>'!r}")
        print("C17 bootstrap unavailable; reconciling the same operation id", file=sys.stderr)
        time.sleep(min(0.5, max(0, deadline - time.monotonic())))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session")
    parser.add_argument("--fresh-character")
    parser.add_argument("--project")
    parser.add_argument("--profile", choices=PROFILES)
    args = parser.parse_args()
    if args.fresh_character:
        if not args.project:
            parser.error("--fresh-character requires --project")
        print(fresh_character(args.socket, args.project, json.loads(args.fresh_character)))
    else:
        if not args.session or not args.profile:
            parser.error("provisioning requires --session and --profile")
        provision(args.socket, args.session, args.profile)
