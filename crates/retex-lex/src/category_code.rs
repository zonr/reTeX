use std::collections::HashMap;
use retex_base::MaybeChar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CategoryCode {
    Escape = 0,      // \
    BeginGroup = 1,  // {
    EndGroup = 2,    // }
    MathShift = 3,   // $
    AlignmentTab = 4, // &
    EndOfLine = 5,   // end of line
    Parameter = 6,   // #
    Superscript = 7, // ^
    Subscript = 8,   // _
    Ignored = 9,     // null, delete
    Space = 10,      // space, tab
    Letter = 11,     // a-z, A-Z
    Other = 12,      // all other characters
    Active = 13,     // ~
    Comment = 14,    // %
    Invalid = 15,    // ^^?
}

impl CategoryCode {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub struct CategoryCodeTable {
    table: HashMap<MaybeChar, CategoryCode>,
}

impl CategoryCodeTable {
    pub fn new() -> Self {
        let mut table = HashMap::new();

        // Set default category codes
        table.insert(MaybeChar::from_char('\\'), CategoryCode::Escape);
        table.insert(MaybeChar::from_char('{'), CategoryCode::BeginGroup);
        table.insert(MaybeChar::from_char('}'), CategoryCode::EndGroup);
        table.insert(MaybeChar::from_char('$'), CategoryCode::MathShift);
        table.insert(MaybeChar::from_char('&'), CategoryCode::AlignmentTab);
        table.insert(MaybeChar::from_char('\r'), CategoryCode::EndOfLine);
        table.insert(MaybeChar::from_char('\n'), CategoryCode::EndOfLine);
        table.insert(MaybeChar::from_char('#'), CategoryCode::Parameter);
        table.insert(MaybeChar::from_char('^'), CategoryCode::Superscript);
        table.insert(MaybeChar::from_char('_'), CategoryCode::Subscript);
        table.insert(MaybeChar::from_char('\0'), CategoryCode::Ignored);
        table.insert(MaybeChar::from_char('\u{7f}'), CategoryCode::Ignored); // DEL
        table.insert(MaybeChar::from_char(' '), CategoryCode::Space);
        table.insert(MaybeChar::from_char('\t'), CategoryCode::Space);
        table.insert(MaybeChar::from_char('~'), CategoryCode::Active);
        table.insert(MaybeChar::from_char('%'), CategoryCode::Comment);

        // Set letters
        for c in 'a'..='z' {
            table.insert(MaybeChar::from_char(c), CategoryCode::Letter);
        }
        for c in 'A'..='Z' {
            table.insert(MaybeChar::from_char(c), CategoryCode::Letter);
        }

        Self { table }
    }

    pub fn get(&self, maybe_char: MaybeChar) -> CategoryCode {
        self.table.get(&maybe_char).copied().unwrap_or(CategoryCode::Other)
    }

    pub fn set(&mut self, maybe_char: MaybeChar, category_code: CategoryCode) {
        self.table.insert(maybe_char, category_code);
    }

    pub fn is_letter(&self, maybe_char: MaybeChar) -> bool {
        self.get(maybe_char) == CategoryCode::Letter
    }

    pub fn is_space(&self, maybe_char: MaybeChar) -> bool {
        self.get(maybe_char) == CategoryCode::Space
    }

    pub fn is_ignored(&self, maybe_char: MaybeChar) -> bool {
        self.get(maybe_char) == CategoryCode::Ignored
    }

    pub fn is_space_or_ignored(&self, maybe_char: MaybeChar) -> bool {
        matches!(self.get(maybe_char), CategoryCode::Space | CategoryCode::Ignored)
    }

    pub fn is_escape(&self, maybe_char: MaybeChar) -> bool {
        self.get(maybe_char) == CategoryCode::Escape
    }

    pub fn is_eol(&self, maybe_char: MaybeChar) -> bool {
        self.get(maybe_char) == CategoryCode::EndOfLine
    }
}

impl Default for CategoryCodeTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_code_as_u8() {
        assert_eq!(CategoryCode::Escape.as_u8(), 0);
        assert_eq!(CategoryCode::BeginGroup.as_u8(), 1);
        assert_eq!(CategoryCode::EndGroup.as_u8(), 2);
        assert_eq!(CategoryCode::Invalid.as_u8(), 15);
    }

