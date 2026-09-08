#!/usr/bin/env python3
"""Disposable PCBoard paging oracle. Requires pyte; credentials stay in logon.expect.

Only a new temporary clone is mounted. Raw receive streams, 80x25 remote-screen
snapshots, prompt boundaries and live-fixture hashes go to a unique target folder.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import bbs_session as bbs
import pyte

ROOT = Path(__file__).resolve().parents[1]
DOS = Path.home() / "dos"
MENU = re.compile(r"Main Board Command\?[^\r\n]*$", re.I)
MORE = re.compile(r"(?:\(H\)elp,\s+More\?|More\?\s*\[Y)[^\r\n]*$", re.I)
ENTER = re.compile(r"Press \(Enter\) to continue\?[^\r\n]*$", re.I)
PAGE_INPUT = re.compile(r"Enter new length \(0\)=continuous[^\r\n]*$", re.I)
NUMBER = re.compile(r"PG[CN](?:22|23|24|46)-(\d{3})")
MORE_HELP_LINE = re.compile(r"^[ \t]+\((?:Enter|Y|N|NS)\)[^\r\n]*", re.M)


def fingerprint(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(root.rglob("*")) if p.is_file()}


def live_fingerprints():
    return {"PCB": fingerprint(DOS / "PCB"), "COMPAT": fingerprint(DOS / "COMPAT"),
            "RA/X00.EXE": hashlib.sha256((DOS / "RA/X00.EXE").read_bytes()).hexdigest()}


def require(view, pattern):
    if not pattern.search(view):
        raise RuntimeError(f"Prompt mismatch: expected {pattern.pattern!r}; sent nothing further")


class Session:
    def __init__(self, sock, out):
        self.sock = sock
        self.raw = (out / "received.bin").open("xb")
        self.transcript = (out / "transcript.txt").open("x", encoding="utf-8")
        self.events_file = (out / "events.jsonl").open("x", encoding="utf-8")
        self.screen = pyte.Screen(80, 25)
        self.stream = pyte.Stream(self.screen)
        self.events = []
        self.phase = "login"

    def close(self):
        for handle in (self.raw, self.transcript, self.events_file):
            handle.close()

    def read(self, sent=None):
        data = bbs.drain(self.sock, 1.0 if self.phase == "login" else 0.25, 15.0)
        offset = self.raw.tell()
        self.raw.write(data)
        self.raw.flush()
        view = bbs.render(data, True)
        self.stream.feed(data.decode("cp437"))
        prompt = next((name for name, regex in (("menu", MENU), ("more", MORE),
                      ("press_enter", ENTER), ("page_input", PAGE_INPUT))
                      if regex.search(view)), "unknown")
        event = {"id": len(self.events), "phase": self.phase, "sent": sent,
                 "raw_offset": offset, "raw_length": len(data), "crlf": data.count(b"\r\n"),
                 "prompt": prompt, "numbered_lines": [int(n) for n in NUMBER.findall(view)],
                 "more_help_lines": MORE_HELP_LINE.findall(view),
                 "text": view, "cursor_1based": [self.screen.cursor.y + 1, self.screen.cursor.x + 1],
                 "screen_80x25": self.screen.display}
        self.events.append(event)
        self.events_file.write(json.dumps(event, ensure_ascii=False) + "\n")
        self.events_file.flush()
        self.transcript.write(f"\n=== event {event['id']} / {self.phase} / SEND {sent!r} ===\n{view}\n")
        self.transcript.flush()
        if not data:
            raise RuntimeError("No output / connection closed; sent nothing further")
        return view

    def send(self, answer, label=None):
        self.sock.sendall(answer.encode("cp437") + b"\r")
        return self.read(answer if label is None else label)

    def settle(self, view):
        for _ in range(120):
            if MENU.search(view):
                return view
            if MORE.search(view) or ENTER.search(view):
                view = self.send("")
            else:
                raise RuntimeError("Unexpected prompt while returning to menu; sent nothing further")
        raise RuntimeError("Menu transition exceeded safety bound")

    def page_length(self, view, value=None):
        require(view, MENU)
        self.phase = f"page_length_{value if value is not None else 'initial'}"
        if value is not None:
            view = self.settle(self.send(f"P {value}"))
        require(view, MENU)
        view = self.send("P")
        chunks = [view]
        for _ in range(120):
            if PAGE_INPUT.search(view):
                break
            require(view, MORE)
            view = self.send("")
            chunks.append(view)
        require(view, PAGE_INPUT)
        match = re.search(r"Page Length is currently set to ([0-9]+)", "".join(chunks), re.I)
        if not match:
            raise RuntimeError("Could not measure current user page length")
        measured = int(match[1])
        if value is not None and measured != value:
            raise RuntimeError(f"Page length readback {measured} != requested {value}")
        return self.settle(self.send("")), measured

    def probe(self, view, page_length, name, response, line_count=None):
        require(view, MENU)
        self.phase = f"p{page_length}_{name}_{response or 'Enter'}"
        print(f"Starting {self.phase}", flush=True)
        first_event = len(self.events)
        view = self.send(f"H {name}")
        more_count = 0
        for _ in range(250):
            if MENU.search(view):
                break
            if MORE.search(view):
                answer = response if more_count == 0 or response == "Y" else ""
                more_count += 1
                view = self.send(answer)
            elif ENTER.search(view):
                view = self.send("")
            else:
                raise RuntimeError(f"Unknown paging prompt in {self.phase}; sent nothing further")
        require(view, MENU)
        events = self.events[first_event:]
        seen = []
        stops = []
        for event in events:
            seen.extend(event["numbered_lines"])
            if event["prompt"] in {"more", "press_enter"}:
                stops.append({"event": event["id"], "prompt": event["prompt"],
                              "sequence": len(stops) + 1, "sent": event["sent"],
                              "after_line": seen[-1] if seen else None,
                              "new_lines": event["numbered_lines"],
                              "more_help_lines": event["more_help_lines"],
                              "cursor_1based": event["cursor_1based"],
                              "crlf": event["crlf"]})
        if line_count is not None:
            expected = list(range(1, line_count + 1))
            if response == "N" and more_count:
                if not seen or seen != expected[:len(seen)]:
                    raise RuntimeError("Aborted output is not a contiguous numbered prefix")
            elif seen != expected:
                raise RuntimeError(f"Missing/duplicated numbered output in {self.phase}: {seen}")
        result = {"case": self.phase, "command": f"H {name}", "page_length": page_length,
                  "first_more_response": response or "Enter", "line_count": line_count,
                  "first_event": first_event, "last_event": events[-1]["id"],
                  "numbered_lines_received": seen, "stops": stops, "status": "complete"}
        print(f"{self.phase}: More={more_count}, PressEnter={sum(s['prompt'] == 'press_enter' for s in stops)}, "
              f"numbered={len(seen)}", flush=True)
        if response == "H":
            for stop in stops:
                print(json.dumps(stop, ensure_ascii=False), flush=True)
        return view, result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--help-boundaries", action="store_true",
                        help="only probe PGC24 with H at the first More, then Enter, for page lengths 1, 2, 5, 23")
    args = parser.parse_args()
    os.umask(0o077)
    parent = ROOT / "target/paging-oracle"
    parent.mkdir(parents=True, exist_ok=True)
    out = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    print(f"CAPTURES={out}", flush=True)
    metadata = {"status": "starting", "output": str(out), "cases": [],
                "mode": "help_boundaries" if args.help_boundaries else "full"}
    before = None
    scratch = None
    proc = None
    session = None
    try:
        before = live_fingerprints()
        metadata["live_before"] = before
        scratch = Path(tempfile.mkdtemp(prefix="pcbpaging-"))
        metadata["scratch"] = str(scratch)
        shutil.copytree(DOS / "PCB", scratch / "PCB", symlinks=False)
        (scratch / "RA").mkdir()
        shutil.copy2(DOS / "RA/X00.EXE", scratch / "RA/X00.EXE")
        (scratch / "COMPAT").mkdir()
        shutil.copy2(DOS / "COMPAT/RUNPCB.BAT", scratch / "COMPAT/RUNPCB.BAT")
        metadata["clone_matches_live_before_edits"] = fingerprint(scratch / "PCB") == before["PCB"]
        if not metadata["clone_matches_live_before_edits"]:
            raise RuntimeError("Live board changed during clone; refusing to launch")
        metadata["fixtures"] = []
        for cls in (True, False):
            for length in (22, 23, 24, 46):
                name = f"PG{'C' if cls else 'N'}{length}"
                path = scratch / "PCB/HELP" / name
                if any(p.name.upper().startswith(name) for p in path.parent.iterdir()):
                    raise RuntimeError(f"Probe help filename collision: {name}")
                # Prefix the first numbered line: CLS must not add a fixture newline.
                data = (("@CLS@" if cls else "") + "".join(
                    f"{name}-{n:03d}\r\n" for n in range(1, length + 1))).encode("cp437")
                path.write_bytes(data)
                (out / name).write_bytes(data)
                metadata["fixtures"].append({"name": name, "lines": length, "cls": cls,
                                             "sha256": hashlib.sha256(data).hexdigest()})
        metadata["original_hlpe_sha256"] = hashlib.sha256((scratch / "PCB/HELP/HLPE").read_bytes()).hexdigest()
        with socket.socket() as reservation:
            reservation.bind(("0.0.0.0", 0))
            port = reservation.getsockname()[1]
        conf = scratch / "oracle.conf"
        conf.write_text(f"""[sdl]
