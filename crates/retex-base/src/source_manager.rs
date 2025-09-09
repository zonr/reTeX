use std::collections::HashMap;
use std::path::PathBuf;
use crate::{MemoryBuffer, SourceLocation};

/// FileId represents a unique identifier for a file in the SourceManager.
/// This follows Clang's approach of using an opaque identifier for files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

impl FileId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn invalid() -> Self {
        Self(u32::MAX)
    }

    pub fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// FileEntry represents information about a loaded file.
/// This is similar to Clang's FileEntry but adapted for our needs.
#[derive(Debug, Clone, PartialEq)]
pub struct FileEntry {
    /// The file path
    pub path: PathBuf,
    /// The buffer containing the file contents
    pub buffer: MemoryBuffer,
    /// Starting offset in the global source location space
    pub start_offset: u32,
    /// Size of the file in bytes
    pub size: u32,
}

impl FileEntry {
    pub fn new(path: PathBuf, buffer: MemoryBuffer, start_offset: u32) -> Self {
        let size = buffer.size() as u32;
        Self {
            path,
            buffer,
            start_offset,
            size,
        }
    }

    /// Get the end offset of this file in the global source location space
    pub fn end_offset(&self) -> u32 {
        self.start_offset + self.size
    }

    /// Check if a source location falls within this file
    pub fn contains_location(&self, loc: SourceLocation) -> bool {
        let offset = loc.offset();
        offset >= self.start_offset && offset < self.end_offset()
    }

    /// Convert a global source location to a local offset within this file
    pub fn location_to_offset(&self, loc: SourceLocation) -> Option<u32> {
        if self.contains_location(loc) {
            Some(loc.offset() - self.start_offset)
        } else {
            None
        }
    }

    /// Convert a local offset within this file to a global source location
    pub fn offset_to_location(&self, offset: u32) -> Option<SourceLocation> {
        if offset <= self.size {
            Some(SourceLocation::new(self.start_offset + offset))
        } else {
            None
        }
    }
}

/// SourceManager handles loading and caching of source files into memory. This is inspired by Clang's SourceManager.
///
/// This object owns the MemoryBuffer objects for all the loaded files and assigns unique [FileId]'s for each unique
/// \\input chain.
///
/// TODO: Allow queries for file information about [SourceLocation].
#[derive(Debug)]
pub struct SourceManager {
    /// Map from FileId to FileEntry
    files: HashMap<FileId, FileEntry>,
    /// Next available FileId
    next_file_id: u32,
    /// Next available offset in the global source location space
    next_source_offset: u32,
}

