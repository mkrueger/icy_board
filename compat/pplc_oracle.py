#!/usr/bin/env python3
"""Compile PPL sources with the original PCBoard PPLC inside DOSBox-X."""

import argparse
import hashlib
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys


REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DOSBOX = "flatpak run com.dosbox_x.DOSBox-X"


def dos_name(source: Path) -> str:
    stem = re.sub(r"[^A-Za-z0-9_]", "", source.stem).upper()
    if not stem:
        stem = "ORACLE"
    digest = hashlib.sha1(str(source.resolve()).encode()).hexdigest()[:3].upper()
    return f"{stem[:4]}{digest}.PPS"


def write_dos_source(source: Path, destination: Path, encoding: str) -> None:
    text = source.read_text(encoding=encoding)
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    if not text.endswith("\n"):
        text += "\n"
    destination.write_bytes(text.replace("\n", "\r\n").encode("cp437"))


def validate_scratch(path: Path) -> None:
    if path.is_absolute() or any(not re.fullmatch(r"[A-Za-z0-9_]{1,8}", part) for part in path.parts):
        raise ValueError(f"scratch path must be relative DOS 8.3 directories: {path}")


def path_digest(source: Path, length: int = 8) -> str:
    return hashlib.sha1(str(source.resolve()).encode()).hexdigest()[:length]


def compile_source(args: argparse.Namespace, source: Path, artifact_stem: str | None = None) -> tuple[Path | None, Path]:
    dos_source_name = dos_name(source)
    dos_stem = Path(dos_source_name).stem
    scratch = args.dos_root / args.scratch
    scratch.mkdir(parents=True, exist_ok=True)
    dos_source = scratch / dos_source_name
    dos_log = scratch / f"{dos_stem}.LOG"
    dos_ppe = scratch / f"{dos_stem}.PPE"
    dos_log.unlink(missing_ok=True)
    dos_ppe.unlink(missing_ok=True)
    write_dos_source(source, dos_source, args.encoding)

    scratch_dos = "\\" + str(args.scratch).replace("/", "\\").strip("\\")
    options = "/NODISP /DISARR" if args.disarr else "/NODISP"
    command = [
        *shlex.split(args.dosbox),
        "-silent",
        "-exit",
        "-c",
        f'mount c "{args.dos_root}"',
        "-c",
        "c:",
        "-c",
        f"cd {scratch_dos}",
        "-c",
        f"{args.pplc} {dos_source_name} {options} > {dos_stem}.LOG",
        "-c",
        "exit",
    ]
    env = os.environ.copy()
    env.setdefault("SDL_VIDEODRIVER", "dummy")
    env.setdefault("SDL_AUDIODRIVER", "dummy")
    result = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"DOSBox-X exited with {result.returncode}: {detail}")
    if not dos_log.is_file():
        raise RuntimeError(f"PPLC did not create {dos_log}")

    output_dir = args.output_dir.resolve() if args.output_dir else source.parent.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    artifact_stem = artifact_stem or source.stem
    output_log = output_dir / f"{artifact_stem}.pcboard.log"
    shutil.copyfile(dos_log, output_log)
    output_ppe = output_dir / f"{artifact_stem}.pcboard.ppe"
    if dos_ppe.is_file() and dos_ppe.stat().st_size > 48:
        shutil.copyfile(dos_ppe, output_ppe)
        return output_ppe, output_log
    output_ppe.unlink(missing_ok=True)
    return None, output_log


def print_log(path: Path) -> None:
    print(path.read_bytes().decode("cp437", errors="replace").replace("\r", "").rstrip())


def run_icy(args: argparse.Namespace, ppe: Path) -> int:
    icboard = args.icboard.resolve()
    if not icboard.is_file():
        raise RuntimeError(f"IcyBoard executable not found: {icboard}; run cargo build -p icboard")
    board = args.run_icy.resolve()
    return subprocess.run([str(icboard), "--ppe", str(ppe.resolve()), str(board)], check=False).returncode


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sources", nargs="+", type=Path, help="UTF-8 PPL source files")
    parser.add_argument("--disarr", action="store_true", help="pass PPLC's /DISARR option")
    parser.add_argument("--output-dir", type=Path, help="write all artifacts into this directory")
    parser.add_argument("--run-icy", type=Path, metavar="BOARD", help="run successful PPEs using this icyboard.toml")
    parser.add_argument("--icboard", type=Path, default=REPO_ROOT / "target/debug/icboard", help="IcyBoard executable")
    parser.add_argument("--dos-root", type=Path, default=Path(os.environ.get("PPLC_ORACLE_DOS_ROOT", "~/dos")).expanduser())
    parser.add_argument("--scratch", type=Path, default=Path("COMPAT/PPLCORCL"), help="DOS 8.3 scratch path below the DOS root")
    parser.add_argument("--pplc", default=r"c:\PCB\PPLC.EXE", help="PPLC path as seen inside DOSBox")
    parser.add_argument("--dosbox", default=os.environ.get("PPLC_ORACLE_DOSBOX", DEFAULT_DOSBOX), help="DOSBox-X command")
    parser.add_argument("--encoding", default="utf-8", help="input source encoding")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    args.dos_root = args.dos_root.resolve()
    try:
        validate_scratch(args.scratch)
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    sources = [source.resolve() for source in args.sources]
    stem_counts: dict[str, int] = {}
    for source in sources:
        key = source.stem.casefold()
        stem_counts[key] = stem_counts.get(key, 0) + 1

    failed = False
    for source in sources:
        if not source.is_file():
            print(f"error: source not found: {source}", file=sys.stderr)
            failed = True
            continue
        try:
            artifact_stem = source.stem
            if stem_counts[source.stem.casefold()] > 1:
                artifact_stem = f"{source.stem}.{path_digest(source)}"
            ppe, log = compile_source(args, source, artifact_stem)
            print(f"=== {source} ===")
            print_log(log)
            print(f"log: {log}")
            if ppe is None:
                print("result: rejected by PPLC")
                failed = True
                continue
            print(f"ppe: {ppe} ({ppe.stat().st_size} bytes)")
            if args.run_icy and run_icy(args, ppe) != 0:
                failed = True
        except (OSError, UnicodeError, RuntimeError) as error:
            print(f"error: {source}: {error}", file=sys.stderr)
            failed = True
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())