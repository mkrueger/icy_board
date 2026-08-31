use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    pub binary_files: Option<Vec<String>>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CompilerData {
    pub language_version: Option<u16>,
    pub defines: Option<Vec<String>>,
}

/// A source-library dependency. Path dependencies are relative to the manifest;
/// Git dependencies are cached below the root package's target directory.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub path: Option<PathBuf>,
    pub git: Option<String>,
    pub rev: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Workspace {
    #[serde(skip)]
    pub file_name: PathBuf,

    #[serde(skip)]
    pub hard_coded_files: Option<Vec<PathBuf>>,

    #[serde(skip)]
    dependency_files: HashSet<PathBuf>,

    #[serde(skip)]
    dependency_modules: HashMap<PathBuf, String>,

    pub package: Package,
    pub compiler: Option<CompilerData>,
    pub data: Option<PackageData>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, Dependency>,
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
            dependencies: BTreeMap::new(),
            formatting: None,
            hard_coded_files: None,
            dependency_files: HashSet::new(),
            dependency_modules: HashMap::new(),
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

        for entry in walkdir::WalkDir::new(base_path.join("src")).into_iter().flatten() {
            if !entry.path().is_file() {
                continue;
            }
            if let Some(ext) = entry.path().extension()
                && ext != "pps"
            {
                continue;
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

    /// Resolves all transitive source libraries and makes their `.pps` files
    /// available through [`Workspace::files`]. The root package remains first,
    /// so its `main.pps` keeps defining the program entry point.
    pub fn resolve_dependencies(&mut self) -> Res<()> {
        if self.hard_coded_files.is_some() || self.dependencies.is_empty() {
            return Ok(());
        }
        let manifest = fs::canonicalize(&self.file_name)?;
        let root = manifest
            .parent()
            .ok_or_else(|| format!("invalid package manifest path: {}", manifest.display()))?;
        let cache = root.join("target").join("ppl-dependencies").join("git");
        let mut files = self.files();
        let root_file_count = files.len();
        let mut visited = HashSet::from([manifest.clone()]);
        collect_dependency_files(&manifest, &self.dependencies, &cache, &mut visited, &mut files, &mut self.dependency_modules)?;
        self.dependency_files = files[root_file_count..].iter().cloned().collect();
        self.hard_coded_files = Some(files);
        Ok(())
    }

    pub fn is_dependency_file(&self, file: &Path) -> bool {
        self.dependency_files.contains(file)
    }

    /// Returns the manifest dependency name that acts as this source file's
    /// implicit module when the file has no explicit `MODULE` declaration.
    pub fn dependency_module(&self, file: &Path) -> Option<&str> {
        self.dependency_modules.get(file).map(String::as_str)
    }

    pub fn runtime(&self) -> u16 {
        self.package.runtime.unwrap_or(LAST_PPE_RUNTIME)
    }

    pub fn language_version(&self) -> u16 {
        if let Some(compiler) = &self.compiler
            && let Some(language_version) = compiler.language_version
        {
            return language_version;
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

fn collect_dependency_files(
    owner_manifest: &Path,
    dependencies: &BTreeMap<String, Dependency>,
    cache: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    dependency_modules: &mut HashMap<PathBuf, String>,
) -> Res<()> {
    let owner_dir = owner_manifest
        .parent()
        .ok_or_else(|| format!("invalid package manifest path: {}", owner_manifest.display()))?;
    for (name, dependency) in dependencies {
        let package_dir = dependency.resolve(name, owner_dir, cache)?;
        let manifest = fs::canonicalize(package_dir.join("ppl.toml"))
            .map_err(|err| format!("dependency '{name}' has no readable ppl.toml at {}: {err}", package_dir.display()))?;
        if !visited.insert(manifest.clone()) {
            continue;
        }
        let workspace = Workspace::load(&manifest)?;
        collect_dependency_files(&manifest, &workspace.dependencies, cache, visited, files, dependency_modules)?;
        for file in workspace.files() {
            dependency_modules.insert(file.clone(), name.clone());
            files.push(file);
        }
    }
    Ok(())
}

impl Dependency {
    fn resolve(&self, name: &str, owner_dir: &Path, cache: &Path) -> Res<PathBuf> {
        match (&self.path, &self.git) {
            (Some(path), None) => {
                if self.rev.is_some() || self.branch.is_some() || self.tag.is_some() {
                    return Err(format!("path dependency '{name}' cannot specify rev, branch, or tag").into());
                }
                Ok(fs::canonicalize(owner_dir.join(path))?)
            }
            (None, Some(git)) => self.resolve_git(name, git, cache),
            (Some(_), Some(_)) => Err(format!("dependency '{name}' must specify either path or git, not both").into()),
            (None, None) => Err(format!("dependency '{name}' must specify path or git").into()),
        }
    }

    fn resolve_git(&self, name: &str, git: &str, cache: &Path) -> Res<PathBuf> {
        let references = [self.rev.as_ref(), self.branch.as_ref(), self.tag.as_ref()].into_iter().flatten().count();
        if references > 1 {
            return Err(format!("Git dependency '{name}' may specify only one of rev, branch, or tag").into());
        }
        let selector = self
            .rev
            .as_ref()
            .map(|value| format!("rev={value}"))
            .or_else(|| self.branch.as_ref().map(|value| format!("branch={value}")))
            .or_else(|| self.tag.as_ref().map(|value| format!("tag={value}")))
            .unwrap_or_else(|| "default".to_string());
        let digest = Sha256::digest(format!("{git}\0{selector}").as_bytes());
        let safe_name: String = name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
            .collect();
        let digest = format!("{digest:x}");
        let destination = cache.join(format!("{safe_name}-{}", &digest[..16]));
        if destination.join("ppl.toml").is_file() {
            if self.rev.is_none() && self.tag.is_none() {
                let destination_name = destination.to_string_lossy().into_owned();
                let reference = self.branch.as_deref().unwrap_or("HEAD");
                run_git(["-C", destination_name.as_str(), "fetch", "--quiet", "--depth", "1", "origin", reference])?;
                run_git(["-C", destination_name.as_str(), "checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
            }
            return Ok(destination);
        }

        fs::create_dir_all(cache)?;
        let temporary = cache.join(format!(".tmp-{}-{}", std::process::id(), fastrand::u64(..)));
        if temporary.exists() {
            fs::remove_dir_all(&temporary)?;
        }
        let temporary_name = temporary.to_string_lossy().into_owned();
        let result = if let Some(rev) = &self.rev {
            run_git(["init", "--quiet", temporary_name.as_str()])?;
            run_git(["-C", temporary_name.as_str(), "remote", "add", "origin", git])?;
            run_git(["-C", temporary_name.as_str(), "fetch", "--quiet", "--depth", "1", "origin", rev])?;
            run_git(["-C", temporary_name.as_str(), "checkout", "--quiet", "--detach", "FETCH_HEAD"])
        } else {
            let selected = self.branch.as_ref().or(self.tag.as_ref());
            let mut arguments = vec!["clone", "--quiet", "--depth", "1"];
            if let Some(selected) = selected {
                arguments.extend(["--branch", selected]);
            }
            arguments.extend([git, temporary_name.as_str()]);
            run_git(arguments)
        };
        if let Err(err) = result {
            let _ = fs::remove_dir_all(&temporary);
            return Err(err);
        }
        if !temporary.join("ppl.toml").is_file() {
            let _ = fs::remove_dir_all(&temporary);
            return Err(format!("Git dependency '{name}' does not contain ppl.toml at its repository root").into());
        }
        match fs::rename(&temporary, &destination) {
            Ok(()) => {}
            Err(_) if destination.join("ppl.toml").is_file() => {
                let _ = fs::remove_dir_all(&temporary);
            }
            Err(err) => return Err(err.into()),
        }
        Ok(destination)
    }
}

fn run_git<I, S>(arguments: I) -> Res<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .args(arguments)
        .output()
        .map_err(|err| format!("failed to run Git: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    Err(format!("Git dependency resolution failed: {}", message.trim()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::PPECompiler,
        parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
    };
    use std::sync::{Arc, Mutex};

    fn package(path: &Path, name: &str, dependencies: BTreeMap<String, Dependency>) -> Workspace {
        fs::create_dir_all(path.join("src")).unwrap();
        let mut workspace = Workspace::default();
        workspace.package.name = name.to_string();
        workspace.file_name = path.join("ppl.toml");
        workspace.dependencies = dependencies;
        workspace.save(&workspace.file_name).unwrap();
        workspace
    }

    fn path_dependency(path: &str) -> Dependency {
        Dependency {
            path: Some(PathBuf::from(path)),
            git: None,
            rev: None,
            branch: None,
            tag: None,
        }
    }

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

    #[test]
    fn resolves_transitive_path_library_sources() {
        let temp = tempfile::tempdir().unwrap();
        let common = package(&temp.path().join("common"), "common", BTreeMap::new());
        fs::write(common.file_name.parent().unwrap().join("src/common.pps"), "MODULE Common\nENDMODULE\n").unwrap();

        let library_dependencies = BTreeMap::from([("common".to_string(), path_dependency("../common"))]);
        let library = package(&temp.path().join("library"), "library", library_dependencies);
        fs::write(library.file_name.parent().unwrap().join("src/library.pps"), "MODULE Library\nENDMODULE\n").unwrap();

        let root_dependencies = BTreeMap::from([("library".to_string(), path_dependency("../library"))]);
        let mut root = package(&temp.path().join("application"), "application", root_dependencies);
        fs::write(root.file_name.parent().unwrap().join("src/main.pps"), "IMPORT Library AS Lib\n").unwrap();

        root.resolve_dependencies().unwrap();
        let names = root
            .files()
            .iter()
            .map(|file| file.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(vec!["main.pps", "common.pps", "library.pps"], names);
        assert!(!root.is_dependency_file(&root.files()[0]));
        assert!(root.is_dependency_file(&root.files()[1]));
    }

    #[test]
    fn resolves_revision_pinned_git_library_sources() {
        let temp = tempfile::tempdir().unwrap();
        let repository = package(&temp.path().join("repository"), "library", BTreeMap::new());
        fs::write(repository.file_name.parent().unwrap().join("src/library.pps"), "MODULE Library\nENDMODULE\n").unwrap();
        let repository_dir = repository.file_name.parent().unwrap();
        run_git(["init", "--quiet", repository_dir.to_string_lossy().as_ref()]).unwrap();
        run_git(["-C", repository_dir.to_string_lossy().as_ref(), "config", "user.name", "PPL Test"]).unwrap();
        run_git(["-C", repository_dir.to_string_lossy().as_ref(), "config", "user.email", "ppl@example.invalid"]).unwrap();
        run_git(["-C", repository_dir.to_string_lossy().as_ref(), "add", "."]).unwrap();
        run_git(["-C", repository_dir.to_string_lossy().as_ref(), "commit", "--quiet", "-m", "library"]).unwrap();
        let revision = Command::new("git")
            .args(["-C", repository_dir.to_string_lossy().as_ref(), "rev-parse", "HEAD"])
            .output()
            .unwrap();
        let revision = String::from_utf8(revision.stdout).unwrap().trim().to_string();

        let dependency = Dependency {
            path: None,
            git: Some(repository_dir.to_string_lossy().into_owned()),
            rev: Some(revision),
            branch: None,
            tag: None,
        };
        let mut root = package(
            &temp.path().join("application"),
            "application",
            BTreeMap::from([("library".to_string(), dependency)]),
        );
        fs::write(root.file_name.parent().unwrap().join("src/main.pps"), "IMPORT Library AS Lib\n").unwrap();

        root.resolve_dependencies().unwrap();
        assert_eq!(2, root.files().len());
        let library = &root.files()[1];
        assert!(root.is_dependency_file(library));
        assert!(library.starts_with(root.file_name.parent().unwrap().join("target/ppl-dependencies/git")));
    }

    #[test]
    fn imports_a_plain_path_library_as_a_virtual_module() {
        let temp = tempfile::tempdir().unwrap();
        let library = package(&temp.path().join("themes"), "themes", BTreeMap::new());
        fs::write(
            library.file_name.parent().unwrap().join("src/colors.pps"),
            "PRIVATE\nINTEGER InternalColor = 1\nPUBLIC\nINTEGER DefaultColor = 7\n",
        )
        .unwrap();
        fs::write(
            library.file_name.parent().unwrap().join("src/theme.pps"),
            "PROCEDURE Apply()\n  COLOR InternalColor + DefaultColor\nENDPROC\n",
        )
        .unwrap();
        let mut root = package(
            &temp.path().join("application"),
            "application",
            BTreeMap::from([("themes".to_string(), path_dependency("../themes"))]),
        );
        fs::write(
            root.file_name.parent().unwrap().join("src/main.pps"),
            "IMPORT themes AS MyTheme\nMyTheme.Apply()\nPRINTLN MyTheme.DefaultColor\n",
        )
        .unwrap();
        root.resolve_dependencies().unwrap();

        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let registry = UserTypeRegistry::icy_board_registry();
        let sources = root
            .files()
            .into_iter()
            .map(|file| {
                let source = fs::read_to_string(&file).unwrap();
                preparse_type_declarations(file.clone(), errors.clone(), &source, &registry, Encoding::Utf8, &root);
                (file, source)
            })
            .collect::<Vec<_>>();
        let asts = sources
            .iter()
            .map(|(file, source)| parse_ast_with_predeclared_types(file.clone(), errors.clone(), source, &registry, Encoding::Utf8, &root))
            .collect::<Vec<_>>();
        PPECompiler::new(&root, registry, errors.clone()).compile(&asts.iter().collect::<Vec<_>>());

        let messages = errors
            .lock()
            .unwrap()
            .errors
            .iter()
            .map(|error| format!("{}: {}", error.file_name.display(), error.error))
            .collect::<Vec<_>>();
        assert!(messages.is_empty(), "library module should compile: {messages:?}");
    }
}
