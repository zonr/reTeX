use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryBuffer {
    data: Arc<Vec<u8>>,
    buffer_name: String,
}

impl MemoryBuffer {
    pub fn from_vec(data: Vec<u8>, buffer_name: String) -> Self {
        Self {
            data: Arc::new(data),
            buffer_name,
        }
    }

    pub fn from_string(text: String, buffer_name: String) -> Self {
        Self::from_vec(text.into_bytes(), buffer_name)
    }

    pub fn from_slice(data: &[u8], buffer_name: String) -> Self {
        Self::from_vec(data.to_vec(), buffer_name)
    }

    pub fn from_str(text: &str, buffer_name: String) -> Self {
        Self::from_string(text.to_string(), buffer_name)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn buffer_name(&self) -> &str {
        &self.buffer_name
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data)
    }

    pub fn get_buffer_start(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn get_buffer_end(&self) -> *const u8 {
        unsafe { self.data.as_ptr().add(self.size()) }
    }

    pub fn offset_from_buffer_start(&self, ptr: *const u8) -> Option<usize> {
        let start_ptr = self.get_buffer_start();
        let end_ptr = self.get_buffer_end();

        if ptr >= start_ptr && ptr <= end_ptr {
            Some(unsafe { ptr.offset_from(start_ptr) as usize })
        } else {
            None
        }
    }

    pub fn char_at(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    pub fn chars(&self) -> impl Iterator<Item = u8> + '_ {
        self.data.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_buffer_from_vec() {
        let data = vec![72, 101, 108, 108, 111]; // "Hello"
        let buffer = MemoryBuffer::from_vec(data.clone(), "test.tex".to_string());
        
        assert_eq!(buffer.data(), data.as_slice());
        assert_eq!(buffer.buffer_name(), "test.tex");
        assert_eq!(buffer.size(), 5);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_memory_buffer_from_string() {
        let text = "Hello, World!".to_string();
        let buffer = MemoryBuffer::from_string(text.clone(), "test.tex".to_string());
        
        assert_eq!(buffer.data(), text.as_bytes());
        assert_eq!(buffer.buffer_name(), "test.tex");
        assert_eq!(buffer.size(), 13);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.as_str().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_memory_buffer_from_slice() {
        let data = b"Hello, World!";
        let buffer = MemoryBuffer::from_slice(data, "test.tex".to_string());
        
        assert_eq!(buffer.data(), data);
        assert_eq!(buffer.buffer_name(), "test.tex");
        assert_eq!(buffer.size(), 13);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_memory_buffer_from_str() {
        let text = "Hello, World!";
        let buffer = MemoryBuffer::from_str(text, "test.tex".to_string());
        
        assert_eq!(buffer.data(), text.as_bytes());
        assert_eq!(buffer.buffer_name(), "test.tex");
        assert_eq!(buffer.size(), 13);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.as_str().unwrap(), text);
    }

    #[test]
    fn test_memory_buffer_empty() {
        let buffer = MemoryBuffer::from_vec(vec![], "empty.tex".to_string());
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.size(), 0);
        assert_eq!(buffer.data(), &[]);
        assert_eq!(buffer.as_str().unwrap(), "");
    }

    #[test]
    fn test_memory_buffer_as_str_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let buffer = MemoryBuffer::from_vec(invalid_utf8, "invalid.tex".to_string());
        
        assert!(buffer.as_str().is_err());
    }

    #[test]
    fn test_memory_buffer_char_at() {
        let buffer = MemoryBuffer::from_str("Hello", "test.tex".to_string());
        
        assert_eq!(buffer.char_at(0), Some(b'H'));
        assert_eq!(buffer.char_at(1), Some(b'e'));
        assert_eq!(buffer.char_at(4), Some(b'o'));
        assert_eq!(buffer.char_at(5), None);
        assert_eq!(buffer.char_at(100), None);
    }

    #[test]
    fn test_memory_buffer_chars_iterator() {
        let buffer = MemoryBuffer::from_str("Hi", "test.tex".to_string());
        let chars: Vec<u8> = buffer.chars().collect();
        
        assert_eq!(chars, vec![b'H', b'i']);
    }

    #[test]
    fn test_memory_buffer_get_buffer_pointers() {
        let buffer = MemoryBuffer::from_str("Hello", "test.tex".to_string());
        
        let start_ptr = buffer.get_buffer_start();
        let end_ptr = buffer.get_buffer_end();
        
        // The pointers should be valid and end should be after start
        assert!(!start_ptr.is_null());
        assert!(!end_ptr.is_null());
        
        // Check that the distance matches the buffer size
        let distance = unsafe { end_ptr.offset_from(start_ptr) } as usize;
        assert_eq!(distance, buffer.size());
    }

    #[test]
    fn test_memory_buffer_offset_from_buffer_start() {
        let buffer = MemoryBuffer::from_str("Hello", "test.tex".to_string());
        
        let start_ptr = buffer.get_buffer_start();
        let end_ptr = buffer.get_buffer_end();
        
        // Test with start pointer
        assert_eq!(buffer.offset_from_buffer_start(start_ptr), Some(0));
        
        // Test with end pointer
        assert_eq!(buffer.offset_from_buffer_start(end_ptr), Some(buffer.size()));
        
        // Test with middle pointer
        let middle_ptr = unsafe { start_ptr.add(2) };
        assert_eq!(buffer.offset_from_buffer_start(middle_ptr), Some(2));
        
        // Test with invalid pointer (before start)
        let before_ptr = unsafe { start_ptr.sub(1) };
        assert_eq!(buffer.offset_from_buffer_start(before_ptr), None);
        
        // Test with invalid pointer (after end)
        let after_ptr = unsafe { end_ptr.add(1) };
        assert_eq!(buffer.offset_from_buffer_start(after_ptr), None);
    }

    #[test]
    fn test_memory_buffer_clone() {
        let buffer1 = MemoryBuffer::from_str("Hello", "test.tex".to_string());
        let buffer2 = buffer1.clone();
        
        // Both buffers should have the same data
        assert_eq!(buffer1.data(), buffer2.data());
        assert_eq!(buffer1.buffer_name(), buffer2.buffer_name());
        assert_eq!(buffer1.size(), buffer2.size());
        
        // They should share the same underlying Arc
        assert_eq!(
            buffer1.data().as_ptr(),
            buffer2.data().as_ptr()
        );
    }

    #[test]
    fn test_memory_buffer_debug() {
        let buffer = MemoryBuffer::from_str("test", "test.tex".to_string());
        let debug_str = format!("{:?}", buffer);
        
        // Should contain the buffer name and some representation
        assert!(debug_str.contains("test.tex"));
    }

    #[test]
    fn test_memory_buffer_with_unicode() {
        let text = "Hello, 世界! 🌍";
        let buffer = MemoryBuffer::from_str(text, "unicode.tex".to_string());
        
        assert_eq!(buffer.as_str().unwrap(), text);
        assert_eq!(buffer.size(), text.len()); // Length in bytes
        
        // Check that we can iterate over bytes
        let bytes: Vec<u8> = buffer.chars().collect();
        assert_eq!(bytes, text.as_bytes());
    }

    #[test]
    fn test_memory_buffer_empty_name() {
        let buffer = MemoryBuffer::from_str("test", "".to_string());
        assert_eq!(buffer.buffer_name(), "");
    }

    #[test]
    fn test_memory_buffer_large_data() {
        let large_data = vec![65u8; 10000]; // 10,000 'A' characters
        let buffer = MemoryBuffer::from_vec(large_data.clone(), "large.tex".to_string());
        
        assert_eq!(buffer.size(), 10000);
        assert_eq!(buffer.data(), large_data.as_slice());
        assert!(!buffer.is_empty());
    }
}