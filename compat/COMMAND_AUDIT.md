# Command compatibility audit: icy_board vs PCBoard 15.x

Why this matters: PPEs answer prompts with `KBDSTUFF`. A command that asks one
prompt too few or too many silently feeds the wrong answer to the wrong
question, and the PPE appears to corrupt data rather than fail. **Prompt count
and order are the compatibility contract**, not the wording.

icy_board may add prompts or improve behaviour, but every prompt PCBoard asks
must still be asked, in the same order, with the same token-skipping rules.

Derived from the observable behaviour of PCBoard 15.x against
`crates/icy_board_engine/src/icy_board/state/user_commands/pcb/`.
Numbers in parentheses are PCBTEXT ids.

Status is from behaviour analysis. Nothing here is oracle-verified yet — see
[README.md](README.md) for how to confirm a row against the real board.

## Ground rules PCBoard applies to every command

1. Input is uppercased, then split on space **and semicolon**; a bare `NS` token
   is removed and decrements the count.
2. The dispatcher checks security, displays the command display file, then calls
   the handler with `NumTokens - 1`. That subtraction is what
   makes "prompt only when no token was supplied" work.
3. Security failure prints TXT_MENUSELUNAVAIL (413); repeated violations add
   TXT_SECURITYVIOL (12) and TXT_AUTODISCONNECTNOW (59), then log off.
4. CMD.LST is consulted **before** all built-in commands, so it can shadow them —
   **except when the input came from type-ahead**, where CMD.LST is skipped
   entirely.
5. Word commands match by prefix, first table entry wins.

## Main menu letters

| Cmd | PCBoard prompt sequence | icy_board | Status |
|---|---|---|---|
| A | delegates to J with a forced `0` token; post-join VIEWCONFMEMBERS (88), SCANMSGBASE (296) | delegates correctly; both post-join prompts present | ✅ |
| B | BLTLISTCMDEXPERT (611) / BLTLISTCOMMAND (224); TEXTTOSCANFOR (70) for `S` | same | ✅ |
| C | LEAVECOMMENT (1), REQRETRECEIPT (630), USEFULLSCREEN (498) + REQUIRESANSI (499) retry, editor loop 163/222 | same | ✅ |
| D | BYEAFTERDOWNLOAD (490), DFLTFILENAMETODNLD (300), DOWNLOADTAGGED (500), FILENAMETODOWNLOAD (61)/(728), PROTOCOLFORXFER (280), GOODBYEAFTERDOWN (550), EDITBATCH (551), REMOVEFILENUMBER (552) | 500/550/551/552 plus the flag prompts, which take a stacked token | ⚠️ PCBoard asks for names in a loop, icy_board asks once |
| E | MSGTO (199) **always**, MSGSUBJ (200), MSGSECURITY (194) loop, REQRETRECEIPT (630), ECHOMESSAGE (221), ROUTETO (636), DESTNEWSGROUP (736), FOLLOWUPNEWSGROUP (737), editor | same; tokens pre-fill 199 instead of skipping it | ✅ |
| F | FILELISTCMDEXPRT (585) / FILELISTCOMMAND (223) | same | ✅ |
| G | FILESAREFLAGGED (603) then CONTINUELOGOFF (605); first token as yes-shortcut | same | ✅ |
| H | HELPPROMPT (63), token skips | same | ✅ |
| I | none | none | ✅ |
| J | JOINCONFNUM (64), TEXTTOSCANFOR (70), PWRDTOJOIN (640), then VIEWCONFMEMBERS (88), SCANMSGBASE (296) | all five, and only on the first join of a conference | ✅ |
| K | MSGNUMBERTOKILL (330), YOURPASSWORD (148) | uses PasswordToReadMessage | ⚠️ wrong prompt id |
| L | DATETOSEARCH (72) when OptionalNewScan, SEARCHFILENAME (71), FILENUMEXPERT (352)/NOVICE (353) | all three; 72 appears only for `N`, while `S` takes the stored date without asking | ✅ |
| M | no prompt; CT/AN/GR/RI token | same + AvatarOn | ✅ extension is fine |
| N | DATETOSEARCH (72), FILENUMEXPERT/NOVICE | same | ✅ |
| O | forced `NumTokens=1` so tokens are ignored; SYSOPUNAVAILABLE then COMMENTINSTEAD (571) **only if user has SEC_C** | gated on SEC_C, asked once, and also asked after a page that rang out | ✅ |
| P | CURPAGELEN (284) then ENTERPAGELENGTH (146); token skips both | same | ✅ |
| Q | MSGSCANCMDEXPERT (613)/MSGSCANCOMMAND (424); tokens skip it; shares R's command parser | same parser, quick-scan number semantics | ✅ |
| R | MSGREADCMDEXPRT (584)/MSGREADCOMMAND (425); per-message loop ENDOFMSGEXPERT (612)/ENDOFMESSAGE (197); MOVE (465)/COPY (569) | prompts and parser match; the capture and QWK commands parse but do nothing yet | ⚠️ |
| S | QNUMTOANSWER (67), **always prompts, ignores tokens** | same | ✅ |
| T | DESIREDPROTOCOL (198); token skips | same | ✅ |
| U | CONTINUEUPLOAD (449), FILENAMETOUPLOAD (68)/(729), PROTOCOLFORXFER (280), GOODBYEAFTERUP (474) | 449, 68 (token aware), 280 when no protocol is set, then 474 | ⚠️ 474 is unconditional; PCBoard only asks it in batch mode |
| V | no prompts; returns if STAT display file missing | built-in settings display as fallback | ⚠️ improvement, divergent |
| W | NEWPASSWORD (152), REENTERPASSWORD (111), CITYSTATE (265), BUSDATAPHONE (113), HOMEVOICEPHONE (114), COMMENTFIELDPROMPT (2), CLSBETWEENMSGS (556), SCROLLMSGBODY (627), USEBIGHEADERS (628), SETFSEDEFAULT (583), DEFAULTWIDEMSGS (637), GETALIASNAME (690), USESHORTDESC (746), SELECTCONFS (325), address block, QWK limits (732-735) | same, then the icy_board extras | ✅ |
| X | no prompt; ON/OFF token | same | ✅ |
| Y | MSGSCANPROMPT (155); tokens skip | same | ✅ |
| Z | DATETOSEARCH (72) conditional, TEXTTOSCANFOR (70), FILENUMEXPERT/NOVICE | all three; `N` prompts, `S` takes the stored date without asking | ✅ |

