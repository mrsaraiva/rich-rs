#!/usr/bin/env python3
"""Parity test for panel module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from io import StringIO


def render_panel(panel, width=40):
    """Render a panel to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(panel, end="")
    return output.getvalue()


def main():
    print("=== Basic Panel ===")

    panel = Panel("Hello, World!")
    output = render_panel(panel, 30)
    lines = output.split('\n')
    print(f"Panel('Hello, World!') lines={len(lines)}")
    for i, line in enumerate(lines):
        print(f"  line[{i}]: len={len(line)}")

    print("\n=== Panel with title ===")

    panel = Panel("Content", title="Title")
    output = render_panel(panel, 30)
    lines = output.split('\n')
    print(f"Panel('Content', title='Title') lines={len(lines)}")
    has_title = any("Title" in line for line in lines)
    print(f"  contains 'Title': {has_title}")

    print("\n=== Panel with subtitle ===")

    panel = Panel("Content", subtitle="Subtitle")
    output = render_panel(panel, 30)
    lines = output.split('\n')
    print(f"Panel('Content', subtitle='Subtitle') lines={len(lines)}")
    has_subtitle = any("Subtitle" in line for line in lines)
    print(f"  contains 'Subtitle': {has_subtitle}")

    print("\n=== Panel.fit ===")

    panel = Panel.fit("Short")
    output = render_panel(panel, 80)
    lines = [l for l in output.split('\n') if l]
    if lines:
        width = len(lines[0])
        print(f"Panel.fit('Short') width={width}")
        print(f"  fits tightly: {width < 30}")

    print("\n=== Panel with padding ===")

    panel = Panel("Padded", padding=(1, 2))
    output = render_panel(panel, 30)
    lines = output.split('\n')
    print(f"Panel('Padded', padding=(1,2)) lines={len(lines)}")
    # Count lines (should have extra for top/bottom padding)
    content_lines = [l for l in lines if "Padded" in l or (l.strip() and "─" not in l and "│" in l)]
    print(f"  content area lines: {len(content_lines)}")


if __name__ == "__main__":
    main()
