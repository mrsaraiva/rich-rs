#!/usr/bin/env python3
"""Parity test for columns module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.columns import Columns
from io import StringIO


def render_columns(columns, width=60):
    """Render columns to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(columns, end="")
    return output.getvalue()


def main():
    print("=== Simple columns ===")

    items = ["apple", "banana", "cherry", "date", "elderberry", "fig"]
    columns = Columns(items)
    output = render_columns(columns, 40)
    lines = [l for l in output.split('\n') if l.strip()]
    print(f"Columns(6 items) at width=40: lines={len(lines)}")
    # Check all items present
    all_present = all(item in output for item in items)
    print(f"  all items present: {all_present}")

    print("\n=== Columns with expand ===")

    items = ["A", "B", "C", "D"]
    columns = Columns(items, expand=True)
    output = render_columns(columns, 40)
    lines = [l for l in output.split('\n') if l.strip()]
    if lines:
        max_len = max(len(l) for l in lines)
        print(f"Columns(expand=True) at width=40: max_line_len={max_len}")

    print("\n=== Columns with equal ===")

    items = ["Short", "Much Longer Text", "X"]
    columns = Columns(items, equal=True)
    output = render_columns(columns, 60)
    lines = [l for l in output.split('\n') if l.strip()]
    print(f"Columns(equal=True) lines={len(lines)}")
    all_present = all(item in output for item in items)
    print(f"  all items present: {all_present}")

    print("\n=== Columns with column_first ===")

    items = ["1", "2", "3", "4", "5"]
    columns_normal = Columns(items, column_first=False)
    columns_cf = Columns(items, column_first=True)
    output_normal = render_columns(columns_normal, 20)
    output_cf = render_columns(columns_cf, 20)
    # They should produce different layouts
    same = output_normal == output_cf
    print(f"Columns(column_first=True) differs from normal: {not same}")

    print("\n=== Columns with right_to_left ===")

    items = ["A", "B", "C"]
    columns_ltr = Columns(items, right_to_left=False)
    columns_rtl = Columns(items, right_to_left=True)
    output_ltr = render_columns(columns_ltr, 30)
    output_rtl = render_columns(columns_rtl, 30)
    # RTL should reverse the order
    same = output_ltr == output_rtl
    print(f"Columns(right_to_left=True) differs from normal: {not same}")

    print("\n=== Narrow width columns ===")

    items = ["Hello World", "Goodbye World"]
    columns = Columns(items)
    output = render_columns(columns, 15)
    lines = [l for l in output.split('\n') if l.strip()]
    print(f"Columns at narrow width=15: lines={len(lines)}")
    # Should stack vertically (2 lines) due to narrow width
    print(f"  items stacked vertically: {len(lines) >= 2}")


if __name__ == "__main__":
    main()
