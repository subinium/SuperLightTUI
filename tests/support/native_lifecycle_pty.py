"""Isolated cursor-query and source-lifecycle acceptance with actual PTYs."""
import errno
import os
from pathlib import Path
import pty
import select
import subprocess
import sys
import tempfile
import time

binary, case = sys.argv[1:]
with tempfile.TemporaryDirectory(prefix="slt-native-lifecycle-") as directory:
    ack = Path(directory) / "late-reply-written"
    master, slave = pty.openpty()
    env = dict(os.environ, TERM="xterm-256color", SLT_NATIVE_LIFECYCLE_CASE=case, SLT_NATIVE_ACK=str(ack))
    child = subprocess.Popen([binary, "--exact", "event::native::lifecycle_tests::native_lifecycle_child", "--nocapture"],
                             stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
    os.close(slave)
    data = bytearray()
    sent = set()
    def send(payload):
        remaining = memoryview(payload)
        while remaining:
            count = os.write(master, remaining)
            assert count > 0
            remaining = remaining[count:]
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
            if case == "fatal_ticket" and b"NATIVE_READY" in data and "key" not in sent:
                send(b"a")
                sent.add("key")
            if b"\x1b[6n" in data and "query" not in sent:
                send(b"\x1b[12;" if case == "late_cursor" else b"\x1b[" + b"1" * 4096)
                sent.add("query")
            if b"NATIVE_TIMED_OUT" in data and "late" not in sent:
                send(b"3R")
                ack.touch()
                sent.add("late")
        child.wait(timeout=2)
        assert child.returncode == 0 and b"NATIVE_PASSED" in data, bytes(data)
        assert data.count(b"\x1b[6n") == (0 if case == "release_wait" else 1), bytes(data)
        print(f"{case}: passed")
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)
