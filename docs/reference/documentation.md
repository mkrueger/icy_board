# Building and maintaining this reference

The reference is a static Sphinx site. Markdown is parsed with MyST and the
existing reStructuredText PPL handbook remains in its original format. Furo
provides navigation, local search, light/dark appearance and source links.
No database, login service or running BBS is needed to read it.

## Build HTML

From the repository root, with Python 3.11 or later:

```sh
python3 -m venv docs/build/venv
docs/build/venv/bin/python -m pip install -r docs/requirements.txt
make -C docs html SPHINXBUILD=build/venv/bin/sphinx-build
docs/build/venv/bin/python docs/check_reference.py docs/build/html
```

Open `docs/build/html/index.html`. Copy the **whole** HTML directory when
distributing an offline edition; search, images and CSS are separate files.
The command above uses warnings-as-errors and continues collecting diagnostics
so a single run can show all broken references.
When switching documentation layouts, build into a fresh output directory:
Sphinx does not delete obsolete files left by an earlier edition.

On Windows, activate a virtual environment, install the same requirements and
run `docs\make.bat html`. The direct cross-platform Sphinx invocation is:

```sh
sphinx-build -b html -W --keep-going -c docs/source . docs/build/html
python docs/check_reference.py docs/build/html
```

The Sphinx source root is the repository root, **not** just `docs/source`.
An explicit document allowlist in the configuration restricts what is
published. This lets the website read command-help Markdown directly from
the help crate and resolve repository-relative links without copying files.
Generated boards, logs and arbitrary repository documents are not a site
content directory.

Links to implementation files point to the source repository instead of
bundling source downloads. References to a local `pcboard/` archive or
`target/` oracle output are marked **local reference** and are not distributed.
The build does not require those local archives or logs to exist. Full-size
screenshots remain available offline.

## PDF

The existing release process builds a PDF from the same document tree:

```sh
make -C docs latex SPHINXBUILD=build/venv/bin/sphinx-build
make -C docs/build/latex
```

The second step needs XeLaTeX, latexmk and the fonts/packages listed in the
release workflow, including DejaVu Sans/Mono, Noto Sans Symbols 2, Droid Sans
Fallback and the `newunicodechar` LaTeX package. Installing the Python
requirements alone is not enough. On distributions with split TeX packages,
install `newunicodechar` through the distribution package manager; it is part
of `texlive-latex-extra` in the Ubuntu release build.
PDF output is useful for printing; HTML is preferable for searching large
function tables and following cross-references.

## Publishing

The documentation workflow validates HTML on pull requests and pushes to
`main`, and uploads a downloadable site artifact. Its manual dispatch can
also deploy to GitHub Pages when `publish` is selected. Configure the
repository's Pages source as **GitHub Actions** before requesting deployment.
Build and validation do not publish a site by themselves.

For another static host, upload the contents of `docs/build/html` unchanged.
The site uses relative internal links and does not require a domain-specific
base URL.

## Versions

The edition number comes from the checked-out workspace version. A build
from `main` documents that development checkout; it must not be presented as
proof of behavior in an older installed release.

Release-tag builds attach the handbook and an offline HTML archive to that
release. To build an older edition locally, use a separate checkout of its tag
and that revision's documentation instructions/dependencies. There is no
automatic historical-version switcher: do not advertise editions that have
not actually been built and hosted.

CI sets `ICY_REFERENCE_REVISION` to the built commit or release tag so source
citations point to that edition. Local builds default to `main`; set this
environment variable to a commit or tag when building a historical checkout.

## Where to edit

| Content | Canonical source |
| --- | --- |
| Start page and top-level navigation | `docs/source/index.rst` |
| Concepts, administration, tools, lookup and troubleshooting | `docs/reference/` |
| Task-oriented operational guides | The relevant Markdown file directly under `docs/` |
| Configuration fields and examples | `docs/configuration/`; old RST locations link here |
| Caller and sysop command topics | `crates/icy_board_help/data/en/`; indexed by `docs/reference/commands.md` |
| PPL language, functions, statements and constants | `docs/source/ppl/` |
| Versioned PPL additions and host APIs | `docs/new_ppl.md` |
| Display macros | `docs/macros.md` |
| Theme and build configuration | `docs/source/conf.py`, `_static/`, `_templates/` |

Do not paste copies of help topics into a second manual. Changes to the help
catalog must also follow its source-hash and terminal-rendering rules. Website
HTML rendering does not validate BBS line wrapping or CP437 output.

Some older narrative handbook chapters remain available through legacy links.
When consolidating another chapter, compare its contents first; do not delete
unique instructions merely because the chapter title matches a newer guide.

## Reference-page standard

For new commands, settings, macros and PPL APIs, document:

1. Purpose and the context where it is available.
2. Exact syntax, types, parameters and defaults.
3. Permissions, state changes, limits and failure behavior.
4. A small example, with fragments labeled as fragments.
5. Version availability and PCBoard differences.
6. Links to related operations and troubleshooting.

Use real implementation and test evidence for behavior claims. Keep
**implemented**, **recognized but unavailable**, **partially implemented** and
**planned** distinct. A parser accepting a field is not proof that the runtime
uses it. Record limitations rather than implying an untested integration works.

## Validation before publishing

- Build HTML with warnings treated as errors.
- Run the local link/anchor and help-coverage checker.
- Run `python -m unittest docs.test_check_reference docs.test_reference_links`
  when changing build validation or source-link handling.
- Confirm search and navigation in a browser, including a narrow viewport.
- Build LaTeX when changing content used by the PDF edition.
- Run the affected runtime tests for behavior changes.
- Validate complete configuration examples with the real loader; do not pass
  partial fragments to the full-board parser.
- Compile changed PPL examples with their documented language/runtime target.
- Check localized terminal changes in English and German in separate locale
  contexts; website translation is not a substitute for terminal validation.

The site build checks documentation structure, not every external service or
all examples in the inherited manual. Claims of runtime verification must name
the actual checks performed.

## Legacy handbook entry points

These preserve earlier repository links while the main navigation points to
the current guides.

```{toctree}
:hidden:

/docs/README
/docs/source/installation
/docs/source/events
/docs/source/customizing/index
/docs/source/configuration/index
```
