"""Drive one accessible-mode child of tests/pty_accessible.rs in a PTY."""

import errno
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time

binary, case = sys.argv[1:]
SECRET = b"s3cr3t-pw"
STEPS = {
    # (text the child must print first, answer typed by the "user" or None)
    "flow": [
        (b"Username: ", b"ada"),
        (b"Password (hidden): ", SECRET),
        (b"Enter a number between 1 and 3 [1]: ", b"9"),
        (b"Invalid: must be a number between 1 and 3", None),
        (b"Enter a number between 1 and 3 [1]: ", b"2"),
        (b"Create account? [y/N]: ", b"y"),
    ],
    "env_default": [(b"Name: ", b"grace")],
    "explicit_off": [],
}[case]

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
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
env.update(
    TERM="xterm-256color",
    SLT_DISABLE_TERMINAL_QUERIES="1",
    SLT_ACCESSIBLE_TEST_CASE=case,
)
if case in ("env_default", "explicit_off"):
    env["SLT_ACCESSIBLE"] = "1"
command = [
    binary,
    "--exact",
    "accessible_pty_child",
    "--nocapture",
    "--test-threads",
    "1",
]
child = subprocess.Popen(
    command, stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True
)
os.close(slave)
raw = bytearray()
step = 0
searched = 0
started = time.monotonic()
try:
    while time.monotonic() - started < 15:
        if step < len(STEPS):
            expected, answer = STEPS[step]
            found = raw.find(expected, searched)
            if found >= 0:
                searched = found + len(expected)
                if answer is not None:
                    # Enter on a terminal sends CR; cooked mode maps it to LF.
                    os.write(master, answer + b"\r")
                step += 1
                continue
        if not select.select([master], [], [], 0.02)[0]:
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
    child.wait(timeout=5)
    data = bytes(raw)
    assert child.returncode == 0, data
    assert step == len(STEPS), (step, data)
    begin = data.index(b"SLT_BEGIN")
    end = data.index(b"SLT_RESULT")
    session = data[begin:end]
    if case == "explicit_off":
        assert b"\x1b[?1049h" in session and b"\x1b[?1049l" in session, data
        assert b"visual=true" in data, data
    else:
        # Line-oriented: not a single escape sequence (no alternate screen,
        # no cursor movement or erase, no SGR colors, no OSC).
        assert b"\x1b" not in session, session
        assert re.search(rb"\x1b\[[0-9;?]*[A-HJKSTfhlmsu]", session) is None, session
        assert SECRET not in data, data
        text = session.replace(b"\r\n", b"\n").decode()
        if case == "flow":
            expected = [
                "Create an account",
                "Focused: Username, text field",
                "Username: ada",
                "Focused: Password, password field",
                "Password (hidden): ",
                "Focused: Role, select, 3 options",
                "1. Viewer (selected)",
                "2. Editor",
                "3. Admin",
                "Enter a number between 1 and 3 [1]: 9",
                "Invalid: must be a number between 1 and 3",
                "Enter a number between 1 and 3 [1]: 2",
                "Selected: Editor",
                "Focused: Create account?, yes or no question",
                "Create account? [y/N]: y",
            ]
            lines = [line for line in text.split("\n")[1:] if line]
            assert lines == expected, (lines, expected)
            assert (
                b"SLT_RESULT user=ada password_len=9 role=Editor create=true answered=true"
                in data
            ), data
        else:
            assert "Focused: Name, text field\nName: grace\n" in text, text
            assert b"SLT_RESULT name=grace" in data, data
    print(f"{case}: passed")
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
