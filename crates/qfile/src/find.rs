use super::{Directory, PathBuf};
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::mpsc::{SendError, Sender};
use walkdir::WalkDir;
/*
The pathfinder module in this code contains functions that are used to find file paths on a system based on certain search criteria. It provides the implementation for the find_paths function which is the main entry point for the path-finding logic.

The get_paths function is used to obtain the list of paths to be searched based on the input directory place which can either be the root directory or all directories on the system. The get_excluded_dirs function is used to obtain the list of directories to be excluded from the search based on the excluded_dirs parameter which is an optional list of directory names.

The find_matching_paths function is where the actual path-finding algorithm is implemented. It takes the list of paths to be searched, the list of names to match against, the list of excluded directories, whether to follow symlinks, and a Sender object used to send the matched file paths.

It uses the rayon crate to parallelize the search over multiple threads for better performance. The algorithm first filters out the excluded directories, and then iterates through the remaining directories to find all the files that match the specified search criteria. If a match is found, the path of the file is sent to the Sender object.

Finally, the find_paths function calls get_paths, get_excluded_dirs, and find_matching_paths to obtain the search parameters and perform the search using the specified Sender object.
 */
pub mod pathfinder {
    use super::*;
    // Get all possible paths to search for files, depending on the directory type
    fn get_paths<T: AsRef<str> + Send + Sync>(place: Directory<T>) -> Vec<String> {
        match place {
            Directory::ThisPlace(root_d) => root_d.iter().map(|x| x.as_ref().to_owned()).collect::<Vec<String>>(),
            Directory::Everywhere => {
                let mut paths = Vec::new();
                if cfg!(unix) {
                    paths.push("/".to_string());
                }
                if cfg!(windows) {
                    for disk in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect::<Vec<char>>() {
                        let temp = format!("{}:\\", disk);
                        if std::path::PathBuf::from(&temp).exists() {
                            paths.push(temp.to_string());
                        }
                    }
                }
                paths
            }
        }
    }
    // Get all excluded directories to skip while searching for files
    fn get_excluded_dirs<E: AsRef<str> + Send + Sync>(excluded_dirs: Option<Vec<E>>) -> HashSet<String> {
        match excluded_dirs {
            Some(values) => values.iter().map(|x| x.as_ref().to_owned()).collect(),
            None => HashSet::new(),
        }
    }
    // Search for all matching files in given paths, based on provided names to search for
    fn find_matching_paths(
        paths: Vec<String>,
        names: Vec<String>,
        excluded_dirs: HashSet<String>,
        follow_link: bool,
        sender: Sender<PathBuf>,
    ) -> Result<(), SendError<PathBuf>> {
        // Spawn a new rayon thread and divide the work into multiple threads to search for matching files
        rayon::spawn(move || {
            paths
                .par_iter() // Parallelize the search for paths
                .for_each_with(sender.clone(), |sender, element| {
                    // Exclude all other paths while searching in a particular path
                    let paths_nch = paths.iter().filter(|x| **x != *element).collect::<Vec<&String>>();
                    let mut excluded_dirs: HashSet<&String> = excluded_dirs.iter().map(|x| x).collect();
                    excluded_dirs.extend(paths_nch);
                    // Walk through each directory entry and search for matching files
                    WalkDir::new(element)
                        .follow_links(follow_link)
                        .into_iter()
                        // Filter out all excluded directories
                        .filter_entry(|entry| !excluded_dirs.contains(&entry.path().display().to_string()))
                        .filter_map(|e| e.ok())
                        .collect::<Vec<_>>()
                        .par_iter() // Parallelize the search for files in each directory entry
                        .for_each_with(sender.clone(), |sender, entry| {
                            names
                                .par_iter() // Parallelize the search for each name in each file
                                .for_each_with(sender.clone(), |sender, name| {
                                    if entry.path().display().to_string().to_lowercase().contains(&name.to_string().to_lowercase()) {
                                        if let Err(err) = sender.send(entry.path().to_path_buf().into()) {
                                            panic!("{}", err);
                                        }
                                    }
                                })
                        });
                });
        });

        Ok(())
    }
    pub fn find_paths<T: AsRef<str> + Send + Sync>(
        place: Directory<T>,
        names: Vec<T>,
        excluded_dirs: Option<Vec<T>>,
        follow_link: bool,
        sender: Sender<PathBuf>,
    ) -> Result<(), SendError<PathBuf>> {
        // Get the paths to search
        let paths = get_paths(place);
        // Get the excluded directories as a HashSet
        let excluded_dirs = get_excluded_dirs(excluded_dirs);
        // Convert names to Vec<String>
        let names = names.iter().map(|x| x.as_ref().to_owned()).collect::<Vec<String>>();
        // Call find_matching_paths to start the search
        find_matching_paths(paths, names, excluded_dirs, follow_link, sender)
    }
}
