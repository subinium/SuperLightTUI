#!/usr/bin/env python3
"""Assert what a real tmux displays for an SLT app (issue #450).

The multiplexer smoke job proves input reaches the app; this script proves the
*screen* is right. It drives `tests/fixtures/terminal_probe` inside an isolated
tmux server and checks, with `capture-pane`:

* render geometry of a bordered box, a CJK row, a VS16 row (U+2764 U+FE0F) and
  a ZWJ family row, after the first paint and after two incremental one-cell
  updates (`t` flips every variant cell between `x` and `y`). The expected
  screen comes from SLT's own `TestBackend` (`probe --expect`), and every
  grapheme's real advance in *this* tmux comes from a CPR probe
  (`probe --measure`), so a one-column VS16 terminal is not misreported;
* SGR styling survives (`capture-pane -e`);
* SGR mouse clicks reach the right cell: raw SGR bytes are written into an
  attached tmux *client* (status line on top, so tmux must translate client to
  pane coordinates) with `mouse off` and `mouse on`, at the pane corners, beside
  and on a toggle target, plus a direct `send-keys -H` injection; the probe
  echoes the zero-based coordinates it received and whether the target toggled;
* alternate-screen enter/exit and a clean exit status.

Known regressions can be marked strict expected-fail with `--known-issue`: the
check must fail (XFAIL) and an unexpected pass (XPASS) fails the run, so the
marker cannot outlive the fix.
"""

from __future__ import annotations

import argparse
import errno
import fcntl
import json
import os
import pathlib
import pty
import re
import select
import shlex
import signal
import struct
import subprocess
import sys
import termios
import threading
import time
import unicodedata
from typing import Callable, TypeVar

MAIN_SCREEN_MARKER = "SLT-MAIN-SCREEN-MARKER"
# Seconds a pane may report a closed pty without an exit status before the
# tmux server is sent the SIGCHLD that a libutempter build can lose.
REAP_NUDGE_AFTER = 0.5
CONTINUATION = object()
T = TypeVar("T")

# Row prefix -> issue whose regression this row demonstrates. A row listed in
# --known-issue must currently fail on terminals where the bug is observable.
KNOWN_ISSUE_ROWS = {
    # TODO(#441): the diff flush pads U+FE0F clusters with a literal space, so
    # on terminals that advance two columns (e.g. tmux 3.5a) every following cell
    # of the row shifts right and the next incremental update lands one column
    # off. Drop `--known-issue 441` from ci.yml once #441 is fixed.
    "441": "VS16 ",
    # TODO(#449): SLT measures a ZWJ family as one 2-column cluster; a
    # multiplexer without grapheme clustering (older tmux) may draw each member
    # separately, so the row is wider than the buffer. Only observable where the
    # CPR advance exceeds the buffer width; drop `--known-issue 449` with #449.
    "449": "ZWJ  ",
}
ZWJ_FAMILY = "\U0001f468‍\U0001f469‍\U0001f467"
VS16_HEART = "❤️"
VS16 = "\ufe0f"


def tmux_misplaces_vs16_padding(version: str) -> bool:
    """Mirror SLT's tmux < 3.5 detection (`tmux -V` prints `tmux 3.4`).

    tmux before 3.5 widens a VS16 cluster but marks its padding one column
    too far right (fixed by tmux 89c1c43, "Write padding character into
    the right position"), leaving a stray cell after the emoji. No byte
    sequence that contains U+FE0F avoids it, so SLT emits such clusters in
    text presentation (without U+FE0F) there and keeps the blank fill.
    """
    match = re.search(r"(\d+)(?:\.(\d+))?", version.removeprefix("tmux "))
    if match is None:
        return True
    return (int(match.group(1)), int(match.group(2) or 0)) < (3, 5)


class CheckFailure(Exception):
    """A rendering or interaction assertion failed."""


def log(message: str) -> None:
    print(message, flush=True)


def wait_for(
    probe: Callable[[], T | None], timeout: float, what: str, interval: float = 0.05
) -> T:
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while True:
        try:
            value = probe()
            if value is not None and value is not False:
                return value
        except subprocess.CalledProcessError as error:
            last_error = error
        if time.monotonic() >= deadline:
            detail = f" (last error: {last_error})" if last_error else ""
            raise CheckFailure(
                f"timed out after {timeout:.1f}s waiting for {what}{detail}"
            )
        time.sleep(interval)


