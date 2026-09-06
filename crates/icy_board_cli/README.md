# Shared command-line localization

The nine command-line programs use `clap` and `clap-i18n-richformatter` through
this crate. It has no dependency on the board engine or the TUI.

- `parse::<Cli>()` parses process arguments, prints localized help/errors and
  preserves the previous exit convention: help exits with 0, parser errors with 1.
- `try_parse_from::<Cli, _, _>(argv)` is the non-exiting test/API variant; include
  the program name as the first argument.
- `command::<Cli>()` creates a command with translated headings and help flags,
  including its subcommands. Use it for existing no-input usage displays.
- `text(domain, key)` supplies application-specific help from the embedded
  English/German Fluent catalogs. The desktop locale selects the language;
  unsupported languages fall back to English.

The formatter handles parser diagnostics. Application/runtime diagnostics are
not automatically translated, and machine-readable output is never rewritten.
Underlying parser error details can retain their original language.

## Compatibility

Keep command names, long/short options, defaults, manual version output, and
stdout/stderr behavior unchanged. Repeated boolean switches remain accepted;
repeated scalar options remain errors. Repeatable list options keep all values.
The legacy `help`, `help command`, and `command help` forms remain supported.

`pplc --cp437` deliberately uses a zero-argument `ArgAction::Set`: absent means
`None` (autodetect), present means `Some(true)`. `SetTrue` on `Option<bool>` would
default to `Some(false)` and silently change compiler behavior.

Add each new help key to both language files. The shared crate tests parse and
render every catalog without language fallback, verify matching keys and
exercise parser compatibility. Binary integration tests additionally verify
English/German help, rich errors, exit codes and unchanged JSON/version output.