"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import re
import socket
import time
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "knight-underlevel", "c05-exiva", "w20b-light", "s13-party", "s08-knight", "s08-sorcerer", "f5-c17a", "f5-c17c", "f5-c13", "f5-c18a", "f5-c17d", "c11-unassigned", "c11-knight", "c12-monk", "p01-sorcerer", "premium")
CREATURE_KEY = re.compile(r"[A-Za-z0-9:._/-]{1,128}")
NAMED_ITEM = re.compile(r"item-([1-9][0-9]{0,9})-(?:[1-9]|1[0-9]|20)")
SPAWNED = re.compile(rb"spawned [0-9a-f]{32} [0-9]+\n")


def _request(socket_path, line, limit):
    """One control exchange; returns up to `limit` answer bytes plus one overflow-detecting byte."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        deadline = time.monotonic() + 20
        control.settimeout(20)
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


def provision(socket_path, session, profile):
    identifier = uuid.UUID(session)
    item = NAMED_ITEM.fullmatch(profile)
    valid_profile = profile in PROFILES or (item is not None and int(item[1]) <= 0xFFFFFFFF)
    if identifier.version != 7 or str(identifier) != session or not valid_profile:
        raise ValueError("invalid qualification request")
    answer = _request(socket_path, f"qualify {session} {profile}\n", 64)
    if answer != b"qualified\n":
        # repr escapes control/non-ASCII bytes; the 65th byte only detects overflow.
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


def inspect_damage(socket_path, session, target):
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or not re.fullmatch(r"[0-9a-f]{32} [1-9][0-9]{0,19}", target):
        raise ValueError("invalid qualification damage inspection")
    answer = bytes(_request(socket_path, f"inspect-damage {session} {target}\n", 128))
    if not re.fullmatch(rb"damage [0-9]{1,20} [0-9a-f]{32}\n", answer):
        raise RuntimeError(f"node refused qualification damage inspection: {answer[:128]!r}")
    return answer.decode("ascii").rstrip("\n")


def force_enflame(socket_path, session, target_rolls):
    identifier = uuid.UUID(session)
    if (
        identifier.version != 7
        or str(identifier) != session
        or not re.fullmatch(r"[0-9a-f]{32} [1-9][0-9]{0,19} [0-9:,]{1,80}", target_rolls)
    ):
        raise ValueError("invalid qualification Enflame roll request")
    answer = bytes(_request(socket_path, f"force-enflame {session} {target_rolls}\n", 64))
    if answer != b"armed\n":
        raise RuntimeError(f"node refused qualification Enflame rolls: {answer[:64]!r}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session", required=True)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--profile")
    group.add_argument("--spawn", metavar="CREATURE_KEY")
    group.add_argument("--inspect-damage", metavar="TARGET")
    group.add_argument("--force-enflame", metavar="TARGET_ROLLS")
    args = parser.parse_args()
    if args.spawn is not None:
        print(spawn(args.socket, args.session, args.spawn))
    elif args.inspect_damage is not None:
        print(inspect_damage(args.socket, args.session, args.inspect_damage))
    elif args.force_enflame is not None:
        force_enflame(args.socket, args.session, args.force_enflame)
    else:
        provision(args.socket, args.session, args.profile)
