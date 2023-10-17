use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, Range, TextDocumentContentChangeEvent,
};

use crate::{definitions, text_document::TextDocument};
use chumsky::Parser;
use std::{cmp::Ordering, collections::HashMap};
use sail_parser::{lexer::lexer, parser::parse_file, cst::DefAux, Spanned};

use chumsky::input::Input;

pub struct File {
    // The source code.
    pub source: TextDocument,

    // Parsed Concrete Syntax Tree.
    pub cst: Option<Spanned<Vec<Spanned<DefAux>>>>,

    // // The parse result if any. If there isn't one then that is because
    // // of a parse error.
    // pub analysis: Option<Analysis>,

    // Go-to definition locations extracted from the file.
    pub definitions: HashMap<String, usize>,

    // Diagnostic errors from parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl File {
    pub fn new(source: String) -> Self {
        let mut f = Self {
            source: TextDocument::new(source),
            cst: None,
            // analysis: None,
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
        let src = self.source.text();
        let (tokens, lex_errs) = lexer().parse(src).into_output_errors();

        self.cst = None;

        let parse_errs = if let Some(tokens) = &tokens {
            let (cst, parse_errs) = parse_file()
                .map_with(|cst, e| (cst, e.span()))
                .parse(tokens.as_slice().spanned((src.len()..src.len()).into()))
                .into_output_errors();

            self.cst = cst;
            parse_errs
        } else {
            Vec::new()
        };

        let mut diagnostics = Vec::with_capacity(self.diagnostics.len());

        for error in lex_errs.into_iter()
            .map(|e| e.map_token(|c| c.to_string()))
            .chain(
                parse_errs
                    .into_iter()
                    .map(|e| e.map_token(|tok| tok.to_string())),
            ) {
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

        let definitions = match &self.cst {
            Some(cst) => crate::analyser::find_definitions(cst, &self.source.text()),
            None => HashMap::new(),
        };

        self.definitions = definitions;
        self.diagnostics = diagnostics;
    }

    // pub fn token_at(&self, position: Position) -> Option<&(sail_parser::Token, sail_parser::Span)> {
    //     // Convert the line/character to an offset.
    //     let offset = self.source.offset_at(&position);
    //     // Binary search for a token that contains the offset.
    //     let tokens = self.tokens.as_ref()?;
    //     let token = tokens.binary_search_by(|(_, span)| {
    //         if span.start <= offset && offset <= span.end {
    //             Ordering::Equal
    //         } else if span.start > offset {
    //             Ordering::Greater
    //         } else {
    //             Ordering::Less
    //         }
    //     });
    //     // If we found a token then return it.
    //     token.ok().map(|i| &tokens[i])
    // }
}
