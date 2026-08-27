#!/usr/bin/env python3
"""Replay every expression shown on the landing page through the yarer binary
and diff the real output against what the markup claims.

A page whose argument is "this evaluator returns exact answers" cannot afford
to print one that is wrong, and hand-transcribed output drifts silently at
every release. This turns that risk into a command.

    python3 tools/verify-transcript.py [path/to/yarer]

Exits non-zero on the first mismatch, so it can gate a deploy.
"""
from __future__ import annotations

import html
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
PAGE = HERE.parent / "index.html"
DEFAULT_BIN = HERE.parent.parent / "yarer" / "target" / "release" / "yarer"

TERMINAL_BLOCK = re.compile(
    r'<pre class="(?:term__body|code code--term[^"]*)"[^>]*>(.*?)</pre>', re.S)
TERMINAL_LINE = re.compile(r'<span class="(ln[^"]*)">(.*?)</span>', re.S)
DISPLAY_BLOCK = re.compile(r'<article class="cell[^"]*">(.*?)</article>', re.S)
CELL_TITLE = re.compile(r"<h3>(.*?)</h3>", re.S)
DISPLAY_ITEM = re.compile(r'<span class="(expr|ret[^"]*)">(.*?)</span>', re.S)
SHELL_BLOCK = re.compile(r'<pre class="code code--shell"[^>]*>(.*?)</pre>', re.S)
INLINE_PAIR = re.compile(
    r'<code class="ex">(.*?)</code>.*?<code class="val">(.*?)</code>', re.S)
SHELL_LINE = re.compile(r'<span class="(ln[^"]*)">(.*?)</span>', re.S)

BANNER = re.compile(r"^(Yarer v\.|License )")


def plain(fragment: str) -> str:
    """Markup fragment to the text a reader actually sees."""
    return html.unescape(re.sub(r"<[^>]+>", "", fragment))


def evaluate(binary: pathlib.Path, expressions: list[str]) -> list[str]:
    """Feed expressions to the REPL and return its output lines."""
    try:
        # Merged, not captured separately: yarer sends its banner and its
        # diagnostics to stderr and its answers to stdout, and an interactive
        # terminal shows both interleaved on one screen. That combined stream
        # is what the page depicts, so it is what has to be compared.
        result = subprocess.run(
            [str(binary)],
            input="\n".join(expressions) + "\n",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=30,
            check=False,
        )
    except FileNotFoundError:
        sys.exit(f"yarer binary not found at {binary}\n"
                 f"build it with: cd {binary.parents[2]} && cargo build --release")
    except subprocess.TimeoutExpired:
        sys.exit(f"yarer timed out on: {expressions}")

    lines = result.stdout.splitlines()

    # A non-zero exit is not a fault here: the page deliberately shows a parse
    # error, and that is exactly how the CLI reports one. Only a run that
    # printed nothing at all means the binary failed to do its job.
    if result.returncode != 0 and not [l for l in lines if not BANNER.match(l)]:
        sys.exit(f"yarer exited {result.returncode} without output")

    while lines and BANNER.match(lines[0]):
        lines.pop(0)
    if lines and lines[-1] == "quit":          # rustyline's parting line on EOF
        lines.pop()
    return lines


def check_terminals(binary: pathlib.Path, page: str) -> list[str]:
    """Terminal blocks print one output line per expression, in order."""
    failures = []
    blocks = TERMINAL_BLOCK.findall(page)
    if not blocks:
        failures.append("no terminal blocks found: has the markup changed?")

    for index, block in enumerate(blocks, start=1):
        spans = TERMINAL_LINE.findall(block)
        entered = [plain(text) for cls, text in spans if "ln--in" in cls]
        claimed = [plain(text) for cls, text in spans if "ln--in" not in cls]
        actual = evaluate(binary, entered)

        status = "match" if actual == claimed else "MISMATCH"
        print(f"  terminal {index}: {len(entered)} expressions, "
              f"{len(claimed)} output lines ... {status}")
        if actual != claimed:
            failures.append(
                f"terminal block {index}\n"
                f"    entered: {entered}\n"
                f"    page:    {claimed}\n"
                f"    binary:  {actual}")
    return failures


