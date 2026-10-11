# Troubleshooting

Start with the failing operation, exact binary version and board configuration
path. Preserve the error and exit status. A process starting, TOML parsing, or
a sysop login succeeding is not proof that a normal caller can use the feature.

## Diagnostic sequence

1. Run `icboard --version` and identify the selected board.
2. Run `icbsetup check icboard.toml` from that board.
3. Inspect the board application log and any relevant command output.
4. Reproduce locally with the same account class, conference and terminal mode.
5. Change one relevant setting at a time; retain a backup.

`check` validates configuration and paths, not public network reachability,
all PPE behavior or every permission decision. Unknown TOML keys may be
ignored, so check the exact spelling in the [format reference](../configuration/README.md).

## Board and connections

| Symptom | Check / next action |
| --- | --- |
| Configuration not found | Pass the intended board explicitly. Check the current directory and `ICB_PATH`; the binaries directory is not necessarily the board directory. |
| Missing-file reports after moving a board | Inspect absolute paths and referenced component files. Ordinary relative paths start at the main configuration directory. |
| Address already in use | Identify the process already listening on the configured port; do not start a second board on the same endpoint. |
| Local session works but remote login does not | Inspect listener enable flags, bind address, port, protocol and host firewall. `--localon` does not test remote transport. |
| Test board is reachable from another machine | An empty Telnet address is a wildcard binding. Stop it and select loopback before continuing the trial. |
| SSH or secure WebSocket fails | Verify the transport-specific key/certificate settings and client protocol. A Telnet client does not validate these listeners. |
| New users cannot log in | Check registration settings, account status, passwords, admission/event state and the relevant log entry. |
| Another tool reports a board lock | Stop or finish the owning board/tool. Do not delete state files to circumvent coordinated writing. |

See [listeners](../configuration/board.md#login-servers),
[startup](../gettingstarted.md) and [events](../events.md).

## Display and menus

| Symptom | Check / next action |
| --- | --- |
| Box-drawing characters are corrupted | Distinguish CP437 from UTF-8 display files. UTF-8 display content needs a BOM; structural TOML uses UTF-8. |
| Wrong language or artwork is displayed | Inspect the configured base name and available language, graphics and security variants. A more specific variant may shadow a newly generated file. |
| Prompt change has no effect | ICBTEXT, menu prompt strings, display files and command help are separate sources. Edit the one actually used by the prompt. |
| A menu choice is visible but cannot run | Check command permission, conference policy, action parameters and the command override lists. Visibility is not authorization. |
| A command runs something unexpected | Look for a conference or global override and chained menu actions. |
| Menu preview works but the live menu does not | Preview does not execute actions and only partially evaluates security. Test with a real caller. |
| Generated help will not overwrite a file | Inspect ownership/local-edit reports. Use dry-run and the documented adoption/replacement workflow, not blanket deletion. |
| Help generation fails at a narrow width | Width accepts 40–79, but the shipped English topics need at least 68 columns. Some examples cannot be wrapped safely. |

References: [display conventions](../source/customizing/display_files.rst),
[menus](../mkicbmnu.md), [macros](../macros.md),
[help generation](../gettingstarted.md#command-help).

## Messages and access

| Symptom | Check / next action |
| --- | --- |
| Message number is not found | Verify the current conference and message area. Numbers are local to a base. |
| Selecting a conference does not grant access | Scan selection and registration/access rights are different. Inspect registration and area expressions. |
| Mail exists but a scan does not show it | Check selected conferences/areas, scan range, last-read marks and private-message visibility. Test inbox mail separately. |
| Sender-password message is readable by others | Sender-password protection is not receiver-only privacy. Review the message-security choices. |
| Old PPE sees only one area | Check its compatibility API and default-area behavior; use the area-aware API where needed. |
| Reader accepts a command but does not perform it | Consult the command topic and implementation limitations. Recognized syntax is not a guarantee that an operation is implemented. |

References: [read messages](../../crates/icy_board_help/data/en/hlpr.md),
[end-of-message actions](../../crates/icy_board_help/data/en/hlpendr.md),
[security](../../crates/icy_board_help/data/en/hlpsec.md),
[PPL compatibility](../source/ppl/compatibility.rst).

## Files and transfers

| Symptom | Check / next action |
| --- | --- |
| Physical file is absent from listings | Inspect the file-area database and scan/import workflow; copying a file is not necessarily importing its metadata. |
| Description changed after scanning | Review metadata precedence and force/overwrite options before scanning again. |
| Download refused despite available time | Check file visibility, locked status, protocol, ratios, byte/file ceilings and accounting balance. These checks are separate. |
| Upload was received but is not listed | Inspect scanner, quarantine, description and publication results. Do not assume reception means acceptance. |
| Transfer stalls | Verify that the caller's terminal supports the selected protocol and that external protocol commands are correctly configured. |
| File-base cleanup would remove unexpected entries | Stop; run the read-only check/list steps and verify the physical directory and selected area before pruning. |

References: [file areas](../icbfile.md),
[upload pipeline](../upload_processing.md),
[transfer protocols](../configuration/components.md#transfer-protocols),
[accounting](../accounting.md).

## Events, mail and PPEs

| Symptom | Check / next action |
| --- | --- |
| Maintenance never begins | Inspect active sessions and managed workers. An uncooperative session or command can delay the drain. |
| An event did not run during downtime | Startup is not catch-up execution. Review the event's schedule and durable history before a manual run. |
| Admission remains closed after a journal failure | Repair the underlying storage problem and follow event recovery; do not discard occurrence records blindly. |
| FTN mail is queued but not delivered | Distinguish scan, poll and toss; inspect link identity, credentials, routing and logs. |
| QWKnet or ZCONNECT replays or loses packets | Preserve checkpoints and queues for diagnosis. Acknowledgement and retry rules are protocol-specific. |
| PPE fails after recompilation | Compare source-language and runtime targets, declarations, dependencies and binary version. |
| `ppld --check` passes but the PPE fails | Static compatibility inspection does not execute the PPE or prove that its board/data assumptions are valid. |
| PPL HTTP request is denied | Check the opt-in policy and destination restrictions; do not remove policy controls merely to hide an error. |

References: [events](../events.md), [FTN](../source/mailer.rst),
[QWKnet](../qwknet.md), [ZCONNECT](../zconnect.md),
[PPL compiler](../pplc.md), [HTTP policy](../configuration/board.md#ppe-http-policy).

## Reporting a reproducible problem

Include the version, platform, command or caller interaction, expected result,
actual result, and the smallest relevant configuration fragment. For display
problems include terminal dimensions, graphics mode, encoding and language.
For access problems describe the non-secret account properties and area policy.

Remove passwords, tokens, SMTP/door credentials, private mail, personal user
data and public network addresses that should not be disclosed. Never attach a
whole production board as a default bug report.
