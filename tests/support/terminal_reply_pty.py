"""Exercise reply/native parser handoff in an isolated real Unix PTY."""
import errno
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import time

binary, case = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 40, 360, 432))
env = os.environ.copy()
env.update(TERM="xterm-256color", SLT_REPLY_REVIEW_CASE=case)
child = subprocess.Popen([binary, "--exact", "event::reply::review_tests::review_pty_child", "--nocapture"],
                         stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
os.close(slave)
data = bytearray()
sent = set()
prefix, suffix = {
    "order": (b"x" * 256 + b"a", b"b"),
    "paste": (b"x\x1b[200~A", b"B\x1b[201~"),
    "utf8": (b"x\xe7", b"\x95\x8c"),
    "csi": (b"x\x1b[", b"D"),
    "ss3": (b"x\x1bO", b"P"),
    "kitty": (b"x\x1b[574", b"42u"),
}.get(case, (b"", b""))
fixture_events = [f"\x1b[{point};{mask}:{kind}u".encode()
                  for point in list(range(57358, 57364)) + list(range(57376, 57455))
                  for mask, kind in [(1, 1), (6, 2), (193, 3)]] + [b"\x1b[97:65;2:1u", b"\x1b[9;2:1u"]
fixture = b"".join(fixture_events)
assert len(fixture_events) == 257 and len(fixture) == 3250
triggers = {
    b"REVIEW_PRIME\r": prefix,
    b"REVIEW_PRIME\n": prefix,
    b"REVIEW_PRIMED": suffix,
    b"\x1b]777;QUERY1\x07": b"\x1b[20",
    b"\x1b]777;QUERY2\x07": b"0~A\x1b[?62;4cB\x1b[201",
    b"\x1b]777;QUERY3\x07": b"~\x1b[?62;4c",
    b"\x1b]777;FUNCTIONAL_QUERY\x07": fixture + b"\x1b[?62;4c",
}
# Mapping parity is separate from the upstream unsplit-burst liveness defect.
# No sleeps or extra wake bytes: the child requests each batch after consuming
# its predecessor, and every batch fits below the native 1024-byte read size.
for batch, offset in enumerate(range(0, len(fixture_events), 64)):
    payload = b"".join(fixture_events[offset:offset + 64])
    assert len(payload) < 1024
    triggers[f"REVIEW_FUNCTIONAL_NATIVE_{batch}\n".encode()] = payload

def write_all(payload):
    remaining = memoryview(payload)
    while remaining:
        written = os.write(master, remaining)
        assert written > 0, "PTY write made no progress"
        remaining = remaining[written:]

try:
    deadline = time.monotonic() + 15
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
        for marker, payload in triggers.items():
            if marker in data and marker not in sent:
                write_all(payload)
                sent.add(marker)
    try:
        child.wait(timeout=2)
    except subprocess.TimeoutExpired as error:
        raise AssertionError((case, bytes(data), sent)) from error
    assert child.returncode == 0 and b"REVIEW_PASSED" in data, bytes(data)
    assert b"\x1b]52;c;?\x07" not in data, "raw query emitted after native ownership"
    print(f"{case}: passed")
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
