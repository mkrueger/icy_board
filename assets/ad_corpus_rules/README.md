# Generated BBS advertisement test catalogs

Source: local user-supplied ads collection. No source files changed; no programs executed.

## Results

- Source files: 11253
- Archives opened (including nested): 7089
- Extracted/loose samples and comments: 15798
- Unique exact member rules: 11220
- Matching member occurrences, tested under unrelated filenames: 11890
- Historical whole-ad literal description candidates: 2973 (report_only)
- Reviewed PCBoard description rules: 39 (32 auto_clean, 7 report_only)
- Total description rules: 3012 (2980 report_only, 32 auto_clean)
- Historical unique raw ZIP comment samples: 44 (provenance only; no current rules)
- Description candidates with overlapping suffix matches: 1
- Extraction errors: 0

## Safety and scope

These are separate **test catalogs**, not installed in the active board.
The historical collection rules are not merged into the shipped defaults;
the 39 curated PCBoard description rules are shared with the default description catalog.
Member rules use SHA-256 plus size, without filename patterns: numbered collection
variants remain separate if their bytes differ. Identical content is deduplicated.
Member rules have no report-only mode: enabling that catalog removes matching files.
Exact matches are evidence of identical corpus bytes, not proof that removal is appropriate
in every upload (for example, an intentionally uploaded historical intro collection).

Packed-member selection uses EXE/COM/SMC/SWC/SFC/BBS/AD/ANS/ASC; arbitrary support
files, NFO/TXT/DOC companions and release FILE_ID.DIZ are not automatically selected.
Files shorter than 32 bytes are excluded (filename-art placeholders are not distinctive content).
Dedicated loose board-ad collections are included, excluding collection metadata,
group ads and misc website/card ads. Selection is heuristic, not a manual content audit.
All samples and exclusions are traceable through provenance.tsv and the work manifest.

The 2973 historical whole-ad description candidates contain complete normalized
blocks, suffix only, **report_only**.
They are tested appended to a synthetic unrelated FILE_ID.DIZ; this does not establish
that these whole adverts occur as footers in real release descriptions. Overlapping
blocks require review and never delete bytes. No wildcard phone-number or filename rules.
Raw ZIP comments remain available in the extraction blobs and provenance for
historical research, not as cleanup rules. The former comment catalog has been
removed. No comment rules are generated or validated now; archive comments use
an independent Preserve (default), Remove or Replace mode instead.

**The description catalog now also includes 39 curated PCBoard rules**, including
the previously missing LiQUiD and Shogunat rules. Of these, **32 are auto_clean**
and seven are report_only. Most use literal lines; Critical Strike allows varying
upload dates/times, and LiQUiD supports an inline suffix. Do not treat the entire
description catalog as report-only. See the [complete PCBoard findings and safety
classifications](../pcboard_diz_audit/README.md) and [before/after descriptions](../pcboard_diz_audit/before-after.md).
The new corpus validation found 58 descriptions to clean with no new ambiguity
or byte differences between the default and combined PCBoard cleaning results.

bac-v10 readme states that phone numbers were masked and line endings edited.
Its exact hashes only match those edited versions, not the unedited historical originals.
Text decoding deliberately follows the engine's UTF-8/CP437 behavior even for Amiga ads;
no implicit encoding, line-ending or masking variants are invented.

Extraction uses content-addressed non-executable blob files, never archive paths.
Limits: 16 MiB/member, 256 MiB/archive, 2 GiB total, 10,000 members/archive, depth 4.
Legacy/solid decompressor internal allocations are not sandboxed.

## Validation

The historical run loaded all then-existing catalogs; every selected member occurrence
matched despite renaming, and every historical whole-ad description source matched its
isolated report-only rule and remained byte-identical. Its comment positives also
remained byte-identical under the former report-only comment rules. Those comment
results are historical only, not validation of current archive comment modes.
The historical description candidate catalog was checked for overlap. Synthetic
unrelated text/filename controls did not match. Current generation loads only member
and description catalogs through `FingerprintData::load_split(member_path, description_path)`.
This is corpus-positive/synthetic-negative validation, not a production false-positive study.


## Overlapping description candidates

- corpus-block-b95e1e3559eb2ed0f4f41b16bf105975046a793cd425e0abb88e107eda8c0f0a

The overlapping candidate originates from `bac-v10/amiga_charset/traders_paradise.txt`.
All four `acid_slam*.txt` examples have distinct exact member fingerprints and
corresponding literal description candidates.

## Reproduction and local extraction

The offline generator is [ad_corpus_rules.rs](../../crates/dizbase/examples/ad_corpus_rules.rs).
Its `extract SOURCE NEW_WORK_DIR` mode creates a fresh content-addressed working copy;
`generate WORK_DIR NEW_OUTPUT_DIR` produces and validates only
[upload_ad_files.toml](upload_ad_files.toml) and
[upload_ad_descriptions.toml](upload_ad_descriptions.toml), plus provenance and a README.
Both modes refuse to overwrite existing destination directories.

Generation is now **two-stage**: the historical-ad generator produces its 2973
description candidates, then [pcboard_diz_audit.rs](../../crates/dizbase/examples/pcboard_diz_audit.rs)
mode `curate AUDIT DEFAULT_RULES CORPUS_RULES NEW_REPORT_DIRECTORY` appends the
reviewed PCBoard rules by ID after validating the merged catalog. Re-running only
the first generator into a fresh directory does **not** include these additions.

The verified extraction for this run is in `target/ad-corpus-20260906-final/`:
`manifest.toml` maps every origin and records source hashes; `blobs/<sha256>` holds
the unpacked bytes, without original executable permissions. Earlier exploratory
extractions and `target/ad-corpus-initial-rules/` are superseded, not the final result.
All 7,051 ZIP/7z source archives in `bbstros` were read; the 7,089 total also
includes nested archives. Source files were hash-checked after extraction.

Enable only the desired category by pointing its upload-processing configuration
field at the corresponding catalog here. Existing active board settings and
live archives have **not** been modified. The shipped description defaults
have been expanded by 37 reviewed PCBoard rules; member defaults are unchanged.
Comment handling is no longer a rule category: choose
`upload_processing.archive_comment_mode` (`preserve`, `remove` or `replace`).
Only `replace` uses `replacement_archive_comment`; an empty replacement clears
the comment, and the replacement string is ignored in the other modes. Non-ZIP
source comments are not available. See the [upload processing guide](../../docs/upload_processing.md).

Existing source/audit reports and the counts above remain historical; no live
corpus was rerun for this configuration change. The upload corpus cleaning demo
now explicitly chooses Remove, which removes all ZIP comments rather than only
the formerly matched BBS-Archives comment.
