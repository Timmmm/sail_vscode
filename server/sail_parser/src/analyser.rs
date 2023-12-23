use std::collections::HashMap;

pub struct Analysis {
    pub definitions: HashMap<String, usize>,
    // pub diagnostics: Vec<Diagnostic>,
}

// fn analyse_file(file: &File) -> Analysis {
//     todo!()
//     // // Find the definitions, and also the uses of them.
//     // for def in file.defs {
//     //     match def {

//     //     }
//     // }
// }
