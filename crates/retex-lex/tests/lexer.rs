use retex_lex::{Lexer, Token, TokenKind, TokenFlags, Preprocessor};
use retex_lex::category_code::CategoryCode;
use retex_base::SourceLocation;

/// Helper constants for common flag combinations
const NO_FLAGS: TokenFlags = TokenFlags::NONE;
const START_OF_LINE: TokenFlags = TokenFlags::START_OF_LINE;

fn assert_tokens_match(input: &str, expected: &[(TokenKind, SourceLocation, u32, TokenFlags, &[u8])]) {
    assert_tokens_match_with_config(input, expected, |_| {});
}



/// Helper function for testing tokens with custom lexer configuration
fn assert_tokens_match_with_config<F>(
    input: &str,
    expected: &[(TokenKind, SourceLocation, u32, TokenFlags, &[u8])],
    lexer_configurer: F
) where
    F: FnOnce(&mut Lexer)
{
    let preprocessor = Preprocessor::new();
    let mut lexer = Lexer::from_bytes(input.as_bytes(), &preprocessor);
    lexer_configurer(&mut lexer);

    let mut token = Token::default();
    let mut actual = Vec::new();

    loop {
        lexer.lex(&mut token);

        let actual_token_data = if token.kind() == TokenKind::ControlWord {
            token.command_identifier().as_bytes()
        } else {
            match token.raw_bytes() {
                Some(raw_bytes) => raw_bytes,
                None => &[],
            }
        };

        actual.push((
            token.kind(),
            token.location(),
            token.length(),
            token.flags(),
            actual_token_data
        ));

        if token.kind() == TokenKind::Eof {
            break;
        }
    }

    if actual.len() != expected.len() {
        panic!("Token count mismatch. Expected {} tokens, got {}.\nExpected: {:#?}\nActual: {:#?}",
               expected.len(), actual.len(), expected, actual);
    }

    for (i, (expected_tuple, actual_tuple)) in expected.iter().zip(actual.iter()).enumerate() {
        if expected_tuple != actual_tuple {
            panic!("Token {} mismatch:\nExpected: {:#?}\nActual: {:#?}",
                   i, expected_tuple, actual_tuple);
        }
    }
}

// Helper function removed as it's not needed for current tests

#[test]
fn test_empty_input() {
    assert_tokens_match("", &[
        (TokenKind::Eof, SourceLocation::new(0), 0, START_OF_LINE, b""),
    ]);
}

