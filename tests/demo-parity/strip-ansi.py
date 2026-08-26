#!/usr/bin/env python3
import re
import sys
from pathlib import Path

ANSI = re.compile(rb"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\))")

if len(sys.argv) != 3:
    raise SystemExit("usage: strip-ansi.py INPUT OUTPUT")
Path(sys.argv[2]).write_bytes(ANSI.sub(b"", Path(sys.argv[1]).read_bytes()))
