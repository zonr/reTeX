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
    table: [CategoryCode; 256],
}

impl CategoryCodeTable {
    pub fn new() -> Self {
        let mut table = [CategoryCode::Other; 256];

        // Set default category codes
        table[b'\\' as usize] = CategoryCode::Escape;
        table[b'{' as usize] = CategoryCode::BeginGroup;
        table[b'}' as usize] = CategoryCode::EndGroup;
        table[b'$' as usize] = CategoryCode::MathShift;
        table[b'&' as usize] = CategoryCode::AlignmentTab;
        table[b'\r' as usize] = CategoryCode::EndOfLine;
        table[b'\n' as usize] = CategoryCode::EndOfLine;
        table[b'#' as usize] = CategoryCode::Parameter;
        table[b'^' as usize] = CategoryCode::Superscript;
        table[b'_' as usize] = CategoryCode::Subscript;
        table[b'\0' as usize] = CategoryCode::Ignored;
        table[127] = CategoryCode::Ignored; // DEL
        table[b' ' as usize] = CategoryCode::Space;
        table[b'\t' as usize] = CategoryCode::Space;
        table[b'~' as usize] = CategoryCode::Active;
        table[b'%' as usize] = CategoryCode::Comment;

        // Set letters
        for c in b'a'..=b'z' {
            table[c as usize] = CategoryCode::Letter;
        }
        for c in b'A'..=b'Z' {
            table[c as usize] = CategoryCode::Letter;
        }

        Self { table }
    }

    pub fn get(&self, byte: u8) -> CategoryCode {
        self.table[byte as usize]
    }

    pub fn set(&mut self, byte: u8, category_code: CategoryCode) {
        self.table[byte as usize] = category_code;
    }

    pub fn is_letter(&self, byte: u8) -> bool {
        self.get(byte) == CategoryCode::Letter
    }

    pub fn is_space(&self, byte: u8) -> bool {
        self.get(byte) == CategoryCode::Space
    }

    pub fn is_ignored(&self, byte: u8) -> bool {
        self.get(byte) == CategoryCode::Ignored
    }

    pub fn is_space_or_ignored(&self, byte: u8) -> bool {
        matches!(self.get(byte), CategoryCode::Space | CategoryCode::Ignored)
    }
    pub fn is_escape(&self, byte: u8) -> bool {
        self.get(byte) == CategoryCode::Escape
    }

    pub fn is_eol(&self, byte: u8) -> bool {
        self.get(byte) == CategoryCode::EndOfLine
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
        assert_eq!(table.get(b'\\'), CategoryCode::Escape);
        assert_eq!(table.get(b'{'), CategoryCode::BeginGroup);
        assert_eq!(table.get(b'}'), CategoryCode::EndGroup);
        assert_eq!(table.get(b'$'), CategoryCode::MathShift);
        assert_eq!(table.get(b'&'), CategoryCode::AlignmentTab);
        assert_eq!(table.get(b'\r'), CategoryCode::EndOfLine);
        assert_eq!(table.get(b'\n'), CategoryCode::EndOfLine);
        assert_eq!(table.get(b'#'), CategoryCode::Parameter);
        assert_eq!(table.get(b'^'), CategoryCode::Superscript);
        assert_eq!(table.get(b'_'), CategoryCode::Subscript);
        assert_eq!(table.get(b'\0'), CategoryCode::Ignored);
        assert_eq!(table.get(127), CategoryCode::Ignored); // DEL
        assert_eq!(table.get(b' '), CategoryCode::Space);
        assert_eq!(table.get(b'\t'), CategoryCode::Space);
        assert_eq!(table.get(b'~'), CategoryCode::Active);
        assert_eq!(table.get(b'%'), CategoryCode::Comment);

        // Test letters
        assert_eq!(table.get(b'a'), CategoryCode::Letter);
        assert_eq!(table.get(b'z'), CategoryCode::Letter);
        assert_eq!(table.get(b'A'), CategoryCode::Letter);
        assert_eq!(table.get(b'Z'), CategoryCode::Letter);

        // Test other characters default to Other
        assert_eq!(table.get(b'0'), CategoryCode::Other);
        assert_eq!(table.get(b'9'), CategoryCode::Other);
        assert_eq!(table.get(b'.'), CategoryCode::Other);
        assert_eq!(table.get(b'!'), CategoryCode::Other);
    }

    #[test]
    fn test_category_code_table_set_get() {
        let mut table = CategoryCodeTable::new();

        // Change a character's category code
        assert_eq!(table.get(b'@'), CategoryCode::Other);
        table.set(b'@', CategoryCode::Letter);
        assert_eq!(table.get(b'@'), CategoryCode::Letter);
    }

    #[test]
    fn test_is_letter() {
        let table = CategoryCodeTable::new();

        assert!(table.is_letter(b'a'));
        assert!(table.is_letter(b'z'));
        assert!(table.is_letter(b'A'));
        assert!(table.is_letter(b'Z'));
        assert!(!table.is_letter(b'0'));
        assert!(!table.is_letter(b' '));
        assert!(!table.is_letter(b'\\'));
    }

    #[test]
    fn test_is_space() {
        let table = CategoryCodeTable::new();

        assert!(table.is_space(b' '));
        assert!(table.is_space(b'\t'));
        assert!(!table.is_space(b'a'));
        assert!(!table.is_space(b'\n'));
        assert!(!table.is_space(b'\0'));
    }

    #[test]
    fn test_is_space_or_ignored() {
        let table = CategoryCodeTable::new();

        assert!(table.is_space_or_ignored(b' '));
        assert!(table.is_space_or_ignored(b'\t'));
        assert!(table.is_space_or_ignored(b'\0'));
        assert!(table.is_space_or_ignored(127)); // DEL
        assert!(!table.is_space_or_ignored(b'a'));
        assert!(!table.is_space_or_ignored(b'\n'));
    }

    #[test]
    fn test_is_escape() {
        let table = CategoryCodeTable::new();

        assert!(table.is_escape(b'\\'));
        assert!(!table.is_escape(b'/'));
        assert!(!table.is_escape(b'a'));
    }

    #[test]
    fn test_default_trait() {
        let table1 = CategoryCodeTable::new();
        let table2 = CategoryCodeTable::default();

        // Both should have the same behavior
        assert_eq!(table1.get(b'\\'), table2.get(b'\\'));
        assert_eq!(table1.get(b'a'), table2.get(b'a'));
        assert_eq!(table1.get(b' '), table2.get(b' '));
    }
}