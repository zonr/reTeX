use retex_lex::{Lexer, Token, TokenKind, TokenFlags};
use retex_lex::category_code::CategoryCode;
use retex_base::{MaybeChar, SourceLocation};
use retex_lex::token::TokenData;
use std::num::NonZeroU8;
use retex_lex::command_identifier::CommandIdentifierTable;

/// Helper constants for common flag combinations
const NO_FLAGS: TokenFlags = TokenFlags::NONE;
const START_OF_LINE: TokenFlags = TokenFlags::START_OF_LINE;

fn assert_tokens_match(input: &str, expected: &[(TokenKind, SourceLocation, u32, TokenFlags, TokenData)]) {
    let command_identifier_table = CommandIdentifierTable::new();
    let mut lexer = Lexer::from_bytes(input.as_bytes(), &command_identifier_table);
    assert_tokens_match_with_lexer(&mut lexer, expected);
}

/// Helper function for testing tokens with custom lexer
fn assert_tokens_match_with_lexer(
    lexer: &mut Lexer,
    expected: &[(TokenKind, SourceLocation, u32, TokenFlags, TokenData)],
) {
    let mut token = Token::default();
    let mut actual = Vec::new();

    loop {
        lexer.lex(&mut token);

        actual.push(token.clone());

        if token.kind() == TokenKind::Eof {
            break;
        }
    }

    if actual.len() != expected.len() {
        panic!("Token count mismatch. Expected {} tokens, got {}.\nExpected: {:#?}\nActual: {:#?}",
               expected.len(), actual.len(), expected, actual);
    }

    for (i, (expected_tuple, act)) in expected.iter().zip(actual.iter()).enumerate() {
        let (exp_kind, exp_loc, exp_len, exp_flags, exp_data) = expected_tuple;

        if *exp_kind != act.kind() ||
            *exp_loc != act.location() ||
            *exp_len != act.length() ||
            *exp_flags != act.flags() {
            panic!("Token {} mismatch:\nExpected: ({:?}, {:?}, {:?}, {:?})\nActual: ({:?}, {:?}, {:?}, {:?})",
                   i, exp_kind, exp_loc, exp_len, exp_flags,
                   act.kind(), act.location(), act.length(), act.flags());
        }

        // Validate token data based on token kind using matches! with guards
        match exp_kind {
            TokenKind::Letter | TokenKind::Other => {
                assert!(matches!(exp_data, TokenData::Char(expected_char) if act.char() == *expected_char),
                    "Token {} data mismatch: expected char {:?}, got char {:?}", i, exp_data, act.char());
            },
            TokenKind::Parameter => {
                assert!(matches!(exp_data, TokenData::ParameterIndex(expected_index) if act.parameter_index() == *expected_index),
                    "Token {} data mismatch: expected parameter {:?}, got parameter {:?}", i, exp_data, act.parameter_index());
            },
            TokenKind::ControlSymbol => {
                assert!(matches!(exp_data, TokenData::Symbol(expected_symbol) if act.symbol() == *expected_symbol),
                    "Token {} data mismatch: expected symbol {:?}, got symbol {:?}", i, exp_data, act.symbol());
            },
            TokenKind::ControlWord | TokenKind::ActiveChar => {
                assert!(matches!(exp_data, TokenData::CommandIdentifier(expected_id)
                    if act.command_identifier().as_bytes() == expected_id.as_bytes()),
                    "Token {} data mismatch: expected command {:?}, got command {:?}", i, exp_data, act.command_identifier());
            },
            _ => {
                // For tokens with TokenData::None (Eof, Unknown, BeginGroup, EndGroup, etc.)
                assert!(matches!(exp_data, TokenData::None),
                    "Token {} data mismatch: expected None data for {:?}, got {:?}", i, exp_kind, exp_data);
            }
        }
    }
}

#[test]
fn test_empty_input() {
    assert_tokens_match("", &[
        (TokenKind::Eof, SourceLocation::new(0), 0, START_OF_LINE, TokenData::None),
    ]);
}

