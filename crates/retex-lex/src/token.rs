use retex_base::{SourceLocation, SourceRange};
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
    InvalidChar,      // category code 15
    Paragraph,
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


#[derive(Debug, Clone)]
pub enum TokenData<'token> {
    /// Raw bytes from the input (used for most token types)
    RawBytes(Option<&'token [u8]>),
    /// Command identifier (used for ControlWord tokens after processing caret notation)
    CommandIdentifier(&'token CommandIdentifier<'token>),
}

#[derive(Debug, Clone)]
pub struct Token<'token> {
    kind: TokenKind,
    flags: TokenFlags,
    location: SourceLocation,
    /// Number of bytes in the input that produces this token
    length: u32,
    data: TokenData<'token>,
}

impl<'token> Token<'token> {
    pub fn new(kind: TokenKind, location: SourceLocation, length: u32) -> Self {
        Self {
            kind,
            flags: TokenFlags::new(),
            location,
            length,
            data: TokenData::RawBytes(None),
        }
    }


    pub fn start_token(&mut self) {
        self.kind = TokenKind::Unknown;
        self.flags = TokenFlags::new();
        self.location = SourceLocation::invalid();
        self.length = 0;
        self.data = TokenData::RawBytes(None);
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

    pub fn raw_bytes(&self) -> Option<&'token [u8]> {
        assert_ne!(self.kind, TokenKind::ControlWord);
        match &self.data {
            TokenData::RawBytes(bytes) => *bytes,
            TokenData::CommandIdentifier(_) => unreachable!(),
        }
    }

    pub fn command_identifier(&self) -> &CommandIdentifier<'token> {
        assert_eq!(self.kind, TokenKind::ControlWord);
        match &self.data {
            TokenData::CommandIdentifier(id) => id,
            TokenData::RawBytes(_) => unreachable!(),
        }
    }

    pub fn set_raw_bytes(&mut self, bytes: &'token [u8]) {
        assert_ne!(self.kind, TokenKind::ControlWord);
        self.data = TokenData::RawBytes(Some(bytes));
    }

    pub fn set_command_identifier(&mut self, identifier: &'token CommandIdentifier<'token>) {
        assert_eq!(self.kind, TokenKind::ControlWord);
        self.data = TokenData::CommandIdentifier(identifier);
    }

    pub fn at_start_of_line(&self) -> bool {
        self.has_flag(TokenFlags::START_OF_LINE)
    }

    pub fn is_identifier(&self) -> bool {
        matches!(self.kind, TokenKind::ControlWord | TokenKind::ControlSymbol | TokenKind::Letter)
    }
}

impl<'token> Default for Token<'token> {
    fn default() -> Self {
        Self {
            kind: TokenKind::Unknown,
            flags: TokenFlags::new(),
            location: SourceLocation::invalid(),
            length: 0,
            data: TokenData::RawBytes(None),
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
    fn test_token_flags_default() {
        let flags = TokenFlags::default();
        assert_eq!(flags, TokenFlags::NONE);
    }

    #[test]
    fn test_token_new() {
        let location = SourceLocation::new(10);
        let token = Token::new(TokenKind::Letter, location, 5);

        assert_eq!(token.kind(), TokenKind::Letter);
        assert_eq!(token.location(), location);
        assert_eq!(token.length(), 5);
        assert_eq!(token.flags(), TokenFlags::NONE);
        assert!(token.raw_bytes().is_none());
    }

    #[test]
    fn test_token_with_text() {
        let location = SourceLocation::new(0);
        let text = b"hello";
        let mut token = Token::new(TokenKind::Letter, location, 5);
        token.set_raw_bytes(text);

        assert_eq!(token.kind(), TokenKind::Letter);
        assert_eq!(token.location(), location);
        assert_eq!(token.length(), 5);
        assert_eq!(token.raw_bytes(), Some(text.as_slice()));
    }

    #[test]
    fn test_token_start_token() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(10), 5);
        token.set_flag(TokenFlags::START_OF_LINE);

        token.start_token();

        assert_eq!(token.kind(), TokenKind::Unknown);
        assert_eq!(token.flags(), TokenFlags::NONE);
        assert!(!token.location().is_valid());
        assert_eq!(token.length(), 0);
        assert!(token.raw_bytes().is_none());
    }