class Tmux:
    """An isolated tmux server (own socket, own config, no user rc files)."""

    def __init__(self, binary: str, artifacts: pathlib.Path, label: str) -> None:
        self.binary = binary
        self.socket = f"slt-render-{label}-{os.getpid()}"
        self.conf = artifacts / f"{self.socket}.conf"
        self.conf.write_text(
            "\n".join(
                [
                    "set -g status on",
                    "set -g status-position top",
                    "set -g escape-time 10",
                    'set -g default-terminal "screen-256color"',
                    "set -g mouse off",
                    "setw -g remain-on-exit on",
                    "",
                ]
            )
        )
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("SLT_")}
        for key in ("TMUX", "TMUX_PANE", "STY", "ZELLIJ", "ZELLIJ_SESSION_NAME"):
            self.env.pop(key, None)
        self.env.update(TERM="xterm-256color", SLT_DISABLE_TERMINAL_QUERIES="1")

    def argv(self, *args: str) -> list[str]:
        return [self.binary, "-L", self.socket, "-f", str(self.conf), *args]

    def run(self, *args: str) -> str:
        result = subprocess.run(
            self.argv(*args),
            env=self.env,
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        )
        return result.stdout

    def display(self, target: str, fmt: str) -> str:
        return self.run("display-message", "-p", "-t", target, fmt).strip()

    def wait_exit_status(self, target: str, timeout: float, what: str) -> str:
        """Wait until the pane's process is reaped; return its exit status.

        `#{pane_dead}` turns 1 as soon as tmux reads EOF on the pty, but
        `#{pane_dead_status}` / `#{pane_dead_signal}` stay empty until tmux
        handles SIGCHLD (PANE_STATUSREADY). The two arrive in either order,
        so wait for the status itself. A signal death is reported as
        `signal N`.

        A tmux built with libutempter (Debian / Ubuntu packages) can lose
        that SIGCHLD for good: on pty EOF, `server_destroy_pane` calls
        `utempter_remove_record`, whose helper runs with SIGCHLD set to
        SIG_DFL, so a pane process that exits during that window stays an
        unreaped zombie and the status never appears. tmux already re-raises
        SIGCHLD after `utempter_add_record` (spawn.c) but not after the
        remove. When the pty has been closed for `REAP_NUDGE_AFTER` seconds
        with no status, send the server that SIGCHLD; its `waitpid(WAIT_ANY)`
        loop then reaps the pane and records the real exit status.
        """
        dead_since: float | None = None
        nudged_at: float | None = None

        def reaped() -> str | None:
            nonlocal dead_since, nudged_at
            fields = self.display(
                target, "#{pane_dead}|#{pane_dead_status}|#{pane_dead_signal}"
            ).split("|")
            if len(fields) != 3 or fields[0] != "1":
                return None
            if fields[1]:
                return fields[1]
            if fields[2]:
                return f"signal {fields[2]}"
            now = time.monotonic()
            if dead_since is None:
                dead_since = now
            if now - dead_since >= REAP_NUDGE_AFTER and (
                nudged_at is None or now - nudged_at >= REAP_NUDGE_AFTER
            ):
                if nudged_at is None:
                    log(
                        f"INFO  {target}: pty closed but tmux has no exit status "
                        f"after {REAP_NUDGE_AFTER:.1f}s (libutempter builds can "
                        "lose SIGCHLD); sending the server SIGCHLD"
                    )
                os.kill(int(self.display(target, "#{pid}")), signal.SIGCHLD)
                nudged_at = now
            return None

        return wait_for(reaped, timeout, what)

    def capture(
        self, target: str, escapes: bool = False, history: bool = False
    ) -> list[str]:
        args = ["capture-pane", "-p", "-t", target]
        if escapes:
            args.append("-e")
        if history:
            args += ["-S", "-"]
        return self.run(*args).split("\n")

    def kill(self) -> None:
        subprocess.run(
            self.argv("kill-server"),
            env=self.env,
            capture_output=True,
            timeout=5,
            check=False,
        )


def fallback_width(char: str) -> int:
    if unicodedata.category(char) in ("Mn", "Me", "Cf") or char == "️":
        return 0
    return 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1


