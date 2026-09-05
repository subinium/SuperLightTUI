"""Run only this suite's child test in an isolated Unix pseudo-terminal."""
import errno
import fcntl
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
command = [binary] if aborting else [binary, "--exact", "runtime_pty_child", "--nocapture"]
child = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
os.close(slave)
raw = bytearray()
replies = 0
acted = False
started = time.monotonic()
try:
    while time.monotonic() - started < 15:
        if not acted and time.monotonic() - started >= .15:
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
    if case == "boundary":
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
