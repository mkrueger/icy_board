# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'Icy Board'
copyright = '2025, Mike Krüger'
author = 'Mike Krüger'

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = []

templates_path = ['_templates']
exclude_patterns = []



# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'alabaster'
html_static_path = ['_static']

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
        r'DECLARE|FUNCTION|PROCEDURE|ENDPROC|ENDFUNC|THEN'
        r')\b'
    )

    _types = (
        r'\b('
        r'BOOLEAN|INTEGER|UNSIGNED|BYTE|WORD|SBYTE|SWORD|MONEY|FLOAT|DOUBLE|REAL|DATE|EDATE|DDATE|TIME|STRING|BIGSTR|TABLE|MESSAGEAREAID|PASSWORD'
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