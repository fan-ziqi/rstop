# rstop product film

**42 seconds · 1920 × 1080 · 30 fps · H.264 / AAC stereo.** Original motion graphics and synthesized score. Native rstop screens use fictional demo data; no third-party footage, logos or music.

Render from the repository root on Linux, with DejaVu and Noto CJK fonts:

```sh
python3 -m pip install -r promo/requirements.txt
python3 website/tools/generate-demo.py
python3 promo/render.py
python3 promo/make_readme_gif.py
```

Output: `artifacts/promo/rstop-product-film-1080p.mp4`. `--preview` renders only a contact sheet, poster and website social image. The MP4 is delivered separately and never included in the Pages artifact.
