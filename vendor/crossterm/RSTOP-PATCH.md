# Crossterm input compatibility patch

Source: crossterm 0.29.0 (MIT), upstream commit `36d95b26a26e64b0f8c12edfe11f410a6d56a812`.

The Unix mio and /dev/tty event sources preserve both Esc events when two legacy
Esc bytes arrive in one read. Other key parsing is unchanged. An unnecessary
closure parenthesis is also removed for warning-free builds on Rust 1.95. rstop tests this
through a real PTY in `scripts/terminal_smoke.py`.

Remove this local patch when the upstream release preserves consecutive Esc keys.

Vendored text uses LF line endings; upstream Markdown trailing spaces are removed.
