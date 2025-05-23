use std::io::Write;

use crate::result::Res;

pub struct ScreenGuard {}

impl ScreenGuard {
    pub fn new() -> Res<Self> {
        let mut stdout = std::io::stdout();
        let mut buffer = vec![];
        let new_screen = b"\x1B[?1049h";
        let reset_cursor = b"\x1B[H";

        buffer.extend_from_slice(new_screen);
        buffer.extend_from_slice(reset_cursor);
        stdout.write_all(&buffer)?;
        stdout.flush()?;
        Ok(ScreenGuard {})
    }
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let mut buffer = vec![];
        let show_cursor = b"\x1B[?25h";
        let old_screen = b"\x1B[?1049l";

        buffer.extend_from_slice(old_screen);
        buffer.extend_from_slice(show_cursor);
        stdout.write_all(&buffer).unwrap();
        stdout.flush().unwrap();
    }
}
