use retex_lex::{Lexer, Token, TokenKind, TokenFlags, Preprocessor};
use retex_base::SourceLocation;

/// Helper constants for common flag combinations
const NO_FLAGS: TokenFlags = TokenFlags::NONE;
const START_OF_LINE: TokenFlags = TokenFlags::START_OF_LINE;

fn assert_tokens_match(input: &str, expected: &[(TokenKind, SourceLocation, u32, TokenFlags, &[u8])]) {
    let preprocessor = Preprocessor::new();
    let mut lexer = Lexer::from_bytes(input.as_bytes(), &preprocessor);

    let mut token = Token::default();
    let mut actual = Vec::new();

    loop {
        lexer.lex(&mut token);
        
        // Validate token data based on type
        let expected_bytes = if let Some((_, _, _, _, expected_data)) = expected.get(actual.len()) {
            *expected_data
        } else {
            &[] // Will be handled by length mismatch error below
        };
        
        if token.kind() == TokenKind::ControlWord {
            let command_id = token.command_identifier();
            assert_eq!(command_id.as_bytes(), expected_bytes, 
                "Control word token {} has incorrect command identifier bytes", actual.len());
        } else if token.kind() != TokenKind::Eof {
            if let Some(raw_bytes) = token.raw_bytes() {
                assert_eq!(raw_bytes, expected_bytes, 
                    "Token {} has incorrect raw bytes", actual.len());
            }
        }
        
        actual.push((
            token.kind(),
            token.location(),
            token.length(),
            token.flags(),
            expected_bytes
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

#[test]
fn test_simple_text_with_data_validation() {
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
fn test_control_word_with_data_validation() {
    assert_tokens_match("\\hello", &[
        (TokenKind::ControlWord, SourceLocation::new(0), 6, START_OF_LINE, b"hello"),
        (TokenKind::Eof, SourceLocation::new(6), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_mixed_tokens_with_data_validation() {
    assert_tokens_match("a\\test{b}", &[
        (TokenKind::Letter, SourceLocation::new(0), 1, START_OF_LINE, b"a"),
        (TokenKind::ControlWord, SourceLocation::new(1), 5, NO_FLAGS, b"test"),
        (TokenKind::BeginGroup, SourceLocation::new(6), 1, NO_FLAGS, b"{"),
        (TokenKind::Letter, SourceLocation::new(7), 1, NO_FLAGS, b"b"),
        (TokenKind::EndGroup, SourceLocation::new(8), 1, NO_FLAGS, b"}"),
        (TokenKind::Eof, SourceLocation::new(9), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_control_symbols_with_data_validation() {
    assert_tokens_match("\\{ \\}", &[
        (TokenKind::ControlSymbol, SourceLocation::new(0), 2, START_OF_LINE, b"\\{"),
        (TokenKind::Space, SourceLocation::new(2), 1, NO_FLAGS, b" "),
        (TokenKind::ControlSymbol, SourceLocation::new(3), 2, NO_FLAGS, b"\\}"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}

#[test]
fn test_parameter_tokens_with_data_validation() {
    assert_tokens_match("#1 #a", &[
        (TokenKind::Parameter, SourceLocation::new(0), 2, START_OF_LINE, b"#1"),
        (TokenKind::Space, SourceLocation::new(2), 1, NO_FLAGS, b" "),
        (TokenKind::Parameter, SourceLocation::new(3), 1, NO_FLAGS, b"#"),
        (TokenKind::Letter, SourceLocation::new(4), 1, NO_FLAGS, b"a"),
        (TokenKind::Eof, SourceLocation::new(5), 0, NO_FLAGS, b""),
    ]);
}