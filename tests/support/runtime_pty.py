"""Run only this suite's child test in an isolated Unix pseudo-terminal."""
import errno
import fcntl
import json
import os
import pty
import re
import resource
import select
import signal
import struct
import subprocess
import sys
import termios
import tempfile
import time

binary, case = sys.argv[1:]
aborting = case.startswith("abort_")
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
master, slave = pty.openpty()
zero = case.startswith("zero")
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 0 if zero else 24, 0 if zero else 40, 0, 0))
env = {key: value for key, value in os.environ.items() if not key.startswith("SLT_") and key not in (
    "TERM_PROGRAM", "TMUX", "STY", "ZELLIJ", "ZELLIJ_SESSION_NAME", "SSH_TTY", "SSH_CONNECTION", "MOSH_IP"
)}
env.update(TERM="xterm-256color", SLT_DISABLE_TERMINAL_QUERIES="1", SLT_RUNTIME_TEST_CASE=case)
focus_directory = tempfile.TemporaryDirectory(prefix="slt-focus-pty-") if case.startswith(("focus_scroll", "pending_")) else None
focus_stage = 0
shutdown_writes = 0
shutdown_input_closed = False
if focus_directory:
    env["SLT_RUNTIME_FOCUS_STATUS"] = os.path.join(focus_directory.name, "status.json")
command = [binary] if aborting else [binary, "--exact", "runtime_pty_child", "--nocapture"]
child = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
os.close(slave)
raw = bytearray()
replies = 0
acted = False
started = time.monotonic()
try:
    while time.monotonic() - started < 15:
        if focus_directory:
            try:
                with open(env["SLT_RUNTIME_FOCUS_STATUS"], encoding="utf-8") as stream:
                    status = json.load(stream)
            except (FileNotFoundError, json.JSONDecodeError):
                status = {}
            if case.startswith("pending_"):
                if case == "pending_disconnect" and not shutdown_input_closed and os.path.exists(os.path.join(focus_directory.name, "status.closed")):
                    try:
                        os.write(master, b"\tignored")
                        shutdown_writes += 1
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        shutdown_input_closed = True
                if focus_stage == 0 and "tick" in status:
                    os.write(master, b"\tZ" if case == "pending_disconnect" else b"\tx" * 100)
                    focus_stage = 1
                elif focus_stage == 1 and status.get("tick", 0) >= 2 and case == "pending_ctrl_c":
                    os.write(master, b"\x03")
                    focus_stage = 2
            elif focus_stage == 0 and status.get("tick", 0) >= 2 and status.get("visible"):
                assert status["focused"] == 0 and status["offset"] == 0, status
                os.write(master, b"\t" * 9 + (b"Z" if case == "focus_scroll_burst" else b""))
                focus_stage = 1
            elif focus_stage == 1 and status.get("focused") == 9 and status.get("visible") and status.get("offset", 0) > 0:
                if case != "focus_scroll_burst":
                    os.write(master, b"Z")
                focus_stage = 2
            elif focus_stage == 2 and status.get("last") == "ZFIELD_09":
                assert status["first"] == "FIELD_00", status
                os.write(master, b"\x1b")
                focus_stage = 3
        elif not acted and time.monotonic() - started >= .15:
            if zero:
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 40, 0, 0))
                os.kill(child.pid, signal.SIGWINCH)
            elif case.startswith("inline"):
                row = 11 if case == "inline_inside" else 1
                os.write(master, f"\x1b[<0;3;{row}M\x1b[<0;3;{row}m".encode())
            acted = True
        if not select.select([master], [], [], .01)[0]:
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
        raw.extend(chunk)
        while replies < raw.count(b"\x1b[6n"):
            os.write(master, b"\x1b[11;1R")
            replies += 1
    child.wait(timeout=2)
    data = bytes(raw)
    if aborting:
        assert child.returncode != 0, data
        assert data.count(b"\x1b[?1049h") == data.count(b"\x1b[?1049l") == 1, data
        assert data.count(b"\x1b[?1000h") == data.count(b"\x1b[?1000l") == 1, data
    else:
        assert child.returncode == 0, data
        assert b"SLT_RESULT" in data, data
    if case.startswith("pending_"):
        assert focus_stage > 0 and b"pending=true" in data, data
        assert data.count(b"\x1b[?1049h") == data.count(b"\x1b[?1049l") == 1, data
        elapsed = time.monotonic() - started
        if case == "pending_ctrl_c":
            assert focus_stage == 2 and elapsed < 3.5, f"Ctrl+C starved behind queued input: {elapsed:.2f}s"
        elif case == "pending_cpu":
            usage = resource.getrusage(resource.RUSAGE_CHILDREN)
            cpu = usage.ru_utime + usage.ru_stime
            assert cpu < elapsed * .5, f"FPS wait spun instead of blocking: CPU={cpu:.2f}s, wall={elapsed:.2f}s"
        elif case == "pending_disconnect":
            assert shutdown_writes > 0 and elapsed < 3.5, "new typing prolonged channel-close draining"
    elif case.startswith("focus_scroll"):
        assert focus_stage == 3 and b"focus_scroll=true" in data, data
        assert data.count(b"\x1b[?1049h") == data.count(b"\x1b[?1049l") == 1, data
    elif case == "boundary":
        assert data.count(b"\x1b[?1049h") == data.count(b"\x1b[?1049l") == 0, data
        assert data.count(b"\x1b[?1000h") == data.count(b"\x1b[?1000l") == 1, data
    elif case == "background":
        assert b"panic=true" in data, data
        assert data.count(b"\x1b[?1049h") == data.count(b"\x1b[?1049l") == 1, data
        assert data.find(b"PANIC_OUTCOME") < data.find(b"\x1b[?1049l"), data
    elif case.startswith("inline"):
        expected = b"clicks=1" if case == "inline_inside" else b"clicks=0"
        assert expected in data, data
    elif zero:
        assert b"delivered=1" in data, data
    elif case == "idle":
        assert int(re.search(rb"SLT_RESULT frames=(\d+)", data).group(1)) == 1, data
        assert time.monotonic() - started < 3, "cancellation ignored the wake"
    print(f"{case}: passed")
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
    if focus_directory:
        focus_directory.cleanup()
