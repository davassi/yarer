# yarer landing page

A single static page for [yarer](https://github.com/davassi/yarer). No build
step, no package manager, no network requests at runtime: every font is a system
font and every asset is inline.

```
index.html                    structure and all copy
style.css                     design tokens and layout
repl.js                       terminal playback and clipboard, both
                              progressive enhancements
og.png                        1200x630 card for link previews
tools/og-card.html            the source that card is rendered from
tools/verify-transcript.py    checks every printed value against the binary
tools/bench-against-bc.py     checks the speed claim against GNU bc
```

The four files at the top are the site. Nothing under `tools/` ships.

## Running it

Open `index.html` in a browser. That is the whole workflow.

The clipboard button prefers the async Clipboard API, which browsers restrict to
secure contexts, so over `file://` it falls back to `document.execCommand`. If
you want to exercise the primary path locally:

```console
python3 -m http.server -d . 8000   # then visit http://localhost:8000
```

## The transcripts are real output

Every value shown in a terminal block was produced by the yarer release binary,
not transcribed from the crate's README. That matters more here than on most
landing pages: a page arguing that an evaluator returns exact answers cannot
afford to print one that is wrong, and hand-copied output drifts silently at
every release.

`tools/verify-transcript.py` turns that risk into a command. It pulls every
expression out of `index.html`, replays it through the binary, and diffs the
real output against what the markup claims.

```console
cd ../yarer && cargo build --release && cd -
python3 tools/verify-transcript.py
```

It exits non-zero on the first mismatch, so it can gate a deploy. Point it at
another build by passing a path: `python3 tools/verify-transcript.py /usr/bin/yarer`.

This is not a hypothetical. The crate's README used to print the Black-Scholes
result as `10.450583572185565` where the binary answers `10.45058357218556`,
and this script is what caught it. Four such transcripts were wrong; 0.4.0
corrected them and put them under a test of the crate's own, so the two now
agree — which is the outcome the script exists to produce, not a reason to
stop running it.

## The speed claim is measured

The hero says yarer is about 40x faster than GNU bc on large integers.
`tools/bench-against-bc.py` is where that number comes from and what keeps it
honest:

```console
python3 tools/bench-against-bc.py
```

It times both programs on four shapes of large-integer work, powers, a
multiplication and a division, subtracts what each process costs before doing
any arithmetic, and exits non-zero if the weakest case falls under what the
page claims. The claimed multiple is read out of `index.html`, so editing the
page is what changes what the script checks.

Measured on the author's machine at 0.4.1: 46x, 46x, 46x and 71x, against a
claim of 40x. The margin is deliberate. A ratio measured on one machine should
not be quoted to the last unit on another.

The Rust snippet in the Library pane is not covered by the script. To check it
after an API change, drop it into `../yarer/examples/` and run
`cargo run --example <name>`.

Captured against **yarer 0.4.1**. Three strings in `index.html` are not
verified by anything: the terminal chrome (`yarer 0.4.1`), the footer
(`Requires Rust 1.88`), and the download count in the footer, read from
crates.io on 2026-08-27. The first two need updating on a release that moves
either. The third only goes up, so it is safe to leave stale and worth
refreshing when it crosses a round number:

```console
curl -s https://crates.io/api/v1/crates/yarer | python3 -c \
  'import json,sys; print(json.load(sys.stdin)["crate"]["downloads"])'
```

## Adding or changing a transcript

The script reads the markup, so the markup is the only place a transcript
lives. Terminal blocks alternate `<span class="ln ln--in">` for what was typed
and `<span class="ln ln--out">` for what came back, one output line per
expression. Bento cells use `<span class="expr">` and `<span class="ret">`, and
may quote only the results worth showing; each `ret` is checked against the
expression directly above it.

A third kind of block holds shell commands rather than expressions. It is
`<pre class="code code--shell">`, its commands are `<span class="ln ln--cmd">`
and the `ln--out` lines after each one are what it must print. The script runs
each command through a real shell with the binary's directory first on `PATH`,
so a pipeline may name `yarer` as often as it likes and every mention resolves
to the build under test. Everything a shell offers is therefore live: keep the
commands to what the page actually claims.

Piping into the REPL prints one line per expression, assignments included. Two
details matter when replaying by hand rather than through the script:

- **Redirect stderr into stdout.** yarer prints answers on stdout but its
  banner and its diagnostics on stderr. An interactive terminal shows both
  interleaved, which is what the page depicts, so a faithful replay needs
  `2>&1`. Without it a parse error simply vanishes from the capture.
- **Ignore the exit code.** A parse error makes the CLI exit non-zero, and one
  of the transcripts shows a parse error on purpose.

The two-line banner belongs to the CLI rather than to any expression, and the
trailing `quit` is what rustyline prints on end of input; the script strips
both.

## The link preview

`og.png` is rendered from `tools/og-card.html` by the browser, so the card and
the page agree on type, colour and spacing without either being copied into the
other:

```console
google-chrome --headless --disable-gpu --hide-scrollbars \
  --screenshot=/tmp/og-tall.png --window-size=1200,900 \
  "file://$PWD/tools/og-card.html"
convert /tmp/og-tall.png -crop 1200x630+0+0 +repage -strip og.png
```

The render is taller than the card and cropped down. At a 630px viewport
Chrome clips the bottom row rather than laying it out inside the box, and
cropping is shorter than fighting it.

The three numbers on the card are the same three the page carries, and go stale
the same way.

## Publishing

The page is static, so any host works. It lives on the `gh-pages` branch of the
crate's own repository, served from the branch root: nothing to build, no
workflow, and no way for these files to reach the published crate, because
`cargo package` only ever sees the branch it is run from.

The canonical address is `https://davassi.github.io/yarer/`, set in
`index.html` as `<link rel="canonical">`, `og:url` and the two image tags.

## Known gaps

- **System typefaces.** The page ships no webfont, which keeps it free of
  network requests and immune to layout shift, at the cost of looking slightly
  different per platform. The wordmark is an image, so it is unaffected.
  Self-hosting a mono face is the upgrade if that matters more than the
  requests.

## Licence

The page follows the crate: MIT or Apache-2.0, at your option.
