use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, Range, TextDocumentContentChangeEvent,
};

use crate::{definitions, text_document::TextDocument};
use chumsky::Parser;
use std::{cmp::Ordering, collections::HashMap};

pub struct File {
    // The source code.
    pub source: TextDocument,

    // The parse result if any. If there isn't one then that is because
    // of a parse error.
    pub tokens: Option<Vec<(sail_parser::Token, sail_parser::Span)>>,

    // Go-to definition locations extracted from the file.
    pub definitions: HashMap<String, usize>,

    // Diagnostic errors from parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl File {
    pub fn new(source: String) -> Self {
        let mut f = Self {
            source: TextDocument::new(source),
            tokens: None,
            definitions: HashMap::new(),
            diagnostics: Vec::new(),
        };
        f.parse();
        f
    }

    pub fn update(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        for change in &changes {
            self.source.update(change);
        }

        self.parse();
    }

    pub fn parse(&mut self) {
        let text = self.source.text();
        let result = sail_parser::lexer().parse(text);
        self.tokens = result.output().cloned();

        let mut definitions = HashMap::with_capacity(self.definitions.len());
        let mut diagnostics = Vec::with_capacity(self.diagnostics.len());

        if let Some(tokens) = &self.tokens {
            definitions::add_definitions(tokens, text, &mut definitions);
        } else {
            diagnostics.push(Diagnostic::new(
                Range::new(Position::new(0, 0), Position::new(0, 0)),
                Some(DiagnosticSeverity::ERROR),
                None,
                Some("Sail".to_string()),
                "Error parsing file".to_string(),
                None,
                None,
            ));
        }
        for error in result.errors().into_iter() {
            let span = error.span();
            let start = self.source.position_at(span.start);
            let end = self.source.position_at(span.end);
            diagnostics.push(Diagnostic::new(
                Range::new(start, end),
                Some(DiagnosticSeverity::ERROR),
                None,
                Some("Sail".to_string()),
                error.to_string(),
                None,
                None,
            ));
        }

        self.definitions = definitions;
        self.diagnostics = diagnostics;
    }

    pub fn token_at(&self, position: Position) -> Option<&(sail_parser::Token, sail_parser::Span)> {
        // Convert the line/character to an offset.
        let offset = self.source.offset_at(&position);
        // Binary search for a token that contains the offset.
        let tokens = self.tokens.as_ref()?;
        let token = tokens.binary_search_by(|(_, span)| {
            if span.start <= offset && offset <= span.end {
                Ordering::Equal
            } else if span.start > offset {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        // If we found a token then return it.
        token.ok().map(|i| &tokens[i])
    }
}
