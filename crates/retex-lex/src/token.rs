use std::num::NonZeroU8;
use retex_base::{SourceLocation, SourceRange, MaybeChar};
use crate::command_identifier::CommandIdentifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Special tokens
    Eof,
    Unknown,

    // Basic TeX tokens
    ControlWord,      // \command (letters after backslash)
    ControlSymbol,    // \{ (single non-letter after backslash)
    BeginGroup,       // {
    EndGroup,         // }
    MathShift,        // $
    AlignmentTab,     // &
    Parameter,        // #
    Superscript,      // ^
    Subscript,        // _
    Space,            // space character
    Letter,           // category code 11
    Other,            // category code 12
    ActiveChar,       // category code 13
    Paragraph,        // \par inserted for empty lines
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenFlags(u8);

impl TokenFlags {
    pub const NONE: Self = Self(0);
    pub const START_OF_LINE: Self = Self(1 << 0);

    pub fn new() -> Self {
        Self::NONE
    }

    pub fn has(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn set(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn clear(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

impl Default for TokenFlags {
    fn default() -> Self {
        Self::new()
    }
}


/// Carries data associated to a token. The actual type depends on token's [TokenKind].
#[derive(Debug, Clone)]
pub enum TokenData<'token> {
    /// No token data
    ///
    /// [TokenKind]'s associated with this data:
    /// * [TokenKind::Eof]
    /// * [TokenKind::Unknown]
    /// * [TokenKind::BeginGroup]
    /// * [TokenKind::EndGroup]
    /// * [TokenKind::MathShift]
    /// * [TokenKind::AlignmentTab]
    /// * [TokenKind::Superscript]
    /// * [TokenKind::Subscript]
    /// * [TokenKind::Space]
    /// * [TokenKind::Paragraph]
    None,

    /// A valid Unicode code point represented as a Unicode scalar value[^1]
    ///
    /// Note that invalid bytes (e.g., invalid UTF-8 sequences) will be converted to U+FFFD (replacement character) if
    /// not discarded.
    ///
    /// [TokenKind]'s associated with this data:
    /// * [TokenKind::Letter]
    /// * [TokenKind::Other]
    ///
    /// [^1]: [Unicode scalar value](https://www.unicode.org/glossary/#unicode_scalar_value)
    Char(char),

    /// Index of a [TokenKind::Parameter] token that represent a macro parameter; The value range is between 1 and 9
    /// (inclusive) according to TeX specification. It is optional to be lenient on singular parameter character without
    /// specifying any parameter index
    ParameterIndex(Option<NonZeroU8>),

    /// Symbol in a [TokenKind::ControlSymbol] token
    ///
    /// Contains `Some(MaybeChar)` for normal control symbols like `\{` or `\%`.
    /// Contains `None` for the case where an escape character `\` appears at the end of input
    /// with no following character, resulting in a control symbol with no actual symbol.
    Symbol(Option<MaybeChar>),

    /// [CommandIdentifier] of a [TokenKind::ControlWord] or [TokenKind::ActiveChar] token
    CommandIdentifier(&'token CommandIdentifier<'token>),
}

/// Represent a token output by [Lexer] and [Preprocessor]. Size is not a primary concern because the input is processed
/// as a stream of tokens and same [Token] instance for previous token is reused for reading the next token.
#[derive(Debug, Clone)]
pub struct Token<'token> {
    kind: TokenKind,
    flags: TokenFlags,
    location: SourceLocation,
    /// Number of bytes in the input that is accounted by this token
    length: u32,
    data: TokenData<'token>,
}

impl<'token> Token<'token> {

    pub fn reset(&mut self) {
        self.kind = TokenKind::Unknown;
        self.flags = TokenFlags::new();
        self.location = SourceLocation::invalid();
        self.length = 0;
        self.data = TokenData::None;
    }

    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: TokenKind) {
        self.kind = kind;
    }

    pub fn is(&self, kind: TokenKind) -> bool {
        self.kind == kind
    }

    pub fn is_not(&self, kind: TokenKind) -> bool {
        self.kind != kind
    }

    pub fn is_one_of(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.kind)
    }

    pub fn location(&self) -> SourceLocation {
        self.location
    }

    pub fn set_location(&mut self, location: SourceLocation) {
        self.location = location;
    }

    pub fn end_location(&self) -> SourceLocation {
        if self.location.is_valid() {
            SourceLocation::new(self.location.offset + self.length)
        } else {
            SourceLocation::invalid()
        }
    }

