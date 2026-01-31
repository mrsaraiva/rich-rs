#!/usr/bin/env python3
"""Parity test for Text wrapping methods."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.text import Text
from rich.console import Console


def format_list(items):
    """Format a list with single quotes like Python."""
    quoted = [f"'{item}'" for item in items]
    return f"[{', '.join(quoted)}]"


def main():
    console = Console(width=80, force_terminal=True)

    print("=== Text.expand_tabs() ===")

    text = Text("Hello\tWorld")
    text.expand_tabs(4)
    print(f'expand_tabs(4) on "Hello\\tWorld" -> plain="{text.plain}", len={len(text.plain)}')

    text = Text("\t\tIndented")
    text.expand_tabs(8)
    print(f'expand_tabs(8) on "\\t\\tIndented" -> len={len(text.plain)}')

    print("\n=== Text.rstrip() ===")

    text = Text("Hello   ")
    text.rstrip()
    print(f'rstrip() on "Hello   " -> plain="{text.plain}", len={len(text.plain)}')

    text = Text("Hello")
    text.rstrip()
    print(f'rstrip() on "Hello" -> plain="{text.plain}", len={len(text.plain)}')

    print("\n=== Text.rstrip_end() ===")

    text = Text("Hello World   ")
    text.rstrip_end(5)
    print(f'rstrip_end(5) on "Hello World   " -> len={len(text.plain)}')

    print("\n=== Text.truncate() ===")

    text = Text("Hello World")
    text.truncate(5, overflow="ellipsis")
    print(f'truncate(5, ellipsis) on "Hello World" -> plain="{text.plain}"')

    text = Text("Hello World")
    text.truncate(5, overflow="crop")
    print(f'truncate(5, crop) on "Hello World" -> plain="{text.plain}"')

    text = Text("Hi")
    text.truncate(5, overflow="crop", pad=True)
    print(f'truncate(5, crop, pad=True) on "Hi" -> plain="{text.plain}", len={len(text.plain)}')

    print("\n=== Text.align() ===")

    text = Text("Hello")
    text.align("right", 10)
    print(f'align(right, 10) on "Hello" -> len={len(text.plain)}, plain="{text.plain}"')

    text = Text("Hello")
    text.align("left", 10)
    print(f'align(left, 10) on "Hello" -> len={len(text.plain)}, plain="{text.plain}"')

    text = Text("Hello")
    text.align("center", 11)
    print(f'align(center, 11) on "Hello" -> len={len(text.plain)}, plain="{text.plain}"')

    text = Text("Hi")
    text.align("center", 6)
    print(f'align(center, 6) on "Hi" -> len={len(text.plain)}, plain="{text.plain}"')

    print("\n=== Text.split() ===")

    text = Text("Hello World Test")
    parts = text.split(" ")
    plains = [t.plain for t in parts]
    print(f'split(" ") on "Hello World Test" -> {format_list(plains)}')

    text = Text("no-separator")
    parts = text.split(" ")
    plains = [t.plain for t in parts]
    print(f'split(" ") on "no-separator" -> {format_list(plains)}')

    print("\n=== Text.wrap() ===")

    # Basic wrap
    text = Text("Hello World How Are You")
    wrapped = text.wrap(console, 10, justify="left")
    line_count = len(wrapped)
    first_plain = wrapped[0].plain if wrapped else ""
    print(f'wrap(10, left) on "Hello World How Are You" -> lines={line_count}, first="{first_plain}"')

    # Wrap with justify full
    text = Text("Hello World Test")
    wrapped = text.wrap(console, 12, justify="full")
    line_count = len(wrapped)
    print(f'wrap(12, full) on "Hello World Test" -> lines={line_count}')

    # Wrap with center
    text = Text("Hi Test")
    wrapped = text.wrap(console, 10, justify="center")
    first_plain = wrapped[0].plain if wrapped else ""
    print(f'wrap(10, center) on "Hi Test" -> first_len={len(first_plain)}')

    # Wrap with fold
    text = Text("Supercalifragilistic")
    wrapped = text.wrap(console, 8, overflow="fold")
    line_count = len(wrapped)
    print(f'wrap(8, overflow=fold) on "Supercalifragilistic" -> lines={line_count}')


if __name__ == "__main__":
    main()