    #[test]
    fn test_token_is_methods() {
        let token = Token::new(TokenKind::Letter, SourceLocation::new(0), 1);

        assert!(token.is(TokenKind::Letter));
        assert!(!token.is(TokenKind::Other));
        assert!(token.is_not(TokenKind::Other));
        assert!(!token.is_not(TokenKind::Letter));
        assert!(token.is_one_of(&[TokenKind::Letter, TokenKind::Other]));
        assert!(!token.is_one_of(&[TokenKind::Other, TokenKind::Space]));
    }

    #[test]
    fn test_token_set_kind() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 1);
        assert_eq!(token.kind(), TokenKind::Letter);

        token.set_kind(TokenKind::Other);
        assert_eq!(token.kind(), TokenKind::Other);
    }

    #[test]
    fn test_token_location_methods() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(10), 5);
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
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 5);
        assert_eq!(token.length(), 5);

        token.set_length(10);
        assert_eq!(token.length(), 10);
    }

    #[test]
    fn test_token_flag_methods() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 1);
        assert!(!token.has_flag(TokenFlags::START_OF_LINE));

        token.set_flag(TokenFlags::START_OF_LINE);
        assert!(token.has_flag(TokenFlags::START_OF_LINE));
        assert_eq!(token.flags(), TokenFlags::START_OF_LINE);

        token.clear_flag(TokenFlags::START_OF_LINE);
        assert!(!token.has_flag(TokenFlags::START_OF_LINE));
    }

    #[test]
    fn test_token_text_methods() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 0);
        assert!(token.raw_bytes().is_none());

        let text = b"test";
        token.set_raw_bytes(text);
        assert_eq!(token.raw_bytes(), Some(text.as_slice()));
    }

    #[test]
    fn test_token_text_as_str_invalid_utf8() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 0);
        let invalid_utf8 = &[0xFF, 0xFE];
        token.set_raw_bytes(invalid_utf8);

        assert_eq!(token.raw_bytes(), Some(invalid_utf8.as_slice()));
    }

    #[test]
    fn test_token_at_start_of_line() {
        let mut token = Token::new(TokenKind::Letter, SourceLocation::new(0), 1);
        assert!(!token.at_start_of_line());

        token.set_flag(TokenFlags::START_OF_LINE);
        assert!(token.at_start_of_line());
    }


    #[test]
    fn test_token_is_identifier() {
        let letter_token = Token::new(TokenKind::Letter, SourceLocation::new(0), 1);
        assert!(letter_token.is_identifier());

        let control_word_token = Token::new(TokenKind::ControlWord, SourceLocation::new(0), 1);
        assert!(control_word_token.is_identifier());

        let control_symbol_token = Token::new(TokenKind::ControlSymbol, SourceLocation::new(0), 1);
        assert!(control_symbol_token.is_identifier());

        let other_token = Token::new(TokenKind::Other, SourceLocation::new(0), 1);
        assert!(!other_token.is_identifier());
    }





    #[test]
    fn test_token_default() {
        let token = Token::default();
        assert_eq!(token.kind(), TokenKind::Unknown);
        assert_eq!(token.flags(), TokenFlags::NONE);
        assert!(!token.location().is_valid());
        assert_eq!(token.length(), 0);
        assert!(token.raw_bytes().is_none());
    }

    #[test]
    fn test_token_end_location_invalid() {
        let mut token = Token::default();
        token.set_length(5);
        assert!(!token.end_location().is_valid());
    }
}