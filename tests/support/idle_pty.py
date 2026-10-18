"""Drive one idle-skip PTY case (#490) and report what the terminal received.

Usage: idle_pty.py BINARY TEST_NAME CASE COLS ROWS TIMEOUT_S SCRIPT_JSON [WATCH...]

SCRIPT_JSON is a list of timed actions relative to child start:
  {"at": 0.8, "write": "a"}                 write bytes (UTF-8, escapes allowed)
  {"at": 1.4, "resize": [100, 30]}          TIOCSWINSZ + SIGWINCH
Each WATCH string is searched in the output; the report gives the first time
it appeared. SLT_IDLE_* variables are passed through to the child.

Prints one JSON object: output chunk timeline, action times, first-seen
times of WATCH strings, child CPU seconds (rusage), wall time, exit status
and the child's `SLT_RESULT {...}` line.
"""

import errno
import fcntl
import json
import os
import pty
import resource
import select
import signal
import struct
import subprocess
import sys
import termios
import time

binary, test_name, case, cols, rows, timeout, script = sys.argv[1:8]
watches = [w.encode() for w in sys.argv[8:]]
cols, rows, timeout = int(cols), int(rows), float(timeout)
script = sorted(json.loads(script), key=lambda action: action["at"])

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
env = {
    key: value
    for key, value in os.environ.items()
    if not key.startswith("SLT_")
    and key
    not in (
        "TERM_PROGRAM",
        "TMUX",
        "STY",
        "ZELLIJ",
        "ZELLIJ_SESSION_NAME",
        "SSH_TTY",
        "SSH_CONNECTION",
        "MOSH_IP",
    )
}
env.update(TERM="xterm-256color", SLT_DISABLE_TERMINAL_QUERIES="1", SLT_IDLE_CASE=case)
for key, value in os.environ.items():
    if key.startswith("SLT_IDLE_") and key != "SLT_IDLE_CASE":
        env[key] = value

before = resource.getrusage(resource.RUSAGE_CHILDREN)
started = time.monotonic()
child = subprocess.Popen(
    [binary, "--exact", test_name, "--nocapture"],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    env=env,
    start_new_session=True,
)
os.close(slave)
raw = bytearray()
chunks = []
actions = []
seen = {}
replies = 0
pending = list(script)
try:
    while time.monotonic() - started < timeout:
        now = time.monotonic() - started
        while pending and pending[0]["at"] <= now:
            action = pending.pop(0)
            if "write" in action:
                os.write(master, action["write"].encode())
            elif "resize" in action:
                width, height = action["resize"]
                fcntl.ioctl(
                    master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0)
                )
                os.kill(child.pid, signal.SIGWINCH)
            actions.append([time.monotonic() - started, action])
        if not select.select([master], [], [], 0.002)[0]:
            if child.poll() is not None:
                break
            continue
        try:
            chunk = os.read(master, 65536)
        except OSError as error:
            if error.errno == errno.EIO:
                break
            raise
        if not chunk:
            break
        at = time.monotonic() - started
        start = max(0, len(raw) - max((len(w) for w in watches), default=0))
        raw.extend(chunk)
        chunks.append([at, len(chunk)])
        for watch in watches:
            if watch.decode() not in seen and raw.find(watch, start) >= 0:
                seen[watch.decode()] = at
        while replies < raw.count(b"\x1b[6n"):
            os.write(master, b"\x1b[11;1R")
            replies += 1
    child.wait(timeout=5)
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
wall = time.monotonic() - started
after = resource.getrusage(resource.RUSAGE_CHILDREN)
cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
data = bytes(raw)
marker = data.rfind(b"SLT_RESULT ")
result = None
if marker >= 0:
    line = data[marker + len(b"SLT_RESULT ") :].split(b"\n", 1)[0].strip()
    result = json.loads(line.decode())
print(
    json.dumps(
        {
            "status": child.returncode,
            "wall": wall,
            "cpu": cpu,
            "bytes": len(data),
            "chunks": chunks,
            "actions": actions,
            "seen": seen,
            "result": result,
            "tail": data[-400:].decode(errors="replace") if result is None else "",
        }
    )
)
