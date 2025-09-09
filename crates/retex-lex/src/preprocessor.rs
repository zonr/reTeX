use std::path::PathBuf;
use retex_base::{SourceManager, FileId, MemoryBuffer};
use crate::lexer::Lexer;
use crate::token::Token;
use crate::command_identifier::CommandIdentifierTable;

/// Entry in the include stack representing a lexer for a particular file
struct IncludeStackEntry<'source, 'idtable> {
    /// The lexer for this file
    lexer: Lexer<'source, 'idtable>,
    /// The file ID in the source manager
    file_id: FileId,
}

/// Preprocessor handles expansion in TeX. It accepts a stream of tokens from [Lexer] and expands each token in the
/// stream and produces a stream of unexapndable tokens.
///
/// The preprocessor manages an include stack to handle file inclusion, following Clang's approach.
pub struct Preprocessor<'source, 'pp> {
    /// Source manager for file management
    source_manager: &'source mut SourceManager,
    /// Stack of active lexers (include stack)
    include_stack: Vec<IncludeStackEntry<'source, 'pp>>,
    /// Command identifier table for managing command names
    command_identifier_table: CommandIdentifierTable<'pp>,
}

impl<'source, 'pp> Preprocessor<'source, 'pp>
where
    'source: 'pp {
    pub fn new(source_manager: &'source mut SourceManager) -> Self {
        Self {
            source_manager,
            include_stack: Vec::new(),
            command_identifier_table: CommandIdentifierTable::new(),
        }
    }

    /// Enter the main input file. This is the entry point for starting lexing.
    /// Following Clang's Preprocessor::EnterMainSourceFile pattern.
    pub fn enter_main_file(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        let file_id = self.source_manager.load_file(path)?;
        self.enter_file(file_id);
        Ok(())
    }

    /// Enter a file by creating a new lexer and switching to it.
    /// If there's a current lexer, it gets pushed onto the include stack.
    pub fn enter_file(&mut self, file_id: FileId) {
        // First check if file exists
        if !self.source_manager.is_file_loaded(file_id) {
            return;
        }

        // Get buffer reference through raw pointer
        if let Some(buffer) = self.source_manager.get_buffer_data(file_id) {
            // SAFETY: Rust can’t allow a struct to contain a field that borrows another field of the same struct.
            // Passing buffer from SourceManager to Lexer creates a self-referential relationship between Preprocessor’s
            // fields (between self.source_manager and self.include_stack that holds Lexer.) The borrow checker
            // disallows this because moving `self` would invalidate references.
            // We bypass that by using raw pointers. This is sound only if:
            // 1. `self.source_manager` outlives all Lexers in `self.include_stack`
            // 2. `Preprocessor` is never moved after a Lexer is created (or else the references would dangle).
            let lexer = unsafe {
                // Get raw pointers to avoid borrow checker issues
                let command_table_ptr = &self.command_identifier_table as *const CommandIdentifierTable<'pp>;

                Lexer::from_memory_buffer(
                    &*(buffer as *const MemoryBuffer),
                    &*command_table_ptr
                )
            };

            self.include_stack.push(IncludeStackEntry { lexer, file_id });
        }
    }

    /// Get the current active lexer (top of include stack)
    fn current_lexer(&mut self) -> Option<&mut Lexer<'source, 'pp>> {
        self.include_stack.last_mut().map(|entry| &mut entry.lexer)
    }

    /// Main interface that shares the same prototype as Lexer's lex method.
    /// Calls into Lexer to get stream of tokens and produces tokens that cannot be expanded further.
    pub fn lex<'token>(&mut self, token: &'token mut Token<'token>) -> bool
    where
        'pp: 'token {

        // Get the current lexer from the include stack
        if let Some(lexer) = self.current_lexer() {
            lexer.lex(token);

            // TODO: Check if the token is a command that needs expansion
            // TODO: If expandable, perform expansion and return expanded tokens
            // TODO: If not expandable, return the token as-is

            true
        } else {
            false
        }
    }
}
