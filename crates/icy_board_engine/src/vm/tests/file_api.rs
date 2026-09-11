use std::sync::Arc;

use crate::icy_board::{
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
};
use dizbase::file_base::FileBase;

#[test]
fn a4_file_pages_roundtrip_metadata_permissions_and_defaults() {
    let root = tempfile::tempdir().unwrap();
    let metadata = root.path().join("dir");
    let path = root.path().join("large.zip");
    std::fs::File::create(&path).unwrap().set_len(2_147_483_648).unwrap();
    let mut base = FileBase::open(root.path(), &metadata).unwrap();
    base.set_description(&path, "Useful tools").unwrap();
    drop(base);
    let output = super::run_ppl_on(
        r#"
FILEENTRY missing
FILEPAGE unset
PRINTLN missing.Valid, ":", missing.Size, ":", unset.Valid, ":", unset.Entries.Len()
DIRECTORY directory = Board.Conferences[0].Directories[0]
FILEPAGE page = directory.Find("TOOLS", 0, 1)
ERROR failure = Error.Last()
PRINTLN page.Valid, ":", failure.OK, ":", page.Entries.Len(), ":", page.HasMore
FILEENTRY entry = page.Entries[0]
PRINTLN entry.Valid, ":", entry.Name, ":", entry.Description, ":", entry.Size, ":", entry.Date > 0, ":", entry.DescriptionTruncated
page = directory.Find("", page.NextAfter)
PRINTLN page.Valid, ":", page.Entries.Len(), ":", page.HasMore
page = directory.Find("absent")
PRINTLN page.Valid, ":", page.Entries.Len(), ":", Error.Last().OK
page = directory.Find("", -1, 10)
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Invalid
page = directory.Find("", 0, 101)
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Limit
page = Board.Conferences[0].Directories[1].Find("")
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Denied
page = Board.Conferences[1].Directories[0].Find("")
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Denied
DIRECTORY invalid
page = invalid.Find("")
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Invalid
page = directory.Find("", 0, 0)
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Invalid
page = directory.Find(STRING.Repeat("x", 1025))
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Limit
page = Board.Conferences[0].Directories[2].Find("")
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.Unavailable
page = Board.Conferences[0].Directories[3].Find("")
PRINTLN page.Valid, ":", Error.Last().Code = ErrCode.IO
page = directory.Find("")
PRINTLN page.Valid, ":", Error.Last().OK
EXIT
"#,
        |board| {
            let mut directories = DirectoryList::default();
            directories.push(FileDirectory {
                name: "Files".into(),
                path: root.path().into(),
                metadata_path: metadata.clone(),
                ..Default::default()
            });
            directories.push(FileDirectory {
                name: "Denied".into(),
                path: root.path().into(),
                metadata_path: metadata.clone(),
                list_security: "FALSE".parse().unwrap(),
                ..Default::default()
            });
            directories.push(FileDirectory {
                name: "Missing".into(),
                path: root.path().into(),
                metadata_path: root.path().join("missing"),
                ..Default::default()
            });
            let corrupt = root.path().join("corrupt");
            std::fs::write(FileBase::database_path(&corrupt), b"not sqlite").unwrap();
            directories.push(FileDirectory {
                name: "Corrupt".into(),
                path: root.path().into(),
                metadata_path: corrupt,
                ..Default::default()
            });
            board.conferences.clear();
            board.conferences.push(Conference {
                directories: Some(Arc::new(directories)),
                ..Default::default()
            });
            let mut denied = board.conferences[0].clone();
            denied.required_security = "FALSE".parse().unwrap();
            board.conferences.push(denied);
        },
    );
    assert_eq!(
        output,
        "0:0:0:0\n1:1:1:0\n1:large.zip:Useful tools:2147483648:1:0\n1:0:0\n1:0:1\n0:1\n0:1\n0:1\n0:1\n0:1\n0:1\n0:1\n0:1\n0:1\n1:1\n"
    );
    assert!(!FileBase::database_path(root.path().join("missing")).exists());
}

#[test]
fn a4_file_pages_require_runtime_400_and_readonly_members() {
    for source in [
        "FILEENTRY entry\nPRINT entry.Valid",
        "FILEPAGE page\nPRINT page.Valid",
        "DECLARE FUNCTION GetPage() FILEPAGE\nPRINT GetPage().Valid\nEXIT\nFUNCTION GetPage() FILEPAGE\nENDFUNC",
    ] {
        let registry = crate::parser::UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(std::sync::Mutex::new(crate::parser::ErrorReporter::default()));
        let mut workspace = crate::compiler::workspace::Workspace::default();
        workspace.package.runtime = Some(340);
        workspace.set_default_language_version(Some(400));
        let ast = crate::parser::parse_ast("test.pps".into(), errors.clone(), source, &registry, crate::parser::Encoding::Utf8, &workspace);
        let mut compiler = crate::compiler::PPECompiler::new(&workspace, registry, errors.clone());
        compiler.compile(&[&ast]);
        assert!(
            errors.lock().unwrap().has_errors() || compiler.create_executable().map_or(true, |executable| executable.to_buffer().is_err()),
            "{source}"
        );
        assert!(super::compile_errors_with_runtime(source, 400).is_empty());
    }
    for source in [
        "FILEENTRY entry\nentry.Size = 0",
        "FILEENTRY entry\nentry.Description = \"changed\"",
        "FILEPAGE page\npage.NextAfter = 0",
        "DIRECTORY directory\nFILEPAGE page = directory.Find()",
        "DIRECTORY directory\nFILEPAGE page = directory.Find(\"\", 0, 15, 1)",
        "DIRECTORY directory\nBOOLEAN marked = directory.Flag()",
        "DIRECTORY directory\nBOOLEAN marked = directory.Flag(\"one\", \"two\")",
    ] {
        assert!(!super::compile_errors_with_runtime(source, 400).is_empty(), "{source}");
    }
}
