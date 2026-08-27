/**
 * yarer landing page - terminal playback and clipboard.
 *
 * The transcript is not stored here. It lives in the markup, which means the
 * page is complete and readable with JavaScript disabled, with JavaScript
 * broken, and to a screen reader that never runs this file at all. Everything
 * below only takes a transcript that is already on screen and reveals it more
 * slowly, so that a first-time reader watches the expressions being entered
 * rather than arriving at a wall of finished output.
 *
 * Every output line in the markup came out of the yarer 0.4.1 release binary.
 * See README.md for the commands that regenerate them.
 */
(() => {
  'use strict';

  /** All tunable timing in one place, rather than scattered as literals. */
  const TIMING = Object.freeze({
    charMs:       26,   // per character while typing an input line
    afterInputMs: 240,  // pause once an input line is complete
    beforeOutMs:  330,  // pause before an answer appears
    betweenMs:    620,  // longer pause between one exchange and the next
    startMs:      340,  // pause after the terminal scrolls into view
    statusMs:     2600, // how long the copy confirmation stays announced
  });

  /** Fraction of the terminal that must be on screen before playback starts. */
  const VISIBILITY_THRESHOLD = 0.35;

  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

  /* ── Terminal playback ────────────────────────────────────────────────── */

  /**
   * @param {HTMLElement} term
   */
  function initTerminal(term) {
    const body = term.querySelector('.term__body code');
    const replay = term.querySelector('[data-term-replay]');
    if (!body) return;

    /** @type {{el: HTMLElement, input: boolean, text: string, typed: HTMLElement|null, rest: HTMLElement|null}[]} */
    const lines = Array.from(body.querySelectorAll('.ln')).map((el) => {
      const input = el.classList.contains('ln--in');
      const text = el.textContent ?? '';
      let typed = null;
      let rest = null;

      if (input) {
        // Split the line in two so characters can move from one half to the
        // other. The halves always sum to the full text, so the line occupies
        // exactly the same box at every frame and the terminal never resizes.
        typed = document.createElement('span');
        typed.className = 'typed';
        typed.textContent = text;
        rest = document.createElement('span');
        rest.className = 'rest';
        el.replaceChildren(typed, rest);
      }

      return { el, input, text, typed, rest };
    });

    if (lines.length === 0) return;

    /** The in-flight run, if any. Cancelling one is how replay interrupts it. */
    let run = null;

    const sleep = (ms, token) => new Promise((resolve) => {
      token.timer = window.setTimeout(resolve, ms);
    });

    function reset() {
      for (const line of lines) {
        line.el.classList.remove('is-hidden', 'is-typing');
        if (line.input && line.typed && line.rest) {
          line.typed.textContent = line.text;
          line.rest.textContent = '';
        }
      }
      term.classList.remove('is-playing');
    }

    function cancel() {
      if (!run) return;
      run.cancelled = true;
      if (run.timer) window.clearTimeout(run.timer);
      run = null;
    }

    function typeLine(line, token) {
      return new Promise((resolve) => {
        const step = () => {
          if (token.cancelled || !line.rest || !line.typed) return resolve();
          const next = line.rest.textContent.charAt(0);
          if (next === '') return resolve();
          line.typed.textContent += next;
          line.rest.textContent = line.rest.textContent.slice(1);
          token.timer = window.setTimeout(step, TIMING.charMs);
        };
        token.timer = window.setTimeout(step, TIMING.charMs);
      });
    }

    async function play() {
      cancel();
      const token = { cancelled: false, timer: 0 };
      run = token;

      term.classList.add('is-playing');
      for (const line of lines) {
        line.el.classList.add('is-hidden');
        if (line.input && line.typed && line.rest) {
          line.typed.textContent = '';
          line.rest.textContent = line.text;
        }
      }
      if (replay) replay.hidden = true;

      try {
        await sleep(TIMING.startMs, token);

        let previousWasOutput = false;
        for (const line of lines) {
          if (token.cancelled) return;

          if (line.input) {
            if (previousWasOutput) await sleep(TIMING.betweenMs, token);
            if (token.cancelled) return;
            line.el.classList.remove('is-hidden');
            line.el.classList.add('is-typing');
            await typeLine(line, token);
            line.el.classList.remove('is-typing');
            if (token.cancelled) return;
            await sleep(TIMING.afterInputMs, token);
          } else {
            await sleep(TIMING.beforeOutMs, token);
            if (token.cancelled) return;
            line.el.classList.remove('is-hidden');
          }
          previousWasOutput = !line.input;
        }

        if (token.cancelled) return;
        term.classList.remove('is-playing');
        if (replay) replay.hidden = false;
      } catch (error) {
        // Nothing above is expected to throw, but a half-revealed transcript
        // would be worse than no animation at all, so recover into the
        // finished state rather than leaving lines hidden.
        console.error('[yarer] terminal playback failed', error);
        reset();
        if (replay) replay.hidden = false;
      } finally {
        if (run === token) run = null;
      }
    }

    if (replay) {
      replay.addEventListener('click', () => {
        if (reduceMotion.matches) return;
        play();
      });
    }

    // If the setting is flipped while the page is open, stop mid-run and show
    // the whole transcript. Honouring the preference only at load would leave
    // a reader who just switched it on watching the thing they turned off.
    reduceMotion.addEventListener('change', (event) => {
      if (!event.matches) return;
      cancel();
      reset();
      if (replay) replay.hidden = true;
    });

    // Browsers clamp setTimeout to roughly one tick a second in a background
    // tab. An intersection observer does not care whether the tab is in front,
    // so without this gate a page opened in a background tab starts typing
    // immediately and crawls, and the reader arrives to a half-finished line
    // creeping across the screen.
    let waitingForFocus = false;

    function start() {
      if (document.visibilityState !== 'visible') {
        waitingForFocus = true;
        return;
      }
      waitingForFocus = false;
      play();
    }

    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') {
        if (waitingForFocus) start();
        return;
      }
      if (!run) return;             // nothing in flight, nothing to rescue
      cancel();
      reset();
      if (replay) replay.hidden = false;
    });

    if (reduceMotion.matches) return;   // transcript already complete in markup

    if (typeof IntersectionObserver !== 'function') {
      start();
      return;
    }

    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer.disconnect();
        start();
      }
    }, { threshold: VISIBILITY_THRESHOLD });

    observer.observe(term);
  }

  /* ── Copy to clipboard ────────────────────────────────────────────────── */

  /**
   * @param {string} text
   * @returns {Promise<boolean>} whether the text reached the clipboard
   */
  async function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      try {
        await navigator.clipboard.writeText(text);
        return true;
      } catch (error) {
        // Permission denied, or an insecure context the check above missed.
        // Fall through rather than reporting a failure we can still avoid.
        console.warn('[yarer] clipboard API refused, trying the fallback', error);
      }
    }

    // execCommand is deprecated but remains the only path in a non-secure
    // context, which includes opening this page straight from the filesystem.
    const field = document.createElement('textarea');
    field.value = text;
    field.setAttribute('readonly', '');
    field.setAttribute('aria-hidden', 'true');
    field.style.cssText = 'position:fixed;top:0;left:-9999px;opacity:0';
    document.body.appendChild(field);

    try {
      field.select();
      return document.execCommand('copy');
    } catch (error) {
      console.warn('[yarer] clipboard fallback failed', error);
      return false;
    } finally {
      field.remove();
    }
  }

  /**
   * @param {HTMLButtonElement} button
   */
  function initCopy(button) {
    const status = document.querySelector('[data-copy-status]');
    const source = document.querySelector(button.dataset.copy ?? '');
    if (!source) return;

    let clear = 0;

    button.addEventListener('click', async () => {
      const ok = await copyText(source.textContent?.trim() ?? '');
      if (!status) return;

      status.textContent = ok
        ? 'Copied to the clipboard.'
        : 'Could not reach the clipboard. Select the command and copy it.';

      window.clearTimeout(clear);
      clear = window.setTimeout(() => { status.textContent = ''; }, TIMING.statusMs);
    });
  }

  /* ── Boot ─────────────────────────────────────────────────────────────── */

  try {
    document.querySelectorAll('[data-term]').forEach((el) => initTerminal(el));
    document.querySelectorAll('[data-copy]').forEach((el) => initCopy(el));
  } catch (error) {
    console.error('[yarer] initialisation failed', error);
  }
})();