#[test]
fn test_simple_text() {
    assert_tokens_match("hello", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_mixed_characters() {
    assert_tokens_match("a1b2c", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, b"1"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"),
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, b"2"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"c"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_special_characters() {
    assert_tokens_match("{}$&^_", &[
        (TokenKind::BeginGroup, SourceLocation::new(0), 1, START_OF_LINE, b"{"),
        (TokenKind::EndGroup, SourceLocation::new(1), 1, NO_FLAGS, b"}"),
        (TokenKind::MathShift, SourceLocation::new(2), 1, NO_FLAGS, b"$"),
        (TokenKind::AlignmentTab, SourceLocation::new(3), 1, NO_FLAGS, b"&"),
        (TokenKind::Superscript, SourceLocation::new(4), 1, NO_FLAGS, b"^"),
        (TokenKind::Subscript, SourceLocation::new(5), 1, NO_FLAGS, b"_"),
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_word_with_caret_notation_in_middle() {
    assert_tokens_match("\\h^^%llo", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 8, START_OF_LINE, b"hello"),
        (TokenKind::Eof, SourceLocation::new(8), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_symbol() {
    assert_tokens_match("\\{  ", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, b"\\{"), // \{
        // Spaces at EOF are skipped - no space token generated
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_symbol_eof() {
    assert_tokens_match("\\", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 1, START_OF_LINE, b"\\"), // \{
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_space() {
    assert_tokens_match("\\  ", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, b"\\ "),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""), // space after control space is skipped
    ]);
}

#[test]
fn test_control_sequence_with_text() {
    assert_tokens_match("\\hello world", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, b"hello"), // \hello
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"w"), // w (space after control word is skipped)
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, b"o"), // o
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"r"), // r
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, b"l"), // l
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, b"d"), // d
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_spaces() {
    assert_tokens_match("a b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b" "), // space
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_multiple_spaces() {
    assert_tokens_match("a   b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b" "), // first space (others skipped)
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_tabs_treated_as_spaces() {
    assert_tokens_match("a\tb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"\t"), // tab
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_parameter_token() {
    assert_tokens_match("#1", &[
        (TokenKind::Parameter, SourceLocation::new(0), 2, START_OF_LINE, b"#1"), // #1
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_parameter_token_with_non_digit() {
    assert_tokens_match("#a", &[
        (TokenKind::Parameter, SourceLocation::new(0), 1, START_OF_LINE, b"#"), // #
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"a"), // a
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_parameter_token_without_digit() {
    assert_tokens_match("#", &[
        (TokenKind::Parameter, SourceLocation::new(0), 1, START_OF_LINE, b"#"), // #
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_active_character() {
    assert_tokens_match("~", &[
        (TokenKind::ActiveChar, SourceLocation::new(0), 1, START_OF_LINE, b"~"), // ~
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_comment() {
    assert_tokens_match("hello%comment\n  ^^?world", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),  // h
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"), // e
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"), // l
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"), // l
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"), // o
        // comment is skipped along with spaces and ignored characters on the next line
        (TokenKind::Letter, SourceLocation::new(19), 1, START_OF_LINE, b"w"), // w
        (TokenKind::Letter, SourceLocation::new(20), 1, NO_FLAGS, b"o"), // o
        (TokenKind::Letter, SourceLocation::new(21), 1, NO_FLAGS, b"r"), // r
        (TokenKind::Letter, SourceLocation::new(22), 1, NO_FLAGS, b"l"), // l
        (TokenKind::Letter, SourceLocation::new(23), 1, NO_FLAGS, b"d"), // d
        (TokenKind::Eof, SourceLocation::new(24), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_newline_handling() {
    assert_tokens_match("a\nb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"\n"), // newline becomes space
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_paragraph_break() {
    assert_tokens_match("\n", &[
        (TokenKind::Paragraph, SourceLocation::new(0), 1, START_OF_LINE, b"\n"),
        (TokenKind::Eof, SourceLocation::new(1), 0, START_OF_LINE, b""),
    ]);
}

#[test]
fn test_start_of_line_flag() {
    assert_tokens_match("a", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Eof, SourceLocation::new(1), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_ignored_characters() {
    // DEL character (127) should be ignored
    let input = format!("a{}b", char::from(127));
    assert_tokens_match(&input, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        // DEL is ignored.
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"), // b (length includes ignored char)
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_invalid_characters() {
    // DEL character (127) should be ignored
    assert_tokens_match_with_config("a|b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::InvalidChar, SourceLocation::new(1), 1, NO_FLAGS, b"|"), // |
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ], |lexer| {
        lexer.set_category_code(b'|', CategoryCode::Invalid);
    });
}

#[test]
fn test_lexer_from_bytes() {
    // Just test that it creates successfully with assert_tokens_match
    // The actual lexing behavior should be the same as from_str
    assert_tokens_match("hello", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}


#[test]
fn test_carriage_return_handling() {
    assert_tokens_match("a\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"\r"), // \r becomes space
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_token_text_preservation() {
    assert_tokens_match("\\test{abc}", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 5, START_OF_LINE, b"test"),
        (TokenKind::BeginGroup, SourceLocation::new(5), 1, NO_FLAGS, b"{"),
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, b"a"), // a
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, b"c"), // c
        (TokenKind::EndGroup, SourceLocation::new(9), 1, NO_FLAGS, b"}"),
        (TokenKind::Eof, SourceLocation::new(10), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_invalid_character() {
    // Character 15 (0x0F) should be treated as Other by default
    let input = format!("a{}b", char::from(15));
    assert_tokens_match(&input, &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, &[15]), // character treated as Other
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_comprehensive_source_locations() {
    // Test that source locations are precisely tracked
    assert_tokens_match("ab{cd}", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"), // a
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"b"), // b
        (TokenKind::BeginGroup, SourceLocation::new(2), 1, NO_FLAGS, b"{"), // {
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"c"), // c
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"d"), // d
        (TokenKind::EndGroup, SourceLocation::new(5), 1, NO_FLAGS, b"}"), // }
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_sequence_locations_and_spacing() {
    // Test control sequence (\\) followed by letters - note that \\ is a control symbol
    assert_tokens_match("\\\\alpha beta", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, b"\\\\"), // \\
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"a"), // a
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"), // l
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"p"), // p
        (TokenKind::Letter, SourceLocation::new(5), 1, NO_FLAGS, b"h"), // h
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, b"a"), // a
        (TokenKind::Space, SourceLocation::new(7), 1, NO_FLAGS, b" "), // space
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, b"b"), // b
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"e"), // e
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, b"t"), // t
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, b"a"), // a
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_unicode_source_locations() {
    // Test with Unicode characters - lexer treats non-ASCII as Other by default
    // α and β are multibyte UTF-8 characters treated as individual bytes
    assert_tokens_match("α{β}", &[
        (TokenKind::Other, SourceLocation::new(0), 1, START_OF_LINE, &[206]), // First byte of α
        (TokenKind::Other, SourceLocation::new(1), 1, NO_FLAGS, &[177]), // Second byte of α
        (TokenKind::BeginGroup, SourceLocation::new(2), 1, NO_FLAGS, b"{"), // {
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, &[206]), // First byte of β
        (TokenKind::Other, SourceLocation::new(4), 1, NO_FLAGS, &[178]), // Second byte of β
        (TokenKind::EndGroup, SourceLocation::new(5), 1, NO_FLAGS, b"}"), // }
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_multiple_space_consolidation_with_locations() {
    assert_tokens_match("a   b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b" "), // Only first space generates token
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"b"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_word_vs_symbol_distinction() {
    assert_tokens_match("\\alpha \\beta", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, b"alpha"),
        (TokenKind::ControlWord, SourceLocation::new(7), 5, NO_FLAGS, b"beta"), // space after control word is skipped
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_symbols() {
    assert_tokens_match("\\{ \\} \\$ \\&", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, b"\\{"),
        (TokenKind::Space, SourceLocation::new(2), 1, NO_FLAGS, b" "),
        (TokenKind::ControlSymbol, SourceLocation::new(3), 2, NO_FLAGS, b"\\}"),
        (TokenKind::Space, SourceLocation::new(5), 1, NO_FLAGS, b" "),
        (TokenKind::ControlSymbol, SourceLocation::new(6), 2, NO_FLAGS, b"\\$"),
        (TokenKind::Space, SourceLocation::new(8), 1, NO_FLAGS, b" "),
        (TokenKind::ControlSymbol, SourceLocation::new(9), 2, NO_FLAGS, b"\\&"),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_mixed_control_sequences() {
    assert_tokens_match("\\alpha\\{\\beta\\}", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, b"alpha"),
        (TokenKind::ControlSymbol, SourceLocation::new(6), 2, NO_FLAGS, b"\\{"),
        (TokenKind::ControlWord, SourceLocation::new(8), 5, NO_FLAGS, b"beta"),
        (TokenKind::ControlSymbol, SourceLocation::new(13), 2, NO_FLAGS, b"\\}"),
        (TokenKind::Eof, SourceLocation::new(15), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_word_space_handling() {
    // Control words consume trailing spaces, control symbols don't
    assert_tokens_match("\\hello   x\\&  y", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, b"hello"),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"x"), // spaces after control word skipped
        (TokenKind::ControlSymbol, SourceLocation::new(10), 2, NO_FLAGS, b"\\&"),
        (TokenKind::Space, SourceLocation::new(12), 1, NO_FLAGS, b" "), // control symbol doesn't consume spaces
        (TokenKind::Letter, SourceLocation::new(14), 1, NO_FLAGS, b"y"), // y
        (TokenKind::Eof, SourceLocation::new(15), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_in_control_sequence() {
    // Control words consume trailing spaces, control symbols don't
    assert_tokens_match("\\^^68^^65^^6C^^6c^^6f^^`^^5c^^38", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 21, START_OF_LINE, b"hello"),
        // (spaces after control word skipped)
        (TokenKind::ControlSymbol, SourceLocation::new(24), 8, NO_FLAGS, b"^^5c^^38"),
        (TokenKind::Eof, SourceLocation::new(32), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_single_char() {
    assert_tokens_match("^^A^^B^^z", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, &[1]),
        (TokenKind::Other, SourceLocation::new(3), 3, NO_FLAGS, &[2]),
        (TokenKind::Other, SourceLocation::new(6), 3, NO_FLAGS, &[58]),
        (TokenKind::Eof, SourceLocation::new(9), 0, NO_FLAGS, b""),
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
        (TokenKind::Letter, SourceLocation::new(6), 3, START_OF_LINE, b"a"), // rest gets combined somehow
        (TokenKind::Eof, SourceLocation::new(9), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_hex() {
    assert_tokens_match("^^0f^^1A^^fF", &[
        (TokenKind::Other, SourceLocation::new(0), 4, START_OF_LINE, &[15]),
        (TokenKind::Other, SourceLocation::new(4), 4, NO_FLAGS, &[26]),
        (TokenKind::Other, SourceLocation::new(8), 4, NO_FLAGS, &[255]),
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_invalid_patterns() {
    assert_tokens_match("^^G1^^xy^A", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, &[7]), // ^^G -> valid caret notation (G-64=7)
        (TokenKind::Other, SourceLocation::new(3), 1, NO_FLAGS, b"1"),
        (TokenKind::Other, SourceLocation::new(4), 3, NO_FLAGS, &[56]), // ^^x -> valid caret notation (x-64=56)
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"y"),
        (TokenKind::Superscript, SourceLocation::new(8), 1, NO_FLAGS, b"^"),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"A"),
        (TokenKind::Eof, SourceLocation::new(10), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_generating_space() {
    assert_tokens_match("a^^`b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 3, NO_FLAGS, b" "),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"b"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_carriage_return_newline_handling() {
    assert_tokens_match("a\r\nb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 2, NO_FLAGS, b"\r"), // \r\n -> space
        (TokenKind::Letter, SourceLocation::new(3), 1, START_OF_LINE, b"b"),
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_carriage_return_alone() {
    assert_tokens_match("a\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"\r"),
        (TokenKind::Letter, SourceLocation::new(2), 1, START_OF_LINE, b"b"),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_comment_with_carriage_return() {
    assert_tokens_match("hello%comment\rworld", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        // comment is skipped until \r, then world starts on new line
        (TokenKind::Letter, SourceLocation::new(14), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(15), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(17), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(18), 1, NO_FLAGS, b"d"),
        (TokenKind::Eof, SourceLocation::new(19), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_comment_with_carriage_return_newline() {
    assert_tokens_match("hello%comment\r\nworld", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        // comment is skipped until \r\n, then world starts on new line
        (TokenKind::Letter, SourceLocation::new(15), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(17), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(18), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(19), 1, NO_FLAGS, b"d"),
        (TokenKind::Eof, SourceLocation::new(20), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_comment_end_of_file() {
    assert_tokens_match("hello%comment", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        // comment goes to EOF
        (TokenKind::Eof, SourceLocation::new(13), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_producing_letters() {
    assert_tokens_match("^^aa", &[
        (TokenKind::Other, SourceLocation::new(0), 4, START_OF_LINE, &[170]),
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_multiple_carriage_returns() {
    assert_tokens_match("a\r\r\rb", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"\r"), // first \r
        (TokenKind::Paragraph, SourceLocation::new(2), 1, START_OF_LINE, b"\r"), // first \r + second \r -> paragraph
        (TokenKind::Paragraph, SourceLocation::new(3), 1, START_OF_LINE, b"\r"), // second \r + third \r -> paragraph
        (TokenKind::Letter, SourceLocation::new(4), 1, START_OF_LINE, b"b"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_incomplete_caret_notation() {
    assert_tokens_match("^^", &[
        (TokenKind::Superscript, SourceLocation::new(0), 1, START_OF_LINE, b"^"), // first ^
        (TokenKind::Superscript, SourceLocation::new(1), 1, NO_FLAGS, b"^"), // second ^
        (TokenKind::Eof, SourceLocation::new(2), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_at_boundary() {
    assert_tokens_match("a^^B", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Other, SourceLocation::new(1), 3, NO_FLAGS, &[2]), // ^^B -> byte 2
        (TokenKind::Eof, SourceLocation::new(4), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_finish_line_behavior_in_comment() {
    assert_tokens_match("start%comment\nend", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"s"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"t"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"a"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"t"),
        // comment processed with finish_line, end starts new line
        (TokenKind::Letter, SourceLocation::new(14), 1, START_OF_LINE, b"e"),
        (TokenKind::Letter, SourceLocation::new(15), 1, NO_FLAGS, b"n"),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, b"d"),
        (TokenKind::Eof, SourceLocation::new(17), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_simple_caret_notation() {
    assert_tokens_match("^^A", &[
        (TokenKind::Other, SourceLocation::new(0), 3, START_OF_LINE, &[1]), // ^^A -> byte 1 -> Other
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_caret_notation_del_char() {
    assert_tokens_match("a^^?b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"b"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_custom_category_codes() {
    assert_tokens_match_with_config("hello@world!test", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(5), 1, NO_FLAGS, b"@"),
        (TokenKind::Letter, SourceLocation::new(6), 1, NO_FLAGS, b"w"),
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, b"d"),
        (TokenKind::ControlWord, SourceLocation::new(11), 5, NO_FLAGS, b"test"), // !test (! is escape, followed by letters)
        (TokenKind::Eof, SourceLocation::new(16), 0, NO_FLAGS, b""),
    ], |lexer| {
        // Make '@' a letter instead of Other
        lexer.set_category_code(b'@', CategoryCode::Letter);
        // Make '!' an escape character instead of Other
        lexer.set_category_code(b'!', CategoryCode::Escape);
    });
}

#[test]
fn test_custom_comment_character() {
    assert_tokens_match_with_config("hello;this is comment\nworld", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"h"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"o"),
        // ;this is comment\n is skipped
        (TokenKind::Letter, SourceLocation::new(22), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(23), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(24), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(25), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(26), 1, NO_FLAGS, b"d"),
        (TokenKind::Eof, SourceLocation::new(27), 0, NO_FLAGS, b""),
    ], |lexer| {
        // Make ';' a comment character instead of Other
        lexer.set_category_code(b';', CategoryCode::Comment);
    });
}

#[test]
fn test_custom_space_character() {
    assert_tokens_match_with_config("a_b", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"_"), // _ (now a space)
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"b"),
        (TokenKind::Eof, SourceLocation::new(3), 0, NO_FLAGS, b""),
    ], |lexer| {
        // Make '_' a space character instead of Subscript
        lexer.set_category_code(b'_', CategoryCode::Space);
    });
}

#[test]
fn test_custom_newline_character() {
    assert_tokens_match_with_config("a|b\nc|d\re|f\r\n|%comment|still comment\r\ng", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::Space, SourceLocation::new(1), 1, NO_FLAGS, b"|"),
        // Everything between | and \r is discarded
        (TokenKind::Letter, SourceLocation::new(4), 1, START_OF_LINE, b"c"),
        (TokenKind::Space, SourceLocation::new(5), 1, NO_FLAGS, b"|"),
        (TokenKind::Letter, SourceLocation::new(8), 1, START_OF_LINE, b"e"),
        (TokenKind::Space, SourceLocation::new(9), 1, NO_FLAGS, b"|"),
        (TokenKind::Paragraph, SourceLocation::new(13), 1, START_OF_LINE, b"|"), // \r\n| -> paragraph
        // Everything between % and \r is considered comment text
        (TokenKind::Letter, SourceLocation::new(38), 1, START_OF_LINE, b"g"),
        (TokenKind::Eof, SourceLocation::new(39), 0, NO_FLAGS, b""),
    ], |lexer| {
        // Make '_' a space character instead of Subscript
        lexer.set_category_code(b'|', CategoryCode::EndOfLine);
        lexer.set_category_code(b'\r', CategoryCode::Other);
        lexer.set_category_code(b'\n', CategoryCode::Other);
    });
}

#[test]
fn test_spaces_before_eol_skipped() {
    // Test that spaces before various EOL characters are completely skipped
    assert_tokens_match("word   \ntext", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"d"),
        // 3 spaces before \n are skipped - no space token generated
        (TokenKind::Space, SourceLocation::new(7), 1, NO_FLAGS, b"\n"), // \n becomes space token
        (TokenKind::Letter, SourceLocation::new(8), 1, START_OF_LINE, b"t"),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, b"x"),
        (TokenKind::Letter, SourceLocation::new(11), 1, NO_FLAGS, b"t"),
        (TokenKind::Eof, SourceLocation::new(12), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_spaces_before_eof_skipped() {
    // Test that spaces at end of file are completely skipped
    assert_tokens_match("word   ", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"d"),
        // 3 spaces at EOF are skipped - no space token generated
        (TokenKind::Eof, SourceLocation::new(7), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_spaces_between_words_preserved() {
    // Test that spaces between non-EOL characters are preserved as tokens
    assert_tokens_match("word   text", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"w"),
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"o"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"r"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"d"),
        (TokenKind::Space, SourceLocation::new(4), 1, NO_FLAGS, b" "), // first space generates token
        // Additional spaces are skipped by existing logic
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"t"),
        (TokenKind::Letter, SourceLocation::new(8), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(9), 1, NO_FLAGS, b"x"),
        (TokenKind::Letter, SourceLocation::new(10), 1, NO_FLAGS, b"t"),
        (TokenKind::Eof, SourceLocation::new(11), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_multiple_custom_category_codes() {
    // Test with multiple custom category code changes
    assert_tokens_match_with_config("@hello!world&test;comment\nend", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"@"), // @ (now a letter)
        (TokenKind::Letter, SourceLocation::new(1), 1, NO_FLAGS, b"h"),
        (TokenKind::Letter, SourceLocation::new(2), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(3), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"l"),
        (TokenKind::Letter, SourceLocation::new(5), 1, NO_FLAGS, b"o"),
        (TokenKind::ControlWord, SourceLocation::new(6), 6, NO_FLAGS, b"world"), // !world (! is escape)
        (TokenKind::BeginGroup, SourceLocation::new(12), 1, NO_FLAGS, b"&"), // & (now begin group)
        (TokenKind::Letter, SourceLocation::new(13), 1, NO_FLAGS, b"t"),
        (TokenKind::Letter, SourceLocation::new(14), 1, NO_FLAGS, b"e"),
        (TokenKind::Letter, SourceLocation::new(15), 1, NO_FLAGS, b"s"),
        (TokenKind::Letter, SourceLocation::new(16), 1, NO_FLAGS, b"t"),
        // ;comment\n is skipped (; is now comment character)
        (TokenKind::Letter, SourceLocation::new(26), 1, START_OF_LINE, b"e"),
        (TokenKind::Letter, SourceLocation::new(27), 1, NO_FLAGS, b"n"),
        (TokenKind::Letter, SourceLocation::new(28), 1, NO_FLAGS, b"d"),
        (TokenKind::Eof, SourceLocation::new(29), 0, NO_FLAGS, b""),
    ], |lexer| {
        // Make '@' a letter
        lexer.set_category_code(b'@', CategoryCode::Letter);
        // Make '!' an escape character
        lexer.set_category_code(b'!', CategoryCode::Escape);
        // Make '&' a begin group character
        lexer.set_category_code(b'&', CategoryCode::BeginGroup);
        // Make ';' a comment character
        lexer.set_category_code(b';', CategoryCode::Comment);
    });
}
