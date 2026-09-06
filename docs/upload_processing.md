# Upload processing and own advertising

Configure the upload-processing page in **icbsetup**. Use **After successful
processing** or **Manual approval** to enable the pipeline for incoming uploads;
**Immediate** publishes without running it. Upload statistics and credits are
awarded after transfer, even if processing later fails. Quarantined files can be
reviewed, reprocessed, approved or rejected in **icbadmin**.

The single **Remove advertisements** switch controls recognized advertising
members, text-member findings and known description footers. It does not
authorize removing arbitrary release branding. ZIP repacking/compression,
archive comment mode and the virus scanner remain separate options.

## Two advertisement rule files

The shared switch is `upload_processing.remove_advertisements`. Each category
has its own editable path in setup and its own shipped TOML catalog:

| Configuration key | Default filename / shipped asset | TOML section |
| :--- | :--- | :--- |
| `upload_processing.advertisement_file_rules` | [upload_ad_files.toml](../assets/upload_ad_files.toml) | `[[fingerprint]]`, `[[text_member_rule]]` |
| `upload_processing.advertisement_description_rules` | [upload_ad_descriptions.toml](../assets/upload_ad_descriptions.toml) | `[[description_rule]]` |

An empty path (`""`) disables only that category. Relative paths resolve against
the board root, not the current working directory. Spaces and semicolons are
literal characters in each single path, not list separators. Place the shipped
catalogs at the configured locations. When removal is enabled, missing or
unreadable files, malformed TOML and invalid selected patterns are processing
errors requiring upload review; they are not silently ignored. An intentionally
empty catalog is accepted, but a file containing only other rule categories is
a configuration error.

Legacy fingerprints identify complete advertising files using hashes plus sizes,
or filename regexes plus raw case-sensitive byte keywords. Their semantics are
unchanged, including filename-independent exact hashes and automatic removal.
Description rules identify bounded blocks using either regex `lines` or
`literal_lines`, with `report_only`, `review` or `auto_clean` actions.

A combined TOML catalog can still be used: set **both paths** to that same file.
Each path selects only its own categories. Loading uses
`FingerprintData::load_split(member_path, description_path)`. New text examples
are maintained in the default member catalog, not automatically merged into
historical combined or generated corpus catalogs. Copy selected text rules into
a custom member catalog if that is the file configured for the board.

The old `advertisement_rules` and `advertisement_comment_rules` settings are
removed; no implicit fallback. No comment-rule catalog is used.

### Whole-file text templates

Use `[[text_member_rule]]` in the **existing member rule file**. No third path
or new setup switch is needed. Each rule has:

| Field | Meaning |
|---|---|
| `id` | Nonempty identifier, unique among text rules in this catalog. |
| `action` | `report_only` (default), `review`, or `auto_clean`. |
| `max_bytes` | Maximum original byte length, default 16 KiB, hard ceiling 128 KiB. This is a safety limit, not an exact fingerprint size. |
| `lines` | Ordered list of `{ literal = '...' }` and `{ regex = '...' }` entries; every line of the entire normalized member must match. |

Literal entries are readable text, not regular expressions. Regex entries use
Rust regex syntax and are implicitly anchored to the entire normalized line.
Single-quoted TOML strings preserve regex backslashes. A template must contain
at least twelve literal letters in total, 1–512 line entries and a valid size
limit. Invalid fields, duplicate IDs, regex-only templates and invalid regexes
fail catalog loading. The minimum literal count is only a guardrail, not proof
that a user-authored rule is safe.

Normalization validates UTF-8/BOM first, otherwise assumes CP437; case, horizontal
whitespace, CRLF/LF/CR, ANSI SGR colors and PCBoard `@Xhh` colors are normalized.
Leading/trailing empty lines are ignored; internal empty lines are significant
and use `{ literal = '' }`. Encoding detection is necessarily heuristic for
legacy text. Other control sequences, NUL, executable headers, malformed BOM
payloads and non-padding data behind DOS EOF block matching. The implementation
does not silently skip unknown paragraphs, arbitrary ANSI cursor/erase commands
or extra documentation. Input is bounded to 512 physical lines.

There is deliberately no filename restriction: renamed copies remain detectable.
Canonical FILE_ID/DESC.SDI description names are excluded from these whole-member
rules. Existing byte fingerprints retain their previous behavior.

Processing order and results:

1. Existing byte fingerprints run first; a matching legacy rule still removes
   its known bytes, even if a text rule would be `report_only`.
2. Otherwise text templates are matched against the original member bytes.
3. One `report_only` match keeps the member and records name, rule, encoding and
   raw SHA-256 in the upload report/log or icbfile output. It does not by itself
   rewrite an archive or require quarantine review. Other configured steps may
   still run, and normal publish policy remains in force.
4. One `auto_clean` match removes the complete member, not selected text lines.
5. `review`, or multiple matching text rules regardless of action/order, aborts
   the archive rewrite and preserves the original for review.

The shipped file contains four reviewed **auto_clean** templates: the BBS Archives
transit banner, one Clipper Workshop layout, one Cutting Edge layout and the
observed Buggerer Deluxe/Footer PPE template with variable upload stamp.
These are intentionally not broad rules for all advertisements from those boards.
Buggerer's copied description is fixed to the observed release, not arbitrary
software documentation. New rules should start with `report_only`; these four
templates were approved after reviewing their matches. Existing default hashes continue to remove their
known variants as before.