def terminal_columns(line: str, widths: dict[str, int], cols: int) -> list[object]:
    """Lay a captured row out on a column grid using the terminal's advances."""
    grid: list[object] = [" "] * cols
    known = sorted(widths, key=len, reverse=True)
    col = 0
    index = 0
    while index < len(line) and col < cols:
        token = next((s for s in known if line.startswith(s, index)), line[index])
        width = (
            widths.get(token, fallback_width(token))
            if len(token) == 1
            else widths[token]
        )
        index += len(token)
        if width == 0:
            # A combining code point tmux did not merge: attach to the cell.
            if col > 0 and isinstance(grid[col - 1], str):
                grid[col - 1] = str(grid[col - 1]) + token
            continue
        grid[col] = token
        for extra in range(1, width):
            if col + extra < cols:
                grid[col + extra] = CONTINUATION
        col += width
    return grid


def normalized(symbol: object) -> object:
    return symbol.replace("️", "") if isinstance(symbol, str) else symbol


def describe(grid: list[object]) -> str:
    return "".join("~" if cell is CONTINUATION else str(cell) for cell in grid)


def row_mismatches(
    expected_row: list[list[object]],
    captured: str,
    widths: dict[str, int],
    cols: int,
    text_presentation: bool = False,
) -> list[str]:
    expected: list[object] = [" "] * cols
    advance: dict[int, int] = {}
    for x, symbol, buffer_width in expected_row:
        x = int(x)
        expected[x] = symbol
        for extra in range(1, int(buffer_width)):
            expected[x + extra] = CONTINUATION
        shown = str(symbol)
        if text_presentation and VS16 in shown:
            # SLT emitted the cluster without U+FE0F: its advance is the
            # bare base's, and the buffer's extra column is blank fill.
            shown = shown.replace(VS16, "") or " "
            advance[x] = widths.get(
                shown, fallback_width(shown) if len(shown) == 1 else 1
            )
        else:
            advance[x] = widths.get(shown, int(buffer_width))
    actual = terminal_columns(captured, widths, cols)
    problems = []
    owner = 0
    for col in range(cols):
        want, got = expected[col], actual[col]
        if want is not CONTINUATION:
            owner = col
        if normalized(want) == normalized(got):
            continue
        # A cluster the terminal draws narrower than the buffer leaves fill
        # cells; a blank there is correct geometry, not a defect.
        if want is CONTINUATION and got == " " and col >= owner + advance.get(owner, 1):
            continue
        problems.append(
            f"col {col}: expected {describe([want])!r}, terminal has {describe([got])!r}"
        )
    if problems:
        problems.insert(0, f"expected {describe(expected).rstrip()!r}")
        problems.insert(1, f"terminal {describe(actual).rstrip()!r}")
    return problems


