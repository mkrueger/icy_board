from pathlib import Path
import tempfile
import unittest

from sphinx.application import Sphinx

from docs.source.reference_links import repository_url, source_target


class SourceLinkTests(unittest.TestCase):
    def test_relative_target(self) -> None:
        root = Path.cwd()
        self.assertEqual(
            Path("crates/example.rs"),
            source_target(root, "docs/reference/tools", "../../crates/example.rs"),
        )

    def test_encoded_target(self) -> None:
        root = Path.cwd()
        self.assertEqual(
            Path("data/sample file.txt"),
            source_target(root, "docs/index", "../data/sample%20file.txt"),
        )

    def test_target_cannot_escape_source_tree(self) -> None:
        with self.assertRaises(ValueError):
            source_target(Path.cwd(), "index", "../outside.txt")

    def test_repository_url_quotes_branch_and_path(self) -> None:
        self.assertEqual(
            "https://github.com/example/board/blob/release%2F1/data/sample%20file.txt#L12",
            repository_url("https://github.com/example/board/", "release/1", Path("data/sample file.txt"), "L12"),
        )

    def test_build_does_not_depend_on_or_distribute_local_reference_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs").mkdir()
            (root / "crates").mkdir()
            (root / "target").mkdir()
            (root / "pcboard").mkdir()
            (root / "crates/example.rs").write_text("// public source\n", encoding="utf-8")
            (root / "target/oracle.log").write_text("LOCAL TEST OUTPUT", encoding="utf-8")
            (root / "pcboard/ORIGINAL.C").write_text("LOCAL REFERENCE SOURCE", encoding="utf-8")
            (root / "conf.py").write_text(
                'extensions = ["myst_parser", "docs.source.reference_links"]\n'
                'root_doc = "docs/index"\n'
                'html_theme_options = {"source_repository": "https://github.com/example/board", "source_branch": "main"}\n',
                encoding="utf-8",
            )
            (root / "docs/index.md").write_text(
                "# Reference\n\n"
                "[Source](../crates/example.rs)\n\n"
                "[Local log](../target/oracle.log)\n\n"
                "[Original](../pcboard/ORIGINAL.C)\n\n"
                "[Absent log](../target/absent.log)\n\n"
                "[Absent original](../pcboard/ABSENT.C)\n",
                encoding="utf-8",
            )
            output = root / "output"
            app = Sphinx(
                str(root), str(root), str(output), str(root / "doctrees"),
                "html", status=None, warningiserror=True, freshenv=True,
                confoverrides={"html_theme": "furo"},
            )
            app.build(force_all=True)
            self.assertEqual(0, app.statuscode)
            html = (output / "docs/index.html").read_text(encoding="utf-8")
            self.assertIn("https://github.com/example/board/blob/main/crates/example.rs", html)
            self.assertIn("Local log (local reference)", html)
            self.assertIn("Absent log (local reference)", html)
            self.assertIn("Original (local reference)", html)
            self.assertIn("Absent original (local reference)", html)
            self.assertEqual([], list((output / "_downloads").rglob("*")))


if __name__ == "__main__":
    unittest.main()
