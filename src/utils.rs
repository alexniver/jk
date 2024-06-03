use std::{cmp::Ordering, path::PathBuf};

pub fn sort_files(files: &mut Vec<PathBuf>) {
    files.sort_by(|a, b| {
        if a.is_dir() && b.is_file() {
            Ordering::Less
        } else if a.is_file() && b.is_dir() {
            Ordering::Greater
        } else {
            if let (Some(a_name), Some(b_name)) = (a.file_name(), b.file_name()) {
                a_name.cmp(b_name)
            } else {
                Ordering::Equal
            }
        }
    });
}