impl SourceManager {
    /// Create a new SourceManager
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_file_id: 0,
            next_source_offset: 0,
        }
    }

    /// Load a file from a path and return its FileId
    pub fn load_file(&mut self, path: PathBuf) -> Result<FileId, std::io::Error> {
        let contents = std::fs::read(&path)?;
        let buffer_name = path.to_string_lossy().to_string();
        let buffer = MemoryBuffer::from_vec(contents, buffer_name);

        Ok(self.add_buffer(buffer, Some(path)))
    }

    /// Add a memory buffer as a file and return its FileId
    pub fn add_buffer(&mut self, buffer: MemoryBuffer, path: Option<PathBuf>) -> FileId {
        let file_id = FileId::new(self.next_file_id);
        self.next_file_id += 1;

        let path = path.unwrap_or_else(|| PathBuf::from(buffer.buffer_name()));
        let file_entry = FileEntry::new(path, buffer, self.next_source_offset);

        // Update next offset for the next file
        self.next_source_offset = file_entry.end_offset();

        self.files.insert(file_id, file_entry);
        file_id
    }

    /// Get a FileEntry by FileId
    pub fn get_file(&self, file_id: FileId) -> Option<&FileEntry> {
        self.files.get(&file_id)
    }

    /// Get a mutable FileEntry by FileId
    pub fn get_file_mut(&mut self, file_id: FileId) -> Option<&mut FileEntry> {
        self.files.get_mut(&file_id)
    }

    /// Get the buffer data for a file
    pub fn get_buffer_data(&self, file_id: FileId) -> Option<&MemoryBuffer> {
        self.get_file(file_id).map(|entry| &entry.buffer)
    }

    /// Get the file path for a file
    pub fn get_file_path(&self, file_id: FileId) -> Option<&PathBuf> {
        self.get_file(file_id).map(|entry| &entry.path)
    }

    /// Get a slice of buffer data for a specific range
    pub fn get_buffer_slice(&self, file_id: FileId, start: u32, len: u32) -> Option<&[u8]> {
        let file_entry = self.get_file(file_id)?;
        let start_idx = start as usize;
        let end_idx = (start + len) as usize;

        if end_idx <= file_entry.buffer.size() {
            Some(&file_entry.buffer.data()[start_idx..end_idx])
        } else {
            None
        }
    }

    /// Get the number of loaded files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if a file is loaded
    pub fn is_file_loaded(&self, file_id: FileId) -> bool {
        self.files.contains_key(&file_id)
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id() {
        let id = FileId::new(42);
        assert_eq!(id.as_u32(), 42);
        assert!(id.is_valid());

        let invalid = FileId::invalid();
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_file_entry() {
        let buffer = MemoryBuffer::from_str("Hello, World!", "test.tex".to_string());
        let path = PathBuf::from("test.tex");
        let entry = FileEntry::new(path.clone(), buffer, 100);

        assert_eq!(entry.path, path);
        assert_eq!(entry.start_offset, 100);
        assert_eq!(entry.size, 13);
        assert_eq!(entry.end_offset(), 113);

        let loc = SourceLocation::new(105);
        assert!(entry.contains_location(loc));
        assert_eq!(entry.location_to_offset(loc), Some(5));

        let out_of_range = SourceLocation::new(200);
        assert!(!entry.contains_location(out_of_range));
        assert_eq!(entry.location_to_offset(out_of_range), None);

        assert_eq!(entry.offset_to_location(5), Some(SourceLocation::new(105)));
        assert_eq!(entry.offset_to_location(20), None);
    }

    #[test]
    fn test_source_manager_add_buffer() {
        let mut sm = SourceManager::new();
        let buffer = MemoryBuffer::from_str("Hello", "test.tex".to_string());

        let file_id = sm.add_buffer(buffer, None);
        assert!(file_id.is_valid());
        assert_eq!(sm.file_count(), 1);
        assert!(sm.is_file_loaded(file_id));
    }

    #[test]
    fn test_source_manager_multiple_files() {
        let mut sm = SourceManager::new();

        let buffer1 = MemoryBuffer::from_str("First", "first.tex".to_string());
        let file_id1 = sm.add_buffer(buffer1, None);

        let buffer2 = MemoryBuffer::from_str("Second", "second.tex".to_string());
        let file_id2 = sm.add_buffer(buffer2, None);

        assert_ne!(file_id1, file_id2);
        assert_eq!(sm.file_count(), 2);

        let file1 = sm.get_file(file_id1).unwrap();
        let file2 = sm.get_file(file_id2).unwrap();

        assert_eq!(file1.start_offset, 0);
        assert_eq!(file1.size, 5);
        assert_eq!(file2.start_offset, 5);
        assert_eq!(file2.size, 6);
    }

    #[test]
    fn test_source_manager_buffer_operations() {
        let mut sm = SourceManager::new();
        let buffer = MemoryBuffer::from_str("Hello, World!", "test.tex".to_string());
        let file_id = sm.add_buffer(buffer, Some(PathBuf::from("test.tex")));

        let buffer = sm.get_buffer_data(file_id).unwrap();
        assert_eq!(buffer.data(), b"Hello, World!");

        let path = sm.get_file_path(file_id).unwrap();
        assert_eq!(path, &PathBuf::from("test.tex"));

        let slice = sm.get_buffer_slice(file_id, 0, 5).unwrap();
        assert_eq!(slice, b"Hello");

        let out_of_range = sm.get_buffer_slice(file_id, 0, 100);
        assert_eq!(out_of_range, None);
    }

    #[test]
    fn test_source_manager_empty() {
        let sm = SourceManager::new();
        assert_eq!(sm.file_count(), 0);

        let invalid_id = FileId::new(0);
        assert!(!sm.is_file_loaded(invalid_id));
        assert_eq!(sm.get_file(invalid_id), None);
    }
}
