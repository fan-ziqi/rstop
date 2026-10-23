# rstop product website

Static site, with no package installation or build step. Published by `.github/workflows/pages.yml` to **https://fan-ziqi.github.io/rstop/**.

Preview from the repository root:

```sh
python3 -m http.server 4173 --directory website/dist
```

The website renders terminal cells produced by **`rstop::ui::draw`**, the same Rust renderer used by the terminal application. It preserves the native layout, text, braille plots and colors. There is no separate HTML redesign of the product. Regenerate all eight views, nine themes and three terminal sizes:

```sh
python3 website/tools/generate-native-demo.py
```

One terminal contains all eight views, with nine theme buttons beneath the window. The browser displays inline SVG text and shapes from the native cells; table interactions reuse native row and detail layouts. Lists support filtering in the terminal's own filter field, process columns support sorting, and rows open their native detail panels. Use 1–8 to change views, ↑↓ to select rows, / or F3 to edit the filter, T to toggle the process tree, and Esc to close details or clear the filter. These are bounded demonstrations, not a browser port of the complete CLI.

The generator checks every exported character and style against the native terminal buffer. Process filter syntax follows `src/filter.rs`; other list filters use text search. No process management, packet capture or host commands run in the browser.

The older native SVG reference assets can also be regenerated after building the binary:

```sh
python3 website/tools/generate-demo.py --binary target/debug/rstop
```

All telemetry is fictional; no host metrics are collected. Data updates while visible and stops for reduced motion. Runtime assets and the licensed DejaVu terminal font are served locally, with no JavaScript dependencies.

The product film is delivered separately. No video is embedded or deployed with the website.
