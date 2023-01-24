// Initial implementation will just use walkdir to re-read all the files
// every 30 seconds.

use crate::file::File;
use std::{
    collections::{HashMap, HashSet},
    fs,
};
use tower_lsp::lsp_types::Url;
use walkdir::WalkDir;

#[derive(Default)]
pub struct Files {
    folders: HashSet<Url>,
    files: HashMap<Url, File>,
}

pub fn scan_folders(folders: HashSet<Url>) -> HashMap<Url, File> {
    let mut files = HashMap::new();

    for folder in folders {
        if folder.scheme() != "file" {
            continue;
        }
        if let Ok(path) = folder.to_file_path() {
            for entry in WalkDir::new(path) {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file()
                        && entry.path().extension() == Some("sail".as_ref())
                    {
                        let path = entry.path();
                        if let Ok(source) = fs::read_to_string(path) {
                            let file = File::new(source);
                            if let Some(path_str) = path.to_str() {
                                let mut url = folder.clone();
                                dbg!(&url, &path_str);
                                // TODO: This is a hack to get around Windows paths and
                                // a bug in Url::set_path. https://github.com/servo/rust-url/issues/864
                                let mut path_windows = path_str.replace("\\", "/");
                                if !path_windows.starts_with('/') {
                                    path_windows.insert(0, '/');
                                }
                                url.set_path(&path_windows);
                                files.insert(url, file);
                            }
                        }
                    }
                }
            }
        }
    }

    files
}

impl Files {
    pub fn add_folder(&mut self, folder: Url) {
        self.folders.insert(folder);
    }

    pub fn remove_folder(&mut self, folder: &Url) {
        self.folders.remove(folder);
    }

    pub fn add_file(&mut self, url: Url, file: File) {
        self.files.insert(url, file);
    }

    pub fn remove_file(&mut self, url: &Url) {
        self.files.remove(url);
    }

    pub fn all_files(&self) -> impl Iterator<Item = (&Url, &File)> {
        self.files.iter()
    }

    pub fn update(&mut self, files: HashMap<Url, File>) {
        self.files = files;
    }

    pub fn folders(&self) -> &HashSet<Url> {
        &self.folders
    }
}
