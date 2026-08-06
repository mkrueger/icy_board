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

```sh
SDL_VIDEODRIVER=dummy flatpak run com.dosbox_x.DOSBox-X -silent -exit \
  -c "mount c $HOME/dos" -c "c:" -c "cd \\COMPAT" \
  -c "c:\\PCB\\PPLC.EXE T1.PPS > T1.LOG" -c "exit"
```

`.PPS` files **must** have CRLF line endings. With LF only, `PPLC.EXE` prints
"Source compilation complete" and emits an empty program — a silent failure, so
always check that the resulting `.PPE` is a plausible size.

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

Printing to the screen rather than writing a file with `FOPEN` means one fixture
works for both the original and icy_board, with no DOS paths involved.

**The `.out` files record what PCBoard does, including behaviour that looks like
a bug.** They are the specification; do not edit them to match icy_board.
