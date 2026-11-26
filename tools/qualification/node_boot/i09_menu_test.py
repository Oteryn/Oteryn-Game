"""Locate and build the Bevy menu test only when the selected batch includes I09."""
import json
import os
import subprocess
import sys

from client_replay import selected_rows


def main():
    try:
        selected, _ = selected_rows(os.environ.get("OTERYN_E2E_ROW_BATCH"))
    except ValueError:
        # The normal row validator reports invalid batch selection.
        return
    if "I09.client" not in selected:
        return
    build = subprocess.run(
        [os.environ.get("CARGO", "cargo"), "test", "--locked", "-p", "oteryn-client",
         "--features", "bevy-shell", "--lib", "--no-run", "--message-format=json"],
        stdout=subprocess.PIPE, text=True, check=False,
    )
    tests = []
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message["reason"] == "compiler-message":
            rendered = message["message"]["rendered"]
            if rendered:
                print(rendered, end="", file=sys.stderr)
        if (message["reason"] == "compiler-artifact"
                and message["target"]["name"] == "oteryn_client"
                and message["profile"]["test"] and message["executable"]):
            tests.append(message["executable"])
    build.check_returncode()
    if len(tests) != 1:
        raise RuntimeError("expected one Bevy library test executable for I09")
    print(tests[0])


if __name__ == "__main__":
    main()
