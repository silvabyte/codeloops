#!/usr/bin/env python3
"""Native-manager transport fixture. Real manager acceptance is a separate run."""
import json
import os
from pathlib import Path
import plistlib
import shlex
import signal
import socket
import subprocess
import sys
import time

root = Path(os.environ["CODELOOPS_TEST_SERVICE_DIR"])
state_file = root / "state.json"
state = json.loads(state_file.read_text()) if state_file.exists() else {}
args = sys.argv[1:]
program = Path(sys.argv[0]).name
with (root / "commands.jsonl").open("a") as commands:
    commands.write(json.dumps([program, *args]) + "\n")


def save():
    state_file.write_text(json.dumps(state))


def stop():
    if state.get("running"):
        try:
            os.kill(state["pid"], signal.SIGINT)
        except ProcessLookupError:
            pass
        host, port = state["address"].rsplit(":", 1)
        for _ in range(100):
            with socket.socket() as listener:
                try:
                    listener.bind((host, int(port)))
                    break
                except OSError:
                    time.sleep(0.025)
        else:
            raise RuntimeError("fixture service did not stop")
    state["running"] = False
    save()


def start(command):
    if os.environ.get("CODELOOPS_TEST_SERVICE_FAIL"):
        print("fixture manager refused startup", file=sys.stderr)
        sys.exit(1)
    with Path(state.get("log", root / "service.log")).open("ab") as log:
        process = subprocess.Popen(
            command, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
            start_new_session=True,
        )
    state.update(running=True, pid=process.pid, address=command[command.index("--address") + 1])
    save()


if program == "journalctl":
    log = root / "service.log"
    print(log.read_text()[-65536:] if log.exists() else "No service logs yet.")
elif program == "systemctl":
    assert args.pop(0) == "--user"
    action = args.pop(0)
    if action == "show":
        if "--property=LoadState" in args:
            print("loaded" if state.get("running") or (state.get("definition") and Path(state["definition"]).exists()) else "not-found")
        else:
            print("active" if state.get("running") else "inactive")
    elif action == "enable":
        state["definition"] = args[0]
        save()
    elif action == "restart":
        definition = Path(state["definition"]).read_text()
        line = next(line for line in definition.splitlines() if line.startswith("ExecStart="))
        start([arg.replace("%%", "%").replace("$$", "$") for arg in shlex.split(line.split("=", 1)[1])])
    elif action == "stop":
        stop()
    else:
        assert action in ["show-environment", "daemon-reload", "disable"], action
elif program == "launchctl":
    action = args.pop(0)
    if action == "print":
        if args[0].count("/") == 1:
            sys.exit(0)
        if not state.get("running"):
            sys.exit(113)
        print("state = running")
    elif action == "bootstrap":
        state["definition"] = args[1]
        definition = plistlib.loads(Path(args[1]).read_bytes())
        state["log"] = definition["StandardOutPath"]
        start(definition["ProgramArguments"])
    elif action == "bootout":
        stop()
    else:
        assert action in ["enable", "disable"], action
else:
    raise RuntimeError(program)
