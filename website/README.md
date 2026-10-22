# rstop product website

Static site, with no package installation or build step. Published by `.github/workflows/pages.yml` to **https://fan-ziqi.github.io/rstop/**.

Preview from the repository root:

```sh
python3 -m http.server 4173 --directory website/dist
```

The eight views and nine themes are rendered by rstop itself from fictional data. Regenerate after building the binary:

```sh
python3 website/tools/generate-demo.py --binary target/debug/rstop
```

Screenshots contain no host telemetry. Page animations illustrate behavior; the site does not collect system metrics. All runtime assets are local. Respect `prefers-reduced-motion`.

The standalone product film and its rendering instructions are in [../promo](../promo/README.md). No video is embedded or deployed with the website.
