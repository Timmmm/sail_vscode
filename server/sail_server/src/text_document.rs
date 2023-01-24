#![allow(unused)]

use tower_lsp::lsp_types::{
    Position as LspPosition, Range as LspRange, TextDocumentContentChangeEvent,
};

type ByteIndex = usize;
type LineIndex = usize;
// VSCode "characters" are UTF-16 code points.
type CharIndexUTF16 = usize;

pub struct TextDocument {
    // The text.
    content: String,
    // The start of each line in bytes. These could be lazily calculated but
    // that is a bit tricky because of the borrow checker.
    line_offsets: Vec<ByteIndex>,
}

impl TextDocument {
    pub fn new(content: String) -> Self {
        let line_offsets = compute_line_offsets(&content, true, 0);
        Self {
            content,
            line_offsets,
        }
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    #[cfg(test)]
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    #[cfg(test)]
    pub fn text_range(&self, range: &LspRange) -> &str {
        let byte_begin = self.offset_at(&range.start);
        let byte_end = self.offset_at(&range.end);
        &self.content[byte_begin..byte_end]
    }

    // Based on this code https://github.com/microsoft/vscode-languageserver-node/blob/master/textDocument/src/main.ts#L222
    // Apply a change to the document.
    pub fn update(&mut self, change: &TextDocumentContentChangeEvent) {
        if let Some(range) = change.range {
            // Get the corresponding byte range.
            let byte_begin = self.offset_at(&range.start);
            let byte_end = self.offset_at(&range.end);
            self.content
                .replace_range(byte_begin..byte_end, &change.text);

            // Calculate the line offsets for the inserted text.
            let added_line_offsets = compute_line_offsets(&change.text, false, byte_begin);

            // The line offsets that we need to delete. It is the lines one past
            // the actual positions. Because if you edit line 10, you need to change
            // the line offset for line 11.
            let delete_line_offset_begin = range.start.line as usize + 1;
            let delete_line_offset_end = range.end.line as usize + 1;

            // Update the offsets after the splice according to the length change
            // of the modified region.
            let len_before = byte_end - byte_begin;
            let len_after = change.text.len();
            if len_before != len_after {
                for offset in &mut self.line_offsets[delete_line_offset_end..] {
                    *offset += len_after;
                    *offset -= len_before;
                }
            }

            // Insert the new offsets.
            self.line_offsets.splice(
                delete_line_offset_begin..delete_line_offset_end,
                added_line_offsets.clone(),
            );
        } else {
            // Just completely change the text.
            self.content = change.text.clone();
            self.line_offsets = compute_line_offsets(&self.content, true, 0);
        }
    }

    // Convert a row/column position to a byte index.
    pub fn offset_at(&self, position: &LspPosition) -> ByteIndex {
        let line_begin = self.line_start(position.line as usize);
        let line_end = self.line_start(position.line as usize + 1);
        let line = &self.content[line_begin..line_end];

        line_begin + character_to_line_offset(line, position.character as usize)
    }

    // Convert a byte index to a row/column position.
    pub fn position_at(&self, offset: usize) -> LspPosition {
        // Clamp to valid range.
        let offset = std::cmp::min(offset, self.content.len());

        // Binary search for a line offset that is greater than the offset.
        match self.line_offsets.binary_search(&offset) {
            Ok(line) => {
                // offset actually matches one of the line_offsets. Just return it.
                LspPosition {
                    line: line as u32,
                    character: 0,
                }
            }
            Err(line) => {
                // line is the line one past the one that contains the offset.
                let line = line - 1;
                let character = self.position_at_line(line, offset);

                LspPosition {
                    line: line as u32,
                    character: character as u32,
                }
            }
        }
    }

    fn line_start(&self, line_index: LineIndex) -> ByteIndex {
        self.line_offsets
            .get(line_index)
            .copied()
            .unwrap_or_else(|| self.content.len())
    }

    // Given a byte offset, what is the corresponding character?
    fn position_at_line(&self, line: LineIndex, offset: usize) -> CharIndexUTF16 {
        let line_start = self.line_offsets[line];

        assert!(line_start <= offset);

        // We have to scan through the line, counting the characters.
        let line_text = &self.content[line_start..offset];
        line_text.chars().map(char::len_utf16).sum()
    }
}

// Given a UTF-16 codepoint offset in a bit of text, convert it to a byte offset.
// Out-of-bounds offsets just return line.len().
fn character_to_line_offset(line: &str, character: CharIndexUTF16) -> ByteIndex {
    let mut utf16_pos = 0;

    for (byte_pos, ch) in line.char_indices() {
        if utf16_pos == character {
            return byte_pos;
        }
        utf16_pos += ch.len_utf16();
    }

    line.len()
}

fn compute_line_offsets(text: &str, is_at_line_start: bool, text_offset: usize) -> Vec<usize> {
    // VSCode treats `\r\n` or `\n` *or* `\r` as a single line end.
    let mut line_offsets = Vec::new();
    if is_at_line_start {
        line_offsets.push(text_offset);
    }
    let text = text.as_bytes();
    for i in 0..text.len() {
        match text[i] {
            b'\n' => line_offsets.push(text_offset + i + 1),
            b'\r' => {
                // This is a new line *unless* the next character is \n.
                if text.get(i + 1) != Some(&b'\n') {
                    line_offsets.push(text_offset + i + 1)
                }
            }
            _ => {}
        }
    }
    line_offsets
}

#[cfg(test)]
mod test {