    pub fn range(&self) -> SourceRange {
        SourceRange::new(self.location(), self.end_location())
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn set_length(&mut self, length: u32) {
        self.length = length;
    }

    pub fn flags(&self) -> TokenFlags {
        self.flags
    }

    pub fn set_flag(&mut self, flag: TokenFlags) {
        self.flags.set(flag);
    }

    pub fn clear_flag(&mut self, flag: TokenFlags) {
        self.flags.clear(flag);
    }

    pub fn has_flag(&self, flag: TokenFlags) -> bool {
        self.flags.has(flag)
    }

    pub fn char(&self) -> char {
        assert!(matches!(self.kind, TokenKind::Letter | TokenKind::Other));
        match &self.data {
            TokenData::Char(ch) => *ch,
            _ => unreachable!(),
        }
    }

    pub fn parameter_index(&self) -> Option<NonZeroU8> {
        assert_eq!(self.kind, TokenKind::Parameter);
        match &self.data {
            TokenData::ParameterIndex(index) => *index,
            _ => unreachable!(),
        }
    }

    pub fn symbol(&self) -> Option<MaybeChar> {
        assert_eq!(self.kind, TokenKind::ControlSymbol);
        match &self.data {
            TokenData::Symbol(maybe_char) => *maybe_char,
            _ => unreachable!(),
        }
    }

    pub fn command_identifier(&self) -> &CommandIdentifier<'token> {
        assert!(matches!(self.kind, TokenKind::ControlWord | TokenKind::ActiveChar));
        match &self.data {
            TokenData::CommandIdentifier(id) => id,
            _ => unreachable!(),
        }
    }

    pub fn set_token_data(&mut self, data: TokenData<'token>) {
        match data {
            TokenData::None => (),
            TokenData::Char(_) => assert!(matches!(self.kind, TokenKind::Letter | TokenKind::Other)),
            TokenData::ParameterIndex(_) => assert_eq!(self.kind, TokenKind::Parameter),
            TokenData::Symbol(_) => assert_eq!(self.kind, TokenKind::ControlSymbol),
            TokenData::CommandIdentifier(_) => assert!(matches!(self.kind, TokenKind::ControlWord | TokenKind::ActiveChar)),
        }
        self.data = data;
    }

    pub fn at_start_of_line(&self) -> bool {
        self.has_flag(TokenFlags::START_OF_LINE)
    }
}

impl<'token> Default for Token<'token> {
    fn default() -> Self {
        Self {
            kind: TokenKind::Unknown,
            flags: TokenFlags::new(),
            location: SourceLocation::invalid(),
            length: 0,
            data: TokenData::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use retex_base::SourceLocation;

    #[test]
    fn test_token_flags() {
        let mut flags = TokenFlags::new();
        assert_eq!(flags, TokenFlags::NONE);
        assert!(!flags.has(TokenFlags::START_OF_LINE));

        flags.set(TokenFlags::START_OF_LINE);
        assert!(flags.has(TokenFlags::START_OF_LINE));

        flags.clear(TokenFlags::START_OF_LINE);
        assert!(!flags.has(TokenFlags::START_OF_LINE));
    }

    #[test]
    fn test_token_creation() {
        let mut token = Token::default();
        let location = SourceLocation::new(10);

        token.set_kind(TokenKind::Letter);
        token.set_location(location);
        token.set_length(5);

        assert_eq!(token.kind(), TokenKind::Letter);
        assert_eq!(token.location(), location);
        assert_eq!(token.length(), 5);
        assert_eq!(token.flags(), TokenFlags::NONE);
    }

    #[test]
    fn test_token_with_char() {
        let location = SourceLocation::new(0);
        let ch = 'h';
        let mut token = Token::default();

        token.set_kind(TokenKind::Letter);
        token.set_location(location);
        token.set_length(1);
        token.set_token_data(TokenData::Char(ch));

        assert_eq!(token.kind(), TokenKind::Letter);
        assert_eq!(token.location(), location);
        assert_eq!(token.length(), 1);
        assert_eq!(token.char(), ch);
    }

    #[test]
    fn test_token_reset() {
        let mut token = Token::default();

        token.set_kind(TokenKind::Letter);
        token.set_location(SourceLocation::new(10));
        token.set_length(5);
        token.set_flag(TokenFlags::START_OF_LINE);

        token.reset();

        assert_eq!(token.kind(), TokenKind::Unknown);
        assert_eq!(token.flags(), TokenFlags::NONE);
        assert!(!token.location().is_valid());
        assert_eq!(token.length(), 0);
    }

    #[test]
    fn test_token_is_methods() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Letter);

        assert!(token.is(TokenKind::Letter));
        assert!(!token.is(TokenKind::Other));
        assert!(token.is_not(TokenKind::Other));
        assert!(!token.is_not(TokenKind::Letter));
        assert!(token.is_one_of(&[TokenKind::Letter, TokenKind::Other]));
        assert!(!token.is_one_of(&[TokenKind::Other, TokenKind::Space]));
    }

