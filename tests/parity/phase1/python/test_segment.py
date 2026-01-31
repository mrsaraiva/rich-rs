#!/usr/bin/env python3
"""Parity test for segment module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.segment import Segment
from rich.style import Style


def format_style(style):
    """Format style for comparison."""
    if style is None:
        return "None"
    return str(style)


def format_list(items):
    """Format list with single quotes like Python repr."""
    return "[" + ", ".join(f"'{item}'" for item in items) + "]"


def format_list_of_lists(items):
    """Format list of lists with single quotes."""
    inner = ", ".join(format_list(sublist) for sublist in items)
    return "[" + inner + "]"


def format_tuple_list(items):
    """Format list of tuples."""
    return "[" + ", ".join(f"('{t}', '{s}')" for t, s in items) + "]"


def format_tuple_int_list(items):
    """Format list of tuples with int."""
    return "[" + ", ".join(f"('{t}', {n})" for t, n in items) + "]"


def main():
    print("=== Segment Creation ===")

    seg = Segment("hello")
    print(f'Segment("hello") -> text="{seg.text}", style={format_style(seg.style)}')

    seg = Segment("hello", Style.parse("bold"))
    print(f'Segment("hello", bold) -> text="{seg.text}", style={format_style(seg.style)}')

    print("\n=== cell_length ===")

    seg = Segment("hello")
    print(f'Segment("hello").cell_length -> {seg.cell_length}')

    seg = Segment("你好")
    print(f'Segment("你好").cell_length -> {seg.cell_length}')

    seg = Segment("hello你好")
    print(f'Segment("hello你好").cell_length -> {seg.cell_length}')

    print("\n=== split_cells ===")

    seg = Segment("hello")
    left, right = seg.split_cells(3)
    print(f'Segment("hello").split_cells(3) -> ("{left.text}", "{right.text}")')

    seg = Segment("hello")
    left, right = seg.split_cells(0)
    print(f'Segment("hello").split_cells(0) -> ("{left.text}", "{right.text}")')

    seg = Segment("hello")
    left, right = seg.split_cells(10)
    print(f'Segment("hello").split_cells(10) -> ("{left.text}", "{right.text}")')

    seg = Segment("你好世界")
    left, right = seg.split_cells(4)
    print(f'Segment("你好世界").split_cells(4) -> ("{left.text}", "{right.text}")')

    seg = Segment("你好世界")
    left, right = seg.split_cells(3)
    print(f'Segment("你好世界").split_cells(3) -> ("{left.text}", "{right.text}")')

    print("\n=== split_lines ===")

    segments = [Segment("a\nb\nc")]
    lines = list(Segment.split_lines(segments))
    result = [[s.text for s in line] for line in lines]
    print(f'split_lines([Segment("a\\nb\\nc")]) -> {format_list_of_lists(result)}')

    segments = [Segment("hello"), Segment("\n"), Segment("world")]
    lines = list(Segment.split_lines(segments))
    result = [[s.text for s in line] for line in lines]
    print(f'split_lines([Segment("hello"), Segment("\\n"), Segment("world")]) -> {format_list_of_lists(result)}')

    print("\n=== simplify ===")

    bold = Style.parse("bold")
    italic = Style.parse("italic")

    segments = [Segment("a", bold), Segment("b", bold), Segment("c", italic)]
    simplified = list(Segment.simplify(segments))
    result = [(s.text, format_style(s.style)) for s in simplified]
    print(f'simplify([("a", bold), ("b", bold), ("c", italic)]) -> {format_tuple_list(result)}')

    segments = [Segment("a"), Segment("b"), Segment("c")]
    simplified = list(Segment.simplify(segments))
    result = [s.text for s in simplified]
    print(f'simplify([("a"), ("b"), ("c")]) -> {format_list(result)}')

    print("\n=== adjust_line_length ===")

    line = [Segment("hello")]
    adjusted = Segment.adjust_line_length(line, 10)
    result = [(s.text, len(s.text)) for s in adjusted]
    print(f'adjust_line_length([Segment("hello")], 10) -> {format_tuple_int_list(result)}')

    line = [Segment("hello world")]
    adjusted = Segment.adjust_line_length(line, 5)
    result = [s.text for s in adjusted]
    print(f'adjust_line_length([Segment("hello world")], 5) -> {format_list(result)}')

    line = [Segment("hello")]
    adjusted = Segment.adjust_line_length(line, 5, pad=False)
    result = [s.text for s in adjusted]
    print(f'adjust_line_length([Segment("hello")], 5, pad=False) -> {format_list(result)}')


if __name__ == "__main__":
    main()
