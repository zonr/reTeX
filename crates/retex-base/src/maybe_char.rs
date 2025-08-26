use core::fmt;

/// Represents data that is either a valid Unicode code point or an arbitrary byte value
///
/// Ideally we want:
/// ```rust
/// pub enum MaybeChar {
///     Char(char),
///     NonCharByte(u8),
/// }
/// ```
///
/// However, Rust (1.89.0) is unable to leverage the spare bits in [char] to make it as compact as a 4-byte integer.
///
/// Internally encodes either a Unicode scalar value or a byte with a marker at MSB.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct MaybeChar(u32);

/// User-facing “enum view” for pattern-matching ergonomics.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MaybeCharEnumView {
    Char(char),
    NonCharByte(u8),
}
impl MaybeChar {
    const NON_CHAR_BYTE_TAG: u32 = 0x1000_0000;
    const NON_CHAR_BYTE_MASK: u32 = 0xFF;

    #[inline]
    pub fn from_char(c: char) -> Self {
        MaybeChar(c as u32)
    }

    /// Creates a [MaybeChar] for a non-unicode character. Assumption has been made that non-unicode characters are
    /// always handled at byte level.
    #[inline]
    pub fn from_non_char_byte(b: u8) -> Self {
        MaybeChar(Self::NON_CHAR_BYTE_TAG | (b as u32))
    }

    #[inline]
    pub fn is_char(self) -> bool {
        (self.0 & Self::NON_CHAR_BYTE_TAG) != Self::NON_CHAR_BYTE_TAG
    }

    #[inline]
    pub fn is_non_char_byte(self) -> bool {
        (self.0 & Self::NON_CHAR_BYTE_TAG) == Self::NON_CHAR_BYTE_TAG
    }

    #[inline]
    pub fn enum_view(self) -> MaybeCharEnumView {
        if self.is_char() {
            // Safety: MaybeChar was constructed by casting char into u32
            MaybeCharEnumView::Char(unsafe { char::from_u32_unchecked(self.0) })
        } else {
            MaybeCharEnumView::NonCharByte((self.0 & Self::NON_CHAR_BYTE_MASK) as u8)
        }
    }

    /// Returns if stored a valid Unicode character
    #[inline]
    pub fn as_char(self) -> Option<char> {
        match self.enum_view() {
            MaybeCharEnumView::Char(c) => Some(c),
            _ => None,
        }
    }

    /// Encodes this character as UTF-8 into the provided byte buffer, and then returns the subslice of the buffer that
    /// contains the encoded character.
    ///
    /// For [MaybeChar::Char], delegates to [char::encode_utf8].
    /// For [MaybeChar::NonCharByte], writes the single byte and returns length 1.
    #[inline]
    pub fn encode_utf8(self, dst: &mut [u8]) -> &[u8] {
        match self.enum_view() {
            MaybeCharEnumView::Char(c) => c.encode_utf8(dst).as_bytes(),
            MaybeCharEnumView::NonCharByte(b) => {
                dst[0] = b;
                &dst[0..1]
            }
        }
    }
}

impl fmt::Debug for MaybeChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.enum_view() {
            MaybeCharEnumView::Char(c) => f.debug_tuple("Char").field(&c).finish(),
            MaybeCharEnumView::NonCharByte(b) => f.debug_tuple("NonCharByte").field(&b).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_char_from_ascii() {
        let maybe_char = MaybeChar::from_char('A');
        assert!(maybe_char.is_char());
        assert!(!maybe_char.is_non_char_byte());
        assert_eq!(maybe_char.as_char(), Some('A'));

        match maybe_char.enum_view() {
            MaybeCharEnumView::Char(c) => assert_eq!(c, 'A'),
            _ => panic!("Expected Char variant"),
        }
    }

    #[test]
    fn test_maybe_char_from_non_char_byte() {
        let maybe_char = MaybeChar::from_non_char_byte(0xFF);
        assert!(!maybe_char.is_char());
        assert!(maybe_char.is_non_char_byte());
        assert_eq!(maybe_char.as_char(), None);

        match maybe_char.enum_view() {
            MaybeCharEnumView::NonCharByte(b) => assert_eq!(b, 0xFF),
            _ => panic!("Expected NonCharByte variant"),
        }
    }

    #[test]
    fn test_maybe_char_unicode_chars() {
        // Test various Unicode characters
        let chars = ['€', '🚀', 'α', '中', '\0', '\u{10FFFF}'];

        for &c in &chars {
            let maybe_char = MaybeChar::from_char(c);
            assert!(maybe_char.is_char());
            assert!(!maybe_char.is_non_char_byte());
            assert_eq!(maybe_char.as_char(), Some(c));

            match maybe_char.enum_view() {
                MaybeCharEnumView::Char(decoded) => assert_eq!(decoded, c),
                _ => panic!("Expected Char variant for {}", c),
            }
        }
    }

    #[test]
    fn test_maybe_char_byte_values() {
        // Test all possible byte values
        for byte in 0..=255u8 {
            let maybe_char = MaybeChar::from_non_char_byte(byte);
            assert!(!maybe_char.is_char());
            assert!(maybe_char.is_non_char_byte());
            assert_eq!(maybe_char.as_char(), None);

            match maybe_char.enum_view() {
                MaybeCharEnumView::NonCharByte(b) => assert_eq!(b, byte),
                _ => panic!("Expected NonCharByte variant for byte {}", byte),
            }
        }
    }