class Checker:
    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.probe = str(pathlib.Path(args.probe).resolve())
        self.artifacts = pathlib.Path(args.artifacts)
        self.artifacts.mkdir(parents=True, exist_ok=True)
        self.tmux = Tmux(args.tmux, self.artifacts, "check")
        self.known_rows = {KNOWN_ISSUE_ROWS[issue]: issue for issue in args.known_issue}
        self.widths: dict[str, int] = {}
        self.text_presentation = False
        self.cols = 0
        self.rows = 0
        self.state = {"variant": "x", "toggled": 0, "hits": 0, "down": None}
        self.snapshots = 0
        self.xfailed: list[str] = []

    # -- helpers -----------------------------------------------------------

    def save(self, name: str, lines: list[str]) -> None:
        self.snapshots += 1
        path = self.artifacts / f"{self.snapshots:02d}-{name}.txt"
        path.write_text("\n".join(lines))

    def expected_grid(self) -> list[list[list[object]]]:
        down = self.state["down"]
        argv = [
            self.probe,
            "--expect",
            str(self.cols),
            str(self.rows),
            str(self.state["variant"]),
            str(self.state["toggled"]),
            str(self.state["hits"]),
        ]
        if down is not None:
            argv += [str(down[0]), str(down[1])]
        return json.loads(
            subprocess.run(
                argv, capture_output=True, text=True, check=True, timeout=10
            ).stdout
        )

    def label(self, expected_row: list[list[object]]) -> str:
        return "".join(str(cell[1]) for cell in expected_row)

    def compare_screen(
        self, name: str, timeout: float = 4.0, judge_known: bool = True
    ) -> None:
        """Wait until every row matches (known-issue rows aside) and is stable."""
        expected = self.expected_grid()
        previous: list[str] | None = None
        deadline = time.monotonic() + timeout
        while True:
            lines = self.tmux.capture("probe")
            lines += [""] * (self.rows - len(lines))
            failures: dict[int, list[str]] = {}
            for y, row in enumerate(expected):
                problems = row_mismatches(
                    row, lines[y], self.widths, self.cols, self.text_presentation
                )
                if problems:
                    failures[y] = problems
            blocking = {
                y: p
                for y, p in failures.items()
                if self.known_issue(expected[y]) is None
            }
            stable = previous == lines
            previous = lines
            if (not blocking and stable) or time.monotonic() >= deadline:
                break
            time.sleep(0.1)
        self.save(name, lines)
        if blocking:
            report = [f"{name}: rendered screen differs from TestBackend"]
            for y, problems in blocking.items():
                report.append(f"  row {y} ({self.label(expected[y]).rstrip()!r}):")
                report += [f"    {p}" for p in problems]
            raise CheckFailure("\n".join(report))
        for y, row in enumerate(expected):
            issue = self.known_issue(row)
            # Known-issue rows are judged on the render frames only; mouse
            # phases re-check the rest of the screen without repeating them.
            if issue is None or not judge_known:
                continue
            title = f"{name} row {y} ({self.label(row).strip()!r})"
            if not self.issue_observable(issue):
                if y in failures:
                    raise CheckFailure(
                        f"{title} failed although #{issue} is not observable here:\n    "
                        + "\n    ".join(failures[y])
                    )
                log(
                    f"PASS  {title} (#{issue} not observable with this terminal's CPR widths)"
                )
            elif y in failures:
                self.xfailed.append(title)
                log(f"XFAIL {title}: known regression #{issue}")
                for problem in failures[y]:
                    log(f"        {problem}")
            else:
                raise CheckFailure(
                    f"XPASS {title}: #{issue} no longer reproduces. The fix landed; "
                    f"remove `--known-issue {issue}` from .github/workflows/ci.yml."
                )
        log(f"PASS  {name}: geometry matches TestBackend ({self.rows} rows)")

    def known_issue(self, expected_row: list[list[object]]) -> str | None:
        text = self.label(expected_row)
        for prefix, issue in self.known_rows.items():
            if text.startswith(prefix):
                return issue
        return None

    def issue_observable(self, issue: str) -> bool:
        if issue == "441":
            # Only terminals that advance two columns for U+2764 U+FE0F shift.
            return self.widths.get(VS16_HEART, 2) >= 2
        if issue == "449":
            # Buffer width is 2; a wider terminal advance cannot line up.
            return self.widths.get(ZWJ_FAMILY, 2) > 2
        return True

    # -- phases ------------------------------------------------------------

    def measure(self) -> None:
        out = self.artifacts / "widths.json"
        out.unlink(missing_ok=True)
        command = f"{shlex.quote(self.probe)} --measure {shlex.quote(str(out))}"
        self.tmux.run(
            "new-session", "-d", "-s", "measure", "-x", "80", "-y", "24", command
        )
        status = self.tmux.wait_exit_status(
            "measure", 15, "the CPR width probe to exit"
        )
        if status != "0" or not out.exists():
            lines = self.tmux.capture("measure")
            self.save("measure-failed", lines)
            raise CheckFailure(
                f"CPR width probe exited with status {status!r}: "
                + " | ".join(line for line in lines if line.strip())
            )
        self.widths = json.loads(out.read_text())
        self.tmux.run("kill-session", "-t", "measure")
        pretty = ", ".join(
            f"U+{'+'.join(f'{ord(c):04X}' for c in s)}={w}"
            for s, w in self.widths.items()
        )
        log(f"INFO  tmux cursor advance per grapheme (CPR): {pretty}")

    def start_probe(self) -> None:
        self.tmux.run(
            "new-session",
            "-d",
            "-s",
            "probe",
            "-x",
            "80",
            "-y",
            "24",
            shlex.quote(self.probe),
        )
        size = self.tmux.display("probe", "#{pane_width} #{pane_height}").split()
        self.cols, self.rows = int(size[0]), int(size[1])
        wait_for(
            lambda: any(
                line.startswith("keys:") for line in self.tmux.capture("probe")
            ),
            15,
            "the probe's first frame",
        )
        if self.tmux.display("probe", "#{alternate_on}") != "1":
            raise CheckFailure("probe is running but tmux reports alternate_on=0")
        if any(MAIN_SCREEN_MARKER in line for line in self.tmux.capture("probe")):
            raise CheckFailure("main-screen marker is visible while the probe runs")
        log(f"PASS  alternate screen entered (pane {self.cols}x{self.rows})")

    def render_frames(self) -> None:
        self.compare_screen("frame-1-first-paint")
        for frame, variant in ((2, "y"), (3, "x")):
            self.tmux.run("send-keys", "-t", "probe", "t")
            self.state["variant"] = variant
            wait_for(
                lambda: f"variant={variant}" in self.tmux.capture("probe")[0],
                5,
                f"variant={variant} frame",
            )
            self.compare_screen(f"frame-{frame}-incremental-{variant}")

    def check_styles(self) -> None:
        lines = self.tmux.capture("probe", escapes=True)
        self.save("styled-escapes", lines)
        row = next((line for line in lines if "STYLED" in line), None)
        if row is None:
            raise CheckFailure("capture-pane -e has no STYLED row")
        before = row[: row.index("STYLED")]
        reset = max(before.rfind("\x1b[0m"), before.rfind("\x1b[m"))
        params: list[str] = []
        for match in re.finditer(
            r"\x1b\[([0-9;:]*)m", before[reset + 1 :] if reset >= 0 else before
        ):
            params += match.group(1).replace(":", ";").split(";")
        joined = ";".join(params)
        if "1" not in params:
            raise CheckFailure(
                f"STYLED is not bold in capture-pane -e (SGR {joined!r})"
            )
        if not (
            "31" in params or "91" in params or re.search(r"38;5;(1|9)(;|$)", joined)
        ):
            raise CheckFailure(f"STYLED is not red in capture-pane -e (SGR {joined!r})")
        log(f"PASS  capture-pane -e keeps bold red on STYLED (SGR {joined!r})")

    def target_cell(self) -> tuple[int, int]:
        for y, line in enumerate(self.tmux.capture("probe")):
            index = line.find("] TARGET")
            if index > 0:
                return index - 2, y
        raise CheckFailure("click target not found on screen")

    def expect_click(
        self, name: str, x: int, y: int, inject: Callable[[int, int], None]
    ) -> None:
        tx, ty = self.target_cell()
        inside = ty == y and tx <= x < tx + len("[ ] TARGET")
        inject(x, y)
        self.state["down"] = (x, y)
        if inside:
            self.state["toggled"] ^= 1
            self.state["hits"] += 1
        echo = f"MOUSE down=({x},{y}) hits={self.state['hits']}"
        try:
            wait_for(
                lambda: any(
                    line.startswith(echo) for line in self.tmux.capture("probe")
                ),
                5,
                echo,
            )
        except CheckFailure as error:
            lines = self.tmux.capture("probe")
            self.save(f"{name}-missing-echo", lines)
            seen = next(
                (line for line in lines if line.startswith("MOUSE")), "<no MOUSE row>"
            )
            raise CheckFailure(f"{name}: {error}; probe shows {seen!r}") from None
        self.compare_screen(name, judge_known=False)
        verb = "toggled the target" if inside else "missed the target as expected"
        log(f"PASS  {name}: click at pane cell ({x},{y}) {verb}")

    def mouse(self) -> None:
        client = Client(self.tmux, self.cols, self.rows + 1)
        try:
            client.start()
            offset_y = (
                1
                if self.tmux.run("show-options", "-gv", "status-position").strip()
                == "top"
                else 0
            )
            pane_top = int(self.tmux.display("probe", "#{pane_top}")) + offset_y
            pane_left = int(self.tmux.display("probe", "#{pane_left}"))

            def via_client(x: int, y: int) -> None:
                col, row = pane_left + x + 1, pane_top + y + 1
                client.write(f"\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m".encode())

            tx, ty = self.target_cell()
            corners = ((0, 0), (self.cols - 1, self.rows - 1))
            for mode in ("off", "on"):
                self.tmux.run("set-option", "-g", "mouse", mode)
                for corner in corners:
                    self.expect_click(
                        f"client-mouse-{mode}-corner-{corner[0]}x{corner[1]}",
                        *corner,
                        via_client,
                    )
                self.expect_click(
                    f"client-mouse-{mode}-beside-target", tx - 1, ty, via_client
                )
                self.expect_click(
                    f"client-mouse-{mode}-on-target", tx + 1, ty, via_client
                )
            client.detach()
        finally:
            client.close()

        def via_send_keys(x: int, y: int) -> None:
            raw = f"\x1b[<0;{x + 1};{y + 1}M\x1b[<0;{x + 1};{y + 1}m"
            self.tmux.run(
                "send-keys", "-t", "probe", "-H", *(f"{b:02x}" for b in raw.encode())
            )

        self.tmux.run("set-option", "-g", "mouse", "off")
        self.expect_click("send-keys-H-on-target", tx + 5, ty, via_send_keys)

    def exit_cleanly(self) -> None:
        self.tmux.run("send-keys", "-t", "probe", "q")
        status = self.tmux.wait_exit_status("probe", 10, "probe exit")
        # Include history: tmux prints "Pane is dead" on the last row, which
        # scrolls the restored main screen up by one line.
        lines = self.tmux.capture("probe", history=True)
        self.save("after-exit", lines)
        if status != "0":
            raise CheckFailure(f"probe exited with status {status!r}")
        if self.tmux.display("probe", "#{alternate_on}") != "0":
            raise CheckFailure("alternate screen still active after exit")
        if not any(MAIN_SCREEN_MARKER in line for line in lines):
            raise CheckFailure(
                "main screen was not restored (marker missing after exit)"
            )
        if any("SLT-PROBE" in line for line in lines):
            raise CheckFailure("probe rows leaked onto the main screen after exit")
        log("PASS  alternate screen left, main screen restored, exit status 0")

    def run(self) -> None:
        version = subprocess.run(
            [self.args.tmux, "-V"], capture_output=True, text=True, check=True
        ).stdout.strip()
        log(f"INFO  {version}")
        self.text_presentation = tmux_misplaces_vs16_padding(version)
        if self.text_presentation:
            log(
                "INFO  tmux < 3.5 misplaces VS16 padding (tmux 89c1c43): SLT emits "
                "VS16 clusters in text presentation; their rows expect blank fill"
            )
        self.tmux.kill()
        try:
            self.measure()
            self.start_probe()
            self.render_frames()
            self.check_styles()
            self.mouse()
            self.exit_cleanly()
        finally:
            self.tmux.kill()
        if self.xfailed:
            log(
                f"DONE  all checks passed; {len(self.xfailed)} known-issue row check(s) XFAIL"
            )
        else:
            log("DONE  all checks passed")


