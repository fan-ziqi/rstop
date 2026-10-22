# rstop product film

**94 seconds · 3840 × 2160 and 1920 × 1080 · 30 fps · H.264 / AAC stereo.** Eight native views, process filtering/sorting/trees/details, nine themes and recording/export commands. Camera push-ins and pans keep terminal text readable. Original soft piano score, without percussion or sub-bass. All telemetry is fictional.

Render from the repository root on Linux, with DejaVu and Noto CJK fonts:

```sh
python3 -m pip install -r promo/requirements.txt
python3 promo/capture_film.py
python3 promo/render_film.py
```

Outputs: `artifacts/promo/v2/rstop-feature-film-v2-4k.mp4` and `rstop-feature-film-v2-1080p.mp4`, with chapters. `--preview` renders stills and a contact sheet. The film is delivered separately and does not modify the website.

The README GIF remains independent: `python3 promo/make_readme_gif.py`. Its shared helpers are in `render.py`.
