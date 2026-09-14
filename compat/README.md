# PCBoard compatibility oracle

icy_board is a rewrite of PCBoard, so "is this correct?" is really "does the
original do this?". The files here drive the **real PCBoard 15.4/M** inside
DOSBox-X and capture its behaviour as golden test data.

Nothing proprietary lives in this directory. The DOS installation, the PCBoard
binaries and the leaked C source are **never** copied into the repository — only
the `.pps` sources we write and the `.out` files the original produced.

## What you need

* `flatpak run com.dosbox_x.DOSBox-X`
* A PCBoard installation at `~/dos/PCB` and a FOSSIL driver at `~/dos/RA/X00.EXE`
* `~/dos/COMPAT` as a scratch directory (mounted as `C:\COMPAT`)

## Two oracles

### 1. Compiler oracle — run the original `PPLC.EXE`

`pplc_oracle.py` handles the DOS scratch copy, CP437 conversion, mandatory CRLF
line endings, DOSBox-X invocation and artifact collection:

```sh
python3 compat/pplc_oracle.py test.pps
python3 compat/pplc_oracle.py test.pps --disarr
python3 compat/pplc_oracle.py test.pps --run-icy icb/icyboard.toml
```

The outputs are `test.pcboard.log` and, when compilation succeeds,
`test.pcboard.ppe` beside the source. Use `--output-dir` to collect several
results elsewhere. A rejected source or failed IcyBoard run makes the wrapper
exit nonzero. `--disarr` passes the original compiler's `/DISARR` option, which
disables its normal array-dimension checks and should only be used when that
behavior is what the probe is testing.

When several inputs have the same filename stem, their collected artifacts get
a short path-derived suffix so one result cannot overwrite another.

The [DECLARE compatibility audit](DECLARE_AUDIT.md) records 23 authored probes
against PPLC 3.40: legacy implementation-signature precedence, multidimensional
formal syntax, and the stricter language-400 contract. It separates compiler
logs and decoded signatures from source-derived array-call semantics and
IcyBoard regression tests; it is not a claim of universal or byte-identical
compatibility. It also documents the runtime-400 whole-array parameter flag and
the recompile requirement for affected unreleased PPEs.

The lower-level invocation is:

```sh
SDL_VIDEODRIVER=dummy flatpak run com.dosbox_x.DOSBox-X -silent -exit \
  -c "mount c $HOME/dos" -c "c:" -c "cd \\COMPAT" \
  -c "c:\\PCB\\PPLC.EXE T1.PPS > T1.LOG" -c "exit"
```

`.PPS` files **must** have CRLF line endings. With LF only, `PPLC.EXE` prints
"Source compilation complete" and emits an empty program — a silent failure, so
always check that the resulting `.PPE` is a plausible size.

### Date/time runtime oracle without a serial connection

```sh
python3 compat/datetime_oracle.py
CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine --lib pcboard_datetime_ --locked
```

`datetime_oracle.py` clones `~/dos/PCB` into a private temporary directory,
compiles the authored `datetime.pps` with PPLC 3.40, and runs PCBoard's `/PPE:`
entry point. Only the clone is mounted in DOSBox-X. The fixture's `PRINTLN`
output is redirected to a DOS file with `FPUTLN`; no login, modem, or live BBS
configuration changes are needed. The harness checks the completion marker,
output boundaries and line count, and fingerprints the live installation.
Captures, compiler output and hashes are retained under `target/datetime-oracle/`.
It terminates only its own child process group on timeout.

`datetime.out` was captured from PCBoard 15.4/M on 2026-09-14: 120 result lines,
including three wrapping checksums over all 65,536 DATE values (date parts and
weekday, DDATE text, and DATE text roundtrips). The VM test compares the same
fixture in language modes 340 and 400. This covers the installed original's
American date order and colon time separator, not every DOS country setting.

The governing source paths are `pcb-libs/SOURCE/MISC/JULIAN.C` (`breakdate`,
`datetojulian`, `juliantodate`) and `pcb-main/SOURCE/PPL/{VAR,EVALP}.CPP`
(coercions, validation, `TIMEAP`, `MKDATE`). DDATE delegates to CodeBase routines
whose implementation is absent from this source tree; its fixed-width parsing,
Gregorian dates, year-zero anomaly and signed Julian offset are oracle-tested.
VALDATE deliberately keeps the original coarse month/day/year checks, including
acceptance of February 31 with a two-digit year. VALTIME and TOTIME deliberately
retain their different parsing rules and signed overflow behavior.

An initial probe also tried `MKDATE(2024, 13, 32)`. Its original implementation
reads outside a local array; the observed result is not a portable date rule.
IcyBoard returns zero for out-of-array MKDATE months instead of emulating an
arbitrary memory read. DATE parsing retains the observed contiguous two-row day
table behavior for months 13..24; larger indices return zero. Unicode input must
not panic. These safety boundaries are not claims of identical undefined behavior.

