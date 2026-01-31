#!/usr/bin/env python3
"""Parity test for cells module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.cells import cell_len, set_cell_size, chop_cells


def main():
    print("=== cell_len ===")
    print(f'cell_len("hello") -> {cell_len("hello")}')
    print(f'cell_len("") -> {cell_len("")}')
    print(f'cell_len("你好") -> {cell_len("你好")}')
    print(f'cell_len("hello你好") -> {cell_len("hello你好")}')
    print(f'cell_len("😀") -> {cell_len("😀")}')

    print("\n=== set_cell_size ===")
    print(f'set_cell_size("hello", 5) -> "{set_cell_size("hello", 5)}"')
    print(f'set_cell_size("hello", 10) -> "{set_cell_size("hello", 10)}"')
    print(f'set_cell_size("hello world", 5) -> "{set_cell_size("hello world", 5)}"')
    print(f'set_cell_size("你好世界", 4) -> "{set_cell_size("你好世界", 4)}"')
    print(f'set_cell_size("你好世界", 5) -> "{set_cell_size("你好世界", 5)}"')
    print(f'set_cell_size("hello", 0) -> "{set_cell_size("hello", 0)}"')

    print("\n=== chop_cells ===")
    print(f'chop_cells("hello", 3) -> {chop_cells("hello", 3)}')
    print(f'chop_cells("abcdef", 2) -> {chop_cells("abcdef", 2)}')
    print(f'chop_cells("你好世界", 4) -> {chop_cells("你好世界", 4)}')
    print(f'chop_cells("你好世界", 5) -> {chop_cells("你好世界", 5)}')
    print(f'chop_cells("a你b好", 3) -> {chop_cells("a你b好", 3)}')


if __name__ == "__main__":
    main()
