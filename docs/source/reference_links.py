"""Keep source citations out of the distributable documentation bundle."""

from pathlib import Path
from urllib.parse import quote, unquote, urlsplit

from docutils import nodes
from sphinx import addnodes
from sphinx.application import Sphinx


def source_target(root: Path, docname: str, target: str) -> Path:
    path = unquote(urlsplit(target).path)
    base = root if path.startswith("/") else root / Path(docname).parent
    return (base / path.lstrip("/")).resolve().relative_to(root.resolve())


def repository_url(repository: str, revision: str, path: Path, fragment: str = "") -> str:
    url = f"{repository.rstrip('/')}/blob/{quote(revision, safe='')}/{quote(path.as_posix())}"
    return f"{url}#{quote(fragment)}" if fragment else url


def rewrite_source_links(app: Sphinx, doctree: nodes.document) -> None:
    root = Path(app.srcdir)
    theme = app.config.html_theme_options
    for node in list(doctree.findall()):
        if not isinstance(node, (addnodes.download_reference, addnodes.pending_xref)):
            continue
        target = node.get("reftarget", "")
        url = urlsplit(target)
        if url.scheme or url.netloc or not url.path:
            continue
        path = source_target(root, node.get("refdoc", app.env.docname), target)
        local_only = bool(path.parts) and path.parts[0] in {"pcboard", "target"}
        if local_only:
            node.replace_self(nodes.inline("", node.astext() + " (local reference)"))
        elif isinstance(node, addnodes.download_reference):
            if path.parts[:1] == ("assets",) and path.suffix.lower() in {".png", ".jpg", ".jpeg", ".svg", ".gif", ".webp"}:
                continue
            reference = nodes.reference(
                "",
                "",
                *node.children,
                refuri=repository_url(
                    theme["source_repository"], theme["source_branch"], path, url.fragment
                ),
            )
            node.replace_self(reference)


def setup(app: Sphinx) -> dict[str, object]:
    # Run before Sphinx's download collector can copy locally available files.
    app.connect("doctree-read", rewrite_source_links, priority=100)
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}
