use std::{
    fs,
    path::{Path, PathBuf},
};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    Res,
    executable::{LAST_PPE_RUNTIME, LAST_PPL_LANGUAGE_VERSION},
    formatting::FormattingOptions,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: Version,
    pub runtime: Option<u16>,
    pub authors: Option<Vec<String>>,
}

impl Package {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn authors(&self) -> &Option<Vec<String>> {
        &self.authors
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageData {
    pub text_files: Option<Vec<String>>,
    pub art_files: Option<Vec<String>>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CompilerData {
    pub language_version: Option<u16>,
    pub defines: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Workspace {
    #[serde(skip)]
    pub file_name: PathBuf,

    #[serde(skip)]
    pub hard_coded_files: Option<Vec<PathBuf>>,

    pub package: Package,
    pub compiler: Option<CompilerData>,
    pub data: Option<PackageData>,
    formatting: Option<FormattingOptions>,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            file_name: PathBuf::new(),
            package: Package {
                name: String::new(),
                runtime: None,
                version: Version::new(0, 1, 0),
                authors: None,
            },
            compiler: None,
            data: None,
            formatting: None,
            hard_coded_files: None,
        }
    }
}
impl Workspace {
    pub fn formatting(&self) -> &FormattingOptions {
        self.formatting.as_ref().unwrap_or(&FormattingOptions::DEFAULT)
    }

    pub fn load<P: AsRef<Path>>(file_name: P) -> Res<Self> {
        let toml_str = fs::read_to_string(file_name.as_ref())?;
        let mut res: Workspace = toml::from_str(&toml_str)?;
        res.file_name = file_name.as_ref().to_path_buf();
        Ok(res)
    }

    pub fn save<P: AsRef<Path>>(&self, file_name: P) -> Res<()> {
        let toml_str = toml::to_string(self)?;
        fs::write(file_name.as_ref(), toml_str)?;
        Ok(())
    }

    pub fn target_path(&self, version: u16) -> PathBuf {
        let Some(base_path) = self.file_name.parent() else {
            return PathBuf::from("target");
        };

        let path = match version {
            100 => "pcboard_15.0",
            200 => "pcboard_15.10",
            300 => "pcboard_15.20",
            310 => "pcboard_15.21",
            320 => "pcboard_15.22",
            330 => "pcboard_15.30",
            340 => "pcboard_15.40",
            _ => "icboard",
        };
        base_path.join("target").join(path)
    }

    pub fn files(&self) -> Vec<PathBuf> {
        if let Some(hard_coded_files) = &self.hard_coded_files {
            return hard_coded_files.clone();
        }
        let mut files = Vec::new();
        let Some(base_path) = self.file_name.parent() else {
            return files;
        };

        for entry in walkdir::WalkDir::new(&base_path.join("src")).into_iter().flatten() {
            if !entry.path().is_file() {
                continue;
            }
            if let Some(ext) = entry.path().extension() {
                if ext != "pps" {
                    continue;
                }
            }
            files.push(entry.path().to_path_buf());
        }

        files.sort_by(|a, b| {
            if a.file_stem().unwrap() == "main" {
                std::cmp::Ordering::Less
            } else if b.file_stem().unwrap() == "main" {
                std::cmp::Ordering::Greater
            } else {
                a.cmp(b)
            }
        });
        files
    }

    pub fn runtime(&self) -> u16 {
        self.package.runtime.unwrap_or(LAST_PPE_RUNTIME)
    }

    pub fn language_version(&self) -> u16 {
        if let Some(compiler) = &self.compiler {
            if let Some(language_version) = compiler.language_version {
                return language_version;
            }
        }
        self.runtime().min(LAST_PPL_LANGUAGE_VERSION)
    }

    /// Takes a language version the caller found elsewhere, for a workspace that
    /// does not state one itself. A manifest is explicit, so it is left alone.
    pub fn set_default_language_version(&mut self, version: Option<u16>) {
        let Some(version) = version else {
            return;
        };
        if self.compiler.as_ref().and_then(|compiler| compiler.language_version).is_some() {
            return;
        }
        self.compiler.get_or_insert_with(CompilerData::default).language_version = Some(version);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_runtime_and_language_are_independent() {
        let workspace = Workspace::default();
        assert_eq!(LAST_PPE_RUNTIME, workspace.runtime());
        assert_eq!(LAST_PPL_LANGUAGE_VERSION, workspace.language_version());
    }

    #[test]
    fn an_older_runtime_remains_the_default_language() {
        let mut workspace = Workspace::default();
        workspace.package.runtime = Some(340);
        assert_eq!(340, workspace.language_version());
    }

    #[test]
    fn a_default_language_version_is_taken_when_none_is_stated() {
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(Some(340));
        assert_eq!(340, workspace.language_version());
    }

    #[test]
    fn a_stated_language_version_wins_over_the_default() {
        let mut workspace = Workspace::default();
        workspace.compiler.get_or_insert_with(CompilerData::default).language_version = Some(350);
        workspace.set_default_language_version(Some(340));
        assert_eq!(350, workspace.language_version());
    }

    #[test]
    fn no_default_leaves_the_workspace_alone() {
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(None);
        assert_eq!(LAST_PPL_LANGUAGE_VERSION, workspace.language_version());
    }
}