### 2. Runtime oracle — a live BBS session over TCP

DOSBox-X bridges the DOS serial port to a socket and `bbs_session.py` speaks to
it from the host:

```sh
pkill -f dosbox-x; sleep 2; rm -f ~/dos/PCB/NODE1/ENDPCB
cd ~/dos/COMPAT && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy setsid \
  flatpak run com.dosbox_x.DOSBox-X -conf nullmodem.conf >/tmp/board.log 2>&1 &
python3 compat/bbs_session.py --wait 30 --idle 4 --total 25 --max-steps 45 \
  --script compat/logon.expect --until "command:" --send "ZZTEST"
```

Hard-won details, all of which cost a debugging cycle:

* `serial1=nullmodem port:2323 transparent:1`. The `modem` serial type accepts
  TCP connections but PCBoard still sees no carrier and exits.
* The client must connect **before** PCBoard starts, otherwise PCBoard exits
  immediately with errorlevel 5 ("caller goodbye"). `--wait` retries the connect
  from before DOS has finished booting.
* Do not pass `-silent`; it means "quit after the autoexec", not "be quiet".
* The autoexec must put `c:\pcb` on the `PATH`, load the FOSSIL driver
  (`x00 e`), and start PCBoard from a batch file. `-c "call foo.bat"` does not
  run batch files.
* PCBoard exits after every call, so the batch file has to loop to serve more
  than one session.

## Driving the session

`bbs_session.py` answers prompts by pattern rather than by position, because the
logon questionnaire is not a fixed-length list — a single extra or missing
question silently shifts every later answer onto the wrong prompt.

`logon.expect` holds the rules as `REGEX=RESPONSE`, first match wins, so put
specific patterns first. It covers both first-time registration and returning
logon for the throwaway `ORACLE TESTER` account, which exists only inside the
local DOSBox image.

Positional `--send` lines are still supported and run *after* the expect script
finishes, which is where board commands belong.

To run a PPE, append a 64-byte record to `~/dos/PCB/GEN/CMD.LST` (16 bytes
command name, 32 bytes PPE path, 16 zero bytes) and type that command at the
`command:` prompt.

## Writing to the board

Expect rules split on the first `=`, and most PCBoard prompts contain one, so a
rule has to stop before it: `To \(Enter\)=ALL` works, `To \(Enter\)=.ALL.\?=ALL`
sends nonsense. Rules only run before `--until` matches, so a command that has
to be typed at the menu belongs in the rules (`command:=E`) rather than in a
`--send`, otherwise the session stops at the menu with the command unsent.

This writes a message and saves it:

```sh
python3 compat/bbs_session.py --wait 40 --idle 4 --total 20 --max-steps 45 \
  --script compat/logon.expect \
  --expect "Text Entry Command\?=S" --expect "alone to end=oracle body line" \
  --expect "  2: =" --expect "To \(Enter\)=ALL" \
  --expect "Subject \(Enter\)=ORACLE TEST MSG" \
  --expect "Message Security \(H\)=" --expect "command:=E" \
  --until "NOTHINGMATCHES"
```

`Message Security` is asked between the subject and the text, which is easy to
miss when answers are sent by position instead of by pattern.

## Golden test format

Test programs print `name=value` lines between `---BEGIN---` and `---END---`:

```
PRINTLN "---BEGIN---"
PRINTLN "mid_oob=["+MID("ABC",2,6)+"]"
PRINTLN "---END---"
```

Capture the original's answer with
`sed -n '/---BEGIN---/,/---END---/p'` and commit the `.pps` plus a `.out` file
into `crates/icy_board_engine/tests/test_data/`, where `test_run.rs` picks it up
automatically.

PCBoard talks CP437; icy_board works in UTF-8. `bbs_session.py` decodes CP437 on
the way in, so the fixtures are UTF-8 like the rest of the repository.

Because strings are UTF-8, `MID`/`LEFT`/`RIGHT`/`LEN` count characters where
PCBoard counted bytes. Both agree for the ASCII range, so keep edge-case fixtures
inside it — a case built from box-drawing or accented characters will differ from
the original by design, and its `.out` cannot be copied from the oracle verbatim.

### Type-7 STRING capacity

The `string255_*.pps` probes verify that PCBoard's type-7 `STRING` stores **256
payload bytes**, despite commonly being described as a 255-character type.
Assignment, concatenation, array elements, routine parameters, local variables,
and function results all truncate to 256 bytes. Type-13 `BIGSTR` remains
unbounded. `FPUTLN` and `FWRITE` persist all 300 probe bytes; `FGET` and `FREAD`
truncate only when their destination is type 7. The adjacent `.out` files are
the output captured from PCBoard 15.4/M.

Printing to the screen rather than writing a file with `FOPEN` means one fixture
works for both the original and icy_board, with no DOS paths involved.

**The `.out` files record what PCBoard does, including behaviour that looks like
a bug.** They are the specification; do not edit them to match icy_board.
