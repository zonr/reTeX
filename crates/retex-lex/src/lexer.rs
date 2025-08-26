use std::num::NonZeroU8;
use retex_base::{SourceLocation, MaybeChar};
use crate::token::{Token, TokenKind, TokenFlags, TokenData};
use crate::category_code::{CategoryCode, CategoryCodeTable};
use crate::command_identifier::CommandIdentifierTable;

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
pub struct Lexer<'source, 'idtable> {
    /// The input bytes being lexed
    input: &'source [u8],
    /// Category code table for determining character types
    category_code_table: CategoryCodeTable,
    /// Start position of the next token to be lexed
    next_token_start_pos: usize,
    /// True if we are at the start of a line
    at_start_of_line: bool,
    /// Discard all space tokens
    skip_spaces: bool,
    /// Reference to preprocessor for command identifier management
    command_identifier_table: &'idtable CommandIdentifierTable<'idtable>,
}

impl<'source, 'idtable, 'token> Lexer<'source, 'idtable>
where
    'source: 'token,
    'idtable: 'token {
    pub fn from_bytes(input: &'source [u8], command_identifier_table: &'idtable CommandIdentifierTable<'idtable>) -> Self {
        Self {
            input,
            category_code_table: CategoryCodeTable::new(),
            next_token_start_pos: 0,
            at_start_of_line: true,
            skip_spaces: true,
            command_identifier_table,
        }
    }

    pub fn set_category_code(&mut self, maybe_char: MaybeChar, category_code: CategoryCode) {
        self.category_code_table.set(maybe_char, category_code);
    }


    /// Reads a "logical" character from input. This applies transformation on the input that lexer sees.
    /// This includes: skipping \n next to \r and reducing expanded character like ^^A. Returns a 3-tuple: the byte
    /// being read, number of bytes occupied by the returning byte in the input and a boolean flag indicating if any
    /// transformed have been applied on the input while reading the returning byte.
    ///
    /// TODO: Validate and turn bytes into Unicode char when possible like XeTeX to support unicode:
    /// https://github.com/TeX-Live/texlive-source/blob/2ebb86c/texk/web2c/lib/texmfmp.c#L2657-L2658
    fn get_char_and_size(&self, current_pos: usize) -> Option<(MaybeChar, usize, bool)> {
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
                    return Some((MaybeChar::from_char(decoded as char), 4, true));
                }
            }

            // Check for single character pattern (^^A)
            let decoded = if third_char >= 64 {
                third_char - 64  // ^^A becomes 1, ^^B becomes 2, etc.
            } else {
                third_char + 64  // ^^? becomes 127, etc.
            };
            return Some((MaybeChar::from_char(decoded as char), 3, true));
        }

        // Skip \n next to \r. This follows logic in current TeX engine, for example:
        if ch == b'\r' && current_pos + 1 < self.input.len() && self.input[current_pos + 1] == b'\n' {
            return Some((MaybeChar::from_char('\r'), 2, true));
        }

        Some((MaybeChar::from_char(ch as char), 1, false))
    }

    fn peek_char(&self, current_pos: usize) -> Option<MaybeChar> {
        self.get_char_and_size(current_pos).map(|(maybe_char, _, _)| maybe_char)
    }

    fn consume_char(&self, current_pos: &mut usize) -> usize {
        if let Some((_, size, _)) = self.get_char_and_size(*current_pos) {
            *current_pos += size;
        }
        *current_pos
    }

    /// Forms a token with the given kind using the current token's start and end positions.
    /// Updates next_token_start_pos to prepare for the next token.
    fn form_token_with_data<'a>(
        &mut self, token:
        &mut Token<'a>,
        kind: TokenKind,
        token_data: TokenData<'a>,
        cur_token_end_pos: usize) {

        let start_location = SourceLocation::new(self.next_token_start_pos as u32);

        token.set_kind(kind);
        token.set_location(start_location);
        token.set_length((cur_token_end_pos - self.next_token_start_pos) as u32);
        token.set_token_data(token_data);

        // Update start position for next token
        self.next_token_start_pos = cur_token_end_pos;
    }

    fn form_token(&mut self, token: &mut Token, kind: TokenKind, cur_token_end_pos: usize) {
        self.form_token_with_data(token, kind, TokenData::None, cur_token_end_pos);
    }

    fn form_token_with_char(
        &mut self,
        token: &mut Token,
        kind: TokenKind,
        ch: MaybeChar,
        cur_token_end_pos: usize) {

        self.form_token_with_data(
            token,
            kind,
            TokenData::Char(ch.as_char().unwrap_or(char::REPLACEMENT_CHARACTER)),
            cur_token_end_pos);
    }

    /// Reads raw bytes from input and advances next_token_start_pos until EOL. This Handles "\r\n"
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
    fn lex_control_sequence(&mut self, token: &mut Token<'token>, current_pos: &mut usize) {
        // Skip the escape character
        self.consume_char(current_pos);

        // Check if next character is a letter
        if let Some((maybe_char, size, is_transformed)) = self.get_char_and_size(*current_pos) {
            if self.category_code_table.is_letter(maybe_char) {
                self.consume_char(current_pos);
                self.lex_control_word_continue(token, current_pos, maybe_char, size, is_transformed);
            } else {
                self.consume_char(current_pos);
                // Control symbol: read one character and skip subsequence spaces after a control space (an escape char
                // followed by a space: "\ ").
                self.skip_spaces = self.category_code_table.is_space(maybe_char);
                let symbol_data = TokenData::Symbol(Some(maybe_char));
                self.form_token_with_data(token, TokenKind::ControlSymbol, symbol_data, *current_pos);
            }
        } else {
            // End of input after backslash - treat as control symbol with no symbol
            self.form_token_with_data(token, TokenKind::ControlSymbol, TokenData::Symbol(None), *current_pos);
        }
    }

    /// We just read and consumed the first letter of a control word after the escape character.
    /// Read all remaining letters to form the complete control word token.
    fn lex_control_word_continue(
        &mut self,
        token: &mut Token<'token>,
        current_pos: &mut usize,
        first_ch: MaybeChar,
        first_ch_size: usize,
        is_first_ch_transformed: bool) {

        let control_word_start = *current_pos - first_ch_size;

        // Local buffer for UTF-8 encoding
        let mut utf8_buffer = [0u8; 4];

        // Switch to use owned bytes when transformation has been applied on input (such as caret notation or Unicode
        // normalization)
        let mut owned_name_bytes: Option<Vec<u8>> = if is_first_ch_transformed {
            Some(first_ch.encode_utf8(&mut utf8_buffer).to_vec())
        } else {
            None
        };

        while owned_name_bytes.is_none() {
            if let Some((ch, _, is_transformed)) = self.get_char_and_size(*current_pos) {
                if !self.category_code_table.is_letter(ch) {
                    break
                }

                if is_transformed {
                    let control_word_bytes = &self.input[control_word_start..*current_pos];
                    owned_name_bytes = Some(control_word_bytes.to_vec());
                    owned_name_bytes.as_mut().unwrap().extend_from_slice(ch.encode_utf8(&mut utf8_buffer));
                }
                self.consume_char(current_pos);
            } else {
                break;
            }
        }

        // Continue collecting letters if we have owned bytes
        if let Some(ref mut owned_bytes) = owned_name_bytes {
            while let Some((ch, _, _)) = self.get_char_and_size(*current_pos) {
                if self.category_code_table.is_letter(ch) {
                    owned_bytes.extend_from_slice(ch.encode_utf8(&mut utf8_buffer));
                    self.consume_char(current_pos);
                } else {
                    break;
                }
            }
        }

        // Get command identifier from preprocessor
        let name_bytes = match owned_name_bytes {
            Some(ref owned) => owned.as_slice(),
            None => &self.input[control_word_start..*current_pos],
        };

        // Form the control word token
        let command_identifier = self.command_identifier_table.get_or_insert(name_bytes);
        self.form_token_with_data(token, TokenKind::ControlWord, TokenData::CommandIdentifier(command_identifier), *current_pos);

        // After reading a control word, switch to skipping spaces state
        self.skip_spaces = true;
    }

    /// We just read a parameter character (#) that may start a parameter token.
    /// Read the digit that follows (if any) to form a parameter reference like #1, #2, etc.
    fn lex_parameter_token(&mut self, token: &mut Token<'token>, current_pos: &mut usize) {
        // Skip the # character
        self.consume_char(current_pos);

        // Check if followed by a digit
        let mut parameter_data = TokenData::ParameterIndex(None);
        if let Some(ch) = self.peek_char(*current_pos) {
            if let Some(c) = ch.as_char().filter(|c| c.is_ascii_digit()) {
                parameter_data = TokenData::ParameterIndex(NonZeroU8::new(c as u8 - b'0'));
                self.consume_char(current_pos);
            }
        }

        self.form_token_with_data(token, TokenKind::Parameter, parameter_data, *current_pos);
    }

    pub fn lex(&mut self, token: &mut Token<'token>) {
        token.reset();

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

                        if ch != MaybeChar::from_char('\r') && ch != MaybeChar::from_char('\n') {
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
                        // Skip spaces before EOL or EOF according to TeX rules - only emit a space token if we hit
                        // bytes other than space, EOL and EOF

                        // Form a token so in the case where we need to emit a space token for this space, the output
                        // token refers to the first space
                        self.form_token(token, TokenKind::Space, self.consume_char(&mut current_pos));

                        // Skip all subsequent spaces
                        let mut emit_space_token = false;
                        while let Some(next_ch) = self.peek_char(current_pos) {
                            if self.category_code_table.is_space(next_ch) {
                                self.consume_char(&mut current_pos);
                                continue;
                            }

                            // Only emit a space token if encountering a non-EOL bytes
                            emit_space_token = !self.category_code_table.is_eol(next_ch);
                            break;
                        }

                        // Point to the next non-space pos
                        self.next_token_start_pos = current_pos;
                        if !emit_space_token {
                            // Ignore all spaces and restart the loop to get a token based on next byte
                            continue;
                        }

                        // Note the token has been formed at the beginning of the case, so just return
                        return;
                    },
                    CategoryCode::Letter => {
                        self.form_token_with_char(token, TokenKind::Letter, ch, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Other => {
                        self.form_token_with_char(token, TokenKind::Other, ch, self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Active => {
                        let mut utf8_buffer = [0u8; 4];
                        let active_char = ch.encode_utf8(&mut utf8_buffer);
                        self.form_token_with_data(
                            token,
                            TokenKind::ActiveChar,
                            TokenData::CommandIdentifier(self.command_identifier_table.get_or_insert(active_char)),
                            self.consume_char(&mut current_pos));
                        return;
                    },
                    CategoryCode::Comment => {
                        self.finish_line();
                        continue;
                    },
                    CategoryCode::Invalid => {
                        // Skip invalid char.
                        //
                        // TODO: Add diagnosis instead of discarding silently.
                        self.consume_char(&mut current_pos);
                        self.next_token_start_pos = current_pos;
                        continue;
                    },
                }
            } else {
                // End of file
                self.form_token(token, TokenKind::Eof, current_pos);
                return;
            }
        }
    }
}
