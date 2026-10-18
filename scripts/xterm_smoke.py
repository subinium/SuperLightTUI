#!/usr/bin/env python3
"""Smoke-test the SLT probe inside a real xterm on a headless X server (#450).

Run under `xvfb-run`; needs `xterm` and `xdotool`. Pixels are never compared.
Instead the probe reports what it received:

* `probe --measure` prints each grapheme of the view in raw mode and reads the
  cursor advance back with CPR, so the JSON width report is xterm's own
  answer. CJK must advance two columns and box drawing one; the emoji widths
  are logged for #449.
* the interactive probe mirrors its state (`SLT_PROBE_REPORT`) after every
  frame. `xdotool` types keys and clicks cells through XTEST, exactly like a
  user, and the report must show the key effect, the zero-based cell of every
  click, and the target toggle. The cell size is derived from the window
  geometry, so the check does not depend on the font xterm picked.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shlex
import subprocess
import sys
import time
from typing import Callable, TypeVar

COLS = 80
ROWS = 24
CLASS_NAME = "sltprobe"
T = TypeVar("T")


class CheckFailure(Exception):
    """An xterm smoke assertion failed."""


def log(message: str) -> None:
    print(message, flush=True)


def wait_for(probe: Callable[[], T | None], timeout: float, what: str) -> T:
    deadline = time.monotonic() + timeout
    while True:
        value = probe()
        if value is not None and value is not False:
            return value
        if time.monotonic() >= deadline:
            raise CheckFailure(f"timed out after {timeout:.0f}s waiting for {what}")
        time.sleep(0.1)


def read_json(path: pathlib.Path) -> dict | None:
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def xdotool(*args: str) -> str:
    return subprocess.run(
        ["xdotool", *args], capture_output=True, text=True, check=True, timeout=20
    ).stdout


class XtermSmoke:
    def __init__(self, args: argparse.Namespace) -> None:
        self.probe = str(pathlib.Path(args.probe).resolve())
        self.xterm = args.xterm
        self.artifacts = pathlib.Path(args.artifacts)
        self.artifacts.mkdir(parents=True, exist_ok=True)
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("SLT_")}
        for key in ("TMUX", "STY", "ZELLIJ", "TERM_PROGRAM"):
            self.env.pop(key, None)
        self.env.update(LANG="C.UTF-8", LC_ALL="C.UTF-8")
        self.env["SLT_DISABLE_TERMINAL_QUERIES"] = "1"

    def launch(self, command: str, env: dict[str, str]) -> subprocess.Popen:
        argv = [
            self.xterm,
            "-u8",
            "-name",
            CLASS_NAME,
            "-geometry",
            f"{COLS}x{ROWS}+0+0",
            "-b",
            "0",
            "-e",
            "sh",
            "-c",
            command,
        ]
        log_file = open(self.artifacts / "xterm.log", "ab")
        return subprocess.Popen(argv, env=env, stdout=log_file, stderr=log_file)

    def measure(self) -> None:
        out = self.artifacts / "widths.json"
        out.unlink(missing_ok=True)
        status = self.artifacts / "measure.status"
        status.unlink(missing_ok=True)
        command = (
            f"{shlex.quote(self.probe)} --measure {shlex.quote(str(out))}; "
            f"echo $? > {shlex.quote(str(status))}"
        )
        process = self.launch(command, self.env)
        try:
            wait_for(
                lambda: status.exists() and status.read_text().strip(),
                60,
                "the CPR width probe",
            )
            process.wait(timeout=20)
        finally:
            if process.poll() is None:
                process.kill()
        if status.read_text().strip() != "0":
            raise CheckFailure(
                f"width probe exited with status {status.read_text().strip()}"
            )
        widths = json.loads(out.read_text())
        pretty = ", ".join(
            f"U+{'+'.join(f'{ord(c):04X}' for c in s)}={w}" for s, w in widths.items()
        )
        log(f"INFO  xterm cursor advance per grapheme (CPR): {pretty}")
        for symbol, expected in (("─", 1), ("┌", 1), ("한", 2), ("漢", 2)):
            if widths.get(symbol) != expected:
                raise CheckFailure(
                    f"xterm advanced U+{ord(symbol):04X} by {widths.get(symbol)}, expected {expected}"
                )
        log("PASS  CPR width report: box drawing = 1 column, CJK = 2 columns")

    def target_cell(self) -> tuple[int, int]:
        grid = json.loads(
            subprocess.run(
                [self.probe, "--expect", str(COLS), str(ROWS), "x", "0", "0"],
                capture_output=True,
                text=True,
                check=True,
                timeout=10,
            ).stdout
        )
        for y, row in enumerate(grid):
            text = "".join(cell[1] for cell in row)
            if "TARGET" in text:
                bracket = next(cell[0] for cell in row if cell[1] == "[")
                return bracket + 1, y
        raise CheckFailure("TestBackend view has no TARGET row")

    def interactive(self) -> None:
        report = self.artifacts / "state.json"
        report.unlink(missing_ok=True)
        status = self.artifacts / "probe.status"
        status.unlink(missing_ok=True)
        env = dict(self.env, SLT_PROBE_REPORT=str(report))
        command = f"{shlex.quote(self.probe)}; echo $? > {shlex.quote(str(status))}"
        process = self.launch(command, env)
        try:
            self.drive(report, status)
            process.wait(timeout=20)
        finally:
            if process.poll() is None:
                process.kill()

    def drive(self, report: pathlib.Path, status: pathlib.Path) -> None:
        state = wait_for(lambda: read_json(report), 60, "the probe's first frame")
        if (state["cols"], state["rows"]) != (COLS, ROWS):
            raise CheckFailure(
                f"probe sees {state['cols']}x{state['rows']}, expected {COLS}x{ROWS}"
            )
        log(f"PASS  probe running in xterm at {COLS}x{ROWS}")

        window = xdotool("search", "--sync", "--classname", CLASS_NAME).split()[0]
        geometry = dict(
            line.split("=", 1)
            for line in xdotool("getwindowgeometry", "--shell", window).split()
        )
        left, top = int(geometry["X"]), int(geometry["Y"])
        cell_w, cell_h = int(geometry["WIDTH"]) / COLS, int(geometry["HEIGHT"]) / ROWS
        log(
            f"INFO  xterm window {geometry['WIDTH']}x{geometry['HEIGHT']} at {left},{top}; cell {cell_w:g}x{cell_h:g}px"
        )

        def pixel(x: int, y: int) -> tuple[str, str]:
            return str(int(left + (x + 0.5) * cell_w)), str(
                int(top + (y + 0.5) * cell_h)
            )

        # Without a window manager X focus follows the pointer: park it on
        # the terminal so XTEST key events reach xterm.
        xdotool("mousemove", "--sync", *pixel(COLS // 2, ROWS - 2))
        xdotool("key", "t")
        wait_for(
            lambda: (read_json(report) or {}).get("variant") == "y",
            15,
            "variant=y after key t",
        )
        log("PASS  key t reached the app")

        tx, ty = self.target_cell()
        hits = 0
        toggled = 0
        for x, y in ((0, 0), (tx - 2, ty), (tx, ty), (COLS - 1, ROWS - 1)):
            inside = y == ty and tx - 1 <= x < tx - 1 + len("[ ] TARGET")
            if inside:
                hits += 1
                toggled ^= 1
            xdotool("mousemove", "--sync", *pixel(x, y), "click", "1")

            def matches() -> bool:
                now = read_json(report) or {}
                return (
                    now.get("down") == [x, y]
                    and now.get("hits") == hits
                    and now.get("toggled") == toggled
                )

            try:
                wait_for(matches, 15, f"click at cell ({x},{y})")
            except CheckFailure as error:
                raise CheckFailure(
                    f"{error}; probe reports {read_json(report)}"
                ) from None
            verb = "toggled the target" if inside else "missed the target as expected"
            log(f"PASS  xterm click at cell ({x},{y}) reported as ({x},{y}) and {verb}")

        xdotool("key", "q")
        code = wait_for(
            lambda: status.exists() and status.read_text().strip(), 20, "probe exit"
        )
        if code != "0":
            raise CheckFailure(f"probe exited with status {code}")
        log("PASS  probe exited with status 0 after q")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--probe", required=True, help="path to the slt-terminal-probe binary"
    )
    parser.add_argument("--xterm", default="xterm", help="xterm binary")
    parser.add_argument(
        "--artifacts", default="target/xterm-smoke", help="report output directory"
    )
    args = parser.parse_args()
    if not os.environ.get("DISPLAY"):
        log("FAIL  DISPLAY is not set; run under xvfb-run")
        return 1
    smoke = XtermSmoke(args)
    try:
        smoke.measure()
        smoke.interactive()
    except (CheckFailure, subprocess.SubprocessError) as error:
        log(f"FAIL  {error}")
        return 1
    log("DONE  xterm smoke passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