class Client:
    """A real tmux client attached through a PTY we control, like a terminal."""

    def __init__(self, tmux: Tmux, cols: int, rows: int) -> None:
        self.tmux = tmux
        self.cols = cols
        self.rows = rows
        self.pid = 0
        self.fd = -1
        self.output = bytearray()
        self.stop = threading.Event()
        self.reader: threading.Thread | None = None

    def start(self) -> None:
        argv = self.tmux.argv("attach-session", "-t", "probe")
        pid, fd = pty.fork()
        if pid == 0:
            os.execvpe(argv[0], argv, self.tmux.env)
        self.pid, self.fd = pid, fd
        fcntl.ioctl(
            fd, termios.TIOCSWINSZ, struct.pack("HHHH", self.rows, self.cols, 0, 0)
        )
        self.reader = threading.Thread(target=self.drain, daemon=True)
        self.reader.start()
        wait_for(
            lambda: self.tmux.display("probe", "#{session_attached}") not in ("", "0"),
            10,
            "tmux client to attach",
        )
        # Let tmux finish its terminal feature probing before injecting input.
        time.sleep(0.5)

    def drain(self) -> None:
        while not self.stop.is_set():
            try:
                ready, _, _ = select.select([self.fd], [], [], 0.05)
                if ready:
                    chunk = os.read(self.fd, 65536)
                    if not chunk:
                        return
                    self.output.extend(chunk)
            except OSError as error:
                if error.errno in (errno.EIO, errno.EBADF):
                    return
                raise

    def write(self, data: bytes) -> None:
        os.write(self.fd, data)

    def detach(self) -> None:
        self.tmux.run("detach-client", "-s", "probe")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            waited, _ = os.waitpid(self.pid, os.WNOHANG)
            if waited == self.pid:
                self.pid = 0
                return
            time.sleep(0.05)
        raise CheckFailure("tmux client did not exit after detach-client")

    def close(self) -> None:
        self.stop.set()
        if self.pid:
            try:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        if self.reader is not None:
            self.reader.join(timeout=1)
        if self.fd >= 0:
            try:
                os.close(self.fd)
            except OSError:
                pass


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--probe", required=True, help="path to the slt-terminal-probe binary"
    )
    parser.add_argument("--tmux", default="tmux", help="tmux binary to test")
    parser.add_argument(
        "--artifacts",
        default="target/tmux-render-check",
        help="capture output directory",
    )
    parser.add_argument(
        "--known-issue",
        action="append",
        default=[],
        choices=sorted(KNOWN_ISSUE_ROWS),
        help="strict expected-fail for the row reproducing this issue (repeatable)",
    )
    args = parser.parse_args()
    try:
        Checker(args).run()
    except CheckFailure as error:
        log(f"FAIL  {error}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
