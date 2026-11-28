#!/usr/bin/env python3
"""Obtain a real, single-use Platform grant for a disposable row character."""

import base64
import hashlib
import json
import re
import secrets
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def request(opener, url, *, data=None, headers=None, statuses=(200,)):
    request = urllib.request.Request(url, data, headers or {})
    try:
        response = opener.open(request, timeout=5)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        if response.code not in statuses:
            raise RuntimeError("Platform response refused")
        body = response.read(65537)
        if len(body) > 65536:
            raise RuntimeError("Platform response exceeds limit")
        return body, response.headers


def csrf(body):
    return re.search(rb'name="_csrf" value="([a-fA-F0-9]{64})"', body)[1].decode("ascii")


def form(values):
    return urllib.parse.urlencode(values).encode("ascii")


def loopback(url):
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme != "http" or parsed.hostname != "127.0.0.1" or parsed.path not in ("", "/"):
        raise RuntimeError("Platform endpoint is outside the disposable stack")
    return url.rstrip("/")


def main():
    work, account, character, world, channel, authority = sys.argv[1:]
    work = Path(work)
    identity = json.loads((work / "identity.json").read_text())
    report = json.loads((work / "portal-report.json").read_text())
    accounts = json.loads((work / "row-accounts.json").read_text())
    public = loopback(report["baseUrls"]["public"])
    gateway = loopback(report["baseUrls"]["gateway"])
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    credentials = accounts[account]
    login, headers = request(opener, public + "/en/login")
    cookie = headers.get("Set-Cookie", "").split(";", 1)[0]
    _, headers = request(
        opener, public + "/en/login",
        data=form({**credentials, "_csrf": csrf(login)}),
        headers={"Origin": public, "Cookie": cookie}, statuses=(302, 303),
    )
    cookie = next(value.split(";", 1)[0] for value in headers.get_all("Set-Cookie")
                  if value.startswith("__Host-oteryn_session="))
    verifier = secrets.token_urlsafe(48)
    state = secrets.token_urlsafe(32)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    fields = dict(client_id=identity["client"], redirect_uri=identity["callback"], response_type="code",
                  scope="game:ticket", state=state, code_challenge=challenge, code_challenge_method="S256")
    path = public + "/oauth/authorize"
    body, _ = request(opener, path + "?" + form(fields).decode(), headers={"Cookie": cookie})
    _, headers = request(opener, path, data=form({**fields, "_csrf": csrf(body), "decision": "approve"}),
                         headers={"Origin": public, "Cookie": cookie}, statuses=(302, 303))
    callback = urllib.parse.urlsplit(headers["Location"])
    if callback._replace(query="").geturl() != identity["callback"]:
        raise RuntimeError("Platform callback mismatch")
    values = urllib.parse.parse_qs(callback.query, strict_parsing=True)
    if values["state"] != [state] or len(values["code"]) != 1:
        raise RuntimeError("Platform authorization state mismatch")
    token, _ = request(opener, public + "/oauth/token", data=form(dict(
        grant_type="authorization_code", code=values["code"][0], code_verifier=verifier,
        client_id=identity["client"], redirect_uri=identity["callback"],
    )))
    ticket, _ = request(opener, public + "/api/v1/game-auth/tickets", data=b'{"protocol_version":1}',
                        headers={"Content-Type": "application/json",
                                 "Authorization": "Bearer " + json.loads(token)["access_token"]})
    random = bytearray(secrets.token_bytes(16))
    random[:6] = (time.time_ns() // 1_000_000).to_bytes(6, "big")
    random[6] = random[6] & 15 | 0x70
    random[8] = random[8] & 63 | 0x80
    attempt = str(uuid.UUID(bytes=bytes(random)))
    body = dict(protocol_version=2, game_login_ticket=json.loads(ticket)["ticket"], attempt_ref=attempt,
                character_id=character, channel_id=channel, offer=dict(
                    client_build="node-boot-qualification", client_platform="windows",
                    transports=[dict(protocol_major=1, transport_profile=1, alpn="oteryn-game/1")]))
    raw, _ = request(opener, gateway + "/v1/login", data=json.dumps(body).encode(),
                     headers={"Content-Type": "application/json"})
    result = json.loads(raw)
    endpoint = result["endpoint"]
    if (result["attempt_ref"], result["world_id"], result["channel_id"]) != (attempt, world, channel):
        raise RuntimeError("Platform grant scope mismatch")
    if f'{endpoint["host"]}:{endpoint["port"]}' != authority or endpoint["tls_server_name"] != "localhost":
        raise RuntimeError("Platform grant endpoint mismatch")
    grant = result["grant"]
    if grant["profile"] != "oteryn-pre-admission-v1" or not re.fullmatch(r"[A-Za-z0-9_.-]{1,8192}", grant["token"]):
        raise RuntimeError("Platform grant response invalid")
    sys.stdout.write(grant["token"])


if __name__ == "__main__":
    try:
        main()
    except (AttributeError, KeyError, IndexError, OSError, RuntimeError, TypeError, ValueError):
        # Never print a ticket, credential, grant, or Platform response.
        sys.exit("platform_grant_issuance_failed")