    #[test]
    fn test_maybe_char_equality() {
        let char_a1 = MaybeChar::from_char('A');
        let char_a2 = MaybeChar::from_char('A');
        let char_b = MaybeChar::from_char('B');
        let byte_65 = MaybeChar::from_non_char_byte(65); // ASCII 'A'
        let byte_65_2 = MaybeChar::from_non_char_byte(65);
        let byte_66 = MaybeChar::from_non_char_byte(66); // ASCII 'B'

        // Test char equality
        assert_eq!(char_a1, char_a2);
        assert_ne!(char_a1, char_b);

        // Test byte equality
        assert_eq!(byte_65, byte_65_2);
        assert_ne!(byte_65, byte_66);

        // Test char vs byte inequality (even with same ASCII value)
        assert_ne!(char_a1, byte_65);
    }

    #[test]
    fn test_maybe_char_hash() {
        use std::collections::HashMap;

        let mut map = HashMap::new();

        let char_a = MaybeChar::from_char('A');
        let byte_65 = MaybeChar::from_non_char_byte(65);

        map.insert(char_a, "char A");
        map.insert(byte_65, "byte 65");

        // Both should be stored as different keys
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&char_a), Some(&"char A"));
        assert_eq!(map.get(&byte_65), Some(&"byte 65"));
    }

    #[test]
    fn test_maybe_char_debug_format() {
        let char_a = MaybeChar::from_char('A');
        let byte_65 = MaybeChar::from_non_char_byte(65);

        let char_debug = format!("{:?}", char_a);
        let byte_debug = format!("{:?}", byte_65);

        assert!(char_debug.contains("Char('A')"));
        assert!(byte_debug.contains("NonCharByte(65)"));
    }

    #[test]
    fn test_maybe_char_enum_view_equality() {
        let char_view = MaybeCharEnumView::Char('X');
        let byte_view = MaybeCharEnumView::NonCharByte(88);

        assert_eq!(char_view, MaybeCharEnumView::Char('X'));
        assert_eq!(byte_view, MaybeCharEnumView::NonCharByte(88));
        assert_ne!(char_view, byte_view);
    }

    #[test]
    fn test_maybe_char_copy_clone() {
        let original = MaybeChar::from_char('Z');
        let copied = original;
        let cloned = original.clone();

        assert_eq!(original, copied);
        assert_eq!(original, cloned);
        assert_eq!(copied, cloned);

        // Verify all have the same behavior
        assert!(original.is_char());
        assert!(copied.is_char());
        assert!(cloned.is_char());
    }

    #[test]
    fn test_maybe_char_as_char_method() {
        // Test char variant
        let char_variant = MaybeChar::from_char('🎉');
        assert_eq!(char_variant.as_char(), Some('🎉'));

        // Test byte variant
        let byte_variant = MaybeChar::from_non_char_byte(0x80);
        assert_eq!(byte_variant.as_char(), None);
    }

    #[test]
    fn test_maybe_char_boundary_values() {
        // Test boundary Unicode values
        let min_char = MaybeChar::from_char('\0');
        let max_char = MaybeChar::from_char('\u{10FFFF}');

        assert!(min_char.is_char());
        assert!(max_char.is_char());
        assert_eq!(min_char.as_char(), Some('\0'));
        assert_eq!(max_char.as_char(), Some('\u{10FFFF}'));

        // Test boundary byte values
        let min_byte = MaybeChar::from_non_char_byte(0);
        let max_byte = MaybeChar::from_non_char_byte(255);

        assert!(min_byte.is_non_char_byte());
        assert!(max_byte.is_non_char_byte());

        match min_byte.enum_view() {
            MaybeCharEnumView::NonCharByte(b) => assert_eq!(b, 0),
            _ => panic!("Expected NonCharByte"),
        }

        match max_byte.enum_view() {
            MaybeCharEnumView::NonCharByte(b) => assert_eq!(b, 255),
            _ => panic!("Expected NonCharByte"),
        }
    }

    #[test]
    fn test_maybe_char_internal_representation() {
        // Test that the internal representation is as expected
        let char_a = MaybeChar::from_char('A');
        let byte_65 = MaybeChar::from_non_char_byte(65);

        // Char 'A' should have value 65 (ASCII) without the tag
        assert_eq!(char_a.0, 65);

        // Byte 65 should have the tag bit set
        assert_eq!(byte_65.0, MaybeChar::NON_CHAR_BYTE_TAG | 65);

        // Verify the tag is properly set/unset
        assert_eq!(char_a.0 & MaybeChar::NON_CHAR_BYTE_TAG, 0);
        assert_eq!(byte_65.0 & MaybeChar::NON_CHAR_BYTE_TAG, MaybeChar::NON_CHAR_BYTE_TAG);
    }

    #[test]
    fn test_maybe_char_encode_utf8() {
        let mut buffer = [0u8; 4];

        // Test char encoding
        let char_a = MaybeChar::from_char('A');
        let encoded = char_a.encode_utf8(&mut buffer);
        assert_eq!(encoded, b"A");

        // Test multi-byte UTF-8 char
        let mut buffer2 = [0u8; 4];
        let emoji = MaybeChar::from_char('🎉');
        let encoded2 = emoji.encode_utf8(&mut buffer2);
        assert_eq!(encoded2, "🎉".as_bytes());

        // Test non-char byte encoding
        let mut buffer3 = [0u8; 4];
        let byte_255 = MaybeChar::from_non_char_byte(255);
        let encoded3 = byte_255.encode_utf8(&mut buffer3);
        assert_eq!(encoded3, &[255]);
        assert_eq!(encoded3.len(), 1);
    }
}