def check_displays(binary: pathlib.Path, page: str) -> list[str]:
    """Bento cells quote only the results they care about, so each printed
    result is checked against the expression immediately above it."""
    failures = []
    articles = DISPLAY_BLOCK.findall(page)
    if not articles:
        failures.append("no property cells found: has the markup changed?")

    for body in articles:
        heading = CELL_TITLE.search(body)
        title = heading.group(1) if heading else "<untitled cell>"
        # Scoped to the whole article on purpose: expr and ret spans live only
        # inside the display, so nothing else can be picked up, and the parse
        # survives any wrapper put between the display and its heading.
        items = DISPLAY_ITEM.findall(body)
        expressions = [plain(text) for kind, text in items if kind == "expr"]
        actual = evaluate(binary, expressions)

        cursor, previous, ok = 0, None, True
        for kind, raw in items:
            if kind == "expr":
                previous = actual[cursor] if cursor < len(actual) else "<no output>"
                cursor += 1
                continue
            if plain(raw) != previous:
                ok = False
                failures.append(
                    f"cell {plain(title)!r}\n"
                    f"    page:   {plain(raw)!r}\n"
                    f"    binary: {previous!r}")

        print(f"  cell {plain(title)!r}: {len(expressions)} expressions ... "
              f"{'match' if ok else 'MISMATCH'}")
    return failures


def run_shell(binary: pathlib.Path, command: str) -> list[str]:
    """Run one shell command with this binary's directory first on PATH.

    Prepending the directory rather than rewriting the command text is what
    lets a pipeline name `yarer` as many times as it likes — `yarer -e ...`
    and `... | yarer` both resolve to the build under test without the markup
    having to know where it lives.
    """
    env = {**os.environ, "PATH": f"{binary.parent}{os.pathsep}{os.environ['PATH']}"}
    try:
        result = subprocess.run(
            command,
            shell=True,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,   # the page depicts one interleaved stream
            text=True,
            timeout=30,
            check=False,
        )
    except subprocess.TimeoutExpired:
        sys.exit(f"timed out running: {command}")
    return result.stdout.splitlines()


def check_shells(binary: pathlib.Path, page: str) -> list[str]:
    """Shell blocks hold commands, not expressions.

    A `ln--cmd` line is run as written; the `ln--out` lines that follow it,
    up to the next command, are what it must print. A block with no commands
    is a markup error rather than a vacuous pass.
    """
    failures = []
    for index, block in enumerate(SHELL_BLOCK.findall(page), start=1):
        spans = SHELL_LINE.findall(block)
        pairs: list[tuple[str, list[str]]] = []
        for cls, text in spans:
            if "ln--cmd" in cls:
                pairs.append((plain(text), []))
            elif pairs:
                pairs[-1][1].append(plain(text))
            else:
                failures.append(f"shell block {index}: output before any command")

        if not pairs:
            failures.append(f"shell block {index}: no commands found")
            continue

        ok = True
        for command, claimed in pairs:
            actual = run_shell(binary, command)
            if actual != claimed:
                ok = False
                failures.append(
                    f"shell block {index}\n"
                    f"    command: {command}\n"
                    f"    page:    {claimed}\n"
                    f"    shell:   {actual}")
        print(f"  shell {index}: {len(pairs)} commands ... "
              f"{'match' if ok else 'MISMATCH'}")
    return failures


def check_inline(binary: pathlib.Path, page: str) -> list[str]:
    """Values quoted inside a sentence rather than inside a transcript.

    Prose makes claims too, and a number in a paragraph is as capable of going
    stale as one in a terminal. The pair is marked explicitly, `<code class=
    "ex">` for the expression and `<code class="val">` for what it answers,
    because guessing which inline code spans are expressions would pick up
    things like `5!=3` that are being discussed rather than evaluated.
    """
    failures = []
    for expr, claimed in INLINE_PAIR.findall(page):
        expr, claimed = plain(expr), plain(claimed)
        actual = evaluate(binary, [expr])
        got = actual[0] if actual else "<no output>"
        ok = got == claimed
        print(f"  inline {expr!r} ... {'match' if ok else 'MISMATCH'}")
        if not ok:
            failures.append(f"inline claim {expr!r}\n"
                            f"    page:   {claimed!r}\n"
                            f"    binary: {got!r}")
    return failures


def main() -> int:
    binary = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_BIN
    if not PAGE.is_file():
        sys.exit(f"page not found at {PAGE}")

    page = PAGE.read_text(encoding="utf-8")
    print(f"verifying {PAGE.name} against {binary}")

    failures = (check_terminals(binary, page)
                + check_displays(binary, page)
                + check_shells(binary, page)
                + check_inline(binary, page))

    print()
    if failures:
        print("FAIL: the page disagrees with the binary\n")
        print("\n".join(failures))
        return 1
    print("PASS: every value on the page came out of this binary")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