    use super::*;

    use tower_lsp::lsp_types::Range as LspRange;

    #[test]
    fn empty_content() {
        let text = "".to_string();
        let document = TextDocument::new(text);
        assert_eq!(document.line_count(), 1);
        assert_eq!(document.offset_at(&LspPosition::new(0, 0)), 0);
        assert_eq!(document.position_at(0), LspPosition::new(0, 0));
    }

    #[test]
    fn single_line() {
        let text = "Hello World".to_string();
        let document = TextDocument::new(text.clone());
        assert_eq!(document.line_count(), 1);

        for (char_index, (byte_index, _)) in text.char_indices().enumerate() {
            assert_eq!(
                document.offset_at(&LspPosition::new(0, char_index as u32)),
                byte_index
            );
            assert_eq!(
                document.position_at(byte_index),
                LspPosition::new(0, char_index as u32)
            );
        }
    }

    #[test]
    fn multiple_lines() {
        let text = "ABCDE\nFGHIJ\nKLMNO\n".to_string();
        let document = TextDocument::new(text.clone());
        assert_eq!(document.line_count(), 4);

        for (char_index, (byte_index, _)) in text.char_indices().enumerate() {
            let line = char_index / 6;
            let column = char_index % 6;

            assert_eq!(
                document.offset_at(&LspPosition::new(line as u32, column as u32)),
                byte_index
            );
            assert_eq!(
                document.position_at(byte_index),
                LspPosition::new(line as u32, column as u32)
            );
        }

        assert_eq!(document.offset_at(&LspPosition::new(3, 0)), 18);
        assert_eq!(document.offset_at(&LspPosition::new(3, 1)), 18);
        assert_eq!(document.position_at(18), LspPosition::new(3, 0));
        assert_eq!(document.position_at(19), LspPosition::new(3, 0));
    }

    #[test]
    fn starts_with_new_line() {
        let document = TextDocument::new("\nABCDE".to_string());
        assert_eq!(document.line_count(), 2);
        assert_eq!(document.position_at(0), LspPosition::new(0, 0));
        assert_eq!(document.position_at(1), LspPosition::new(1, 0));
        assert_eq!(document.position_at(6), LspPosition::new(1, 5));
    }

    #[test]
    fn new_line_characters() {
        let text = "ABCDE\rFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 2);

        let text = "ABCDE\nFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 2);

        let text = "ABCDE\r\nFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 2);

        let text = "ABCDE\n\nFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 3);

        let text = "ABCDE\r\rFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 3);

        let text = "ABCDE\n\rFGHIJ".to_string();
        assert_eq!(TextDocument::new(text).line_count(), 3);
    }

    #[test]
    fn get_text_range() {
        let text = "12345\n12345\n12345".to_string();
        let document = TextDocument::new(text.clone());
        assert_eq!(document.text(), text);
        // assert_eq!(document.text_range(&LspRange::new(LspPosition::new(-1, 0), LspPosition::new(0, 5))), "12345");
        assert_eq!(
            document.text_range(&LspRange::new(
                LspPosition::new(0, 0),
                LspPosition::new(0, 5)
            )),
            "12345"
        );
        assert_eq!(
            document.text_range(&LspRange::new(
                LspPosition::new(0, 4),
                LspPosition::new(1, 1)
            )),
            "5\n1"
        );
        assert_eq!(
            document.text_range(&LspRange::new(
                LspPosition::new(0, 4),
                LspPosition::new(2, 1)
            )),
            "5\n12345\n1"
        );
        assert_eq!(
            document.text_range(&LspRange::new(
                LspPosition::new(0, 4),
                LspPosition::new(3, 1)
            )),
            "5\n12345\n12345"
        );
        assert_eq!(
            document.text_range(&LspRange::new(
                LspPosition::new(0, 0),
                LspPosition::new(3, 5)
            )),
            text
        );
    }

