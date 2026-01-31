#!/usr/bin/env python3
"""Parity test for padding module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.padding import Padding


def main():
    print("=== Padding.unpack ===")

    # Single value
    result = Padding.unpack(2)
    print(f"Padding.unpack(2) -> {result}")

    # Single value tuple
    result = Padding.unpack((3,))
    print(f"Padding.unpack((3,)) -> {result}")

    # Two values (vertical, horizontal)
    result = Padding.unpack((1, 4))
    print(f"Padding.unpack((1, 4)) -> {result}")

    # Four values (top, right, bottom, left)
    result = Padding.unpack((1, 2, 3, 4))
    print(f"Padding.unpack((1, 2, 3, 4)) -> {result}")

    # Zero padding
    result = Padding.unpack(0)
    print(f"Padding.unpack(0) -> {result}")

    print("\n=== Padding properties ===")

    p = Padding("Test", (1, 2, 3, 4))
    print(f"Padding((1, 2, 3, 4)).top -> {p.top}")
    print(f"Padding((1, 2, 3, 4)).right -> {p.right}")
    print(f"Padding((1, 2, 3, 4)).bottom -> {p.bottom}")
    print(f"Padding((1, 2, 3, 4)).left -> {p.left}")

    print("\n=== Padding.indent ===")

    p = Padding.indent("Indented", 4)
    print(f"Padding.indent(4).left -> {p.left}")
    print(f"Padding.indent(4).expand -> {str(p.expand).lower()}")


if __name__ == "__main__":
    main()
