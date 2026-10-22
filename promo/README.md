# rstop product film

**14 seconds · 4K and 1080p · 30 fps · H.264 / AAC stereo.** Fixed compositions, crisp cuts and a six-view grid. Native process filtering, memory sorting and trees; GPU metrics; all eight views; nine themes; recording, replay and export. English titles and an original soft piano score. All telemetry is fictional.

Render from the repository root on Linux with DejaVu fonts:

```sh
python3 -m pip install -r promo/requirements.txt
python3 promo/capture_film.py
python3 promo/render_film.py
```

Outputs: `artifacts/promo/short/rstop-14s-4k.mp4` and `rstop-14s-1080p.mp4`. `--preview` renders stills and a contact sheet. The film is delivered separately from the website.

The README GIF remains independent: `python3 promo/make_readme_gif.py`. Its shared helpers are in `render.py`.
