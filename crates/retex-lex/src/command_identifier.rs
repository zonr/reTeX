use std::collections::HashMap;
use std::cell::RefCell;
use std::string::FromUtf8Error;

/// Identifies a command in the document. A command in TeX cannot be typeset directly. It influences typesetting
/// indirectly by carrying out assignment of a value to an internal states or produces material that can be typeset.
/// There are three type of commands:
///
/// * Primitives: built-in commands in TeX; Most of them are non-expandable (e.g., `\let`) but some of them are (e.g.,
///   `\jobname`)
/// * Macros; user-defined commands; They are expandable unless expansion is inhibited (e.g., via `\noexpand`)
/// * Conditionals: a primitive (e.g., `\if`) or a macro (e.g., define via `\newif`) that branches
///
/// Note that command identifier might not be a valid UTF-8 characters hence `&[u8]` is used as underlying container.
///
/// All data has the same lifetime as its containing [CommandIdentifierTable] (`'idtable`).
#[derive(Debug)]
pub struct CommandIdentifier<'idtable> {
    bytes: &'idtable [u8],
}

impl <'idtable> CommandIdentifier<'idtable> {
    pub fn new(bytes: &'idtable [u8]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &'idtable [u8] {
        self.bytes
    }

    pub fn as_utf8(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.bytes.to_vec())
    }
}

impl<'idtable> PartialEq for CommandIdentifier<'idtable> {
    fn eq(&self, other: &Self) -> bool {
        // Since CommandIdentifiers with the same content always reference the same instance,
        // we can compare by pointer address for optimal performance
        std::ptr::eq(self as *const Self, other as *const Self)
    }
}

impl<'idtable> Eq for CommandIdentifier<'idtable> {}

impl<'idtable> std::hash::Hash for CommandIdentifier<'idtable> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the pointer address since identical content always has the same reference
        (self as *const Self).hash(state);
    }
}

/// A table for managing command identifiers; This provides a consistent value for mapping command identifier to a value
/// (e.g., macro definition.)
pub struct CommandIdentifierTable<'idtable> {
    arena: bumpalo::Bump,
    table: RefCell<HashMap<&'idtable [u8], &'idtable CommandIdentifier<'idtable>>>,
}

impl <'idtable> CommandIdentifierTable<'idtable> {
    pub fn new() -> Self {
        Self {
            arena: bumpalo::Bump::new(),
            table: RefCell::new(HashMap::new()),
        }
    }

    /// Get a command identifier by name, or insert a new one if it doesn't exist
    pub fn get_or_insert(&'idtable self, name_bytes: &[u8]) -> &'idtable CommandIdentifier<'idtable> {
        if let Some(command_identifier) = self.table.borrow().get(name_bytes) {
            return command_identifier;
        }

        // Allocate the name string in the arena first to get a stable reference
        let stable_identifier = self.arena.alloc_slice_copy(name_bytes);

        // Create the CommandIdentifier with the stable name reference
        let command_identifier = self.arena.alloc(CommandIdentifier::new(stable_identifier));

        // Insert into the table using the stable name as key
        self.table.borrow_mut().insert(stable_identifier, command_identifier);

        command_identifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_command_identifier_as_utf8_valid() {
        let bytes = b"hello";
        let identifier = CommandIdentifier::new(bytes);

        let result = identifier.as_utf8();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_command_identifier_as_utf8_invalid() {
        let bytes = &[0xFF, 0xFE]; // Invalid UTF-8
        let identifier = CommandIdentifier::new(bytes);

        let result = identifier.as_utf8();
        assert!(result.is_err());
    }

    #[test]
    fn test_command_identifier_equality() {
        let table = CommandIdentifierTable::new();

        // Get identifiers from table - same content should return same reference
        let id1 = table.get_or_insert(b"hello");
        let id2 = table.get_or_insert(b"hello"); // Same content
        let id3 = table.get_or_insert(b"world"); // Different content

        // Same content should be equal (same reference)
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Verify they are actually the same reference
        assert!(std::ptr::eq(id1, id2));
        assert!(!std::ptr::eq(id1, id3));
    }

    #[test]
    fn test_command_identifier_hash() {
        let table = CommandIdentifierTable::new();

        let id1 = table.get_or_insert(b"hello");
        let id2 = table.get_or_insert(b"world");

        let mut map = HashMap::new();
        map.insert(id1, "value1");
        map.insert(id2, "value2");

        assert_eq!(map.get(&id1), Some(&"value1"));
        assert_eq!(map.get(&id2), Some(&"value2"));

        // Test that duplicate content uses same hash (same reference)
        let id1_duplicate = table.get_or_insert(b"hello");
        assert_eq!(map.get(&id1_duplicate), Some(&"value1"));
    }

    #[test]
    fn test_command_identifier_table_get_or_insert_new() {
        let table = CommandIdentifierTable::new();
        let name_bytes = b"hello";

        let identifier = table.get_or_insert(name_bytes);
        assert_eq!(identifier.as_bytes(), name_bytes);
    }

    #[test]
    fn test_command_identifier_table_get_or_insert_existing() {
        let table = CommandIdentifierTable::new();
        let name_bytes = b"hello";

        // Insert first time
        let id1 = table.get_or_insert(&name_bytes.clone());

        // Insert same name again - should return same reference
        let id2 = table.get_or_insert(&name_bytes.clone());

        // Should be the same object (same memory location)
        assert!(std::ptr::eq(id1, id2));
        assert_eq!(id1.as_bytes(), id2.as_bytes());
    }
}

