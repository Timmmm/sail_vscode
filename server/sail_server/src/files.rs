// Initial implementation will just use walkdir to re-read all the files
// every 30 seconds.

use crate::file::File;
use std::{
    collections::{HashMap, HashSet},
    fs, path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Default)]
pub struct Files {
    folders: HashSet<PathBuf>,
    files: HashMap<PathBuf, File>,
}

pub fn scan_folders(folders: HashSet<PathBuf>) -> HashMap<PathBuf, File> {
    let mut files = HashMap::new();

    for folder in folders {
        for entry in WalkDir::new(folder) {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file()
                        && entry.path().extension() == Some("sail".as_ref())
                    {
                        let path = entry.path();
                        match fs::read_to_string(path) {
                            Ok(source) => {
                                let file = File::new(source);
                                files.insert(path.to_owned(), file);
                            }
                            Err(e) => {
                                eprintln!("Error reading file {}: {:?}", path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error scanning folder: {:?}", e);
                }
            }
        }
    }

    files
}

impl Files {
    pub fn add_folder(&mut self, folder: PathBuf) {
        self.folders.insert(folder);
    }

    pub fn remove_folder(&mut self, folder: &Path) {
        self.folders.remove(folder);
    }

    pub fn add_file(&mut self, url: PathBuf, file: File) {
        self.files.insert(url, file);
    }

    pub fn remove_file(&mut self, url: &Path) {
        self.files.remove(url);
    }

    pub fn all_files(&self) -> impl Iterator<Item = (&PathBuf, &File)> {
        self.files.iter()
    }

    pub fn update(&mut self, files: HashMap<PathBuf, File>) {
        self.files = files;
    }

    pub fn folders(&self) -> &HashSet<PathBuf> {
        &self.folders
    }
}
