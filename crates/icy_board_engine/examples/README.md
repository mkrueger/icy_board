# Upload corpus experiment

`upload_corpus` is an offline test-board helper, not a general-purpose importer.
It expects the board layout used by `target/debug/test`, modifies **conference
1** (Main Board is 0), and takes the normal exclusive `BoardLock`.

Arguments: `prepare|process|finish BOARD SOURCE RULES`.

- **prepare:** refuses an existing experiment directory; hashes every source
  file with SHA-256 and saves sizes before processing. Files directly in the
  source root are recorded but excluded. Creates one area per subdirectory,
  preserving existing areas, with its own numbered menu. Backs up the board
  and conference configurations, installs rules, and enqueues verified copies.
- **process:** invokes `UploadProcessor` with cleanup and Deflate level 9.
  The per-file scanner is disabled only for this phase; nothing is published.
  Interrupted `Processing` records require inspection rather than blind retry.
- **finish:** runs one real `clamscan` process over all retained payloads. Only
  explicit `OK` results from a successful scan (exit 0 or 1), combined with
  successful archive processing, permit publication through the shared
  publisher. Infections, resource-limit alerts, missing verdicts, scanner
  failures, malformed archives and publication conflicts remain quarantined.
  Revalidates all source hashes and writes per-file and per-area statistics.

Artifacts live in `BOARD/upload-corpus`: `baseline.toml`, `mapping.tsv`,
`outcomes.tsv`, `report.md`, scanner version/output, configuration backups,
rules snapshot, area files/catalogs and quarantine records. Processing events
are appended to the board's `icboard.log` through the same logging facade used
by interactive uploads. ClamAV detection messages include the original source
name and quarantine ID.

The final savings percentage compares **published file pairs only** and combines
advertisement/comment cleanup with recompression; it is not a measurement of
compression alone. Quarantine storage and catalog overhead are excluded.
Deflate 9 is this repacker's maximum, not a claim of globally optimal ZIP size.
Record the actual ClamAV database date: old signatures cannot establish current
malware freedom. Legitimate descriptions are retained; no description insertion,
own advertisements or new archive comments are configured for this experiment.

The helper deliberately does not silently restart a partially completed final
publication phase. Inspect its durable records before recovery. Configuration
backups can contain board secrets and must not be published with reports.