## Word commands

Built-in table is 30 entries: ALIAS, BD, BROADCAST, BU, BYE,
CHAT, DB, DOOR, DOWNLOAD, FLAG, HELP, JOIN, LANG, MENU, NEWS, NODE, OPEN, PPE,
QWK, REPLY, RM, RM+, RM-, SELECT, TEST, TS, UB, UPLOAD, USERS, WHO.

icy_board resolves these in `try_find_command` (`state/mod.rs:833-960`).

| Cmd | Notes | Status |
|---|---|---|
| ALIAS, BYE, BROADCAST, CHAT, FLAG, HELP, JOIN, LANG, MENU, NEWS, PPE, QWK, REPLY, RM/RM+/RM-, SELECT, TEST, USERS, WHO | present, prompt sequences line up | ✅ |
| DOOR / OPEN | PCBoard adds PWRDFORDOOR (415) and CONTINUEDOOR (604) when files are flagged | both present; the sysop skips them as in PCBoard | ✅ |
| SELECT | PCBoard can ask SELECTCONFFLAGS (564) | ⚠️ 1 missing |
| TS | area token `S` handling is a TODO | 🚧 partial |
| BD, BU | delegate to D/U with the batch flag on, as PCBoard does | ✅ |
| DB, UB, NODE | PCBoard aliases for download / upload / chat | ✅ recognised |
| OPEN | the matcher compared against mixed case `"Open"` while the command is upper-cased first | ✅ fixed |
| ! | repeats the last command of five characters or more, help in HLP! | ✅ |
| AREA | icy_board extension, not in PCBoard's table | ⚠️ extension |

Unrecognised words fall through to the door list when the user has OPEN access,
before TXT_INVALIDENTRY. icy_board goes straight to
InvalidEntry.

Abbreviation rule differs: PCBoard requires the typed string to
reach a minimum length before it accepts an abbreviation,
while icy_board accepts any prefix. Table order also differs, and first match
wins in both, so an abbreviation can resolve to a different command.

## Numeric commands (sysop functions)

PCBoard maps 1-16. icy_board implements `1`, `2`, `4`, `5`, `6`, `11`, `12`, `13` and `16`.

