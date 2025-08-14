use crate::lexer::Lexer;
use crate::token::Token;
use crate::command_identifier::{CommandIdentifier, CommandIdentifierTable};

/// Preprocessor handles expansion in TeX. It accepts a stream of tokens from [Lexer] and expands each token in the
/// stream and produces a stream of unexapndable tokens.
///
///
pub struct Preprocessor<'pp> {
    lexer: Option<Box<Lexer<'pp, 'pp>>>,
    command_identifier_table: CommandIdentifierTable<'pp>,
}

impl<'pp> Preprocessor<'pp> {
    pub fn new() -> Self {
        Self {
            lexer: None,
            command_identifier_table: CommandIdentifierTable::new(),
        }
    }

    /// Main interface that shares the same prototype as Lexer's lex method.
    /// Calls into Lexer to get stream of tokens and produces tokens that cannot be expanded further.
    pub fn lex<'token>(&mut self, token: &'token mut Token<'token>)
    where
        'pp: 'token {
        // For now, just delegate to the lexer
        // TODO: Add expansion logic here
        if let Some(ref mut lexer) = self.lexer {
            lexer.lex(token);
        }

        // TODO: Check if the token is a command that needs expansion
        // TODO: If expandable, perform expansion and return expanded tokens
        // TODO: If not expandable, return the token as-is
    }

    /// Looks up a command identifier by name and creates one if it doesn't exist
    pub fn lookup_command_identifier(&'pp self, name_bytes: &[u8]) -> &'pp CommandIdentifier<'pp> {
        self.command_identifier_table.get_or_insert(name_bytes)
    }
}