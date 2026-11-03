"""Root-only node-boot control request; carries no proof or database credential."""

import argparse
import socket
import uuid


PROFILES = ("explorer", "knight", "paladin", "sorcerer", "druid", "monk")


def provision(socket_path, session, profile):
    identifier = uuid.UUID(session)
    if identifier.version != 7 or str(identifier) != session or profile not in PROFILES:
        raise ValueError("invalid qualification request")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        control.settimeout(20)
        control.connect(socket_path)
        control.sendall(f"qualify {session} {profile}\n".encode("ascii"))
        control.shutdown(socket.SHUT_WR)
        answer = bytearray()
        while len(answer) <= 64:
            chunk = control.recv(65 - len(answer))
            if not chunk:
                break
            answer.extend(chunk)
        if answer != b"qualified\n":
            raise RuntimeError("node refused qualification provisioning")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--session", required=True)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    args = parser.parse_args()
    provision(args.socket, args.session, args.profile)
