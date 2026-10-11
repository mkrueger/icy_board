# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'Icy Board'
copyright = '2024-2026, Mike Krüger'
author = 'Mike Krüger'

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = ['myst_parser', 'reference_links']

templates_path = ['_templates']
root_doc = 'docs/source/index'
source_suffix = {'.rst': 'restructuredtext', '.md': 'markdown'}
include_patterns = [
    'INSTALL.md',
    'docs/*.md',
    'docs/configuration/*.md',
    'docs/reference/*.md',
    'docs/source/**',
    'crates/icy_board_help/data/en/*.md',
    'crates/tree-sitter-ppl/README.md',
    'compat/README.md',
    'compat/COMMAND_AUDIT.md',
    'compat/OPTIONS_AUDIT.md',
    'compat/DECLARE_AUDIT.md',
]
exclude_patterns = [
    'docs/build/**',
    'docs/source/customizing/internationalization.rst',
    'docs/source/customizing/look_and_feel.rst',
    'docs/source/customizing/menus.rst',
    'docs/ftn_review.md',
    'docs/message_entry_reading_audit_2026-09-06.md',
    'docs/x86_native_dos_door_requirements.md',
]
myst_heading_anchors = 6
myst_enable_extensions = ['colon_fence', 'deflist']


# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'furo'
html_title = 'Icy Board Reference'
html_theme_options = {
    'source_repository': 'https://github.com/mkrueger/icy_board/',
    'source_branch': os.environ.get('ICY_REFERENCE_REVISION', 'main'),
    'source_directory': '',
    'navigation_with_keys': True,
}
html_static_path = ['_static']
html_css_files = ['reference.css']
html_additional_pages = {'index': 'start.html'}
html_copy_source = False
html_show_sourcelink = True

import re
from pygments.lexer import RegexLexer, bygroups
from pygments import token
from sphinx.highlighting import lexers

class PPLLexer(RegexLexer):
    """
    Lightweight PPL (PCBoard Programming Language) lexer for docs.
    Focus: readability in manuals (not a full compiler-grade tokenizer).
    """
    name = 'PPL'
    aliases = ['ppl']
    flags = re.IGNORECASE | re.MULTILINE

    _keywords = (
        r'\b('
        r'IF|ELSEIF|ELSE|ENDIF|WHILE|ENDWHILE|FOR|NEXT|ENDFOR|REPEAT|UNTIL|'
        r'LOOP|ENDLOOP|BREAK|CONTINUE|RETURN|GOSUB|GOTO|SELECT|CASE|DEFAULT|ENDSELECT|'
        r'DECLARE|FUNCTION|PROCEDURE|ENDPROC|ENDFUNC|TYPE|ENDTYPE|BEGIN|END|THEN|VAR|LET'
        r')\b'
    )

    _types = (
        r'\b('
        r'BOOLEAN|INTEGER|UNSIGNED|BYTE|WORD|SBYTE|SWORD|MONEY|FLOAT|DOUBLE|REAL|DATE|EDATE|DDATE|TIME|STRING|BIGSTR|TABLE|MESSAGEAREAID|PASSWORD|CONFERENCE|DIRECTORY|AREA|DOOR|USER'
        r')\b'
    )

    _builtins = (
        r'\b('
        # Common predefined functions & constants subset (optional highlight)
        r'LEN|LOWER|UPPER|MID|LEFT|RIGHT|SPACE|INSTRR?|REPLACE(?:STR)?|STRIP(?:STR|ATX)?|LTRIM|RTRIM|TRIM|RANDOM|DATE|TIME|YEAR|MONTH|DAY|DOW|HOUR|MIN|SEC|TIMEAP|I2S|S2I|ABS|BAND|BOR|BXOR|BNOT|ISBITSET|EXIST|CRC32|MEGANUM|GETMSGHDR|CONFINFO|NEW_CONFINFO|AREA_ID|GETBANKBAL|PCBACCOUNT|PCBACCSTAT|STACKLEFT|STACKERR|TOKENSTR|GETTOKEN|TOKCOUNT|INKEY|TINKEY|KINKEY|MINKEY|VALDATE|VALTIME|T(?:O(?:BIGSTR|BOOLEAN|BYTE|DATE|DREAL|EDATE|INTEGER|MONEY|REAL|SBYTE|SWORD|TIME|UNSIGNED|WORD)|OSTRING)'
        r')\b'
    )

    tokens = {
        'root': [
            # Preprocessor directives ;$DEFINE / ;$IF / ;#Version etc
            (r'(;\$(?:IF|ELIF|ELSE|ENDIF|DEFINE|UNDEF|INCLUDE)\b)(.*)$',
             bygroups(token.Comment.Preproc, token.Comment.Preproc)),
            (r'(;#[A-Za-z_][A-Za-z0-9_]*)', token.Name.Constant),

            # Comments (semicolon, apostrophe, or leading star)
            (r';.*$', token.Comment.Single),
            (r"'[^\n]*", token.Comment.Single),
            (r'^\s*\*.*$', token.Comment.Single),

            # Block comment style (if ever introduced) /* ... */ (non-greedy)
            (r'/\*', token.Comment.Multiline, 'block-comment'),

            # Strings (double-quoted); allow escaped quotes
            (r'"([^"\\]|\\.)*"', token.String),

            # Hex (legacy) like FFFFh or 0x1A2B
            (r'\b[0-9A-F]+[Hh]\b', token.Number.Hex),
            (r'\b0x[0-9A-Fa-f]+\b', token.Number.Hex),

            # Numbers (integer / float)
            (r'\b\d+\.\d+\b', token.Number.Float),
            (r'\b\d+\b', token.Number.Integer),

            # Operators / compound assignment
            (r'(\+=|-=|\*=|/=|%=|&=|\|=)', token.Operator),
            (r'(\^|!=|==|=|<=|>=|<|>|[%*/+\-&\|!])', token.Operator),

            # Punctuation
            (r'[\(\)\[\]\{\},.:]', token.Punctuation),

            # Keywords
            (_keywords, token.Keyword),

            # Types
            (_types, token.Keyword.Type),

            # Builtins (functions / common helpers)
            (_builtins, token.Name.Builtin),

            # Labels  :LabelName
            (r':[A-Za-z_][A-Za-z0-9_]*', token.Name.Label),

            # Identifiers
            (r'[A-Za-z_][A-Za-z0-9_]*', token.Name),

            # Whitespace
            (r'\s+', token.Text),
        ],

        'block-comment': [
            (r'[^*/]+', token.Comment.Multiline),
            (r'/\*', token.Comment.Multiline, '#push'),
            (r'\*/', token.Comment.Multiline, '#pop'),
            (r'[*\/]', token.Comment.Multiline),
        ],
    }