    #[test]
    fn invalid_inputs() {
        let text = "Hello World".to_string();
        let document = TextDocument::new(text.clone());

        // invalid position
        assert_eq!(
            document.offset_at(&LspPosition::new(0, text.len() as u32)),
            text.len()
        );
        assert_eq!(
            document.offset_at(&LspPosition::new(0, text.len() as u32 + 3)),
            text.len()
        );
        assert_eq!(document.offset_at(&LspPosition::new(2, 3)), text.len());
        // assert_eq!(document.offset_at(&LspPosition::new(-1, 3)), 0);
        // assert_eq!(document.offset_at(&LspPosition::new(0, -3)), 0);
        // assert_eq!(document.offset_at(&LspPosition::new(1, -3)), text.len());

        // invalid offsets
        // assert_eq!(document.position_at(-1), LspPosition::new(0, 0));
        assert_eq!(
            document.position_at(text.len()),
            LspPosition::new(0, text.len() as u32)
        );
        assert_eq!(
            document.position_at(text.len() + 3),
            LspPosition::new(0, text.len() as u32)
        );
    }

    // Full updates.

    #[test]
    fn one_full_update() {
        let mut document = TextDocument::new("abc123".to_string());
        document.update(&TextDocumentContentChangeEvent {
            text: "efg456".to_string(),
            range: None,
            range_length: None,
        });
        assert_eq!(document.text(), "efg456");
    }

    #[test]
    fn several_full_content_updates() {
        let mut document = TextDocument::new("abc123".to_string());
        document.update(&TextDocumentContentChangeEvent {
            text: "hello".to_string(),
            range: None,
            range_length: None,
        });
        document.update(&TextDocumentContentChangeEvent {
            text: "world".to_string(),
            range: None,
            range_length: None,
        });
        assert_eq!(document.text(), "world");
    }

    // Incremental updates.

    // assumes that only "\n" is used
    fn assert_valid_line_numbers(doc: &TextDocument) {
        let text = doc.text();
        let mut expected_line_number = 0;
        for (i, ch) in doc.text().char_indices() {
            assert_eq!(doc.position_at(i).line, expected_line_number);
            if ch == '\n' {
                expected_line_number += 1;
            }
        }
        assert_eq!(doc.position_at(text.len()).line, expected_line_number);
    }

    fn range_for_substring(doc: &TextDocument, needle: &str) -> LspRange {
        let offset = doc.text().find(needle).unwrap();
        LspRange {
            start: doc.position_at(offset),
            end: doc.position_at(offset + needle.len()),
        }
    }

    fn range_after_substring(doc: &TextDocument, needle: &str) -> LspRange {
        let offset = doc.text().find(needle).unwrap();
        let pos = doc.position_at(offset + needle.len());
        LspRange {
            start: pos,
            end: pos,
        }
    }

