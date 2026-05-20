#[derive(Debug, Clone, Default)]
pub struct TextInput {
    pub buffer: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(initial: impl Into<String>) -> Self {
        let buffer: String = initial.into();
        let cursor = buffer.chars().count();
        Self { buffer, cursor }
    }

    pub fn insert(&mut self, c: char) {
        let byte_pos = self.byte_cursor();
        self.buffer.insert(byte_pos, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_end = self.byte_cursor();
        let prev = self.buffer[..byte_end]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let byte_start = byte_end - prev;
        self.buffer.replace_range(byte_start..byte_end, "");
        self.cursor -= 1;
    }

    pub fn delete(&mut self) {
        let byte_pos = self.byte_cursor();
        if byte_pos >= self.buffer.len() {
            return;
        }
        let next = self.buffer[byte_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.buffer.replace_range(byte_pos..byte_pos + next, "");
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let len = self.buffer.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.chars().count();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    fn byte_cursor(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_insert_and_cursor() {
        let mut t = TextInput::new("");
        t.insert('a');
        t.insert('b');
        assert_eq!(t.buffer, "ab");
        assert_eq!(t.cursor, 2);
        t.move_left();
        t.insert('X');
        assert_eq!(t.buffer, "aXb");
        assert_eq!(t.cursor, 2);
    }

    #[test]
    fn text_input_backspace_multibyte() {
        let mut t = TextInput::new("héllo");
        t.move_end();
        t.backspace();
        assert_eq!(t.buffer, "héll");
        t.move_left();
        t.move_left();
        assert_eq!(t.cursor, 2);
        t.backspace();
        assert_eq!(t.buffer, "hll");
        assert_eq!(t.cursor, 1);
    }

    #[test]
    fn text_input_delete_at_end_is_noop() {
        let mut t = TextInput::new("ab");
        t.move_end();
        t.delete();
        assert_eq!(t.buffer, "ab");
        assert_eq!(t.cursor, 2);
    }
}
