"""Play a scripted terminal for tests/terminal_caps_pty.rs in a real Unix PTY.

The child is the `terminal_caps_pty_child` test. This script answers the
startup probe according to the case, then asserts on the bytes the child wrote.
"""

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
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

env = {
    key: value
    for key, value in os.environ.items()
    if not key.startswith("SLT_")
    and key
    not in (
        "TERM_PROGRAM",
        "TERM_PROGRAM_VERSION",
        "COLORTERM",
        "NO_COLOR",
        "TMUX",
        "STY",
        "ZELLIJ",
        "ZELLIJ_SESSION_NAME",
        "SSH_TTY",
        "SSH_CONNECTION",
        "SSH_CLIENT",
        "MOSH_IP",
        "WT_SESSION",
        "KITTY_WINDOW_ID",
        "VTE_VERSION",
        "KONSOLE_VERSION",
        "ConEmuANSI",
    )
}
env["SLT_CAPS_TEST_CASE"] = case
if case in ("truecolor", "no_truecolor"):
    # Identified host (queries allowed) with no truecolor identity.
    env.update(TERM="xterm-256color", TERM_PROGRAM="Apple_Terminal")
elif case.startswith("meta"):
    env.update(TERM="xterm-ghostty", TERM_PROGRAM="ghostty")
    if case == "meta_dumb":
        env["TERM"] = "dumb"
else:
    env.update(TERM="xterm-kitty")

DA_QUERY = b"\x1b[?u\x1b[c\x1b[>c"
DA_REPLY = b"\x1b[?62;22c\x1b[>1;4000;29c"
TC_QUERY = b"\x1bP+q5463\x1b\\"
CPR_QUERY = b"\x1b[6n"
# Advance (columns) the scripted terminal reports per width-probe sample:
# VS16, ZWJ family (drawn as three emoji), skin tone, flag.
WIDTH_ADVANCES = [2, 6, 2, 2]

child = subprocess.Popen(
    [binary, "--exact", "terminal_caps_pty_child", "--nocapture"],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    env=env,
    start_new_session=True,
)
os.close(slave)
data = bytearray()
answered_da = False
answered_tc = False
cprs = 0
try:
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if not select.select([master], [], [], 0.01)[0]:
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
        if not answered_da and DA_QUERY in data:
            reply = (b"\x1b[?1u" if case == "kitty_reply" else b"") + DA_REPLY
            os.write(master, reply)
            answered_da = True
        if not answered_tc and TC_QUERY in data:
            if case == "truecolor":
                os.write(master, b"\x1bP1+r5463\x1b\\")
            answered_tc = True
        while case == "width_probe" and cprs < data.count(CPR_QUERY):
            advance = WIDTH_ADVANCES[cprs] if cprs < len(WIDTH_ADVANCES) else 1
            os.write(master, f"\x1b[2;{advance + 1}R".encode())
            cprs += 1
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired as error:
        raise AssertionError((case, bytes(data))) from error
    out = bytes(data)
    assert child.returncode == 0 and b"SLT_CAPS_RESULT frames=5" in out, out

    if case.startswith("kitty"):
        assert DA_QUERY in out, out
        push, pop = b"\x1b[>3u", b"\x1b[<1u"
        if case == "kitty_reply":
            assert out.count(push) == 1 and out.count(pop) == 1, out
            assert out.find(push) < out.find(b"CAPS"), "push after first frame"
        else:
            assert push not in out and pop not in out, out
    elif case == "truecolor":
        assert b"\x1b[38;2;1;2;3m" in out, out
    elif case == "no_truecolor":
        assert b"\x1b[38;2;" not in out and b"\x1b[38;5;" in out, out
    elif case == "meta":
        assert out.count(b"\x1b]2;caps: idle\x07") == 1, out
        assert out.count(b"\x1b[22;2t") == 1 and out.count(b"\x1b[23;2t") == 1, out
        assert out.count(b"\x1b]9;4;1;50\x07") == 1, out
        assert out.count(b"\x1b[6 q") == 1, out
        exit_at = out.find(b"\x1b[?1049l")
        assert exit_at > 0, out
        teardown = out[out.rfind(b"\x1b]9;4;1;50\x07") :]
        for sequence in (b"\x1b]9;4;0\x07", b"\x1b[0 q", b"\x1b[23;2t"):
            assert sequence in teardown and teardown.find(sequence) < teardown.find(
                b"\x1b[?1049l"
            ), (sequence, out)
    elif case == "meta_dumb":
        for sequence in (b"\x1b]2;", b"\x1b]9;4", b"\x1b[22;2t", b"\x1b]7501", b"\x1b[6 q", b"\x1b[0 q"):
            assert sequence not in out, (sequence, out)
    elif case == "clipboard":
        assert out.count(b"\x1b]52;c;U0xULU9TQzUyLVBST0JF\x1b\\") == 1, out
        assert b"SLT_CAPS_CLIPBOARD Some(Sent)" in out, out
        assert b"[slt]" not in out, out
    elif case == "width_probe":
        assert cprs == 4, (cprs, out)
        assert b"\x1b[2;1H\xe2\x9d\xa4\xef\xb8\x8f\x1b[6n" in out, out
        erase = out.find(b"\x1b[2;1H\x1b[2K")
        assert erase > 0 and erase < out.find(b"CAPS"), out
    print(f"{case}: passed")
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