#[test]
fn test_simple_text() {
    assert_tokens_match("hello", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_mixed_characters() {
    assert_tokens_match("a1b2c", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('1')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')),
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('2')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('c')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_special_characters() {
    assert_tokens_match("{}$&^_", &[
        (TokenKind::BeginGroup, SourceLocation::new(0), 1, START_OF_LINE, TokenData::None),
        (TokenKind::EndGroup, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None),
        (TokenKind::MathShift, SourceLocation::new(2), 1, NO_FLAGS, TokenData::None),
        (TokenKind::AlignmentTab, SourceLocation::new(3), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Superscript, SourceLocation::new(4), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Subscript, SourceLocation::new(5), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_word_with_caret_notation_in_middle() {
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\te^^?st".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 3, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"te"))),
        // ^^? is DEL which is ignored.
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, TokenData::Char('s')),
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Eof, SourceLocation::new(8), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_symbol() {
    assert_tokens_match("\\{  ", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, TokenData::Symbol(Some(MaybeChar::from_char('{')))),
        // Spaces at EOF are skipped - no space token generated
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_symbol_eof() {
    assert_tokens_match("\\", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Symbol(None)),
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_space() {
    assert_tokens_match("\\  ", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, TokenData::Symbol(Some(MaybeChar::from_char(' ')))),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None), // space after control space is skipped
    ]);
}

#[test]
fn test_control_sequence_with_text() {
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\test hello".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 5, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"test"))),
        // Space after control word is skipped
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_spaces() {
    assert_tokens_match("a b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // space
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_multiple_spaces() {
    assert_tokens_match("a   b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // first space (others skipped)
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_tabs_treated_as_spaces() {
    assert_tokens_match("a\tb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // tab
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_parameter_token() {
    assert_tokens_match("#1", &[
        (TokenKind::Parameter, SourceLocation::new(0), 2, START_OF_LINE, TokenData::ParameterIndex(NonZeroU8::new(1))), // #1
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_parameter_token_with_non_digit() {
    assert_tokens_match("#a", &[
        (TokenKind::Parameter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::ParameterIndex(None)), // #
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('a')), // a
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_parameter_token_without_digit() {
    assert_tokens_match("#", &[
        (TokenKind::Parameter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::ParameterIndex(None)), // #
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_active_character() {
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("@".as_bytes(), &id_table);
    // Make @ an active character instead of Other
    lexer.set_category_code(MaybeChar::from_char('@'), CategoryCode::Active);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ActiveChar, SourceLocation::new(0), 1, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"@"))),
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_comment() {
    assert_tokens_match("hello%comment\n  ^^?world", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),  // h
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')), // e
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')), // l
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')), // l
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')), // o
        // comment is skipped along with spaces and ignored characters on the next line
        (TokenKind::Letter, SourceLocation::new(19), 1, START_OF_LINE, TokenData::Char('w')), // w
        (TokenKind::Letter, SourceLocation::new(20), 1, NO_FLAGS, TokenData::Char('o')), // o
        (TokenKind::Letter, SourceLocation::new(21), 1, NO_FLAGS, TokenData::Char('r')), // r
        (TokenKind::Letter, SourceLocation::new(22), 1, NO_FLAGS, TokenData::Char('l')), // l
        (TokenKind::Letter, SourceLocation::new(23), 1, NO_FLAGS, TokenData::Char('d')), // d
        (TokenKind::Eof, SourceLocation::new(24), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_newline_handling() {
    assert_tokens_match("a\nb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // newline becomes space
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_paragraph_break() {
    assert_tokens_match("\n", &[
        (TokenKind::Paragraph, SourceLocation::new(0), 1, START_OF_LINE, TokenData::None),
        (TokenKind::Eof, SourceLocation::new(1), 0, START_OF_LINE, TokenData::None),
    ]);
}

#[test]
fn test_start_of_line_flag() {
    assert_tokens_match("a", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_ignored_characters() {
    // DEL character (127) should be ignored
    let input = format!("a{}b", char::from(127));
    assert_tokens_match(&input, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        // DEL is ignored.
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')), // b (length includes ignored char)
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_invalid_characters() {
    let command_identifier_table = CommandIdentifierTable::new();
    let mut lexer = Lexer::from_bytes("a|b".as_bytes(), &command_identifier_table);
    lexer.set_category_code(MaybeChar::from_char('|'), CategoryCode::Invalid);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        // | is invalid and should be ignored.
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_lexer_from_bytes() {
    // Just test that it creates successfully with assert_tokens_match
    // The actual lexing behavior should be the same as from_str
    assert_tokens_match("hello", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}


#[test]
fn test_carriage_return_handling() {
    assert_tokens_match("a\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // \r becomes space
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_token_text_preservation() {
    // Test that control words preserve their exact text representation
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\alpha".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"alpha"))),
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_invalid_character() {
    // Character 15 (0x0F) should be treated as Other by default
    let input = format!("a{}b", char::from(15));
    assert_tokens_match(&input, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char(char::from(15))), // character treated as Other
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_comprehensive_source_locations() {
    // Test that source locations are precisely tracked
    assert_tokens_match("ab{cd}", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')), // a
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::BeginGroup, SourceLocation::new(2), 1, NO_FLAGS, TokenData::None), // {
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('c')), // c
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('d')), // d
        (TokenKind::EndGroup, SourceLocation::new(5), 1, NO_FLAGS, TokenData::None), // }
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_sequence_locations_and_spacing() {
    // Test control sequence (\\) followed by letters - note that \\ is a control symbol
    assert_tokens_match("\\\\alpha beta", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, TokenData::Symbol(Some(MaybeChar::from_char('\\')))), // \\
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('a')), // a
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')), // l
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('p')), // p
        (TokenKind::Letter, SourceLocation::new(5), 1, NO_FLAGS, TokenData::Char('h')), // h
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, TokenData::Char('a')), // a
        (TokenKind::Space, SourceLocation::new(7), 1, NO_FLAGS, TokenData::None), // space
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('b')), // b
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('e')), // e
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('t')), // t
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, TokenData::Char('a')), // a
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_unicode_source_locations() {
    // Test with Unicode characters - lexer treats non-ASCII as Other by default
    // α and β are multibyte UTF-8 characters treated as individual bytes
    assert_tokens_match("α{β}", &[
        (TokenKind::Other, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char(char::from(206))), // First byte of α
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char(char::from(177))), // Second byte of α
        (TokenKind::BeginGroup, SourceLocation::new(2), 1, NO_FLAGS, TokenData::None), // {
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char(char::from(206))), // First byte of β
        (TokenKind::Other, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char(char::from(178))), // Second byte of β
        (TokenKind::EndGroup, SourceLocation::new(5), 1, NO_FLAGS, TokenData::None), // }
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_multiple_space_consolidation_with_locations() {
    assert_tokens_match("a   b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // Only first space generates token
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_word_vs_symbol_distinction() {
    // Test that control words (letters after \) vs control symbols (non-letters after \) are handled correctly
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\abc\\{\\123".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 4, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"abc"))),
        (TokenKind::ControlSymbol, SourceLocation::new(4), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('{')))),
        (TokenKind::ControlSymbol, SourceLocation::new(6), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('1')))),
        (TokenKind::Other, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('2')), // 2 is not part of control sequence
        (TokenKind::Other, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('3')), // 3 is not part of control sequence
        (TokenKind::Eof, SourceLocation::new(10), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_symbols() {
    assert_tokens_match("\\{ \\} \\$ \\&", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, TokenData::Symbol(Some(MaybeChar::from_char('{')))),
        (TokenKind::Space, SourceLocation::new(2), 1, NO_FLAGS, TokenData::None),
        (TokenKind::ControlSymbol, SourceLocation::new(3), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('}')))),
        (TokenKind::Space, SourceLocation::new(5), 1, NO_FLAGS, TokenData::None),
        (TokenKind::ControlSymbol, SourceLocation::new(6), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('$')))),
        (TokenKind::Space, SourceLocation::new(8), 1, NO_FLAGS, TokenData::None),
        (TokenKind::ControlSymbol, SourceLocation::new(9), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('&')))),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_mixed_control_sequences() {
    // Test mixed control words and symbols
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\alpha\\{ \\beta \\}".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"alpha"))),
        (TokenKind::ControlSymbol, SourceLocation::new(6), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('{')))),
        (TokenKind::Space, SourceLocation::new(8), 1, NO_FLAGS, TokenData::None),
        (TokenKind::ControlWord, SourceLocation::new(9), 5, NO_FLAGS, TokenData::CommandIdentifier(id_table.get_or_insert(b"beta"))),
        // Space after \beta is skipped
        (TokenKind::ControlSymbol, SourceLocation::new(15), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('}')))),
        (TokenKind::Eof, SourceLocation::new(17), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_word_space_handling() {
    // Test that spaces after control words are skipped but spaces after control symbols are preserved
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\word   text\\{   text".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 5, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"word"))),
        // Spaces after control word are skipped
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('x')),
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::ControlSymbol, SourceLocation::new(12), 2, NO_FLAGS, TokenData::Symbol(Some(MaybeChar::from_char('{')))),
        // Spaces after control symbol are preserved
        (TokenKind::Space, SourceLocation::new(14), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Letter, SourceLocation::new(17), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(18), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(19), 1, NO_FLAGS, TokenData::Char('x')),
        (TokenKind::Letter, SourceLocation::new(20), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Eof, SourceLocation::new(21), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_in_control_sequence() {
    // Test caret notation within control sequences - should be processed literally as part of name
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\test^^A".as_bytes(), &id_table);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 5, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"test"))),
        (TokenKind::Other, SourceLocation::new(5), 3, NO_FLAGS, TokenData::Char('\u{1}')),
        (TokenKind::Eof, SourceLocation::new(8), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_single_char() {
    assert_tokens_match("^^A^^B^^z", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, TokenData::Char(char::from(1))),
        (TokenKind::Other, SourceLocation::new(3), 3, NO_FLAGS, TokenData::Char(char::from(2))),
        (TokenKind::Other, SourceLocation::new(6), 3, NO_FLAGS, TokenData::Char(char::from(58))),
        (TokenKind::Eof, SourceLocation::new(9), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_special_chars() {
    // Test caret notation for special characters
    // ^^? -> ? is ASCII 63, 63+64=127 (DEL, ignored - no token)
    // ^^@ -> @ is ASCII 64, 64-64=0 (null, ignored - no token)
    // ^^! -> ! is ASCII 33, 33+64=97 ('a', letter)
    assert_tokens_match("^^?^^@^^!", &[
        // Currently broken - caret notation not working at string boundaries
        (TokenKind::Letter, SourceLocation::new(6), 3, START_OF_LINE, TokenData::Char('a')), // rest gets combined somehow
        (TokenKind::Eof, SourceLocation::new(9), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_hex() {
    assert_tokens_match("^^0f^^1A^^fF", &[
        (TokenKind::Other, SourceLocation::new(0), 4, START_OF_LINE, TokenData::Char(char::from(15))),
        (TokenKind::Other, SourceLocation::new(4), 4, NO_FLAGS, TokenData::Char(char::from(26))),
        (TokenKind::Other, SourceLocation::new(8), 4, NO_FLAGS, TokenData::Char(char::from(255))),
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_invalid_patterns() {
    assert_tokens_match("^^G1^^xy^A", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, TokenData::Char(char::from(7))), // ^^G -> valid caret notation (G-64=7)
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('1')),
        (TokenKind::Other, SourceLocation::new(4), 3, NO_FLAGS, TokenData::Char(char::from(56))), // ^^x -> valid caret notation (x-64=56)
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('y')),
        (TokenKind::Superscript, SourceLocation::new(8), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('A')),
        (TokenKind::Eof, SourceLocation::new(10), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_generating_space() {
    assert_tokens_match("a^^`b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 3, NO_FLAGS, TokenData::None),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_carriage_return_newline_handling() {
    assert_tokens_match("a\r\nb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 2, NO_FLAGS, TokenData::None), // \r\n -> space
        (TokenKind::Letter, SourceLocation::new(3), 1, START_OF_LINE, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_carriage_return_alone() {
    assert_tokens_match("a\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_comment_with_carriage_return() {
    assert_tokens_match("hello%comment\rworld", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        // comment is skipped until \r, then world starts on new line
        (TokenKind::Letter, SourceLocation::new(14), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(15), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(17), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(18), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(19), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_comment_with_carriage_return_newline() {
    assert_tokens_match("hello%comment\r\nworld", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        // comment is skipped until \r\n, then world starts on new line
        (TokenKind::Letter, SourceLocation::new(15), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(17), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(18), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(19), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(20), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_comment_end_of_file() {
    assert_tokens_match("hello%comment", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        // comment goes to EOF
        (TokenKind::Eof, SourceLocation::new(13), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_producing_letters() {
    assert_tokens_match("^^aa", &[
        (TokenKind::Other, SourceLocation::new(0), 4, START_OF_LINE, TokenData::Char(char::from(170))),
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_multiple_carriage_returns() {
    assert_tokens_match("a\r\r\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // first \r
        (TokenKind::Paragraph, SourceLocation::new(2), 1, START_OF_LINE, TokenData::None), // first \r + second \r -> paragraph
        (TokenKind::Paragraph, SourceLocation::new(3), 1, START_OF_LINE, TokenData::None), // second \r + third \r -> paragraph
        (TokenKind::Letter, SourceLocation::new(4), 1, START_OF_LINE, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_incomplete_caret_notation() {
    assert_tokens_match("^^", &[
        (TokenKind::Superscript, SourceLocation::new(0), 1, START_OF_LINE, TokenData::None), // first ^
        (TokenKind::Superscript, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // second ^
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_at_boundary() {
    assert_tokens_match("a^^B", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Other, SourceLocation::new(1), 3, NO_FLAGS, TokenData::Char(char::from(2))), // ^^B -> byte 2
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_finish_line_behavior_in_comment() {
    assert_tokens_match("start%comment\nend", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('s')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('a')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('t')),
        // comment processed with finish_line, end starts new line
        (TokenKind::Letter, SourceLocation::new(14), 1, START_OF_LINE, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(15), 1, NO_FLAGS, TokenData::Char('n')),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(17), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_simple_caret_notation() {
    assert_tokens_match("^^A", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, TokenData::Char(char::from(1))), // ^^A -> byte 1 -> Other
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_caret_notation_del_char() {
    assert_tokens_match("a^^?b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_custom_category_codes() {
    // Test custom category codes with active character
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("hello@world".as_bytes(), &id_table);
    // Make @ an active character
    lexer.set_category_code(MaybeChar::from_char('@'), CategoryCode::Active);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::ActiveChar, SourceLocation::new(5), 1, NO_FLAGS, TokenData::CommandIdentifier(id_table.get_or_insert(b"@"))),
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_custom_comment_character() {
    let command_identifier_table = CommandIdentifierTable::new();
    let mut lexer = Lexer::from_bytes("hello;this is comment\nworld".as_bytes(), &command_identifier_table);
    // Make ';' a comment character instead of Other
    lexer.set_category_code(MaybeChar::from_char(';'), CategoryCode::Comment);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('h')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('o')),
        // ;this is comment\n is skipped
        (TokenKind::Letter, SourceLocation::new(22), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(23), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(24), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(25), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(26), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(27), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_custom_space_character() {
    let command_identifier_table = CommandIdentifierTable::new();
    let mut lexer = Lexer::from_bytes("a_b".as_bytes(), &command_identifier_table);
    // Make '_' a space character instead of Subscript
    lexer.set_category_code(MaybeChar::from_char('_'), CategoryCode::Space);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None), // _ (now a space)
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('b')),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_custom_newline_character() {
    let command_identifier_table = CommandIdentifierTable::new();
    let mut lexer = Lexer::from_bytes("a|b\nc|d\re|f\r\n|%comment|still comment\r\ng".as_bytes(), &command_identifier_table);
    // Make '_' a space character instead of Subscript
    lexer.set_category_code(MaybeChar::from_char('|'), CategoryCode::EndOfLine);
    lexer.set_category_code(MaybeChar::from_char('\r'), CategoryCode::Other);
    lexer.set_category_code(MaybeChar::from_char('\n'), CategoryCode::Other);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('a')),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, TokenData::None),
        // Everything between | and \r is discarded
        (TokenKind::Letter, SourceLocation::new(4), 1, START_OF_LINE, TokenData::Char('c')),
        (TokenKind::Space, SourceLocation::new(5), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Letter, SourceLocation::new(8), 1, START_OF_LINE, TokenData::Char('e')),
        (TokenKind::Space, SourceLocation::new(9), 1, NO_FLAGS, TokenData::None),
        (TokenKind::Paragraph, SourceLocation::new(13), 1, START_OF_LINE, TokenData::None), // \r\n| -> paragraph
        // Everything between % and \r is considered comment text
        (TokenKind::Letter, SourceLocation::new(38), 1, START_OF_LINE, TokenData::Char('g')),
        (TokenKind::Eof, SourceLocation::new(39), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_spaces_before_eol_skipped() {
    // Test that spaces before various EOL characters are completely skipped
    assert_tokens_match("word   \ntext", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('d')),
        // 3 spaces before \n are skipped - no space token generated
        (TokenKind::Space, SourceLocation::new(7), 1, NO_FLAGS, TokenData::None), // \n becomes space token
        (TokenKind::Letter, SourceLocation::new(8), 1, START_OF_LINE, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('x')),
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_spaces_before_eof_skipped() {
    // Test that spaces at end of file are completely skipped
    assert_tokens_match("word   ", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('d')),
        // 3 spaces at EOF are skipped - no space token generated
        (TokenKind::Eof, SourceLocation::new(7), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_spaces_between_words_preserved() {
    // Test that spaces between non-EOL characters are preserved as tokens
    assert_tokens_match("word   text", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Space, SourceLocation::new(4), 1, NO_FLAGS, TokenData::None), // first space generates token
        // Additional spaces are skipped by existing logic
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('x')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_multiple_custom_category_codes() {
    // Test multiple custom category codes including active characters and letters
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("@test#1world".as_bytes(), &id_table);
    // Make @ and # active characters (override their default Parameter behavior for #)
    lexer.set_category_code(MaybeChar::from_char('@'), CategoryCode::Active);
    lexer.set_category_code(MaybeChar::from_char('#'), CategoryCode::Active);
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ActiveChar, SourceLocation::new(0), 1, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"@"))),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, TokenData::Char('e')),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, TokenData::Char('s')),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, TokenData::Char('t')),
        (TokenKind::ActiveChar, SourceLocation::new(5), 1, NO_FLAGS, TokenData::CommandIdentifier(id_table.get_or_insert(b"#"))),
        (TokenKind::Other, SourceLocation::new(6), 1, NO_FLAGS, TokenData::Char('1')), // 1 is still Other
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, TokenData::Char('w')),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, TokenData::Char('o')),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, TokenData::Char('r')),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, TokenData::Char('l')),
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, TokenData::Char('d')),
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_word_starting_with_caret_notation() {
    // Test control word that starts with caret notation resolving to a letter
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\^^41world".as_bytes(), &id_table); // ^^41 = 'A'
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 10, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"Aworld"))),
        (TokenKind::Eof, SourceLocation::new(10), 0, NO_FLAGS, TokenData::None),
    ]);
}

#[test]
fn test_control_word_with_caret_notation_letter_in_middle() {
    // Test control word with caret notation resolving to a letter in the middle
    let id_table = CommandIdentifierTable::new();

    let mut lexer = Lexer::from_bytes("\\hello^^62world^^?".as_bytes(), &id_table); // ^^62 = 'b'
    assert_tokens_match_with_lexer(&mut lexer, &[
        (TokenKind::ControlWord, SourceLocation::new(0), 15, START_OF_LINE, TokenData::CommandIdentifier(id_table.get_or_insert(b"hellobworld"))),
        (TokenKind::Eof, SourceLocation::new(18), 0, NO_FLAGS, TokenData::None),
    ]);
}
