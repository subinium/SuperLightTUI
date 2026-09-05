# Standalone Browser Application

This crate owns its `cdylib`, wasm-bindgen entry point, and runtime handle.
It uses repository path dependencies for source development and has its own
workspace so native default features are not unified into its browser build.

From the SuperLightTUI repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/slt-wasm/examples/browser --target web --dev
python3 -m http.server 8080 --directory crates/slt-wasm/examples/browser
```

Open `http://localhost:8080`. Running the example needs no npm installation.
The page retains the returned Rust handle and disposes it on `pagehide`.

The optional `browser-tests` feature exports a compiled Rust probe used by
`browser.test.cjs`. The private npm package pins Playwright for testing, not
publication. With Node.js 20 or newer, from the repository root:

```sh
wasm-pack build crates/slt-wasm/examples/browser --target web --dev -- --features browser-tests
cd crates/slt-wasm/examples/browser
npm ci
npx playwright install chromium
npm test
```

The runner starts and closes its own local HTTP server. `PLAYWRIGHT_MODULE` can
point to an existing Playwright module, and `CHROME_EXECUTABLE` to installed
Chrome; that configuration does not need the Chromium installation step.
`SLT_BROWSER_SCREENSHOT` overrides the public example screenshot path, which
otherwise uses the operating system's temporary directory.

See the repository's `docs/WASM.md` for mount options, clipboard/IME boundaries,
fatal-error handling, and exact-version registry consumer instructions. Passing
these source tests does not establish registry publication or physical OS IME
compatibility.
