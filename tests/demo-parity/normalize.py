#!/usr/bin/env python3
import re
import sys
from pathlib import Path

CSI = rb"(?:\x1b\[[0-9;?]*[ -/]*[@-~])*"
PATTERN = re.compile(
    rb"rendered in " + CSI + rb"([0-9]+\.[0-9])" + CSI
    + rb"ms" + CSI + rb" \((cold|warm) cache\)"
)


def normalize(data: bytes) -> bytes:
    matches = list(PATTERN.finditer(data))
    if len(matches) != 2 or [match.group(2) for match in matches] != [b"cold", b"warm"]:
        raise ValueError("capture must contain exactly one cold and one warm timing sentence")
    output = bytearray(data)
    for match in reversed(matches):
        start, end = match.span(1)
        output[start:end] = b"0.0"
    return bytes(output)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: normalize.py INPUT OUTPUT")
    source = Path(sys.argv[1]).read_bytes()
    Path(sys.argv[2]).write_bytes(normalize(source))