    #[test]
    fn incrementally_removing_content() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "".to_string(),
            range: Some(range_for_substring(&document, "hello, world!")),
            range_length: None,
        });
        assert_eq!(document.text(), "function abc() {\n  console.log(\"\");\n}");
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_removing_multi_line_content() {
        let mut document =
            TextDocument::new("function abc() {\n  foo();\n  bar();\n  \n}".to_string());
        assert_eq!(document.line_count(), 5);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "".to_string(),
            range: Some(range_for_substring(&document, "  foo();\n  bar();\n")),
            range_length: None,
        });
        assert_eq!(document.text(), "function abc() {\n  \n}");
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_removing_multi_line_content_2() {
        let mut document =
            TextDocument::new("function abc() {\n  foo();\n  bar();\n  \n}".to_string());
        assert_eq!(document.line_count(), 5);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "".to_string(),
            range: Some(range_for_substring(&document, "foo();\n  bar();")),
            range_length: None,
        });
        assert_eq!(document.text(), "function abc() {\n  \n  \n}");
        assert_eq!(document.line_count(), 4);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_adding_content() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: ", world!".to_string(),
            range: Some(range_after_substring(&document, "hello")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "function abc() {\n  console.log(\"hello, world!\");\n}"
        );
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_adding_multi_line_content() {
        let mut document = TextDocument::new(
            "function abc() {\n  while (true) {\n    foo();\n  };\n}".to_string(),
        );
        assert_eq!(document.line_count(), 5);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "\n    bar();".to_string(),
            range: Some(range_after_substring(&document, "foo();")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "function abc() {\n  while (true) {\n    foo();\n    bar();\n  };\n}"
        );
        assert_eq!(document.line_count(), 6);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_single_line_content_more_chars() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "hello, test case!!!".to_string(),
            range: Some(range_for_substring(&document, "hello, world!")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "function abc() {\n  console.log(\"hello, test case!!!\");\n}"
        );
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_single_line_content_less_chars() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "hey".to_string(),
            range: Some(range_for_substring(&document, "hello, world!")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "function abc() {\n  console.log(\"hey\");\n}"
        );
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_single_line_content_same_num_of_chars() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "world, hello!".to_string(),
            range: Some(range_for_substring(&document, "hello, world!")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "function abc() {\n  console.log(\"world, hello!\");\n}"
        );
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_multi_line_content_more_lines() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        assert_eq!(document.line_count(), 3);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "\n//hello\nfunction d(){".to_string(),
            range: Some(range_for_substring(&document, "function abc() {")),
            range_length: None,
        });
        assert_eq!(
            document.text(),
            "\n//hello\nfunction d(){\n  console.log(\"hello, world!\");\n}"
        );
        assert_eq!(document.line_count(), 5);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_multi_line_content_less_lines() {
        let mut document = TextDocument::new("a1\nb1\na2\nb2\na3\nb3\na4\nb4\n".to_string());
        assert_eq!(document.line_count(), 9);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "xx\nyy".to_string(),
            range: Some(range_for_substring(&document, "\na3\nb3\na4\nb4\n")),
            range_length: None,
        });
        assert_eq!(document.text(), "a1\nb1\na2\nb2xx\nyy");
        assert_eq!(document.line_count(), 5);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_multi_line_content_same_num_of_lines_and_chars() {
        let mut document = TextDocument::new("a1\nb1\na2\nb2\na3\nb3\na4\nb4\n".to_string());
        assert_eq!(document.line_count(), 9);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "\nxx1\nxx2".to_string(),
            range: Some(range_for_substring(&document, "a2\nb2\na3")),
            range_length: None,
        });
        assert_eq!(document.text(), "a1\nb1\n\nxx1\nxx2\nb3\na4\nb4\n");
        assert_eq!(document.line_count(), 9);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn incrementally_replacing_multi_line_content_same_num_of_lines_but_diff_chars() {
        let mut document = TextDocument::new("a1\nb1\na2\nb2\na3\nb3\na4\nb4\n".to_string());
        assert_eq!(document.line_count(), 9);
        assert_valid_line_numbers(&document);
        document.update(&TextDocumentContentChangeEvent {
            text: "\ny\n".to_string(),
            range: Some(range_for_substring(&document, "a2\nb2\na3")),
            range_length: None,
        });
        assert_eq!(document.text(), "a1\nb1\n\ny\n\nb3\na4\nb4\n");
        assert_eq!(document.line_count(), 9);
        assert_valid_line_numbers(&document);
    }

    // #[test]
    // fn incrementally_replacing_multi_line_content_huge_number_of_lines() {
    // 	let mut document = TextDocument::new("a1\ncc\nb1".to_string());
    // 	assert_eq!(document.line_count(), 3);
    // 	assert_valid_line_numbers(&document);
    // 	let text = new Array(20000).join("\ndd"); // a string with 19999 `\n`
    // 	document.update( &TextDocumentContentChangeEvent{
    // 		text: "".to_string(),
    // 		range: Some(range_for_substring(&document, "")),
    // 		range_length: None,
    // 	});
    // 	document.update( [{ text, range: Ranges.forSubstring(document, "\ncc") }], 1);
    // 	assert_eq!(document.text(), "a1" + text + "\nb1");
    // 	assert_eq!(document.line_count(), 20001);
    // 	assert_valid_line_numbers(&document);
    // }

    #[test]
    fn several_incremental_content_changes() {
        let mut document =
            TextDocument::new("function abc() {\n  console.log(\"hello, world!\");\n}".to_string());
        document.update(&TextDocumentContentChangeEvent {
            text: "defg".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(0, 12),
                LspPosition::new(0, 12),
            )),
            range_length: None,
        });
        document.update(&TextDocumentContentChangeEvent {
            text: "hello, test case!!!".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 15),
                LspPosition::new(1, 28),
            )),
            range_length: None,
        });
        document.update(&TextDocumentContentChangeEvent {
            text: "hij".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(0, 16),
                LspPosition::new(0, 16),
            )),
            range_length: None,
        });

        assert_eq!(
            document.text(),
            "function abcdefghij() {\n  console.log(\"hello, test case!!!\");\n}"
        );
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn basic_append() {
        let mut document = TextDocument::new("foooo\nbar\nbaz".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);

        document.update(&TextDocumentContentChangeEvent {
            text: " some extra content".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 3),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foooo\nbar some extra content\nbaz");
        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 29);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn multi_line_append() {
        let mut document = TextDocument::new("foooo\nbar\nbaz".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);

        document.update(&TextDocumentContentChangeEvent {
            text: " some extra\ncontent".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 3),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foooo\nbar some extra\ncontent\nbaz");
        assert_eq!(document.offset_at(&LspPosition::new(3, 0)), 29);
        assert_eq!(document.line_count(), 4);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn basic_delete() {
        let mut document = TextDocument::new("foooo\nbar\nbaz".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);

        document.update(&TextDocumentContentChangeEvent {
            text: "".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 0),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foooo\n\nbaz");
        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 7);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn multi_line_delete() {
        let mut document = TextDocument::new("foooo\nbar\nbaz".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);

        document.update(&TextDocumentContentChangeEvent {
            text: "".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(0, 5),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foooo\nbaz");
        assert_eq!(document.offset_at(&LspPosition::new(1, 0)), 6);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn single_character_replace() {
        let mut document = TextDocument::new("foooo\nbar\nbaz".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);

        document.update(&TextDocumentContentChangeEvent {
            text: "z".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 2),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foooo\nbaz\nbaz");
        assert_eq!(document.offset_at(&LspPosition::new(2, 0)), 10);
        assert_valid_line_numbers(&document);
    }

    #[test]
    fn multi_character_replace() {
        let mut document = TextDocument::new("foo\nbar".to_string());

        assert_eq!(document.offset_at(&LspPosition::new(1, 0)), 4);

        document.update(&TextDocumentContentChangeEvent {
            text: "foobar".to_string(),
            range: Some(LspRange::new(
                LspPosition::new(1, 0),
                LspPosition::new(1, 3),
            )),
            range_length: None,
        });
        assert_eq!(document.text(), "foo\nfoobar");
        assert_eq!(document.offset_at(&LspPosition::new(1, 0)), 4);
        assert_valid_line_numbers(&document);
    }

    /* TODO: Not clear that these should pass.

    #[test]
    fn invalid_update_ranges() {
        // // Before the document starts -> before the document starts
        // let mut document = TextDocument::new("foo\nbar".to_string());
        // document.update(&TextDocumentContentChangeEvent {
        // 	text: "abc123",
        // 	range: Some(LspRange::new(LspPosition::new(-2, 0), LspPosition::new(-1, 3))),
        // 	range_length: None,
        // });
        // assert_eq!(document.text(), "abc123foo\nbar");
        // assert_valid_line_numbers(&document);

        // // Before the document starts -> the middle of document
        // let mut document = TextDocument::new("foo\nbar".to_string());
        // document.update(&TextDocumentContentChangeEvent {
        // 	text: "foobar".to_string(),
        // 	range: Some(LspRange::new(LspPosition::new(-1, 0), LspPosition::new(0, 3))),
        // 	range_length: None,
        // });
        // assert_eq!(document.text(), "foobar\nbar");
        // assert_eq!(document.offset_at(&LspPosition::new(1, 0)), 7);
        // assert_valid_line_numbers(&document);

        // The middle of document -> after the document ends
        let mut document = TextDocument::new("foo\nbar".to_string());
        document.update(&TextDocumentContentChangeEvent {
            text: "foobar".to_string(),
            range: Some(LspRange::new(LspPosition::new(1, 0), LspPosition::new(1, 10))),
            range_length: None,
        });
        assert_eq!(document.text(), "foo\nfoobar");
        assert_eq!(document.offset_at(&LspPosition::new(1, 1000)), 10);
        assert_valid_line_numbers(&document);

        // After the document ends -> after the document ends
        let mut document = TextDocument::new("foo\nbar".to_string());
        document.update(&TextDocumentContentChangeEvent {
            text: "abc123".to_string(),
            range: Some(LspRange::new(LspPosition::new(3, 0), LspPosition::new(6, 10))),
            range_length: None,
        });
        assert_eq!(document.text(), "foo\nbarabc123");
        assert_valid_line_numbers(&document);

        // // Before the document starts -> after the document ends
        // let mut document = TextDocument::new("foo\nbar".to_string());
        // document.update(&TextDocumentContentChangeEvent {
        // 	text: "entirely new content".to_string(),
        // 	range: Some(LspRange::new(LspPosition::new(-1, 1), LspPosition::new(2, 10000))),
        // 	range_length: None,
        // });
        // assert_eq!(document.text(), "entirely new content");
        // assert_eq!(document.line_count(), 1);
        // assert_valid_line_numbers(&document);
    }

    */

    // TODO: Test non-ASCII characters, emojis, etc.
    // TODO: Fuzzing!
}
