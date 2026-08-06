#!/usr/bin/env python3
"""Minimal driver for the PCBoard oracle: talks to the DOSBox-X virtual modem over TCP.

Reads whatever the BBS sends, decodes CP437 and optionally strips ANSI so the
session is readable in a terminal.

Two drive modes:

* ``--send LINE`` (repeatable) fires the lines in order, one per quiet period.
  Simple, but it drifts as soon as the board asks one prompt more or less than
  expected.
* ``--expect 'REGEX=RESPONSE'`` (repeatable, or ``--script FILE``) answers based
  on what the board actually printed. This is what the oracle harness uses: the
  logon questionnaire is not a fixed-length list, so positional sends desync.
"""
import argparse
import re
import socket
import sys
import time

ANSI_RE = re.compile(rb"\x1b\[[0-9;?]*[A-Za-z]|\x1b[()][A-B0-9]|\x1b[=>]")


def render(raw: bytes, strip_ansi: bool) -> str:
    if strip_ansi:
        raw = ANSI_RE.sub(b"", raw)
    return raw.decode("cp437", errors="replace")


def drain(sock, idle: float, total: float) -> bytes:
    """Collect output until the board is idle for `idle` seconds (or `total` elapses)."""
    buf = b""
    deadline = time.time() + total
    last = time.time()
    sock.settimeout(0.3)
    while time.time() < deadline:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            buf += chunk
            last = time.time()
        except socket.timeout:
            if buf and time.time() - last >= idle:
                break
    return buf


def connect(host: str, port: int, wait: float):
    """Connect as soon as the virtual modem's listener appears.

    PCBoard drops to DOS with errorlevel 5 ("caller said goodbye") if it starts
    without carrier, so the client has to be attached before the board launches.
    """
    deadline = time.time() + wait
    while True:
        try:
            return socket.create_connection((host, port), timeout=5)
        except OSError:
            if time.time() >= deadline:
                raise
            time.sleep(0.25)


def load_rules(expect_args, script_path):
    """Build the ordered REGEX->RESPONSE table from --expect flags and/or --script."""
    raw = []
    if script_path:
        with open(script_path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line and not line.startswith("#"):
                    raw.append(line)
    raw.extend(expect_args)

    rules = []
    for entry in raw:
        pattern, sep, response = entry.partition("=")
        if not sep:
            raise SystemExit(f"bad rule (need REGEX=RESPONSE): {entry!r}")
        rules.append((re.compile(pattern, re.IGNORECASE), response))
    return rules


def run_expect(sock, rules, until, args, first_read: bytes) -> None:
    """Answer prompts by matching the board's most recent output against `rules`.

    Only the tail of each read is matched so that text scrolled by earlier in the
    session cannot re-trigger a rule.
    """
    view = render(first_read, True)
    last_pattern = None
    repeats = 0
    for step in range(args.max_steps):
        if until and until.search(view):
            print("--- until matched, stopping ---")
            return

        tail = view[-400:]
        for pattern, response in rules:
            if pattern.search(tail):
                # The board re-asks a question when it rejects the answer, so an
                # unchanging prompt means the rule is wrong rather than pending.
                repeats = repeats + 1 if pattern is last_pattern else 0
                if repeats >= 3:
                    print(f"--- stuck on {pattern.pattern!r}, stopping ---")
                    print(tail)
                    return
                last_pattern = pattern
                print(f"--- match {pattern.pattern!r} -> {response!r} ---")
                sock.sendall(response.encode("cp437") + b"\r")
                break
        else:
            print(f"--- no rule matched (step {step}), stopping ---")
            print(tail)
            return

        view = render(drain(sock, args.idle, args.total), not args.raw)
        print(view)

    print("--- max steps reached ---")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--send", action="append", default=[], help="line to send (repeatable)")
    ap.add_argument(
        "--expect",
        action="append",
        default=[],
        metavar="REGEX=RESPONSE",
        help="answer REGEX with RESPONSE whenever it shows up (repeatable)",
    )
    ap.add_argument("--script", help="file of REGEX=RESPONSE rules, one per line (# comments)")
    ap.add_argument("--until", help="stop once this regex appears")
    ap.add_argument("--max-steps", type=int, default=60, help="safety cap on expect iterations")
    ap.add_argument("--idle", type=float, default=2.0, help="seconds of silence that ends a read")
    ap.add_argument("--total", type=float, default=25.0, help="max seconds per read")
    ap.add_argument("--wait", type=float, default=0.0, help="seconds to keep retrying the connect")
    ap.add_argument("--raw", action="store_true", help="keep ANSI escapes")
    ap.add_argument("--hex", action="store_true", help="also dump hex of the first read")
    args = ap.parse_args()

    rules = load_rules(args.expect, args.script)
    until = re.compile(args.until, re.IGNORECASE) if args.until else None

    with connect(args.host, args.port, args.wait) as sock:
        first = drain(sock, args.idle, args.total)
        print("--- connect ---")
        print(render(first, not args.raw))
        if args.hex:
            print("--- hex (first 200) ---")
            print(first[:200].hex(" "))

        if rules:
            run_expect(sock, rules, until, args, first)

        # Sends run last: the expect script is what gets the session to a state
        # where a fixed command sequence is meaningful.
        for line in args.send:
            print(f"--- send: {line!r} ---")
            sock.sendall(line.encode("cp437") + b"\r")
            print(render(drain(sock, args.idle, args.total), not args.raw))
    return 0


if __name__ == "__main__":
    sys.exit(main())
