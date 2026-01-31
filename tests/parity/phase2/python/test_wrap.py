#!/usr/bin/env python3
"""Parity test for wrap module (divide_line)."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich._wrap import divide_line


def main():
    print("=== divide_line() ===")

    # Simple word wrap
    text = "Hello World Test"
    offsets = divide_line(text, 10, fold=False)
    print(f"divide_line('Hello World Test', 10) -> {list(offsets)}")

    # Word boundary
    text = "Hello World"
    offsets = divide_line(text, 5, fold=False)
    print(f"divide_line('Hello World', 5) -> {list(offsets)}")

    # Fold long word
    text = "Supercalifragilistic"
    offsets = divide_line(text, 8, fold=True)
    print(f"divide_line('Supercalifragilistic', 8, fold=True) -> {list(offsets)}")

    # No fold long word
    text = "Supercalifragilistic"
    offsets = divide_line(text, 8, fold=False)
    print(f"divide_line('Supercalifragilistic', 8, fold=False) -> {list(offsets)}")

    # Multiple words fit
    text = "A B C D E F"
    offsets = divide_line(text, 3, fold=False)
    print(f"divide_line('A B C D E F', 3) -> {list(offsets)}")

    # Empty string
    text = ""
    offsets = divide_line(text, 10, fold=False)
    print(f"divide_line('', 10) -> {list(offsets)}")

    # Single word fits
    text = "Hello"
    offsets = divide_line(text, 10, fold=False)
    print(f"divide_line('Hello', 10) -> {list(offsets)}")

    # Whitespace handling
    text = "Hello   World"
    offsets = divide_line(text, 8, fold=False)
    print(f"divide_line('Hello   World', 8) -> {list(offsets)}")


if __name__ == "__main__":
    main()
