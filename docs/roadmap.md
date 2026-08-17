# Roadmap/TODO

- [x] Implement all search commands (text/file search)
- [x] Mail QWK support
- [x] Solution for file base meta data 
  - I don't want a "DIRS" file but there is metadata that is not included in the .ZIP - let's say uploader.
    Every other thing is easy to extract from the ZIP file.
  - The file base lives in SQLite now, dizbase reads the archives through unarc-rs.
- [x] Support more compression algorithms
  - The ZIP library PR went in - zip 8.6 ships shrink, reduce and implode behind its "legacy-zip"
    feature, so the files from the 80' all extract.
  - unarc-rs covers the rest: ARC, ARJ, LHA/LZH, ZOO, ACE, SQZ, HA, HYP, ICE, UC2, RAR5, 7z,
    tar and the single file formats.
- [x] icbfile - convert an existing PCBoard file base, and bring one into a single shape
  - Repacks every archive to ZIP under a lower case name and drops the BBStros on the way,
    keeping the description and download counter of the entry it replaces.
- [x] ICBSETUP needs a "cmd editor"
- [x] Rework the mkicbmnu - due to changes in the icbsetup menu system that got broken
- [x] Look at the NEWS/INTRO feature of PCBOARD how that really works
- [x] Logon mail scan
- [x] Finish internal message reader - I don't like it have a message reader PPE going but needs to be finished in any case…
- [x] Implement RM command
- [x] Implement SELECT command
- [x] Implement group chat (CHAT command)
- [x] Finish SSH/Websocket support
- [x] Timed events - the board clears itself before an event runs, documented in docs/source/events.rst
- [x] FTN mailer usable as a leaf/point - icbmailer scan/poll/toss over BinkP client,
      config in ftn.toml, docs in docs/source/mailer.rst; schedule via timed event or cron.
      No BinkP server/answering side planned. Netmail still lands in one dump base;
      AKA/link setup is hand-edited TOML for now.
- [x] Finish/sync commands & help files - every PCBoard user command (A-Z, the word commands and
      the DB/UB/NODE/OPEN aliases) resolves, "!" repeats the last command again, and every help
      file in the help directory is reachable. Help ids follow PCBoard's HELP.C table, so BD/DB
      share the D help and BU/UB the U one, and the MORE? help is built from PCBTEXT like PCBoard.
      Only the sysop help (hlp1-hlp15) is unwritten - PCBoard never shipped those either.
      German help set is still 20 of 52 files.
- [x] PPL dBase3 statements/functions
- [x] Go through the PCBoard options and ensure they're working - atm some do - some don't
  - compat/COMMAND_AUDIT.md tracks where a command still answers differently than PCBoard did
  - compat/OPTIONS_AUDIT.md tracks the configuration switches - 19 of 126 options and 12 of 29
    sysop security levels are editable in ICBSetup but read by nobody

-> 1st BETA

After BETA

- [x] PPL tree-sitter grammar - crates/tree-sitter-ppl parses the whole language up to 4.01 and
      ships highlight/locals/fold/indent queries, so Neovim, Helix and friends read PPL. ppl-lsp
      still parses on its own.
- [ ] PPL web statements/functions
- [ ] FTN polish (optional): netmail to users, ICBSetup AKA/link editor, AreaFix if needed
- [ ] Self-service password reset using email
- [ ] Web Frontend (IcyTerm can run as Webassembly but needs the data from somewhere)
- [ ] Support for IcyAnim - no need to use icy_play in icy_board anymore