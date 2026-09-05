"""Run public SLT consumers in a real PTY with no rescue/wake input."""
import errno
import fcntl
import os
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time

binary, mode, case = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 40, 360, 432))
env = {key: value for key, value in os.environ.items() if not key.startswith("SLT_") and key not in (
    "TERM_PROGRAM", "TMUX", "STY", "ZELLIJ", "ZELLIJ_SESSION_NAME", "SSH_TTY", "SSH_CONNECTION", "MOSH_IP"
)}
env.update(TERM="xterm-256color", SLT_DISABLE_TERMINAL_QUERIES="1", SLT_BURST_CASE=case, SLT_BURST_MODE=mode)
child = subprocess.Popen([binary, "--exact", "public_burst_child", "--nocapture"],
                         stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
os.close(slave)
functional = b"".join(f"\x1b[{point};{mask}:{kind}u".encode()
                      for point in list(range(57358, 57364)) + list(range(57376, 57455))
                      for mask, kind in [(1, 1), (6, 2), (193, 3)]) + b"\x1b[97:65;2:1u\x1b[9;2:1u"
assert len(functional) == 3250
paste = ("paste:" + "x" * 3072 + "\n\x1b[?62;4c\u754c").encode()
payload = {
    "functional": functional,
    "paste": b"\x1b[200~" + paste + b"\x1b[201~",
    "split": b"a" * 1023 + b"\xe7",
    "partial_idle": b"x\x1b[200~unfinished",
    "async_partial": b"x\x1b[200~unfinished",
    "async_idle": b"",
    "resize": b"",
}.get(case)
if case.startswith("ascii_"):
    payload = b"x" * int(case.split("_")[1])
assert payload is not None, case
split_tails = [b"\x95\x8c\x1b[", b"D\x1bO", b"P\x1b[1;", b"5D\x1b[200~payload\x1b[?62;4c\x1b[201", b"~"]
data = bytearray()
started = False
split_sent = set()
input_writes = []
cursor_replies = 0

def send_once(value):
    written = os.write(master, value)
    assert written == len(value), ("fixture write was partial", written, len(value))
    input_writes.append(written)

try:
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
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
        data.extend(chunk)
        assert len(data) <= 8 * 1024 * 1024, "consumer exceeded bounded output capture"
        while cursor_replies < data.count(b"\x1b[6n"):
            assert not started, "cursor reply after payload would be extra wake input"
            assert os.write(master, b"\x1b[11;1R") == len(b"\x1b[11;1R")
            cursor_replies += 1
        if not started and b"\x1b]777;BURST_READY\x07" in data:
            started = True
            if payload:
                send_once(payload)
            if case == "resize":
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 18, 50, 450, 324))
                os.kill(child.pid, signal.SIGWINCH)
        if case == "split":
            for index, tail in enumerate(split_tails, 1):
                marker = f"\x1b]777;BURST_SPLIT_{index}\x07".encode()
                if marker in data and index not in split_sent:
                    send_once(tail)
                    split_sent.add(index)
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired as error:
        raise AssertionError((mode, case, "consumer did not terminate", bytes(data)[-8000:])) from error
    assert started and child.returncode == 0, (mode, case, bytes(data)[-8000:])
    assert b"SLT_BURST_RESULT" in data, bytes(data)[-8000:]
    if case == "split":
        assert len(input_writes) == 6 and len(split_sent) == 5
    elif payload:
        assert input_writes == [len(payload)], "burst was not delivered in a single write"
    else:
        assert not input_writes, "idle/resize case injected keyboard input"
    summary = re.search(rb"SLT_BURST_RESULT[^\r\n]*", data).group().decode()
    print(f"{mode}/{case}: writes={input_writes} {summary}")
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
