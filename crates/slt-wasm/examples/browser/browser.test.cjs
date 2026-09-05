const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

const root = __dirname;
const server = http.createServer((request, response) => {
  const file = path.resolve(root, '.' + decodeURIComponent(new URL(request.url, 'http://localhost').pathname));
  if (!file.startsWith(root + path.sep)) { response.writeHead(403).end(); return; }
  fs.readFile(file, (error, content) => {
    if (error) { response.writeHead(404).end(); return; }
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    response.end(content);
  });
});

async function run() {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROME_EXECUTABLE });
  const results = [];
  async function fixture(mode = 0, options = {}) {
    const context = await browser.newContext({ viewport: options.mobile ? { width: 390, height: 844 } : { width: 900, height: 700 }, deviceScaleFactor: options.scale || 1 });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`${origin}/test.html`);
    await page.evaluate(async ({ mode, fps, fit, manual }) => {
      if (manual) {
        const pending = new Map();
        let next = 0;
        window.requestAnimationFrame = callback => { pending.set(++next, callback); return next; };
        window.cancelAnimationFrame = id => pending.delete(id);
        window.advance = async time => {
          const callbacks = [...pending.values()]; pending.clear();
          callbacks.forEach(callback => callback(time));
          await Promise.resolve();
        };
        window.pendingFrames = () => pending.size;
      }
      const module = await import('./pkg/slt_browser_example.js');
      await module.default();
      window.module = module;
      window.probe = new module.Probe(document.querySelector('#host'), mode, fps, fit);
    }, { mode, fps: options.fps ?? 60, fit: !!options.fit, manual: !!options.manual });
    if (!options.manual) await page.waitForFunction(() => probe.frames() >= 3 || !probe.running());
    return { page, context, errors };
  }
  async function waitChange(page, action) {
    const before = await page.evaluate(() => probe.frames());
    await action();
    await page.waitForFunction(before => probe.frames() > before + 1, before);
  }
  async function close(test, label) {
    assert.deepEqual(test.errors, [], `${label}: page errors`);
    await test.page.evaluate(() => { probe.dispose(); window.other?.dispose(); });
    await test.context.close();
    results.push(label);
  }
  async function cell(page, x, y) {
    return page.evaluate(({ x, y }) => {
      const grid = document.querySelector('#host pre').getBoundingClientRect();
      return { x: grid.left + (x + .5) * grid.width / 16, y: grid.top + (y + .5) * grid.height / 6 };
    }, { x, y });
  }
  try {
    {
      const test = await fixture(); const { page } = test;
      await page.waitForFunction(() => probe.timers());
      assert.match(await page.locator('#host pre').textContent(), /^ABCDEFGH/);
      for (const [phase, expected] of [[1, /^X\s+$/], [2, /^\s+$/], [3, /^界A👩‍💻B/], [4, /^short\s+$/], [5, /MODAL/], [6, /^base\s+$/]]) {
        await waitChange(page, () => page.evaluate(phase => probe.set_phase(phase), phase));
        assert.match(await page.locator('#host pre').textContent(), expected);
      }
      assert.equal(await page.locator('#caller').textContent(), 'caller-owned');
      const mutations = await page.evaluate(async () => {
        let count = 0;
        const observer = new MutationObserver(changes => count += changes.length);
        observer.observe(document.querySelector('#host pre'), { childList: true, attributes: true, subtree: true });
        await new Promise(resolve => setTimeout(resolve, 100)); observer.disconnect();
        return count;
      });
      assert.equal(mutations, 0, 'unchanged complete redraw does not mutate grid');
      await page.evaluate(() => probe.dispose());
      await page.waitForFunction(() => probe.dropped());
      const stoppedAt = await page.evaluate(() => probe.frames());
      await page.evaluate(() => { probe.dispose(); window.dispatchEvent(new Event('resize')); });
      await page.waitForTimeout(80);
      assert.equal(await page.evaluate(() => probe.frames()), stoppedAt);
      assert.equal(await page.locator('#host textarea').count(), 0);
      assert.equal(await page.locator('#caller').textContent(), 'caller-owned');
      await page.evaluate(() => { probe = new module.Probe(document.querySelector('#host'), 0, 60, false); });
      await page.waitForFunction(() => probe.frames() >= 3);
      assert.equal(await page.locator('#host pre').count(), 1);
      await page.evaluate(() => { probe.drop_handle(); });
      await page.waitForFunction(() => probe.dropped());
      assert.equal(await page.evaluate(() => probe.running()), false);
      await close(test, 'real frames: fresh buffer, wide text, modal removal, timers, DOM diff, post-RAF dispose');
    }
    for (const scale of [1, 2]) {
      const test = await fixture(0, { scale, mobile: scale === 2 }); const { page } = test;
      for (let y = 0; y < 6; y++) {
        for (let x = 0; x < 16; x++) {
          const position = await cell(page, x, y);
          await page.mouse.click(position.x, position.y);
        }
      }
      await page.waitForTimeout(60);
      const recorded = await page.evaluate(() => probe.events());
      for (let y = 0; y < 6; y++) for (let x = 0; x < 16; x++) {
        assert.match(recorded, new RegExp(`Down\\(Left\\), x: ${x}, y: ${y},`));
      }
      const before = await page.locator('#host pre').boundingBox();
      await waitChange(page, () => page.evaluate(() => probe.set_phase(3)));
      const after = await page.locator('#host pre').boundingBox();
      assert.deepEqual(before, after);
      const symbols = await page.locator('#host pre span').evaluateAll(cells => cells.slice(0, 6).map(c => ({ text: c.textContent, display: getComputedStyle(c).display, width: c.getBoundingClientRect().width })));
      assert.equal(symbols[1].text, ''); assert.equal(symbols[1].display, 'none');
      assert.ok(Math.abs(symbols[0].width - symbols[2].width * 2) < .1);
      const p = await cell(page, 2, 1);
      await page.mouse.move(p.x, p.y); await page.mouse.wheel(0, 40);
      await page.waitForTimeout(50);
      assert.match(await page.evaluate(() => probe.events()), /ScrollDown, x: 2, y: 1,/);
      await page.screenshot({ path: path.join(require('node:os').tmpdir(), `slt-browser-scale-${scale}.png`) });
      await close(test, `fixed geometry: all 96 cell centers, padding/border, wide continuations, hover/wheel, scale ${scale}`);
    }
    {
      const test = await fixture(1); const { page } = test;
      await waitChange(page, () => page.evaluate(() => {
        const host = document.querySelector('#host');
        host.style.transformOrigin = '0 0';
        host.style.transform = 'translate(50px, 30px) scale(1.5)';
      }));
      const bounds = await page.evaluate(() => {
        const grid = document.querySelector('#host pre').getBoundingClientRect();
        const input = document.querySelector('#host textarea').getBoundingClientRect();
        return { grid: grid.toJSON(), input: input.toJSON() };
      });
      for (const field of ['x', 'y', 'width', 'height']) {
        assert.ok(Math.abs(bounds.grid[field] - bounds.input[field]) < .05, `CSS transformed ${field}: ${JSON.stringify(bounds)}`);
      }
      for (const [x, y] of [[0, 0], [15, 0], [0, 5], [15, 5], [7, 2]]) {
        const position = await cell(page, x, y);
        assert.equal(await page.evaluate(p => document.elementFromPoint(p.x, p.y).tagName, position), 'TEXTAREA');
        await page.mouse.click(position.x, position.y);
      }
      await page.waitForTimeout(50);
      assert.match(await page.evaluate(() => probe.events()), /Down\(Left\), x: 15, y: 5,/);
      await page.locator('#host').focus(); await page.keyboard.type('transformed');
      await page.waitForFunction(() => probe.text() === 'transformed');
      await page.locator('#outside').focus(); await page.keyboard.type('outside');
      assert.equal(await page.locator('#outside').inputValue(), 'outside');
      await close(test, 'actual CSS translate/scale: overlay bounds, editable corners, hit testing, focused typing and outside input');
    }
    {
      const test = await fixture(1); const { page } = test;
      await page.evaluate(() => {
        const before = document.createElement('button'); before.id = 'before-remount';
        before.textContent = 'Before mount';
        const host = document.querySelector('#host'); host.before(before);
        window.oldProbe = probe;
        probe.dispose();
        probe = new module.Probe(host, 1, 60, false);
        before.focus();
      });
      await page.waitForFunction(() => probe.frames() >= 3 && oldProbe.dropped());
      assert.equal(await page.locator('#host').getAttribute('tabindex'), '0');
      await page.keyboard.press('Tab');
      assert.equal(await page.locator('#host textarea').evaluate(element => element === document.activeElement), true);
      await page.keyboard.type('remounted');
      await page.waitForFunction(() => probe.text() === 'remounted');
      await page.locator('#outside').focus();
      await page.keyboard.press('Shift+Tab');
      assert.equal(await page.locator('#host textarea').evaluate(element => element === document.activeElement), true);
      await page.evaluate(() => oldProbe.dispose());
      assert.equal(await page.locator('#host').getAttribute('tabindex'), '0');
      await close(test, 'same-JS-tick dispose/remount retains tabindex and trusted Tab enters only the new input');
    }
    {
      const test = await fixture(1); const { page, context } = test;
      const sink = page.locator('#host textarea'); await page.locator('#host').focus();
      assert.equal(await sink.evaluate(element => element === document.activeElement), true);
      await page.keyboard.type('abc');
      await page.waitForFunction(() => probe.text() === 'abc');
      assert.match(await page.evaluate(() => probe.events()), /code: Char\('a'\)/, 'plain character shortcuts keep Key events');
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      await page.evaluate(() => navigator.clipboard.writeText('PASTE-한글-👩‍💻'));
      await page.keyboard.press(process.platform === 'darwin' ? 'Meta+V' : 'Control+V');
      await page.waitForFunction(() => probe.text() === 'abcPASTE-한글-👩‍💻');
      await page.evaluate(() => {
        const sink = document.querySelector('#host textarea');
        const clip = new DataTransfer(); clip.setData('text/plain', '-menu');
        sink.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clip, bubbles: true, cancelable: true }));
        sink.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        const enter = new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true });
        sink.dispatchEvent(enter); window.composingEnterCancelled = enter.defaultPrevented;
        sink.value = 'ㅎ'; sink.dispatchEvent(new InputEvent('input', { data: 'ㅎ', inputType: 'insertCompositionText', isComposing: true, bubbles: true }));
        sink.dispatchEvent(new CompositionEvent('compositionend', { data: '한', bubbles: true }));
        sink.dispatchEvent(new InputEvent('input', { data: '한', inputType: 'insertText', bubbles: true }));
        sink.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        sink.dispatchEvent(new CompositionEvent('compositionend', { data: '', bubbles: true }));
        sink.dispatchEvent(new InputEvent('input', { data: 'é', inputType: 'insertText', bubbles: true }));
      });
      await page.waitForFunction(() => probe.text() === 'abcPASTE-한글-👩‍💻-menu한é');
      assert.equal(await page.evaluate(() => composingEnterCancelled), false);
      assert.doesNotMatch(await page.evaluate(() => probe.events()), /code: Enter/);
      assert.equal(await page.evaluate(() => {
        const rect = document.querySelector('#host pre').getBoundingClientRect();
        return document.elementFromPoint(rect.left + 10, rect.top + 10).tagName;
      }), 'TEXTAREA', 'native context-menu target is editable');
      const button = await page.locator('#host pre span').evaluateAll(cells => {
        const start = cells.findIndex((c, i) => c.textContent === 'C' && cells[i + 1]?.textContent === 'l');
        if (start < 0) throw new Error('rendered Click button missing');
        const r = cells[start].getBoundingClientRect();
        return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
      });
      await page.mouse.click(button.x, button.y);
      await page.waitForFunction(() => probe.phase() === 1);
      await page.mouse.click(button.x, button.y);
      await page.waitForFunction(() => probe.double_clicked());
      await page.locator('#outside').focus(); await page.keyboard.type('outside');
      assert.equal(await page.locator('#outside').inputValue(), 'outside');
      await page.evaluate(() => { window.other = new module.Probe(document.querySelector('#second'), 1, 30, false); });
      await page.waitForFunction(() => other.frames() > 2);
      await page.locator('#second textarea').focus(); await page.keyboard.type('second');
      await page.waitForFunction(() => other.text() === 'second');
      assert.equal(await page.evaluate(() => probe.text()), 'abcPASTE-한글-👩‍💻-menu한é');
      await close(test, 'trusted typing/paste, synthetic paste/composition/dead-key commits, no composing Enter, button click, two mounts/outside input');
    }
    {
      const test = await fixture(2); const { page } = test;
      const divider = await page.locator('#host pre span').evaluateAll(cells => {
        const cell = cells.find(c => /│|┃/.test(c.textContent));
        if (!cell) throw new Error('splitter divider missing');
        const r = cell.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
      });
      const ratio = await page.evaluate(() => probe.ratio());
      await page.mouse.move(divider.x, divider.y); await page.mouse.down();
      await page.waitForTimeout(40);
      await page.mouse.move(divider.x + 30, divider.y); await page.waitForTimeout(40);
      assert.ok(await page.evaluate(() => probe.ratio()) > ratio);
      await page.mouse.move(600, 500); await page.mouse.up(); await page.waitForTimeout(50);
      assert.match(await page.evaluate(() => probe.events()), /Up\(Left\), x: 4294967295, y: 4294967295/);
      const ended = await page.evaluate(() => probe.ratio());
      await page.mouse.move(divider.x - 20, divider.y); await page.waitForTimeout(40);
      assert.equal(await page.evaluate(() => probe.ratio()), ended);
      const p = await cell(page, 1, 1);
      await page.mouse.move(p.x, p.y); await page.mouse.down();
      await page.waitForTimeout(30);
      await page.evaluate(() => window.dispatchEvent(new Event('blur')));
      await page.waitForTimeout(30); await page.mouse.up();
      assert.match(await page.evaluate(() => probe.events()), /FocusLost/);
      await page.evaluate(() => window.dispatchEvent(new Event('focus')));
      await page.mouse.move(p.x, p.y); await page.mouse.down();
      await page.waitForTimeout(30);
      await page.evaluate(() => {
        document.querySelector('#host').dispatchEvent(new PointerEvent('pointercancel', { pointerId: 1, bubbles: true }));
      });
      await page.mouse.up(); await page.waitForTimeout(30);
      const stoppedRatio = await page.evaluate(() => probe.ratio());
      await page.mouse.move(p.x + 20, p.y); await page.waitForTimeout(30);
      assert.equal(await page.evaluate(() => probe.ratio()), stoppedRatio);
      for (const button of ['middle', 'right']) {
        await page.keyboard.down('Shift');
        await page.mouse.move(p.x, p.y); await page.mouse.down({ button });
        await page.mouse.move(p.x + 20, p.y); await page.mouse.up({ button });
        await page.keyboard.up('Shift');
      }
      await page.waitForTimeout(50);
      assert.match(await page.evaluate(() => probe.events()), /Drag\(Middle\)/);
      assert.match(await page.evaluate(() => probe.events()), /Drag\(Right\)/);
      await close(test, 'real splitter drag, captured outside release, no stuck subsequent hover');
    }
    {
      const test = await fixture(1, { fit: true }); const { page } = test;
      await waitChange(page, () => page.evaluate(() => {
        const host = document.querySelector('#host');
        host.style.transformOrigin = '0 0'; host.style.transform = 'translate(50px, 30px) scale(1.5)';
        host.focus(); window.preeditSink = host.querySelector('textarea');
      }));
      await page.keyboard.type('A');
      await page.waitForFunction(() => probe.text() === 'A');
      const compose = (type, data) => page.evaluate(({ type, data }) => {
        preeditSink.dispatchEvent(new CompositionEvent(type, { data, bubbles: true }));
      }, { type, data });
      await compose('compositionstart', '');
      const pixels = [];
      for (const stage of ['ㅎ', '하', '한']) {
        await compose('compositionupdate', stage);
        await page.waitForFunction(stage => document.querySelector('[data-slt-preedit]').textContent === stage, stage);
        assert.equal(await page.evaluate(() => probe.text()), 'A', 'preedit is presentation, not app text');
        const style = await page.locator('[data-slt-preedit] span').first().evaluate(element => {
          const css = getComputedStyle(element);
          return { color: css.color, bg: css.backgroundColor, decoration: css.textDecorationLine, opacity: css.opacity, pointer: css.pointerEvents };
        });
        assert.notEqual(style.color, 'rgba(0, 0, 0, 0)');
        assert.notEqual(style.bg, 'rgba(0, 0, 0, 0)');
        assert.equal(style.opacity, '1'); assert.match(style.decoration, /underline/);
        assert.equal(style.pointer, 'none');
        pixels.push(await page.locator('[data-slt-preedit]').screenshot());
      }
      assert.notDeepEqual(pixels[0], pixels[1], 'visible glyph pixels change from ㅎ to 하');
      assert.notDeepEqual(pixels[1], pixels[2], 'visible glyph pixels change from 하 to 한');
      await page.screenshot({ path: process.env.SLT_BROWSER_PREEDIT_SCREENSHOT || path.join(require('node:os').tmpdir(), 'slt-browser-preedit.png') });
      await waitChange(page, () => page.locator('#host').evaluate(host => { host.style.width = '360px'; host.style.height = '192px'; }));
      assert.equal(await page.evaluate(() => preeditSink === document.querySelector('#host textarea') && preeditSink === document.activeElement), true);
      assert.equal(await page.locator('[data-slt-preedit]').textContent(), '한');
      await compose('compositionupdate', '한'.repeat(100));
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent.length > 2);
      const geometry = await page.evaluate(() => {
        const grid = document.querySelector('#host pre'), overlay = grid.querySelector('[data-slt-preedit]');
        const bounds = grid.getBoundingClientRect(), preview = overlay.getBoundingClientRect();
        const width = parseFloat(getComputedStyle(grid).width), scale = bounds.width / width;
        const padding = parseFloat(getComputedStyle(preeditSink).paddingLeft);
        const columns = parseFloat(grid.style.width);
        const glyphs = [...overlay.children].map(glyph => glyph.getBoundingClientRect().toJSON());
        return { grid: bounds.toJSON(), preview: preview.toJSON(), expectedLeft: bounds.left + padding * scale, cellWidth: bounds.width / columns, glyphs,
          hit: document.elementFromPoint(preview.left + 2, preview.top + 2) === preeditSink };
      });
      assert.ok(Math.abs(geometry.preview.left - geometry.expectedLeft) < .05, 'preedit starts at the real local caret');
      assert.ok(geometry.preview.right <= geometry.grid.right + .05);
      assert.equal(geometry.hit, true, 'visible preedit never intercepts pointer input');
      for (const glyph of geometry.glyphs) {
        assert.ok(glyph.right <= geometry.preview.right + .05, 'wide preedit is clipped to the grid');
        assert.ok(Math.abs(glyph.width - 2 * geometry.cellWidth) < .05, 'CJK preedit reserves two cells');
      }
      await compose('compositionupdate', '하');
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent === '하');
      assert.equal(await page.locator('[data-slt-preedit]').textContent(), '하', 'replacement removes the longer preedit');
      await compose('compositionend', '');
      assert.equal(await page.locator('[data-slt-preedit]').isVisible(), false);
      assert.equal(await page.evaluate(() => probe.text()), 'A');
      await compose('compositionstart', '');
      await page.evaluate(() => {
        preeditSink.value = '한';
        preeditSink.dispatchEvent(new InputEvent('input', { data: '한', inputType: 'insertCompositionText', isComposing: true, bubbles: true }));
        const enter = new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true });
        preeditSink.dispatchEvent(enter); window.preeditEnterCancelled = enter.defaultPrevented;
      });
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent === '한');
      assert.equal(await page.locator('[data-slt-preedit]').textContent(), '한', 'composing input fallback also presents preedit');
      await compose('compositionend', '한');
      await compose('compositionend', '한');
      await page.evaluate(() => preeditSink.dispatchEvent(new InputEvent('input', { data: '한', inputType: 'insertText', bubbles: true })));
      await page.waitForFunction(() => probe.text() === 'A한');
      assert.equal(await page.evaluate(() => preeditEnterCancelled), false);
      assert.doesNotMatch(await page.evaluate(() => probe.events()), /code: Enter/);
      await compose('compositionstart', ''); await compose('compositionupdate', 'discard');
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent === 'discard');
      await page.locator('#outside').focus();
      assert.equal(await page.locator('[data-slt-preedit]').isVisible(), false);
      await page.locator('#host').focus(); await compose('compositionend', 'discard');
      await page.waitForTimeout(50); assert.equal(await page.evaluate(() => probe.text()), 'A한');
      await compose('compositionstart', ''); await compose('compositionupdate', 'pending');
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent === 'pending');
      await page.evaluate(() => probe.dispose());
      await page.waitForFunction(() => !document.querySelector('[data-slt-preedit]') && !document.querySelector('#host textarea'));
      await page.keyboard.type('stopped'); await compose('compositionend', 'stopped');
      assert.equal(await page.evaluate(() => probe.text()), 'A한');
      await close(test, 'visible preedit glyph pixels, state isolation, style/width/scale/resize, replace/cancel/blur, exactly-once commit and disposal');
    }
    {
      const test = await fixture(1); const { page } = test;
      await page.locator('#host').focus();
      await page.evaluate(() => {
        const sink = document.querySelector('#host textarea');
        sink.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        sink.dispatchEvent(new CompositionEvent('compositionupdate', { data: '한', bubbles: true }));
      });
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]').textContent === '한');
      await page.evaluate(() => {
        const sink = document.querySelector('#host textarea');
        const original = Element.prototype.setAttribute;
        Element.prototype.setAttribute = function(name, value) {
          if (this.tagName === 'SPAN' && name === 'style' && value.includes('text-underline-offset:2px')) {
            Element.prototype.setAttribute = original; throw new Error('injected preedit paint failure');
          }
          return original.call(this, name, value);
        };
        sink.dispatchEvent(new CompositionEvent('compositionupdate', { data: '하', bubbles: true }));
      });
      await page.waitForFunction(() => !probe.running());
      assert.match(await page.evaluate(() => probe.error()), /preedit.*injected preedit paint failure/s);
      assert.equal(await page.locator('[data-slt-preedit], #host textarea').count(), 0);
      assert.equal(await page.evaluate(() => probe.text()), '');
      await close(test, 'preedit presentation failure stops cleanly and removes overlay plus editable sink');
    }
    for (const mode of [3, 4, 5]) {
      const test = await fixture(mode, { manual: true }); const { page } = test;
      if (mode === 5) await page.evaluate(() => document.querySelector('#host').addEventListener('slt-dispose', () => probe.dispose()));
      await page.evaluate(async () => {
        await advance(0); await advance(20);
        document.querySelector('#host').focus();
        const sink = document.querySelector('#host textarea');
        sink.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        sink.dispatchEvent(new CompositionEvent('compositionupdate', { data: 'pending', bubbles: true }));
        await advance(40);
      });
      assert.equal(await page.evaluate(() => probe.running()), false);
      assert.equal(await page.evaluate(() => pendingFrames()), 0);
      assert.equal(await page.locator('[data-slt-preedit], #host textarea').count(), 0);
      if (mode === 4) assert.match(await page.evaluate(() => probe.error()), /fatal browser frame/);
      else {
        assert.equal(await page.evaluate(() => probe.error()), undefined);
        assert.equal(await page.evaluate(() => probe.dropped()), true);
      }
      await close(test, `isolated lifecycle mode ${mode}: quit / fatal WASM trap / synchronous callback disposal`);
    }
    {
      const test = await fixture(1, { fps: 20, manual: true }); const { page } = test;
      await page.evaluate(async () => { await advance(0); await advance(5); await advance(49); });
      assert.equal(await page.evaluate(() => probe.frames()), 1);
      await page.locator('#host textarea').focus();
      await page.keyboard.type('queued');
      assert.equal(await page.evaluate(() => probe.text()), '');
      await page.evaluate(() => advance(50));
      assert.equal(await page.evaluate(() => probe.text()), 'queued');
      await page.evaluate(() => advance(10000));
      assert.equal(await page.evaluate(() => probe.frames()), 3);
      await page.evaluate(() => { probe.dispose(); });
      assert.equal(await page.evaluate(() => pendingFrames()), 0);
      await close(test, 'controlled RAF pacing, retained queued input, suspended-tab resume without catchup, throttled disposal');
    }
    {
      const test = await fixture(0, { fps: 0, manual: true }); const { page } = test;
      await page.evaluate(async () => { await advance(0); await advance(1); await advance(2); });
      assert.equal(await page.evaluate(() => probe.frames()), 3);
      await page.evaluate(() => { document.querySelector('#host').remove(); });
      await page.evaluate(() => advance(3));
      assert.equal(await page.evaluate(() => probe.frames()), 4);
      await page.evaluate(() => { window.requestAnimationFrame = () => { throw new Error('injected scheduling failure'); }; });
      await page.evaluate(() => advance(4));
      assert.equal(await page.evaluate(() => probe.running()), false);
      assert.match(await page.evaluate(() => probe.error()), /injected scheduling failure/);
      assert.equal(await page.evaluate(() => probe.dropped()), true);
      await close(test, 'uncapped RAF, explicit detached-host ownership, scheduling failure teardown');
    }
    {
      const context = await browser.newContext(); const page = await context.newPage();
      const errors = []; page.on('pageerror', error => errors.push(error.message));
      await page.goto(`${origin}/index.html`);
      await page.waitForFunction(() => document.querySelector('pre')?.textContent.includes('SuperLightTUI Browser'));
      assert.match(await page.locator('pre').textContent(), /Stop/);
      await page.locator('textarea').focus(); await page.keyboard.type('browser demo');
      await page.waitForFunction(() => document.querySelector('pre').textContent.includes('browser demo'));
      await page.screenshot({ path: process.env.SLT_BROWSER_SCREENSHOT || path.join(require('node:os').tmpdir(), 'slt-browser-example.png') });
      await page.evaluate(() => { window.retiredInput = document.querySelector('textarea'); });
      await page.evaluate(() => {
        retiredInput.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        retiredInput.dispatchEvent(new CompositionEvent('compositionupdate', { data: '중', bubbles: true }));
      });
      await page.waitForFunction(() => document.querySelector('[data-slt-preedit]')?.textContent === '중');
      assert.equal(await page.locator('[data-slt-preedit]').isVisible(), true);
      const stop = await page.locator('pre > span').evaluateAll(cells => {
        const label = cells.find((cell, index) => cell.textContent === 'S' && cells[index + 1]?.textContent === 't');
        if (!label) throw new Error('rendered Stop button missing');
        const rect = label.getBoundingClientRect(); return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
      });
      await page.mouse.click(stop.x, stop.y);
      await page.waitForFunction(() => !document.querySelector('textarea'));
      assert.equal(await page.locator('[data-slt-preedit]').count(), 0);
      const stopped = await page.locator('pre').textContent();
      await page.keyboard.type('must-not-insert');
      await page.evaluate(() => {
        retiredInput.focus();
        retiredInput.dispatchEvent(new InputEvent('input', { data: 'retired-input', inputType: 'insertText', bubbles: true }));
        retiredInput.dispatchEvent(new CompositionEvent('compositionend', { data: 'retired-composition', bubbles: true }));
      });
      await page.waitForTimeout(100);
      assert.equal(await page.locator('pre').textContent(), stopped);
      assert.equal(await page.evaluate(() => retiredInput.isConnected), false);
      assert.deepEqual(errors, []);
      await context.close();
      results.push('public example Stop removes its input and ignores later trusted typing and retired-sink events');
    }
    console.log(JSON.stringify({ browser: await browser.version(), tests: results, physicalIME: 'Not tested. Synthetic composition is not OS Korean/Japanese IME evidence.' }, null, 2));
  } finally {
    await browser.close();
    server.close();
  }
}
run().catch(error => { console.error(error); server.close(); process.exitCode = 1; });
