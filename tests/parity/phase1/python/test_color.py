#!/usr/bin/env python3
"""Parity test for color module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.color import Color, ColorSystem


def main():
    print("=== Color Parsing ===")

    # Named colors
    c = Color.parse("red")
    print(f'parse("red") -> type={c.type.name}, number={c.number}')

    c = Color.parse("blue")
    print(f'parse("blue") -> type={c.type.name}, number={c.number}')

    c = Color.parse("green")
    print(f'parse("green") -> type={c.type.name}, number={c.number}')

    # Hex colors
    c = Color.parse("#ff0000")
    print(f'parse("#ff0000") -> type={c.type.name}, triplet={c.triplet}')

    c = Color.parse("#00ff00")
    print(f'parse("#00ff00") -> type={c.type.name}, triplet={c.triplet}')

    # RGB function
    c = Color.parse("rgb(255,128,0)")
    print(f'parse("rgb(255,128,0)") -> type={c.type.name}, triplet={c.triplet}')

    # Color number
    c = Color.parse("color(196)")
    print(f'parse("color(196)") -> type={c.type.name}, number={c.number}')

    # Default
    c = Color.parse("default")
    print(f'parse("default") -> type={c.type.name}')

    print("\n=== ANSI Codes (foreground) ===")

    c = Color.parse("red")
    codes = c.get_ansi_codes(ColorSystem.TRUECOLOR)
    print(f'Standard red -> {";".join(codes)}')

    c = Color.parse("color(196)")
    codes = c.get_ansi_codes(ColorSystem.TRUECOLOR)
    print(f'EightBit(196) -> {";".join(codes)}')

    c = Color.parse("#ff0000")
    codes = c.get_ansi_codes(ColorSystem.TRUECOLOR)
    print(f'TrueColor(255,0,0) -> {";".join(codes)}')

    print("\n=== ANSI Codes (background) ===")

    c = Color.parse("red")
    codes = c.get_ansi_codes(ColorSystem.TRUECOLOR, foreground=False)
    print(f'Standard red bg -> {";".join(codes)}')

    c = Color.parse("#ff0000")
    codes = c.get_ansi_codes(ColorSystem.TRUECOLOR, foreground=False)
    print(f'TrueColor(255,0,0) bg -> {";".join(codes)}')

    print("\n=== Color Downgrade ===")

    c = Color.parse("#ff0000")
    downgraded = c.downgrade(ColorSystem.EIGHT_BIT)
    print(f'#ff0000 -> EIGHT_BIT: type={downgraded.type.name}, number={downgraded.number}')

    c = Color.parse("color(196)")
    downgraded = c.downgrade(ColorSystem.STANDARD)
    print(f'color(196) -> STANDARD: type={downgraded.type.name}, number={downgraded.number}')


if __name__ == "__main__":
    main()
