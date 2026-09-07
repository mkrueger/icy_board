#!/usr/bin/env python3
"""Prompt-only oracle on a disposable copy; never runs a transfer protocol."""
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
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bbs_session as bbs

HERE = Path(__file__).resolve().parent
DOS = Path.home() / "dos"
MENU = r"(?:Main Board )?Command\s*[:?]"


def fingerprint(root):
    return {
        str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(root.rglob("*")) if p.is_file()
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=HERE)
    parser.add_argument("--extended", action="store_true")
    args = parser.parse_args()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    # Refuse to replace evidence or connect to somebody else's listener.
    transcript = (out / "prompts.txt").open("x", encoding="utf-8")
    raw = (out / "received.bin").open("xb")
    before = fingerprint(DOS / "PCB")
    config_before = {
        name: digest for name, digest in fingerprint(DOS / "COMPAT").items()
        if not name.startswith("XFER0907-")
    }
    if any(p.name.upper() in {"ZXA907.TXT", "ZXB907.TXT", "ZXC907.TXT", "ZXN907.ZIP"}
           for p in (DOS / "PCB").rglob("*")):
        raise RuntimeError("A probe filename already exists; refusing any collision")
    with socket.socket() as check:
        check.bind(("0.0.0.0", 2323))
    scratch = Path(tempfile.mkdtemp(prefix="XFER0907-", dir=DOS / "COMPAT"))
    shutil.copytree(DOS / "PCB", scratch / "PCB", symlinks=False)
    (scratch / "RA").mkdir()
    shutil.copy2(DOS / "RA" / "X00.EXE", scratch / "RA" / "X00.EXE")
    (scratch / "COMPAT").mkdir()
    shutil.copy2(DOS / "COMPAT" / "RUNPCB.BAT", scratch / "COMPAT" / "RUNPCB.BAT")
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
serial1=nullmodem port:2323 transparent:1
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
    metadata = {"scratch": str(scratch), "command": command,
                "board_before_sha256": before,
                "live_config_before_sha256": config_before,
                "status": "starting"}
    proc = None

    def log(text):
        print(text, file=transcript, flush=True)
        print(text, flush=True)

    def read(sock):
        data = bbs.drain(sock, 1.0, 12.0)
        raw.write(data)
        raw.flush()
        text = bbs.render(data, True)
        log(text)
        return text

    def send(sock, answer, label=None):
        log(f"--- SEND {label if label is not None else repr(answer)} ---")
        sock.sendall(answer.encode("cp437") + b"\r")
        return read(sock)

    def require(text, pattern):
        if not re.search(pattern, text, re.IGNORECASE):
            raise RuntimeError(f"Prompt mismatch; sent nothing further. Expected {pattern!r}")

    def return_to_menu(sock, view):
        for _ in range(4):
            if re.search(MENU, view, re.IGNORECASE):
                return view
            require(view[-200:], r"Press \(Enter\) to continue\?")
            view = send(sock, "")
        require(view, MENU)
        return view

    try:
        with (out / "dosbox.log").open("xb") as emulator_log:
            proc = subprocess.Popen(command, stdout=emulator_log, stderr=subprocess.STDOUT,
                                    env={**os.environ, "SDL_VIDEODRIVER": "dummy",
                                         "SDL_AUDIODRIVER": "dummy"}, start_new_session=True)
            with bbs.connect("127.0.0.1", 2323, 30) as sock:
                view = read(sock)
                rules = bbs.load_rules([], HERE.parent / "logon.expect")
                rules.insert(0, (re.compile("Scan Message Base Since", re.IGNORECASE), "N"))
                for _ in range(45):
                    if re.search(MENU, view, re.IGNORECASE):
                        break
                    for pattern, answer in rules:
                        if pattern.search(view[-400:]):
                            # Do not register accounts or alter profile settings.
                            if pattern.pattern not in {
                                r"\(Enter\) to continue", r"- more \]-", r"More\? \[Y",
                                "you want graphics", "What is your first name",
                                "What is your last name", "username:",
                                r"Password \(Dots will echo", "password:",
                                "Scan Message Base Since",
                            }:
                                raise RuntimeError(f"Unapproved logon/profile prompt: {pattern.pattern}")
                            view = send(sock, answer, "[fixture password]" if "password" in pattern.pattern.lower() else None)
                            break
                    else:
                        raise RuntimeError("No safe logon rule matched")
                require(view, MENU)
                for cmd in ("D", "BD", "U", "BU"):
                    log(f"=== {cmd}: empty filename abort ===")
                    view = send(sock, cmd)
                    require(view, r"[Ff]ile.*(?:name|upload|download)|[Nn]ame.*file")
                    view = send(sock, "")
                    view = return_to_menu(sock, view)
                metadata["status"] = "four_empty_prompt_probes_complete"
                if args.extended:
                    for cmd in ("D", "BD"):
                        log(f"=== {cmd}: missing stacked filename, protocol N ===")
                        view = send(sock, f"{cmd} N ZXN907.ZIP")
                        require(view, r"not found")
                        require(view, r"Enter the filename to Download")
                        view = return_to_menu(sock, send(sock, ""))
                    log("=== U: empty first description cancels filename ===")
                    view = send(sock, "U N ZXA907.TXT")
                    require(view, r"description")
                    view = send(sock, "")
                    require(view, r"Enter the Filename to Upload")
                    view = return_to_menu(sock, send(sock, ""))
                    for cmd, name in (("U", "ZXB907.TXT"), ("BU", "ZXC907.TXT")):
                        log(f"=== {cmd}: description then explicit None cancellation ===")
                        view = send(sock, f"{cmd} N {name}")
                        require(view, r"description")
                        view = send(sock, "Prompt-only audit; no payload.")
                        require(view, r"\?\s*$")
                        view = send(sock, "")
                        if cmd == "BU":
                            require(view, r"Enter the Filename to Upload")
                            view = send(sock, "")
                        require(view, r"Protocol.*(?:Transfer|Desired)|Desired.*Protocol")
                        view = return_to_menu(sock, send(sock, "N"))
                    log("=== BU: batch-ready prompt, abort before receiver ===")
                    view = send(sock, "BU Y ZXA907.TXT")
                    require(view, r"description")
                    view = send(sock, "Prompt-only audit; no payload.")
                    require(view, r"\?\s*$")
                    view = send(sock, "")
                    require(view, r"Enter the Filename to Upload")
                    view = send(sock, "")
                    require(view, r"\(G\)oodbye after Batch, \(A\)bort")
                    view = return_to_menu(sock, send(sock, "A"))
                    for cmd in ("D", "BD"):
                        log(f"=== {cmd}: existing file, explicit None cancellation ===")
                        view = send(sock, f"{cmd} N ALLFILES.ZIP")
                        if re.search(r"download bytes left available are 0", view, re.IGNORECASE):
                            metadata.setdefault("download_selection_blockers", []).append(cmd + ": zero available download bytes")
                            require(view, r"Enter the filename to Download")
                            view = return_to_menu(sock, send(sock, ""))
                            continue
                        if cmd == "BD":
                            require(view, r"Enter the filename to Download")
                            view = send(sock, "")
                        require(view, r"Protocol.*(?:Transfer|Desired)|Desired.*Protocol")
                        view = return_to_menu(sock, send(sock, "N"))
                    metadata["status"] = "extended_prompt_probes_complete"
                view = send(sock, "G")
    except Exception as error:
        metadata["status"] = "blocked"
        metadata["blocker"] = f"{type(error).__name__}: {error}"
        log(f"--- BLOCKER: {metadata['blocker']} ---")
    finally:
        if proc is not None:
            # Only the process group created above; never pkill/flatpak kill.
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGTERM)
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
            metadata["emulator_exit"] = proc.returncode
        metadata["live_board_unchanged"] = before == fingerprint(DOS / "PCB")
        metadata["live_compat_files_unchanged"] = all(
            p.is_file() and hashlib.sha256(p.read_bytes()).hexdigest() == digest
            for rel, digest in config_before.items() for p in [DOS / "COMPAT" / rel]
        )
        # Keep proprietary cloned data outside the repository for inspection.
        (out / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
        transcript.close()
        raw.close()
    return 0 if metadata["status"] != "blocked" else 1


if __name__ == "__main__":
    raise SystemExit(main())