| # | Function | PCBoard prompts | Status |
|---|---|---|---|
| 1 | view callers log | VIEWCALLERS (212), TEXTTOSCANFOR (70), DELETECALLERSLOG (80) | ✅ |
| 2 | view/print users | VIEWPRINTUSERS (213) | ✅ |
| 3 | pack message base | PACKTHEMSGBASE (79), GENERATENEWINDEX (461), PURGEOLDERTHAN (106), PURGEPRIVRECEIVED (89), RENUMBERDURINGPACK (82), NEWLOWMSGNUM (83) | ❌ missing |
| 4 | recover message | MSGNUMTOACTIVATE (77) | ✅ |
| 5 | quick/header scan | 613/424 via message reader | ✅ |
| 6 | view a text file | TEXTVIEWFILENAME (62) | ✅ |
| 7 | user maintenance | USERMODEXPERT (167)/USERMODNONEXPERT (168), DELETERECORD (561) | ❌ missing |
| 8 | pack users file | PACKTHEUSERSFILE (86), KEEPLOCKEDOUT (105), PURGEOLDERTHAN (106), KEEPSECURITY (107) | ❌ missing |
| 9 | remote DOS | EXITTODOS (90) | ❌ intentionally out of scope? |
| 10 | DOS command | DOSFUNCTION (142) | ❌ intentionally out of scope? |
| 11 | node list (forces `X`) | none | ✅ |
| 12 | log off a node | NODENUMTOLOGOFF (65) | ✅ |
| 13 | view node callers log | NODETOVIEW (66), TEXTTOSCANFOR (70) | ✅ filters the shared log by node |
| 14 | drop node to DOS | NODENUMTODROP (274), DROPNOW (345) | ❌ missing |
| 15 | recycle a node | RECYCLETHRUDOS (348) | ❌ missing |
| 16 | directory listing | ENTERDIRCMD (740) | ⚠️ name, size and date, not a DOS DIR |

PCBoard kept one caller log per node. icy_board keeps a single shared log, so
`write_caller_log` stamps the node on every line and 13 filters on that; `A`
scans every node. The `P` option of 1 and 2 has no printer to go to, so it runs
the listing without stopping instead.

5 is the quick scan with PCBoard's header scan flag: it uses FIVESCANHEADER (158)
instead of QUICKSCANHEADER (725), puts an `A`/`I` column in front of every line
and is the only scan that lists killed messages. It lists and stops rather than
walking into the messages.

12 sends the `Shutdown` message the event scheduler already uses, so the node
shows the text and drops its caller. It refuses to log off the node it runs on.
16 lists name, size and date instead of shelling out to `DIR`, and like 6 it
stays inside the board directory because a PPE can stuff the path.

## Structural issues (fix before the per-command work)

All four are resolved; kept here for the reasoning.

1. ~~**Conference CMD.LST is never loaded.**~~ Fixed: the list is loaded at board
   load time. PCBoard gives the conference list priority over the global one.
2. ~~**CMD.LST must be skipped for type-ahead input**~~. Fixed:
   both interactive command loops now pass the flag the PPL `COMMAND` statement
   was already threading correctly.
3. ~~**Wrong labels in security checks**~~ (`menu_runner.rs`). Fixed: the denial
   message names the command the dispatcher registers.
4. **CMD.LST record layout** is `Name[15]`, `SecLevel`, `File[40]`,
   `ChargePerUse`, `ChargePerMin`. The offsets the
   importer uses are correct, so nothing is misparsed, but the two trailing
   float charge fields are dropped — `Command` has nowhere to put them.

## TODO, in priority order

Priority is "how badly does this break a PPE that stuffs input".

### P0 — breaks stuffed input today

- [x] **W**: NEWPASSWORD (152) + REENTERPASSWORD (111) are now the first two
      prompts and the rest follow PCBoard's order. SCROLLMSGBODY
      (627), USEBIGHEADERS (628), DEFAULTWIDEMSGS (637), SELECTCONFS (325) and
      the four QWK limits (732-735) were added; the icy_board extras moved
      behind the last PCBoard question. Verified against the oracle.
- [x] **R**: PCBoard's read-command parser is reproduced whole, so the read
      loop understands the same 42 words and single letters, the same number
      grammar (`5`, `5+`, `5-`, `10-20`, several groups), and asks the same
      follow-up questions: TEXTTOSCANFOR (70), USERSEARCHTONAME (644),
      USERSEARCHFROMNAME (645), USERSEARCHNAME (567), DATETOSEARCH (72),
      MSGSEARCHFROM (195) and RESUMEALL (483). MOVE (465)/COPY (569) parse but
      the transfer itself is still missing.
- [x] **Q**: shares that parser, so a bare number scans forward rather than
      showing one message, and the message range in the prompt is the real
      low-high pair instead of base-and-count.
- [x] **S**: PCBoard ignores the tokens here, so S clears them
      and always shows the menu and asks QNUMTOANSWER (67).
- [x] **E**: ROUTETO (636), DESTNEWSGROUP (736) and FOLLOWUPNEWSGROUP (737)
      are asked under PCBoard's conditions, the recipient tokens pre-fill the
      MSGTO question instead of skipping it, and the pack-out date retries on a
      bad date. REQRETRECEIPT (630) turned out to be present already.
- [x] Load conference CMD.LST on join; skip CMD.LST for type-ahead input.

