# Reference index

Use this page for direct lookup. For a guided path, start with
[board concepts](concepts.md) or [sysop administration](administration.md).
The website search includes command topics, configuration tables, macros,
PPL functions and PPL statements.

## A–Z subject index

| Subject | Reference |
| --- | --- |
| Access expressions | [Syntax and limitations](../configuration/board.md#security-expression-representation) |
| Accounting, balances and tariffs | [Operator guide](../accounting.md) |
| ANSI, Avatar, RIP and CP437 | [Display files](../source/customizing/display_files.rst), [encoding differences](../differences.md) |
| Archives and `FILE_ID.DIZ` | [File handling](../icbfile.md), [upload processing](../upload_processing.md) |
| Backups | [Backup and restore checklist](administration.md#backups-and-restoring) |
| BinkP / FTN / AreaFix | [Mailer handbook](../source/mailer.rst) |
| Board configuration | [Main TOML schema](../configuration/board.md) |
| Bulletins and surveys | [Component formats](../configuration/components.md#bulletins-and-surveys), [caller commands](commands.md) |
| Caller commands and aliases | [Complete command index](commands.md) |
| Call-waiting screen | [Controls, log viewer and monitors](../source/icy_board.rst) |
| CLI programs | [Tool reference](tools.md) |
| `CMD.LST` / command overrides | [Command configuration](../configuration/commands.md) |
| Conferences / `CNAMES` | [Configuration](../configuration/components.md#conferences), [concepts](concepts.md#conferences-and-message-areas) |
| Credits and download limits | [Accounting](../accounting.md), [security levels](../configuration/components.md#security-levels) |
| dBase | [PPL dBase reference](../source/ppl/dbase.rst) |
| Display macros / `@` codes | [Complete macro reference](../macros.md) |
| Doors / BBSLink | [Configuration](../configuration/components.md#doors-and-bbslink-accounts), [DOS doors](../native_dos_doors.md) |
| Events | [Scheduler operation](../events.md), [file format](../configuration/events.md) |
| File areas / `DIR` / `FILES.BBS` | [Operator guide](../icbfile.md), [schema](../configuration/components.md#file-directories) |
| Groups and registration | [Access concepts](concepts.md#access-registration-and-subscriptions), [registration flags](../../crates/icy_board_help/data/en/hlpreg.md) |
| Help files | [Topics](commands.md), [generation and translation](../gettingstarted.md#command-help) |
| HTTP from PPL | [Policy](../configuration/board.md#ppe-http-policy), [API](../new_ppl.md#http-objects-400) |
| ICBTEXT / `PCBTEXT` | [Editor](tools.md#mkicbtxt-system-prompts), [format](../configuration/data.md) |
| Installation | [Packages and source builds](../../INSTALL.md) |
| JAM / message areas | [Concepts](concepts.md#conferences-and-message-areas), [schema](../configuration/components.md#message-areas) |
| Languages | [Configuration](../configuration/components.md#languages), [customization](customization.md#prompts-and-languages) |
| Listeners | [Telnet, SSH and WebSocket settings](../configuration/board.md#login-servers) |
| Logs | [Log viewer](../source/icy_board.rst), [troubleshooting](troubleshooting.md) |
| Mail / personal inbox | [Concepts](concepts.md#personal-mail), [inbox command](../../crates/icy_board_help/data/en/hlp@.md) |
| Menus | [Editor reference](../mkicbmnu.md), [schema](../configuration/components.md#interactive-menus) |
| Migration | [PCBoard import procedure](../migration.md) |
| Passwords and recovery | [Encoding](../configuration/board.md#password-encoding), [recovery settings](../configuration/board.md#optional-email-password-recovery) |
| Paths | [Board path table](../configuration/board.md#paths) |
| PPL built-in constants | [Constant reference](../source/ppl/constants.rst) |
| PPL data types | [Type reference](../source/ppl/data_types.rst) |
| PPL functions | [Function reference](../source/ppl/functions.rst) |
| PPL language | [Language reference](../source/ppl/language.rst), [versioned additions](../new_ppl.md) |
| PPL preprocessor | [Directives](../source/ppl/preprocessor.rst) |
| PPL statements | [Statement reference](../source/ppl/statements.rst) |
| PPE format | [Binary format](../ppe_format.md) |
| PPE compatibility | [Runtime compatibility](../source/ppl/compatibility.rst), [decompiler checks](../ppl.md#decompiler) |
| Protocols | [Transfer definitions](../configuration/components.md#transfer-protocols) |
| `PWRD` | [Security-level tables](../configuration/components.md#security-levels) |
| QWK / QWKnet | [Caller command](../../crates/icy_board_help/data/en/hlpqwk.md), [networking](../qwknet.md) |
| Quarantine | [Upload pipeline](../upload_processing.md) |
| Subscriptions | [Configuration](../configuration/board.md#events-accounting-and-subscriptions) |
| Sysop numeric commands | [Commands 1–16](commands.md#numeric-sysop-commands) |
| Uploads | [Caller command](../../crates/icy_board_help/data/en/hlpu.md), [operator policy](../upload_processing.md) |
| Users | [System manager](tools.md#icbsm-users-and-groups), [data format](../configuration/data.md) |
| ZCONNECT | [Network guide](../zconnect.md) |

## Reference boundaries

| Material | What it establishes |
| --- | --- |
| Command topics | Built-in English caller help, including explicitly unavailable commands. |
| Configuration tables | Persisted fields, defaults, required values and representation. |
| Tool reference | Operator CLI entry points and links to detailed workflows. |
| PPL handbook | Language, constants, functions, statements and compatibility. |
| New PPL reference | Versioned extensions and host-object APIs beyond classic PCBoard. |
| Compatibility audits | Detailed implementation differences and remaining gaps. |

A documented configuration field may be stored but inactive. A recognized PPL
operation may have compatibility limits. Read the behavior/status notes as
well as the syntax.

## Further references

```{toctree}
:maxdepth: 1

/compat/README
/compat/COMMAND_AUDIT
/compat/OPTIONS_AUDIT
/compat/DECLARE_AUDIT
/crates/tree-sitter-ppl/README
/docs/agent-setup
/docs/reliability_testing
/docs/ppl_compiler_architecture
/docs/hotkey_bars
/docs/new_macros
```
