# Axoupdater resolved release patch

Source: axoupdater 0.10.3, from the crates.io source archive (MIT OR Apache-2.0).
Crates.io archive SHA-256: `0ff120905809a9ec70a810193d34841abfea1abad1ea1161456a81b2965bfd57`

Upstream: https://github.com/axodotdev/axoupdater/tree/v0.10.3
The original source files, manifests and README are retained with both licenses.

`src/lib.rs` adds `configure_release` so rstop can supply a validated public
GitHub release and its official installer URLs without querying the REST API
again. The existing receipt checks, cargo-dist installers, install directory,
Windows executable replacement and failure recovery remain in axoupdater.
Installer downloads also reject HTTP errors before executing their contents.

Remove this patch when upstream supports supplying resolved release metadata.
