"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import json
import secrets
import socket
import subprocess
import time
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "c17a", "c17c")


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
            raise RuntimeError("node refused qualification provisioning")


def fresh_character(socket_path, project, request):
    """Issue a real Platform intent, then ask the existing node bootstrap owner."""
    operation = uuid.UUID(int=(int(time.time() * 1000) << 80)
                          | (7 << 76) | (secrets.randbits(12) << 64)
                          | (2 << 62) | secrets.randbits(62))
    name = "Spell " + "".join(chr(ord("a") + int(c, 16)) for c in operation.hex[-20:])
    request.update(operation=str(operation), name=name)
    containers = subprocess.check_output([
        "docker", "ps", "--filter", f"label=com.docker.compose.project={project}",
        "--filter", "label=com.docker.compose.service=platform", "--format", "{{.ID}}",
    ], text=True, timeout=20).split()
    if len(containers) != 1:
        raise RuntimeError("qualification requires one Platform container")
    php = r'''require "vendor/autoload.php";
$app=require "bootstrap/app.php";
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
$r=json_decode($argv[1],true,512,JSON_THROW_ON_ERROR);
$id=Illuminate\Support\Facades\DB::table("identities")->where("account_id",$r["account"])->value("id");
if (!$id) { exit(1); }
$code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",[
"--identity-id"=>(string)$id,"--operation-id"=>$r["operation"],
"--target-world-id"=>$r["world"],"--requested-name"=>$r["name"],"--requested-sex"=>"Female",
"--profile-revision"=>$r["revisions"][0],"--ruleset-revision"=>$r["revisions"][1],
"--content-revision"=>$r["revisions"][2],"--starter-template-revision"=>$r["revisions"][3]]);
exit($code);'''
    subprocess.run(["docker", "exec", "--user", "www-data", containers[0],
                    "php", "-r", php, json.dumps(request)],
                   check=True, stdout=subprocess.DEVNULL, timeout=30)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        control.settimeout(60)
        control.connect(socket_path)
        control.sendall(f"{operation}\n".encode("ascii"))
        control.shutdown(socket.SHUT_WR)
        answer = bytearray()
        while len(answer) <= 64:
            chunk = control.recv(65 - len(answer))
            if not chunk:
                break
            answer.extend(chunk)
        if answer != b"committed\n":
            raise RuntimeError("node refused fresh spell character bootstrap")
    return name


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