### P1 — missing prompts, wrong answer consumed

Most of this list turned out to be already done when it was re-checked against
the code; only the last three rows were real.

- [x] **J** and **A**: post-join VIEWCONFMEMBERS (88) and SCANMSGBASE (296) were
      already asked, and A already delegates to J with a forced `0` token.
- [x] **O**: COMMENTINSTEAD (571) was already gated on C access, and the
      dispatcher already clears the tokens.
- [x] **DOOR/OPEN**: PWRDFORDOOR (415) and CONTINUEDOOR (604) were already asked.
- [x] **L**, **Z**: `S` no longer asks DATETOSEARCH (72). PCBoard's
      `getdatefromuser` takes the stored date as soon as the date buffer holds
      `S`, so only `N` prompts; `S` beats `N` when both are on the line.
- [x] **U**: PROTOCOLFORXFER (280) is asked when the caller has no usable
      protocol, and the protocol is settled *before* GOODBYEAFTERUP (474), the
      order PCBoard uses. CONTINUEUPLOAD (449) and the filename token were there.
- [x] **D**: PROTOCOLFORXFER (280) is asked instead of quietly transferring
      nothing when the caller's protocol is `N` or unknown.

Blocked on features that do not exist yet rather than on the prompt code:

- [ ] **D**: BYEAFTERDOWNLOAD (490) only fires when the caller captured messages
      to a file. Capture parses but does not run, so the prompt is unreachable —
      do it with the capture work.
- [ ] **D**: DFLTFILENAMETODNLD (300) pre-fills from the last file the caller
      viewed with `F`/`V`. Nothing tracks a last-viewed file yet.
- [ ] **D**: FILENAMETODOWNLOAD (61) is the non-batch wording of (728). There is
      no session batch flag, so the split belongs with the BD/BU work. Both
      consume one token, so this is wording, not contract.

### P2 — smaller divergences

- [ ] **K**: use YOURPASSWORD (148).
- [ ] **SELECT**: SELECTCONFFLAGS (564).
- [x] Fix the `"Open"` case bug and add aliases `NODE`, `DB`, `UB`.
- [ ] Fall back to the door list for unknown words when the user has OPEN access.
- [ ] Match PCBoard's minimum-abbreviation rule instead of accepting any prefix.
- [ ] Implement **BD**/**BU** batch transfer.
- [ ] Decide whether **V** should keep its built-in display when STAT is missing.

### P3 — sysop numeric commands

- [x] 1, 2, 6, 11, 13 — the ones that only needed the caller log, the user file,
      `display_file` and the node table. They also make `sec_1_view_caller_log`,
      `sec_2_view_usr_list`, `sec_6_view_any_file`, `sec_11_view_other_nodes` and
      `sec_13_view_alt_node_callers` live instead of dead config.
- [x] 5, 12, 16 — header scan, forced node logoff and the directory listing.
      `sec_5_list_message_hdr` and `sec_12_logoff_alt_node` are live too now.
- [ ] 14, 15 — drop a node to DOS and recycle it. The signalling 12 uses is
      there; what is missing is a decision on what they should mean here.
- [ ] 3, 8 — packing message bases and the user file. Data destructive, wants
      tests before anything else.
- [ ] 7 — user maintenance, the one that needs a real editor.
- [ ] Decide explicitly whether 9 (remote DOS) and 10 (DOS shell) are out of
      scope; if so, document it rather than leaving them unimplemented.

### Behind the read parser

The words are recognised and consume the right number of tokens, which is what
a stuffing PPE cares about. K, P, U, MOVE, COPY, SET and E act as well, and the
message filter behind `TS`, `FROM`, `TO`, `Y`, `YA`, `N` and `T` skips what the
command did not ask for without asking anything. `S` picks up at the last-read
pointer. These still need the feature underneath: capture and QWK
(`C`/`D`/`Z`/`QWK`), FORWARD, FLAG, export, and the conference walk for
`A`/`ALL`/`WAIT`.

Conferences hold message areas in icy_board, which PCBoard has no notion of. A
board shaped the way PCBoard expects has one area per conference, so MOVE and
COPY ask for the conference and nothing more; the area question only appears
when the target conference really has several.

### Verification

- [ ] Add a golden test per command that asserts the **prompt sequence**, not
      just the final output. A, ALIAS, B, BYE, C, E, G, J, M, P, T, W and X have
      tests today.
- [ ] Confirm each ❌ row against the oracle before fixing, so the fix targets
      real behaviour and not my reading of it.
- [x] Fix the 6 pre-existing `icboard` test failures (cmd_a, cmd_b ×2, cmd_j,
      test_last_cmd ×2) — they were asserting a CP437-decoded BEL.