    #[test]
    fn test_category_code_table_new() {
        let table = CategoryCodeTable::new();

        // Test special characters
        assert_eq!(table.get(MaybeChar::from_char('\\')), CategoryCode::Escape);
        assert_eq!(table.get(MaybeChar::from_char('{')), CategoryCode::BeginGroup);
        assert_eq!(table.get(MaybeChar::from_char('}')), CategoryCode::EndGroup);
        assert_eq!(table.get(MaybeChar::from_char('$')), CategoryCode::MathShift);
        assert_eq!(table.get(MaybeChar::from_char('&')), CategoryCode::AlignmentTab);
        assert_eq!(table.get(MaybeChar::from_char('\r')), CategoryCode::EndOfLine);
        assert_eq!(table.get(MaybeChar::from_char('\n')), CategoryCode::EndOfLine);
        assert_eq!(table.get(MaybeChar::from_char('#')), CategoryCode::Parameter);
        assert_eq!(table.get(MaybeChar::from_char('^')), CategoryCode::Superscript);
        assert_eq!(table.get(MaybeChar::from_char('_')), CategoryCode::Subscript);
        assert_eq!(table.get(MaybeChar::from_char('\0')), CategoryCode::Ignored);
        assert_eq!(table.get(MaybeChar::from_char('\u{7f}')), CategoryCode::Ignored); // DEL
        assert_eq!(table.get(MaybeChar::from_char(' ')), CategoryCode::Space);
        assert_eq!(table.get(MaybeChar::from_char('\t')), CategoryCode::Space);
        assert_eq!(table.get(MaybeChar::from_char('~')), CategoryCode::Active);
        assert_eq!(table.get(MaybeChar::from_char('%')), CategoryCode::Comment);

        // Test letters
        assert_eq!(table.get(MaybeChar::from_char('a')), CategoryCode::Letter);
        assert_eq!(table.get(MaybeChar::from_char('z')), CategoryCode::Letter);
        assert_eq!(table.get(MaybeChar::from_char('A')), CategoryCode::Letter);
        assert_eq!(table.get(MaybeChar::from_char('Z')), CategoryCode::Letter);

        // Test other characters default to Other
        assert_eq!(table.get(MaybeChar::from_char('0')), CategoryCode::Other);
        assert_eq!(table.get(MaybeChar::from_char('9')), CategoryCode::Other);
        assert_eq!(table.get(MaybeChar::from_char('.')), CategoryCode::Other);
        assert_eq!(table.get(MaybeChar::from_char('!')), CategoryCode::Other);
    }

    #[test]
    fn test_category_code_table_set_get() {
        let mut table = CategoryCodeTable::new();

        // Change a character's category code
        assert_eq!(table.get(MaybeChar::from_char('@')), CategoryCode::Other);
        table.set(MaybeChar::from_char('@'), CategoryCode::Letter);
        assert_eq!(table.get(MaybeChar::from_char('@')), CategoryCode::Letter);
    }

    #[test]
    fn test_is_letter() {
        let table = CategoryCodeTable::new();

        assert!(table.is_letter(MaybeChar::from_char('a')));
        assert!(table.is_letter(MaybeChar::from_char('z')));
        assert!(table.is_letter(MaybeChar::from_char('A')));
        assert!(table.is_letter(MaybeChar::from_char('Z')));
        assert!(!table.is_letter(MaybeChar::from_char('0')));
        assert!(!table.is_letter(MaybeChar::from_char(' ')));
        assert!(!table.is_letter(MaybeChar::from_char('\\')));
    }

    #[test]
    fn test_is_space() {
        let table = CategoryCodeTable::new();

        assert!(table.is_space(MaybeChar::from_char(' ')));
        assert!(table.is_space(MaybeChar::from_char('\t')));
        assert!(!table.is_space(MaybeChar::from_char('a')));
        assert!(!table.is_space(MaybeChar::from_char('\n')));
        assert!(!table.is_space(MaybeChar::from_char('\0')));
    }

    #[test]
    fn test_is_space_or_ignored() {
        let table = CategoryCodeTable::new();

        assert!(table.is_space_or_ignored(MaybeChar::from_char(' ')));
        assert!(table.is_space_or_ignored(MaybeChar::from_char('\t')));
        assert!(table.is_space_or_ignored(MaybeChar::from_char('\0')));
        assert!(table.is_space_or_ignored(MaybeChar::from_char('\u{7f}'))); // DEL
        assert!(!table.is_space_or_ignored(MaybeChar::from_char('a')));
        assert!(!table.is_space_or_ignored(MaybeChar::from_char('\n')));
    }

    #[test]
    fn test_is_escape() {
        let table = CategoryCodeTable::new();

        assert!(table.is_escape(MaybeChar::from_char('\\')));
        assert!(!table.is_escape(MaybeChar::from_char('/')));
        assert!(!table.is_escape(MaybeChar::from_char('a')));
    }

    #[test]
    fn test_default_trait() {
        let table1 = CategoryCodeTable::new();
        let table2 = CategoryCodeTable::default();

        // Both should have the same behavior
        assert_eq!(table1.get(MaybeChar::from_char('\\')), table2.get(MaybeChar::from_char('\\')));
        assert_eq!(table1.get(MaybeChar::from_char('a')), table2.get(MaybeChar::from_char('a')));
        assert_eq!(table1.get(MaybeChar::from_char(' ')), table2.get(MaybeChar::from_char(' ')));
    }
}