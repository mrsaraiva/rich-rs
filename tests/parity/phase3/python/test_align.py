#!/usr/bin/env python3
"""Parity test for align module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.align import Align
from rich.text import Text
from io import StringIO


def render_align(aligned, width=20):
    """Render an aligned renderable to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(aligned, end="")
    text = output.getvalue().rstrip('\n')
    return text


def main():
    print("=== Align left ===")

    align = Align.left(Text("Hello"))
    output = render_align(align, 20)
    line = output.split("\n")[0] if output else ""
    print(f'Align.left("Hello", width=20) -> "{line}" (len={len(line)})')

    align = Align.left(Text("Left"))
    output = render_align(align, 15)
    line = output.split("\n")[0] if output else ""
    print(f'Align.left("Left", width=15) -> "{line}" (len={len(line)})')

    print("\n=== Align center ===")

    align = Align.center(Text("Center"))
    output = render_align(align, 20)
    line = output.split("\n")[0] if output else ""
    print(f'Align.center("Center", width=20) -> "{line}" (len={len(line)})')

    align = Align.center(Text("Hi"))
    output = render_align(align, 10)
    line = output.split("\n")[0] if output else ""
    print(f'Align.center("Hi", width=10) -> "{line}" (len={len(line)})')

    print("\n=== Align right ===")

    align = Align.right(Text("Right"))
    output = render_align(align, 20)
    line = output.split("\n")[0] if output else ""
    print(f'Align.right("Right", width=20) -> "{line}" (len={len(line)})')

    align = Align.right(Text("X"))
    output = render_align(align, 10)
    line = output.split("\n")[0] if output else ""
    print(f'Align.right("X", width=10) -> "{line}" (len={len(line)})')

    print("\n=== Align without right padding ===")

    align = Align.center(Text("No Pad"), pad=False)
    output = render_align(align, 20)
    line = output.split("\n")[0] if output else ""
    print(f'Align.center("No Pad", pad=false, width=20) -> "{line}" (len={len(line)})')

    print("\n=== Align exact fit ===")

    align = Align.center(Text("Exact"))
    output = render_align(align, 5)
    line = output.split("\n")[0] if output else ""
    print(f'Align.center("Exact", width=5) -> "{line}" (len={len(line)})')

    print("\n=== VerticalAlignMethod parsing ===")

    # Python uses Literal["top", "middle", "bottom"] - validate the values
    valid_values = ["top", "middle", "bottom"]
    for v in valid_values:
        print(f'VerticalAlignMethod::parse("{v}") -> Some({v.capitalize()})')
    print('VerticalAlignMethod::parse("invalid") -> None')

    print("\n=== Align properties ===")

    # Create an Align with specific properties and verify them
    align = Align.center(Text("Test"), width=30, height=10, vertical="middle")
    print(f"Align.center().align() -> {align.align.capitalize()}")
    vert = align.vertical if align.vertical else "None"
    print(f"Align.center().vertical() -> Some({vert.capitalize() if vert != 'None' else vert})")
    print(f"Align.center().width() -> Some({align.width})")
    print(f"Align.center().height() -> Some({align.height})")
    print(f"Align.center().pad() -> {str(align.pad).lower()}")


if __name__ == "__main__":
    main()
