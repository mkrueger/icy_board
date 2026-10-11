"""Check local HTML links, anchors and coverage of the shared help catalog."""

import argparse
from html.parser import HTMLParser
from pathlib import Path
import sys
import tomllib
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self, path: Path) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.feed(path.read_text(encoding="utf-8"))
        self.close()

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        identifier = attributes.get("id")
        if identifier:
            self.ids.add(identifier)
        if tag == "a" and attributes.get("name"):
            self.ids.add(str(attributes["name"]))
        for attribute in ("href", "src"):
            value = attributes.get(attribute)
            if value:
                self.links.append(value)


def check(output: Path, repository: Path) -> list[str]:
    output = output.resolve()
    pages = {path.resolve(): Page(path) for path in output.rglob("*.html")}
    errors: set[str] = set()
    if not pages:
        return [f"No HTML pages found in {output}"]

    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            target = (path.parent / unquote(url.path)).resolve() if url.path else path
            if target.is_dir():
                target /= "index.html"
            if not target.is_relative_to(output) or not target.is_file():
                errors.add(f"{path.relative_to(output)}: missing local target {link}")
                continue
            if url.fragment and target in pages:
                fragment = unquote(url.fragment)
                if fragment not in pages[target].ids:
                    errors.add(f"{path.relative_to(output)}: missing anchor {link}")

    for required in ("index.html", "docs/source/index.html", "search.html", "searchindex.js"):
        path = output / required
        if not path.is_file() or path.stat().st_size == 0:
            errors.add(f"Missing or empty site entry point: {required}")

    for path in (output / "_downloads").rglob("*"):
        if path.is_file() and path.suffix.lower() not in {".png", ".jpg", ".jpeg", ".svg", ".gif", ".webp"}:
            errors.add(f"Unexpected bundled source or data file: {path.relative_to(output)}")

    catalog = tomllib.loads(
        (repository / "crates/icy_board_help/data/catalog.toml").read_text(encoding="utf-8")
    )
    command_index = output / "docs/reference/commands.html"
    index = pages.get(command_index)
    if index is None:
        errors.add("Missing caller/sysop command index")
    else:
        indexed = {
            (command_index.parent / unquote(urlsplit(link).path)).resolve()
            for link in index.links
            if not urlsplit(link).scheme and not urlsplit(link).netloc
        }
        for topic in catalog["topics"]:
            relative = Path("crates/icy_board_help/data/en") / f"{topic['id']}.html"
            target = output / relative
            if target not in pages:
                errors.add(f"Missing command topic: {relative}")
            if target not in indexed:
                errors.add(f"Command index does not link to: {relative}")
    return sorted(errors)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="Sphinx HTML output directory")
    arguments = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    errors = check(arguments.output, repository)
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        print(f"Reference validation failed: {len(errors)} error(s)", file=sys.stderr)
        return 1
    print("Reference validation passed: local links, anchors, entry points and help coverage")
    return 0


if __name__ == "__main__":
    sys.exit(main())
