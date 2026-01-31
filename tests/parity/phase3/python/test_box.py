#!/usr/bin/env python3
"""Parity test for box module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich import box


def py_bool(b):
    return "true" if b else "false"


def main():
    print("=== Box Constants ===")

    print(f"ASCII.ascii -> {py_bool(box.ASCII.ascii)}")
    print(f"ROUNDED.ascii -> {py_bool(box.ROUNDED.ascii)}")
    print(f"HEAVY.ascii -> {py_bool(box.HEAVY.ascii)}")
    print(f"DOUBLE.ascii -> {py_bool(box.DOUBLE.ascii)}")
    print(f"SQUARE.ascii -> {py_bool(box.SQUARE.ascii)}")

    print("\n=== Box Characters ===")

    # ASCII characters
    print(f"ASCII.top_left -> '{box.ASCII.top_left}'")
    print(f"ASCII.top -> '{box.ASCII.top}'")
    print(f"ASCII.top_right -> '{box.ASCII.top_right}'")

    # ROUNDED characters
    print(f"ROUNDED.top_left -> '{box.ROUNDED.top_left}'")
    print(f"ROUNDED.top -> '{box.ROUNDED.top}'")
    print(f"ROUNDED.top_right -> '{box.ROUNDED.top_right}'")

    # HEAVY characters
    print(f"HEAVY.top_left -> '{box.HEAVY.top_left}'")
    print(f"HEAVY.top -> '{box.HEAVY.top}'")
    print(f"HEAVY.top_right -> '{box.HEAVY.top_right}'")

    print("\n=== get_top ===")

    result = box.SQUARE.get_top([10, 10, 10])
    print(f'SQUARE.get_top([10, 10, 10]) -> "{result}"')

    result = box.ASCII.get_top([10, 10, 10])
    print(f'ASCII.get_top([10, 10, 10]) -> "{result}"')

    result = box.ROUNDED.get_top([10, 10, 10])
    print(f'ROUNDED.get_top([10, 10, 10]) -> "{result}"')

    result = box.HEAVY.get_top([10, 10, 10])
    print(f'HEAVY.get_top([10, 10, 10]) -> "{result}"')

    result = box.DOUBLE.get_top([10, 10, 10])
    print(f'DOUBLE.get_top([10, 10, 10]) -> "{result}"')

    print("\n=== get_row ===")

    result = box.SQUARE.get_row([10, 10, 10], level="head")
    print(f'SQUARE.get_row([10, 10, 10], Head) -> "{result}"')

    result = box.ASCII.get_row([10, 10, 10], level="head")
    print(f'ASCII.get_row([10, 10, 10], Head) -> "{result}"')

    result = box.SQUARE.get_row([10, 10, 10], level="row")
    print(f'SQUARE.get_row([10, 10, 10], Row) -> "{result}"')

    result = box.SQUARE.get_row([10, 10, 10], level="mid")
    print(f'SQUARE.get_row([10, 10, 10], Mid) -> "{result}"')

    result = box.SQUARE.get_row([10, 10, 10], level="foot")
    print(f'SQUARE.get_row([10, 10, 10], Foot) -> "{result}"')

    result = box.SQUARE.get_row([10, 10, 10], edge=False)
    print(f'SQUARE.get_row([10, 10, 10], edge=false) -> "{result}"')

    print("\n=== get_bottom ===")

    result = box.SQUARE.get_bottom([10, 10, 10])
    print(f'SQUARE.get_bottom([10, 10, 10]) -> "{result}"')

    result = box.ASCII.get_bottom([10, 10, 10])
    print(f'ASCII.get_bottom([10, 10, 10]) -> "{result}"')

    result = box.ROUNDED.get_bottom([10, 10, 10])
    print(f'ROUNDED.get_bottom([10, 10, 10]) -> "{result}"')

    result = box.HEAVY.get_bottom([10, 10, 10])
    print(f'HEAVY.get_bottom([10, 10, 10]) -> "{result}"')

    result = box.DOUBLE.get_bottom([10, 10, 10])
    print(f'DOUBLE.get_bottom([10, 10, 10]) -> "{result}"')

    print("\n=== substitute ===")

    # ROUNDED with legacy_windows -> SQUARE
    result = box.ROUNDED.substitute(type("Options", (), {"legacy_windows": True, "ascii_only": False})(), safe=True)
    print(f"ROUNDED.substitute(legacy_windows=true) -> is_square={py_bool(result is box.SQUARE)}")

    # SQUARE with ascii_only -> ASCII
    result = box.SQUARE.substitute(type("Options", (), {"legacy_windows": False, "ascii_only": True})())
    print(f"SQUARE.substitute(ascii_only=true) -> is_ascii={py_bool(result is box.ASCII)}")

    # ASCII stays ASCII
    result = box.ASCII.substitute(type("Options", (), {"legacy_windows": False, "ascii_only": True})())
    print(f"ASCII.substitute(ascii_only=true) -> is_ascii={py_bool(result is box.ASCII)}")

    print("\n=== Single column ===")

    result = box.SQUARE.get_top([10])
    print(f'SQUARE.get_top([10]) -> "{result}"')

    result = box.SQUARE.get_bottom([10])
    print(f'SQUARE.get_bottom([10]) -> "{result}"')


if __name__ == "__main__":
    main()
