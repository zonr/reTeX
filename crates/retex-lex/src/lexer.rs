use retex_base::{MemoryBuffer, SourceLocation};
use crate::token::{Token, TokenKind, TokenFlags};
use crate::category_code::{CategoryCode, CategoryCodeTable};

/// Convert a hexadecimal character to its numeric value
fn hex_char_to_value(ch: u8) -> u8 {
    match ch {
        b'0'..=b'9' => ch - b'0',
        b'a'..=b'f' => ch - b'a' + 10,
        b'A'..=b'F' => ch - b'A' + 10,
        _ => unreachable!(), // Should not happen if is_ascii_hexdigit() was checked
    }
}

/// Turns a text buffer into a stream of tokens.
pub struct Lexer<'a> {
    /// The input bytes being lexed
    input: &'a [u8],
    /// Category code table for determining character types
    category_code_table: CategoryCodeTable,
    /// Start position of the next token to be lexed
    next_token_start_pos: usize,
    /// True if we are at the start of a line
    at_start_of_line: bool,
    /// Discard all space tokens
    skip_spaces: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a MemoryBuffer) -> Self {
        Self::from_bytes(buffer.data())
    }

    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self {
            input,
            category_code_table: CategoryCodeTable::new(),
            next_token_start_pos: 0,
            at_start_of_line: true,
            skip_spaces: true,
        }
    }

    pub fn set_category_code(&mut self, byte: u8, category_code: CategoryCode) {
        self.category_code_table.set(byte, category_code);
    }

    /// Reads a "logical" character from input. This applies transformation on the input that lexer sees.
    /// This includes: skipping \n next to \r and reducing expanded character like ^^A.
    fn get_char_and_size(&self, current_pos: usize) -> Option<(u8, usize)> {
        if current_pos >= self.input.len() {
            return None
        }

        let ch = self.input[current_pos];

        // Handle caret notation (^^A, ^^df, etc.)
        if ch == b'^' && current_pos + 2 < self.input.len() && self.input[current_pos + 1] == b'^' {
            let third_char = self.input[current_pos + 2];

            // Check for lowercase hex pattern (^^ab) first
            if current_pos + 3 < self.input.len() {
                let hex1 = third_char;
                let hex2 = self.input[current_pos + 3];
                if hex1.is_ascii_hexdigit() && hex2.is_ascii_hexdigit() {
                    let decoded = (hex_char_to_value(hex1) << 4) | hex_char_to_value(hex2);
                    return Some((decoded, 4));
                }
            }

            // Check for single character pattern (^^A)
            let decoded = if third_char >= 64 {
                third_char - 64  // ^^A becomes 1, ^^B becomes 2, etc.
            } else {
                third_char + 64  // ^^? becomes 127, etc.
            };
            return Some((decoded, 3));
        }

        // Skip \n next to \r. This follows logic in current TeX engine, for example:
        // https://github.com/TeX-Live/texlive-source/blob/2ebb86c/texk/web2c/lib/texmfmp.c#L2657-L2658
        if ch == b'\r' && current_pos + 1 < self.input.len() && self.input[current_pos + 1] == b'\n' {
            return Some((b'\r', 2));
        }

        Some((ch, 1))
    }

    fn peek_char(&self, current_pos: usize) -> Option<u8> {
        self.get_char_and_size(current_pos).map(|(ch, _)| ch)
    }

    fn consume_char(&self, current_pos: &mut usize) -> usize {
        if let Some((_, size)) = self.get_char_and_size(*current_pos) {
            *current_pos += size;
        }
        *current_pos
    }

    /// Forms a token with the given kind using the current token's start and end positions.
    /// Updates next_token_start_pos to prepare for the next token.
    fn form_token(&mut self, token: &mut Token<'a>, kind: TokenKind, cur_token_end_pos: usize) {
        let start_location = SourceLocation::new(self.next_token_start_pos as u32);
        let text_slice = &self.input[self.next_token_start_pos..cur_token_end_pos];

        token.set_kind(kind);
        token.set_location(start_location);
        token.set_text(text_slice);

        // Update start position for next token
        self.next_token_start_pos = cur_token_end_pos;
    }

    /// Reads raw bytes from input and advances next_token_start_pos until EOL. This Handles "\r\n"
    /// (by skipping \n next to \r). Also prepare lexer states for processing the next line.
    fn finish_line(&mut self) {
        while self.next_token_start_pos < self.input.len() {
            let ch = self.input[self.next_token_start_pos];
            self.next_token_start_pos += 1;

            if ch == b'\r' {
                // Handle \r\n by skipping the following \n if present.
                if self.next_token_start_pos < self.input.len() && self.input[self.next_token_start_pos] == b'\n' {
                    self.next_token_start_pos += 1;
                }
                break;
            } else if ch == b'\n' {
                break;
            }
        }

        if self.next_token_start_pos < self.input.len() {
            self.at_start_of_line = true;
            self.skip_spaces = true;
        }
    }

    /// We just read an escape character (\) that started a control sequence.
    /// Read the control word (letters) or control symbol (single character) that follows.
    fn lex_control_sequence(&mut self, token: &mut Token<'a>, current_pos: &mut usize) {
        // Skip the escape character
        self.consume_char(current_pos);

        // Check if next character is a letter
        if let Some(ch) = self.peek_char(*current_pos) {
            self.consume_char(current_pos);
            if self.category_code_table.is_letter(ch) {
                // Control word: read all letters
                while let Some(ch) = self.peek_char(*current_pos) {
                    if self.category_code_table.is_letter(ch) {
                        self.consume_char(current_pos);
                    } else {
                        break;
                    }
                }

                // After reading a control word, switch to skipping spaces state
                self.skip_spaces = true;
                self.form_token(token, TokenKind::ControlWord, *current_pos);
            } else {
                // Control symbol: read one character and skip subsequence spaces after a control space (an escape char
                // followed by a space: "\ ").
                self.skip_spaces = self.category_code_table.is_space(ch);
                self.form_token(token, TokenKind::ControlSymbol, *current_pos);
            }
        } else {
            // End of input after backslash - treat as control symbol
            self.form_token(token, TokenKind::ControlSymbol, *current_pos);
        }
    }

    /// We just read a parameter character (#) that may start a parameter token.
    /// Read the digit that follows (if any) to form a parameter reference like #1, #2, etc.
    fn lex_parameter_token(&mut self, token: &mut Token<'a>, current_pos: &mut usize) {
        // Skip the # character
        self.consume_char(current_pos);

        // Check if followed by a digit
        if let Some(ch) = self.peek_char(*current_pos) {
            if ch.is_ascii_digit() {
                self.consume_char(current_pos);
            }
        }

        self.form_token(token, TokenKind::Parameter, *current_pos);
    }

    pub fn lex(&mut self, token: &mut Token<'a>) {
        token.start_token();

        loop {
            let mut current_pos = self.next_token_start_pos;

            if self.skip_spaces {
                while let Some(ch) = self.peek_char(current_pos) {
                    if self.category_code_table.is_space_or_ignored(ch) {
                        self.consume_char(&mut current_pos);
                    } else {
                        break;
                    }
                }
                self.skip_spaces = false;
            }

            // Skip any ignored character.
            while let Some(ch) = self.peek_char(current_pos) {
                if self.category_code_table.is_ignored(ch) {
                    self.consume_char(&mut current_pos);
                } else {
                    break;
                }
            }

            // next_token_start_pos might have changed after skipping spaces and ignored characters.
            self.next_token_start_pos = current_pos;

            // Set flag if we're at the start of a line
            if self.at_start_of_line {
                token.set_flag(TokenFlags::START_OF_LINE);
                self.at_start_of_line = false;
            }

            let mut current_pos = self.next_token_start_pos;

            if let Some(ch) = self.peek_char(current_pos) {
                let category_code = self.category_code_table.get(ch);

                // Process the character based on its category code and current state
                match category_code {
                    CategoryCode::Escape => {
                        self.lex_control_sequence(token, &mut current_pos);
                        return;
                    },
                    CategoryCode::BeginGroup => {
                        self.form_token(token, TokenKind::BeginGroup, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::EndGroup => {
                        self.form_token(token, TokenKind::EndGroup, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::MathShift => {
                        self.form_token(token, TokenKind::MathShift, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::AlignmentTab => {
                        self.form_token(token, TokenKind::AlignmentTab, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::EndOfLine => {
                        let token_kind = if token.at_start_of_line() {
                            // Insert a \par token when encountering a newline at the start of line.
                            TokenKind::Paragraph
                        } else {
                            // Insert space token when encountering a newline in the middle of line.
                            TokenKind::Space
                        };
                        self.form_token(token, token_kind, self.consume_char(&mut current_pos));

                        if ch != b'\r' && ch != b'\n' {
                            // This follows how existing TeX engine works where input line is identified by \r and \n
                            // and bytes in the line after CategoryCode::EndOfLine are discarded.
                            self.finish_line();
                        } else {
                            self.at_start_of_line = true;
                            self.skip_spaces = true;
                        }
                        return
                    },
                    CategoryCode::Parameter => {
                        self.lex_parameter_token(token, &mut current_pos);
                        return;
                    },
                    CategoryCode::Superscript => {
                        self.form_token(token, TokenKind::Superscript, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Subscript => {
                        self.form_token(token, TokenKind::Subscript, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Ignored => {
                        // Ignored characters have been skipped at the beginning of the loop.
                        unreachable!()
                    },
                    CategoryCode::Space => {
                        // Produce a space token and skipping all subsequent spaces.
                        self.form_token(token, TokenKind::Space, self.consume_char(&mut current_pos));
                        self.skip_spaces = true;
                        return;
                    },
                    CategoryCode::Letter => {
                        self.form_token(token, TokenKind::Letter, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Other => {
                        self.form_token(token, TokenKind::Other, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Active => {
                        self.form_token(token, TokenKind::ActiveChar, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Comment => {
                        self.finish_line();
                        continue;
                    },
                    CategoryCode::Invalid => {
                        self.form_token(token, TokenKind::InvalidChar, self.consume_char(&mut current_pos));
                        return;
                    },
                }
            } else {
                // End of file
                token.set_kind(TokenKind::Eof);
                token.set_location(SourceLocation::new(current_pos as u32));
                return;
            }
        }
    }
}
