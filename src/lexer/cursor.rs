use std::ops::Range;

pub struct Cursor<'a> {
    input: &'a str,
    pub pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(n)
    }

    pub fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn span_from(&self, start: usize) -> Range<usize> {
        start..self.pos
    }

    pub fn text_from(&self, start: usize) -> &'a str {
        &self.input[start..self.pos]
    }

    pub fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    pub fn advance_by(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }
}
