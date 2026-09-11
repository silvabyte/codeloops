"""Opt-in acceptance against the real user service manager, using isolated state."""
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]


def command(args, env, check=True):
    result = subprocess.run(args, env=env, cwd=REPO, capture_output=True, text=True, timeout=120)
    if check and result.returncode:
        raise RuntimeError(f"{args[0]} failed:\n{result.stdout}\n{result.stderr}")
    return result


def pid(name, env):
    if sys.platform == "darwin":
        result = command(["launchctl", "print", f"gui/{os.getuid()}/{name}"], env, check=False)
        match = re.search(r"^\s*pid = (\d+)$", result.stdout, re.MULTILINE)
        return int(match[1]) if match else 0
    result = command(["systemctl", "--user", "show", name + ".service", "--property=MainPID", "--value"], env)
    return int(result.stdout.strip())


root = Path(tempfile.mkdtemp(prefix="codeloops-native-service-"))
home = root / "home"
home.mkdir()
prefix = root / "install 'quoted' %"
data = root / "history 'quoted' %"
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    address = f"127.0.0.1:{listener.getsockname()[1]}"
env = os.environ.copy()
env.update(
    HOME=str(home), XDG_CONFIG_HOME=str(home / ".config"),
    XDG_DATA_HOME=str(home / ".local/share"),
    CARGO_HOME=os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")),
    RUSTUP_HOME=os.environ.get("RUSTUP_HOME", str(Path.home() / ".rustup")),
)
for key in ["CODELOOPS_ADDRESS", "CODELOOPS_DATA_DIR", "CODELOOPS_TEST_SERVICE_DIR"]:
    env.pop(key, None)
binary = prefix / "bin/codeloops"
make_args = [
    "make", "start", f"PREFIX={prefix}", f"DATA_DIR={data}",
    f"ADDRESS={address}", "PROFILE=native-test", "SETUP_ARGS=--opencode-version fixture",
]
name = None
try:
    output = command(make_args, env)
    print(output.stdout.strip())
    record = json.loads((prefix / "share/codeloops/service.json").read_text())
    name = record["name"]
    assert json.loads(command([str(binary), "service", "status", "--json"], env).stdout)["healthy"]
    first_pid = pid(name, env)
    assert first_pid > 0
    # Kill only the unique fixture's managed main process, then observe native
    # supervision restart it after the launcher has already returned.
    if sys.platform == "darwin":
        command(["launchctl", "kill", "SIGKILL", f"gui/{os.getuid()}/{name}"], env)
    else:
        command(["systemctl", "--user", "kill", "--kill-whom=main", "--signal=SIGKILL", name + ".service"], env)
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        current_pid = pid(name, env)
        if current_pid not in [0, first_pid] and command([str(binary), "health"], env, check=False).returncode == 0:
            break
        time.sleep(0.1)
    else:
        raise RuntimeError("native manager did not recover after the service crash")
    print("PASS native manager: launcher exited, service stayed healthy, crash restarted with a new PID")
    logs = json.loads(command([str(binary), "service", "logs", "--json"], env).stdout)
    assert "CodeLoops listening at" in logs["logs"]
    command([str(binary), "service", "stop"], env)
    assert not json.loads(command([str(binary), "service", "status", "--json"], env).stdout)["running"]
    if sys.platform != "darwin":
        assert command(["systemctl", "--user", "is-enabled", name + ".service"], env, check=False).returncode != 0
    command(make_args, env)
    command([str(binary), "uninstall"], env)
    assert not binary.exists()
    assert not Path(record["path"]).exists()
    assert (data / "archive/history.sqlite3").exists()
    assert pid(name, env) == 0
    print("PASS native manager: logs, stop/disable, restart, and live uninstall with archive preservation")
finally:
    cleanup = command(["make", "uninstall", f"PREFIX={prefix}"], env, check=False)
    if cleanup.returncode:
        raise RuntimeError(f"Native fixture cleanup failed; state preserved at {root}: {cleanup.stderr}")
    if name and sys.platform != "darwin":
        command(["systemctl", "--user", "reset-failed", name + ".service"], env, check=False)
    shutil.rmtree(root)
