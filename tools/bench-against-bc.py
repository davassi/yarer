#!/usr/bin/env python3
"""Time yarer against GNU bc on large integers and check the page's claim.

The page quotes a range. A number like that is worth no more than the
transcripts are: it has to come from a machine, not from a memory of one. This
measures it and fails if the claim no longer holds, so it can gate a deploy the
way verify-transcript.py does.

    python3 tools/bench-against-bc.py [path/to/yarer]

Two rules decide what is measured.

First, every case is checked for byte-identical output before it is timed. A
speed comparison between two programs computing different things is not a
measurement, and this is not hypothetical: `(2^200000)/(3^1000)` was in an
earlier version of this list, where yarer performs an exact rational division
and bc a truncating integer one. Their answers differ by 956 digits and the
case flattered yarer by a wide margin.

Second, the range has to span sizes. The advantage narrows as the operands get
smaller, and a list that only holds the large end reports a ratio the reader
will not reproduce on anything they type by hand.
"""
from __future__ import annotations

import hashlib
import pathlib
import re
import statistics
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
PAGE = HERE.parent / "index.html"
DEFAULT_BIN = HERE.parent.parent / "yarer" / "target" / "release" / "yarer"

CLAIM = re.compile(r"<dd>([0-9]+)x to ([0-9]+)x</dd>")

CASES = [
    "2^50000",
    "3^100000",
    "(2^50000)*(2^50000)",
    "2^100000",
    "2^200000",
    "2^400000",
    "(2^100000)*(3^60000)",
    "2^100000+3^60000",
]

RUNS = 9
BASELINE_RUNS = 15

# A case whose net time is under this is not measured, it is guessed at. The
# process baseline is over a millisecond, so subtracting it from a total only
# slightly larger leaves a difference made mostly of scheduling noise: the same
# small case has come out at 17x and at 39x on consecutive runs of this script.
NOISE_FLOOR_S = 0.002


def digest(text: str) -> str:
    """A result, with bc's line continuations and every newline removed.

    bc folds long output at 69 columns with a trailing backslash and yarer
    prints one unbroken line. Stripping both is what lets the two be compared
    as numbers rather than as formatting.
    """
    return hashlib.md5(text.replace("\\\n", "").replace("\n", "").encode()).hexdigest()


def yarer_out(binary: pathlib.Path, expr: str) -> str:
    return subprocess.run([str(binary), "-e", expr], stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True, check=False).stdout


def bc_out(expr: str) -> str:
    return subprocess.run(["bc"], input=expr + "\n", stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True, check=False).stdout


def best(argv: list[str], stdin: str | None, runs: int) -> float:
    """Fastest wall-clock time of `runs` attempts, in seconds.

    The minimum rather than the mean: a slow run means the machine was busy
    with something else, which says nothing about the program.
    """
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        subprocess.run(argv, input=stdin, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, text=True, check=False)
        times.append(time.perf_counter() - start)
    return min(times)


def main() -> int:
    binary = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_BIN
    if not binary.is_file():
        sys.exit(f"yarer binary not found at {binary}\n"
                 f"build it with: cd {binary.parents[2]} && cargo build --release")
    try:
        subprocess.run(["bc", "--version"], stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, check=False)
    except FileNotFoundError:
        sys.exit("GNU bc is not installed, so there is nothing to compare against")

    claimed = CLAIM.search(PAGE.read_text(encoding="utf-8"))
    if not claimed:
        sys.exit("no 'Nx to Nx' claim found in index.html: has the markup changed?")
    low, high = int(claimed.group(1)), int(claimed.group(2))

    print(f"yarer at {binary}")
    print(f"the page claims {low}x to {high}x on large integers\n")

    # Agreement first. A case that fails here is dropped rather than timed.
    comparable = []
    for case in CASES:
        if digest(yarer_out(binary, case)) == digest(bc_out(case)):
            comparable.append(case)
        else:
            print(f"  {case:<24} DISAGREE, not timed")
    if not comparable:
        sys.exit("\nno case produced the same answer from both: nothing to compare")

    y0 = best([str(binary), "-e", "1"], None, BASELINE_RUNS)
    b0 = best(["bc"], "1\n", BASELINE_RUNS)
    print(f"  process baseline: yarer {y0 * 1000:.2f}ms, bc {b0 * 1000:.2f}ms\n")

    ratios = []
    for case in comparable:
        y = best([str(binary), "-e", case], None, RUNS) - y0
        b = best(["bc"], case + "\n", RUNS) - b0
        if y < NOISE_FLOOR_S:
            print(f"  {case:<24} yarer {y * 1000:8.2f}ms   "
                  f"under the {NOISE_FLOOR_S * 1000:g}ms noise floor, not counted")
            continue
        ratios.append(b / y)
        print(f"  {case:<24} yarer {y * 1000:8.2f}ms   "
              f"bc {b * 1000:9.2f}ms   {b / y:6.1f}x")

    if not ratios:
        sys.exit("\nevery case was too fast to measure: raise the sizes in CASES")

    worst, bestr = min(ratios), max(ratios)
    print(f"\n  slowest {worst:.1f}x   median {statistics.median(ratios):.1f}x   "
          f"fastest {bestr:.1f}x")

    if worst < low:
        print(f"\nFAIL: the page claims from {low}x and the slowest case is {worst:.1f}x")
        return 1
    if bestr > high:
        print(f"\nNOTE: the page claims up to {high}x and a case reached {bestr:.1f}x. "
              f"Not a failure, but the page is understating itself.")
    print(f"\nPASS: the slowest case holds the {low}x floor the page claims")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
