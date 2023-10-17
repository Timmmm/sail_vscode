use std::collections::HashMap;

use sail_parser::{Spanned, cst::DefAux};

// TODO: Definitions should be keyed by kind (at least type, overload, variable and function).
pub fn find_definitions(cst: &Spanned<Vec<Spanned<DefAux>>>, source: &str) -> HashMap<String, usize> {
    let mut definitions = HashMap::new();
    // Find the definitions, and also the uses of them.
    for (def, _span) in cst.0.iter() {
        match def {
            DefAux::OverloadDef(overload_def) => {
                let span = overload_def.id.1;
                if let Some(id) = source.get(span.into_range()) {
                    definitions.insert(id.to_owned(), span.start);
                }
            }
            _ => {}
        }
    }
    definitions
}
