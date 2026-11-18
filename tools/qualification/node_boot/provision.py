"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import os
import re
import select
import socket
import sys
import time
from threading import Thread
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk", "combat-progression", "private-channel", "coin-converters", "rune-training", "sweep", "atelier", "victim")
CREATURE_KEY = re.compile(r"[A-Za-z0-9:._/-]{1,128}")
CAPACITY = re.compile(rb"capacity [1-9][0-9]* [1-9][0-9]*\n")
SPAWNED = re.compile(rb"spawned [0-9a-f]{32} [0-9]+ [0-9]+\n")


# The server bounds each accepted control request at 30 seconds.
CONTROL_REPLY_TIMEOUT = 40
CONTROL_QUEUE_TIMEOUT = 40


def _request(socket_path, line, limit, cancel=None):
    """One control exchange; returns up to `limit` answer bytes plus one overflow-detecting byte."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        queue_deadline = time.monotonic() + CONTROL_QUEUE_TIMEOUT
        control.settimeout(CONTROL_QUEUE_TIMEOUT)
        control.connect(socket_path)
        control.settimeout(max(0.001, queue_deadline - time.monotonic()))
        control.sendall(b"!")
        control.settimeout(max(0.001, queue_deadline - time.monotonic()))
        if control.recv(1) != b"!" or time.monotonic() >= queue_deadline:
            raise TimeoutError("qualification control was not ready in time")
        deadline = time.monotonic() + CONTROL_REPLY_TIMEOUT
        control.settimeout(CONTROL_REPLY_TIMEOUT)
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


def provision(socket_path, session, profile, cancel=None):
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or profile not in PROFILES:
        raise ValueError("invalid qualification request")
    answer = _request(socket_path, f"qualify {session} {profile}\n", 64, cancel)
    if answer != b"qualified\n":
        # repr escapes control/non-ASCII bytes; the 65th byte only detects overflow.
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


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session", required=True)
    parser.add_argument("--watch-stdin", action="store_true")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--profile", choices=PROFILES)
    group.add_argument("--spawn", metavar="CREATURE_KEY")
    group.add_argument("--item-handle", type=int)
    group.add_argument("--recovery-epoch", type=int)
    group.add_argument("--capacity", action="store_true")
    parser.add_argument("--cancel-on-stdin", action="store_true")
    args = parser.parse_args()
    if args.watch_stdin:
        def stop_when_parent_drops_pipe():
            # The parent deliberately sends no bytes. EOF closes the control request
            # even when sudo's outer process and Python have different credentials.
            os.read(0, 1)
            os._exit(125)

        Thread(target=stop_when_parent_drops_pipe, daemon=True).start()
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
    else:
        provision(args.socket, args.session, args.profile, sys.stdin if args.cancel_on_stdin else None)