lexers['PPL'] = PPLLexer(startinline=True)
lexers['ppl'] = PPLLexer(startinline=True)

import pathlib
import re

def load_workspace_version() -> str:
    cargo = pathlib.Path(__file__).resolve().parents[2] / "Cargo.toml"
    try:
        txt = cargo.read_text(encoding="utf-8")
        # Look for [workspace.package] version = "x.y.z"
        m = re.search(r'^\s*version\s*=\s*"([^"]+)"\s*$', txt, re.MULTILINE)
        if m:
            return m.group(1)
    except Exception:
        pass
    return "unknown"

release = load_workspace_version()      # full version string (e.g. 0.1.7)
version = release                       # or split major/minor if you prefer: release.split('.')[:2]

# Provide a substitution usable in RST as |icy_version|
rst_epilog = f"""
.. |icy_version| replace:: {release}
.. |icy_version_short| replace:: {version}
"""
# -- Options for the printed handbook ----------------------------------------
# The manual is the long form and ships as a PDF, so it is set as a book rather
# than as a stack of web pages.

latex_documents = [
    (root_doc, "icyboard.tex", "Icy Board Reference", author, "manual"),
]

latex_toplevel_sectioning = "chapter"

# pdflatex drops the box drawing characters without a word; xelatex takes them
# from the font instead.
latex_engine = "xelatex"

# A link in print is only useful if the address is on the page.
latex_show_urls = "footnote"

latex_elements = {
    "papersize": "a4paper",
    "pointsize": "10pt",
    "figure_align": "H",
    # The board draws itself with CP437 line art, so the monospace face has to
    # carry the box drawing characters or every screen listing comes out ragged.
    "fontpkg": r"""
\usepackage{fontspec}
\usepackage{newunicodechar}
\setmainfont{DejaVu Sans}
\setsansfont{DejaVu Sans}
\setmonofont{DejaVu Sans Mono}[Scale=0.85]
\newfontfamily\icbsymbols{Noto Sans Symbols 2}
\newfontfamily\icbcjk{Droid Sans Fallback}
\newunicodechar{⎀}{{\ttfamily\symbol{"2380}}}
\newunicodechar{␛}{{\icbsymbols\symbol{"241B}}}
\newunicodechar{␠}{{\icbsymbols\symbol{"2420}}}
\newunicodechar{📥}{{\icbsymbols\symbol{"1F4E5}}}
\newunicodechar{界}{{\icbcjk\symbol{"754C}}}
\newunicodechar{💯}{100\%}
\newunicodechar{✅}{\ensuremath{\checkmark}}
\newunicodechar{❌}{\ensuremath{\times}}
\newunicodechar{️}{}
""",
    "sphinxsetup": ",".join(
        [
            "verbatimwithframe=true",
            "VerbatimColor={RGB}{248,248,248}",
            "VerbatimBorderColor={RGB}{200,200,200}",
            "hmargin={2.5cm,2.5cm}",
            "vmargin={2.5cm,2.5cm}",
        ]
    ),
    "maketitle": rf"""
\hypersetup{{pdftitle={{Icy Board Reference}}, pdfauthor={{{author}}}}}
\begin{{titlepage}}
  \centering
  \vspace*{{5cm}}
  {{\Huge\bfseries Icy Board\par}}
  \vspace{{1cm}}
  {{\Large Sysop Handbook and Reference\par}}
  \vspace{{3cm}}
  {{\large Version {release}\par}}
  \vfill
  {{\large {author}\par}}
\end{{titlepage}}
\sphinxtableofcontents
""",
    # The table of contents is part of the title page block above.
    "tableofcontents": "",
}
