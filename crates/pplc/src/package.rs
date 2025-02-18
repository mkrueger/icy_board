use semver::Version;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Package {
    name: String,
    version: Version,
    language_version: u16,
    authors: Vec<String>,
}

impl Package {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn authors(&self) -> &Vec<String> {
        &self.authors
    }

    pub fn language_version(&self) -> u16 {
        self.language_version
    }
}

#[derive(Debug, Deserialize)]
pub struct PackageData {
    pub text_files: Vec<String>,
    pub art_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub package: Package,
    pub data: PackageData,
}