autolock=false
[dosbox]
memsize=16
machine=svga_s3
[cpu]
core=auto
cputype=auto
cycles=max
[serial]
serial1=nullmodem port:{port} transparent:1
serial2=disabled
serial3=disabled
serial4=disabled
[autoexec]
mount c {scratch}
c:
path z:\\;c:\\pcb;c:\\ra
cd \\RA
x00 e
set PCB=/NODE:1 /PORT1F:
set PCBDRIVE=C:
set PCBDIR=\\PCB\\NODE1
set PCBDAT=C:\\PCB\\PCBOARD.DAT
set NODE=1
cd \\PCB\\NODE1
if exist endpcb del endpcb
call c:\\compat\\RUNPCB.BAT
exit
""")
        command = ["flatpak", "run", "--nofilesystem=host", "--nofilesystem=home",
                   f"--filesystem={scratch}", "com.dosbox_x.DOSBox-X",
                   "-defaultconf", "-conf", str(conf)]
        metadata["command"] = command
        metadata["port"] = port
        (out / "oracle.conf").write_bytes(conf.read_bytes())
        with (out / "dosbox.log").open("xb") as emulator_log:
            proc = subprocess.Popen(command, stdout=emulator_log, stderr=subprocess.STDOUT,
                                    env={**os.environ, "SDL_VIDEODRIVER": "dummy", "SDL_AUDIODRIVER": "dummy"},
                                    start_new_session=True)
            metadata["child_pid"] = proc.pid
            with bbs.connect("127.0.0.1", port, 30) as sock:
                session = Session(sock, out)
                view = session.read()
                rules = bbs.load_rules([], ROOT / "compat/logon.expect")
                rules.insert(0, (re.compile("Scan Message Base Since", re.I), "N"))
                approved = {r"\(Enter\) to continue", r"- more \]-", r"More\? \[Y",
                            "you want graphics", "What is your first name", "What is your last name",
                            "username:", r"Password \(Dots will echo", "password:", "Scan Message Base Since"}
                startup_reads = 0
                for _ in range(45):
                    if MENU.search(view):
                        break
                    for pattern, answer in rules:
                        if pattern.search(view[-400:]):
                            if pattern.pattern not in approved:
                                raise RuntimeError(f"Unapproved login/profile prompt: {pattern.pattern}")
                            if pattern.pattern == "you want graphics":
                                view = session.send("Y", "[enable ANSI graphics]")
                            else:
                                view = session.send(answer, "[fixture login response]")
                            break
                    else:
                        # ANSI detection can pause after the version banner, before logon.
                        if startup_reads < 3 and "PCBoard (R) v15.4" in "".join(e["text"] for e in session.events):
                            startup_reads += 1
                            view = session.read()
                            continue
                        raise RuntimeError("No safe fixture login rule matched")
                require(view, MENU)
                metadata["version_banner"] = next((e["text"] for e in session.events if "15.4" in e["text"]), None)
                if metadata["version_banner"] is None:
                    raise RuntimeError("PCBoard 15.4 banner not observed")
                view, initial = session.page_length(view)
                metadata["initial_user_page_length"] = initial
                for length in ((1, 2, 5, 23) if args.help_boundaries else (23, 0, 1, 2, 5, 24)):
                    print(f"Setting and reading back page length {length}", flush=True)
                    view, measured = session.page_length(view, length)
                    metadata.setdefault("page_length_readbacks", []).append(measured)
                    if args.help_boundaries:
                        view, result = session.probe(view, length, "PGC24", "H", 24)
                        metadata["cases"].append(result)
                        (out / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
                        continue
                    for answer in (("", "Y", "N", "NS", "H") if length == 23 else ("N",)):
                        view, result = session.probe(view, length, "E", answer)
                        metadata["cases"].append(result)
                    for cls in (True, False):
                        for count in (22, 23, 24, 46):
                            name = f"PG{'C' if cls else 'N'}{count}"
                            answers = ("", "Y", "N", "NS", "H") if length == 23 and count == 46 else ("",)
                            for answer in answers:
                                view, result = session.probe(view, length, name, answer, count)
                                metadata["cases"].append(result)
                    (out / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
                session.phase = "logoff"
                require(view, MENU)
                session.send("G")
                metadata["status"] = "complete"
    except Exception as error:
        metadata["status"] = "blocked"
        metadata["error"] = f"{type(error).__name__}: {error}"
        print(metadata["error"], flush=True)
    finally:
        if proc is not None:
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                metadata["child_cleanup_signal"] = "SIGTERM"
                os.killpg(proc.pid, signal.SIGTERM)
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    metadata["child_cleanup_signal"] = "SIGKILL"
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
            metadata["emulator_exit"] = proc.returncode
        if session is not None:
            session.close()
        if scratch is not None and (scratch / "COMPAT/EL.LOG").exists():
            metadata["pcboard_exit"] = (scratch / "COMPAT/EL.LOG").read_text().strip()
        if before is not None:
            after = live_fingerprints()
            metadata["live_after"] = after
            metadata["live_unchanged"] = {key: before[key] == after[key] for key in before}
            if not all(metadata["live_unchanged"].values()):
                metadata["status"] = "blocked"
                metadata["integrity_error"] = "Live fixture differs from initial hashes (possibly concurrent activity)"
        (out / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
        print(json.dumps({key: metadata[key] for key in ("status", "live_unchanged", "emulator_exit", "pcboard_exit")
                          if key in metadata}), flush=True)
    return 0 if metadata["status"] == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())