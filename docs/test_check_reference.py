from pathlib import Path
import tempfile
import unittest

from docs.check_reference import check


class ReferenceCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.output = self.root / "html"
        self.write(
            self.root / "crates/icy_board_help/data/catalog.toml",
            '[[topics]]\nid = "hlp@"\n',
        )
        self.write(self.output / "index.html", '<a href="docs/source/index.html">Start</a>')
        self.write(self.output / "docs/source/index.html", '<h1 id="start">Reference</h1>')
        self.write(self.output / "search.html", "<h1>Search</h1>")
        self.write(self.output / "searchindex.js", "Search.setIndex({});")
        self.write(
            self.output / "docs/reference/commands.html",
            '<a href="../../crates/icy_board_help/data/en/hlp%40.html#usage">Inbox</a>',
        )
        self.write(
            self.output / "crates/icy_board_help/data/en/hlp@.html",
            '<h1 id="usage">Inbox</h1>',
        )

    def write(self, path: Path, text: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_valid_site_and_encoded_topic(self) -> None:
        self.assertEqual([], check(self.output, self.root))

    def test_missing_local_page_and_asset(self) -> None:
        self.write(
            self.output / "index.html",
            '<a href="missing.html">Missing</a><img src="missing.png">',
        )
        errors = check(self.output, self.root)
        self.assertTrue(any("missing local target missing.html" in error for error in errors))
        self.assertTrue(any("missing local target missing.png" in error for error in errors))

    def test_missing_fragment(self) -> None:
        self.write(self.output / "index.html", '<a href="docs/source/index.html#absent">Missing</a>')
        self.assertTrue(any("missing anchor" in error for error in check(self.output, self.root)))

    def test_local_directory_and_named_anchor(self) -> None:
        self.write(self.output / "index.html", '<a href="docs/source/#old">Start</a>')
        self.write(self.output / "docs/source/index.html", '<a name="old"></a>')
        self.assertEqual([], check(self.output, self.root))

    def test_external_links_are_not_fetched(self) -> None:
        self.write(
            self.output / "index.html",
            '<a href="https://example.invalid/#missing">External</a>'
            '<a href="//example.invalid/path">External</a>'
            '<a href="mailto:sysop@example.invalid">Mail</a>',
        )
        self.assertEqual([], check(self.output, self.root))

    def test_paths_cannot_escape_the_html_bundle(self) -> None:
        self.write(self.root / "outside.html", "<h1>Outside</h1>")
        self.write(self.output / "index.html", '<a href="../outside.html">Outside</a>')
        self.assertTrue(any("missing local target" in error for error in check(self.output, self.root)))

    def test_topic_must_be_linked_from_command_index(self) -> None:
        self.write(self.output / "docs/reference/commands.html", "<h1>Commands</h1>")
        self.assertTrue(any("does not link to" in error for error in check(self.output, self.root)))

    def test_missing_topic(self) -> None:
        (self.output / "crates/icy_board_help/data/en/hlp@.html").unlink()
        self.assertTrue(any("Missing command topic" in error for error in check(self.output, self.root)))

    def test_missing_search_index(self) -> None:
        (self.output / "searchindex.js").unlink()
        self.assertTrue(any("entry point: searchindex.js" in error for error in check(self.output, self.root)))

    def test_empty_output(self) -> None:
        self.assertEqual(
            [f"No HTML pages found in {self.root / 'absent'}"],
            check(self.root / "absent", self.root),
        )

    def test_source_and_local_logs_are_not_bundled(self) -> None:
        self.write(self.output / "_downloads/hash/oracle.log", "Local oracle output")
        self.assertTrue(any("Unexpected bundled" in error for error in check(self.output, self.root)))

    def test_screenshot_downloads_are_allowed(self) -> None:
        self.write(self.output / "_downloads/hash/screenshot.png", "Image fixture")
        self.assertEqual([], check(self.output, self.root))


if __name__ == "__main__":
    unittest.main()
