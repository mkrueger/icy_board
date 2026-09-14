"""Run the date/time fixture in a disposable PCBoard /PPE session."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import pplc_oracle

ROOT = Path(__file__).resolve().parents[1]


def fingerprint(root):
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(root.rglob("*")) if path.is_file()}


def main():
    os.umask(0o077)
    source = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "compat/datetime.pps"
    parent = ROOT / "target/datetime-oracle"
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    scratch = Path(tempfile.mkdtemp(prefix="pcb-datetime-"))
    live = Path.home() / "dos/PCB"
    before = fingerprint(live)
    metadata = {"status": "starting", "source": str(source), "scratch": str(scratch)}
    process = None
    print(f"CAPTURES={output}", flush=True)
    try:
        shutil.copytree(live, scratch / "PCB")
        if fingerprint(scratch / "PCB") != before:
            raise RuntimeError("Live board changed during cloning")
        (scratch / "COMPAT").mkdir()
        original = source.read_text(encoding="utf-8")
        probe = output / "datetime.pps"
        redirected = re.sub(r"(?m)^(\s*)PRINTLN ", r"\1FPUTLN 1, ", original)
        redirected = redirected.replace("END\n", "FCLOSE 1\nEND\n", 1)
        probe.write_text('FCREATE 1, "C:\\COMPAT\\DATE.OUT", O_WR, S_DN\n' + redirected, encoding="utf-8")
        command = ["flatpak", "run", "--nofilesystem=host", "--nofilesystem=home",
                   f"--filesystem={scratch}", "com.dosbox_x.DOSBox-X", "-defaultconf"]
        arguments = argparse.Namespace(dos_root=scratch, scratch=Path("COMPAT/COMPILE"),
                                       disarr=False, dosbox=" ".join(command), pplc=r"c:\PCB\PPLC.EXE",
                                       output_dir=output, encoding="utf-8")
        ppe, log = pplc_oracle.compile_source(arguments, probe)
        pplc_oracle.print_log(log)
        if ppe is None:
            raise RuntimeError("Original PPLC rejected the fixture")
        shutil.copy2(ppe, scratch / "COMPAT/DATE.PPE")
        batch = "\r\n".join([
            "@echo off", "set PCB=", "set PCBDRIVE=C:", "set PCBDIR=\\PCB\\NODE1",
            "set PCBDAT=C:\\PCB\\PCBOARD.DAT", "set NODE=1", "cd \\PCB\\NODE1",
            "c:\\pcb\\pcboardm.exe /file:C:\\PCB\\PCBOARD.DAT /PPE:C:\\COMPAT\\DATE.PPE",
            "if errorlevel 1 goto failed", "echo complete > c:\\compat\\DONE.TXT", "goto done",
            ":failed", "echo failed > c:\\compat\\FAIL.TXT", ":done", "exit", "",
        ])
        (scratch / "COMPAT/RUN.BAT").write_bytes(batch.encode("ascii"))
        with (output / "dosbox.log").open("xb") as emulator_log:
            process = subprocess.Popen(
                command + ["-silent", "-exit", "-set", "cpu cycles=max", "-c", f'mount c "{scratch}"', "-c", "c:",
                           "-c", "path z:\\;c:\\pcb", "-c", "c:\\compat\\RUN.BAT", "-c", "exit"],
                stdout=emulator_log, stderr=subprocess.STDOUT,
                env={**os.environ, "SDL_VIDEODRIVER": "dummy", "SDL_AUDIODRIVER": "dummy"},
                start_new_session=True,
            )
            process.wait(timeout=180)
        if process.returncode or not (scratch / "COMPAT/DONE.TXT").is_file():
            raise RuntimeError("PCBoard did not complete successfully")
        raw = (scratch / "COMPAT/DATE.OUT").read_bytes()
        (output / "datetime.raw").write_bytes(raw)
        text = raw.decode("cp437").replace("\r\n", "\n")
        if not text.startswith("---BEGIN---\n") or not text.endswith("---END---\n"):
            raise RuntimeError("Incomplete oracle output")
        expected = len(re.findall(r"(?m)^[A-Za-z]+\(", original))
        if len(text.splitlines()) - 2 != expected:
            raise RuntimeError("Missing oracle cases")
        (output / "datetime.out").write_text(text, encoding="utf-8")
        print(text, end="", flush=True)
        metadata.update(status="complete", cases=expected,
                source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                output_sha256=hashlib.sha256(text.encode("utf-8")).hexdigest())
    except Exception as error:
        metadata.update(status="blocked", error=f"{type(error).__name__}: {error}")
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        metadata["live_unchanged"] = fingerprint(live) == before
        if not metadata["live_unchanged"]:
            metadata["status"] = "blocked"
        (output / "run.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(metadata, indent=2), flush=True)
    return 0 if metadata["status"] == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())