See the [historical report-only raw-byte corpus trial](../assets/archive_text_audit/text-rule-trial/README.md)
and its [individual findings](../assets/archive_text_audit/text-rule-trial/findings.tsv).
The review example also supports `--text-rules AUDIT MEMBER_CATALOG NEW_REPORT_DIRECTORY`
to replay a catalog against the audit's original raw blobs without editing archives.

### ZIP archive comments

`upload_processing.archive_comment_mode` explicitly selects `preserve` (default),
`remove`, or `replace`, independently of advertisement removal. Preserve keeps
raw ZIP comment bytes. Remove clears them. Replace uses
`replacement_archive_comment` verbatim; an empty replacement clears the comment.
Stored replacement text is ignored in the other two modes. A comment change can
trigger a rewrite even when unconditional repacking is disabled. Non-ZIP source
comments are not available from the archive reader. No regex/hash comment rules
are involved.

## One own advertisement file

**Own advertisement file** is one optional path, stored as
`upload_processing.advertisement_file`:

- Empty: do not add an advertisement.
- Ordinary file: insert its bytes unchanged under its basename.
- Extension `.ppe`, case-insensitive: run a trusted generator and insert its
  output instead. The PPE itself is not inserted.

Relative configured paths are resolved against the board root. Spaces and
semicolons are literal filename characters, not separators. The previous
`additions` list has been removed; select one path in setup instead.

Own advertising never inserts or replaces `FILE_ID.DIZ`, `FILE_ID.ANS`,
`FILE_ID.PCB` or `DESC.SDI` (case-insensitive). A collision with an existing
archive member requires review rather than overwriting it. Adding advertising
uses the ZIP writer, so a supported non-ZIP archive is converted to ZIP even
when unconditional ZIP repacking is disabled. Unsupported non-archive uploads
are not wrapped in ZIP merely to add advertising.

## PPE invocation contract

The generator runs once per processing attempt, in a separate headless
**icboard** process. Install **icboard** beside **icbadmin** when using offline
administration; a missing worker is a processing error, not a silent fallback.
Normal automatic processing uses the sibling icboard executable in the same
way. The worker does not load or lock the live board configuration, start a
terminal UI or borrow the uploader's session.

Read two arguments with `GETTOKEN` in this order:

| Argument | Meaning |
| :--- | :--- |
| 1 | Absolute, initially empty, temporary output directory, **including a trailing platform path separator**. |
| 2 | Original archive filename from the quarantine record, including extension; not a path to the archive. |

Both tokens are passed literally, including spaces and semicolons. The worker's
current directory and isolated board root are the output directory. No fully
extracted archive directory is provided: the repacker processes archive members
directly. No uploader account or other live board context is supplied. Do not
depend on interactive commands; terminal output is discarded.

Create the advertisement in that directory, close it, and finish normally with
`EXIT` (or fall through). `STOP` means failure. Check `FERR` after file operations:
ordinary PPL file errors use the channel error flag and do not automatically
abort a PPE. Simply leaving the directory empty is a successful decision not to
add advertising for this upload.

The ready-to-compile [example generator](../assets/upload_advertisement.pps)
creates `BOARD.AD` containing a board line and the archive name. Compile it with
**pplc**, install the resulting PPE at a SysOp-controlled path, and select that
path in setup. Text written by the existing `FPUT`/`FPUTLN` runtime is UTF-8 with
a BOM; generated bytes are copied unchanged. Use appropriate binary file I/O
if a specific legacy byte encoding is required.

### Accepted output and failures

- Empty directory: no own advertisement.
- Exactly one regular file: add it under its basename.
- Multiple entries, subdirectories, symbolic links or (on Unix) hard links:
  processing fails and the upload requires review.
- Protected description names, unusable names and existing-member collisions:
  review; never silently replace a file.
- The static or generated advertisement must fit both **16 MiB** and the
  configured maximum member size. The PPE program itself is limited to 16 MiB.
- The worker is terminated after **30 seconds**, including waiting for its
  captured error output. At most 16 KiB of stderr is retained for error reports.
- Load/runtime errors, abnormal termination and timeouts retain the quarantine
  payload and record an error. A temporary output directory is cleaned up after
  the attempt; it is not a permanent asset location.

The configured virus scanner runs on the resulting archive **after** adding
advertising. A scanner failure or infection prevents automatic publication.
Reprocessing runs the PPE again; an advertisement already present in a payload
from an earlier attempt is still subject to the no-overwrite collision rule.

### Trust and resource boundaries

**This is not a sandbox.** Only select trusted PPEs installed by the SysOp.
Uploaded PPEs are never automatically executed as generators. Code runs with
the operating-system permissions of the board/admin process and can access
files outside the output directory or invoke external programs. The separate
process provides a killable worker, not filesystem or network isolation.

The size limit is checked when collecting the result, not a disk quota while
the PPE runs. RAM usage and subprocesses are not sandboxed or quota-controlled;
the timeout targets the worker, not an arbitrary descendant process tree.
Generators must not start detached/background work, leave files open in other
processes or modify unrelated paths. Apply OS-level isolation/resource limits
if stronger guarantees are needed. Concurrent uploads can run separate
generators: use only the supplied private output directory for scratch data.