    #[test]
    fn test_token_set_kind() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Letter);
        assert_eq!(token.kind(), TokenKind::Letter);

        token.set_kind(TokenKind::Other);
        assert_eq!(token.kind(), TokenKind::Other);
    }

    #[test]
    fn test_token_location_methods() {
        let mut token = Token::default();
        token.set_location(SourceLocation::new(10));
        token.set_length(5);

        assert_eq!(token.location(), SourceLocation::new(10));
        assert_eq!(token.end_location(), SourceLocation::new(15));

        let range = token.range();
        assert_eq!(range.start, SourceLocation::new(10));
        assert_eq!(range.end, SourceLocation::new(15));

        token.set_location(SourceLocation::new(20));
        assert_eq!(token.location(), SourceLocation::new(20));
    }

    #[test]
    fn test_token_length_methods() {
        let mut token = Token::default();
        token.set_length(5);
        assert_eq!(token.length(), 5);

        token.set_length(10);
        assert_eq!(token.length(), 10);
    }

    #[test]
    fn test_token_flag_methods() {
        let mut token = Token::default();
        assert!(!token.has_flag(TokenFlags::START_OF_LINE));

        token.set_flag(TokenFlags::START_OF_LINE);
        assert!(token.has_flag(TokenFlags::START_OF_LINE));
        assert_eq!(token.flags(), TokenFlags::START_OF_LINE);

        token.clear_flag(TokenFlags::START_OF_LINE);
        assert!(!token.has_flag(TokenFlags::START_OF_LINE));
    }

    #[test]
    fn test_token_char_methods() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Letter);

        let ch = 't';
        token.set_token_data(TokenData::Char(ch));
        assert_eq!(token.char(), ch);
    }

    #[test]
    fn test_token_parameter_methods() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Parameter);

        let index = 5;
        token.set_token_data(TokenData::ParameterIndex(NonZeroU8::new(index)));
        assert_eq!(token.parameter_index(), NonZeroU8::new(index));
    }

    #[test]
    fn test_token_at_start_of_line() {
        let mut token = Token::default();
        assert!(!token.at_start_of_line());

        token.set_flag(TokenFlags::START_OF_LINE);
        assert!(token.at_start_of_line());
    }


    #[test]
    fn test_token_default() {
        let token = Token::default();
        assert_eq!(token.kind(), TokenKind::Unknown);
        assert_eq!(token.flags(), TokenFlags::NONE);
        assert!(!token.location().is_valid());
        assert_eq!(token.length(), 0);
    }

    #[test]
    fn test_token_end_location_invalid() {
        let mut token = Token::default();
        token.set_length(5);
        assert!(!token.end_location().is_valid());
    }

    #[test]
    fn test_token_with_none() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Eof);
        token.set_token_data(TokenData::None);

        assert_eq!(token.kind(), TokenKind::Eof);
        // No accessor for None data since there's nothing to return
    }

    #[test]
    fn test_token_with_parameter_index() {
        let mut token = Token::default();
        token.set_kind(TokenKind::Parameter);
        let index = 3;
        token.set_token_data(TokenData::ParameterIndex(NonZeroU8::new(index)));

        assert_eq!(token.kind(), TokenKind::Parameter);
        assert_eq!(token.parameter_index(), NonZeroU8::new(index));
    }

    #[test]
    fn test_token_with_symbol() {
        use retex_base::MaybeCharEnumView;

        let mut token = Token::default();
        token.set_kind(TokenKind::ControlSymbol);
        let symbol = MaybeChar::from_char('{');
        token.set_token_data(TokenData::Symbol(Some(symbol)));

        assert_eq!(token.kind(), TokenKind::ControlSymbol);
        let retrieved_symbol = token.symbol();

        // Test that symbol returns Some(symbol)
        assert_eq!(Some(symbol), retrieved_symbol);

        // Test that both resolve to the same character
        let retrieved_symbol = retrieved_symbol.unwrap();
        match (symbol.enum_view(), retrieved_symbol.enum_view()) {
            (MaybeCharEnumView::Char(expected), MaybeCharEnumView::Char(actual)) => {
                assert_eq!(expected, actual);
                assert_eq!(expected, '{');
            },
            _ => panic!("Expected both to be Char variants"),
        }
    }

    #[test]
    fn test_token_with_command_identifier() {
        use crate::command_identifier::CommandIdentifierTable;

        let table = CommandIdentifierTable::new();
        let identifier = table.get_or_insert(b"hello");

        let mut token = Token::default();
        token.set_kind(TokenKind::ControlWord);
        token.set_token_data(TokenData::CommandIdentifier(identifier));

        assert_eq!(token.kind(), TokenKind::ControlWord);
        let retrieved_identifier = token.command_identifier();
        assert_eq!(retrieved_identifier.as_bytes(), b"hello");
    }
}