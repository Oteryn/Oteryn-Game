#!/usr/bin/env python3
"""Provision two disposable staff sessions and verify real Premium ledger grants."""
import base64
import hashlib
import hmac
import http.cookiejar
import json
from pathlib import Path
import re
import secrets
import subprocess
import sys
import time
import urllib.request
import uuid

from platform_grant import NoRedirect, csrf, form, grant_premium, loopback, request


def totp(secret, timestamp):
    key = base64.b32decode(secret + b"=" * (-len(secret) % 8))
    digest = hmac.digest(key, (timestamp // 30).to_bytes(8, "big"), hashlib.sha1)
    offset = digest[-1] & 15
    return f"{(int.from_bytes(digest[offset:offset + 4], 'big') & 0x7fffffff) % 1000000:06d}"


def staff_session(public, credentials):
    jar = http.cookiejar.CookieJar(
        http.cookiejar.DefaultCookiePolicy(secure_protocols=("https", "http")))
    opener = urllib.request.build_opener(
        urllib.request.ProxyHandler({}), NoRedirect(), urllib.request.HTTPCookieProcessor(jar))

    def get(path):
        return request(opener, public + path)[0]

    def post(path, page, fields, statuses=(200,)):
        return request(opener, public + path, data=form({**fields, "_csrf": csrf(page)}),
                       headers={"Origin": public}, statuses=statuses)

    def login():
        return post("/en/login", get("/en/login"), credentials, (302, 303))[1].get("Location")

    login()
    password = {"password": credentials["password"]}
    enrollment, _ = post("/en/mfa/enroll", get("/en/mfa"), password)
    secret = re.search(rb'class="op-prose">([A-Z2-7]+)</p>', enrollment)[1]
    codes, _ = post("/en/mfa/confirm", enrollment,
                    {**password, "code": totp(secret, int(time.time()))})
    recovery = re.search(rb'<textarea[^>]*readonly[^>]*>([^<]+)</textarea>', codes)[1]
    if login() != "/en/mfa/challenge":
        raise RuntimeError("Platform staff MFA challenge unavailable")
    post("/en/mfa/challenge", get("/en/mfa/challenge"),
         {"code": recovery.decode("ascii")}, (302, 303))
    return opener


def run(work, container, accounts):
    report = json.loads((work / "portal-report.json").read_text())
    public = loopback(report["baseUrls"]["public"])
    database = report["database"]
    if not re.fullmatch(r"portal_e2e_[a-z0-9_]+", database):
        raise ValueError("unexpected Platform database name")
    env = json.loads((work / "portal/secrets/env.json").read_text())
    binary = json.loads((work / "identity.json").read_text())["portal_binary"]

    def sql(statement):
        return subprocess.run(
            ["docker", "exec", "-i", container, "psql", "-XqAt", "-v", "ON_ERROR_STOP=1",
             "-U", "game_platform_admin", "-d", database],
            input=statement, text=True, capture_output=True, timeout=20, check=True).stdout.strip()

    def staff():
        account = str(uuid.uuid4())
        credentials = {"email": f"qualification-{account}@example.invalid",
                       "password": secrets.token_urlsafe(48)}
        subprocess.run([binary, "seed", "account", json.dumps({"accountId": account, **credentials})],
                       env=env, capture_output=True, timeout=20, check=True)
        sql("INSERT INTO account_roles(account_id,role_id) "
            f"SELECT '{account}'::uuid,id FROM roles WHERE key='platform_admin';")
        return staff_session(public, credentials)

    proposer, approver = staff(), staff()
    for account in dict.fromkeys(accounts):
        account = str(uuid.UUID(account))
        operation = grant_premium(public, account, proposer, approver)
        if sql("SELECT event_type || ':' || duration_days || ':' || delta_seconds || ':' || "
               "(effective_until > now())::text FROM premium_ledger "
               f"WHERE request_id='{operation}'::uuid AND account_id='{account}'::uuid;") != "GRANT:1:86400:true":
            raise RuntimeError("Platform Premium ledger assertion failed")
        print("PREMIUM_GRANT_EVIDENCE independent_mfa_approval=1 ledger_active=1 duration_days=1")


if __name__ == "__main__":
    try:
        run(Path(sys.argv[1]), sys.argv[2], sys.argv[3:])
    except (OSError, ValueError, TypeError, LookupError, RuntimeError, subprocess.SubprocessError):
        sys.exit("Platform qualification Premium provisioning failed")
