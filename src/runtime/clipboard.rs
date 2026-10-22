use crossterm::{clipboard::CopyToClipboard, execute};
use std::io;

#[derive(Default)]
pub(super) struct Clipboard {
    // X11/Wayland clipboard ownership must outlive the copy operation.
    native: Option<arboard::Clipboard>,
}
impl Clipboard {
    pub fn copy(&mut self, text: String) -> io::Result<&'static str> {
        let remote =
            std::env::var_os("SSH_TTY").is_some() || std::env::var_os("SSH_CONNECTION").is_some();
        if !remote {
            if self.native.is_none() {
                self.native = arboard::Clipboard::new().ok();
            }
            if let Some(clipboard) = &mut self.native
                && clipboard.set_text(&text).is_ok()
            {
                return Ok("Copied selected text");
            }
        }
        execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text))?;
        Ok("Copy sent to terminal clipboard (OSC 52)")
    }
}
