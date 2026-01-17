#!/usr/bin/env python3
"""Parity test for table module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.table import Table
from io import StringIO


def render_table(table, width=60):
    """Render a table to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(table, end="")
    return output.getvalue()


def main():
    print("=== Simple table ===")

    table = Table()
    table.add_column("Name")
    table.add_column("Age")
    table.add_row("Alice", "30")
    table.add_row("Bob", "25")
    output = render_table(table, 40)
    lines = [l for l in output.split('\n') if l]
    print(f"Simple table lines={len(lines)}")
    has_header = any("Name" in line and "Age" in line for line in lines)
    print(f"  has header row: {has_header}")
    has_data = any("Alice" in line for line in lines)
    print(f"  has data row: {has_data}")

    print("\n=== Table.grid ===")

    grid = Table.grid()
    grid.add_column()
    grid.add_column()
    grid.add_row("A", "B")
    grid.add_row("C", "D")
    output = render_table(grid, 40)
    lines = [l for l in output.split('\n') if l]
    print(f"Table.grid() lines={len(lines)}")
    # Grid should have no borders
    has_border = any("│" in line or "─" in line for line in lines)
    print(f"  has borders: {has_border}")

    print("\n=== Table with title ===")

    table = Table(title="My Table")
    table.add_column("Col1")
    table.add_row("Data")
    output = render_table(table, 40)
    lines = [l for l in output.split('\n') if l]
    has_title = any("My Table" in line for line in lines)
    print(f"Table(title='My Table') has_title={has_title}")

    print("\n=== Table with caption ===")

    table = Table(caption="Table caption")
    table.add_column("Col1")
    table.add_row("Data")
    output = render_table(table, 40)
    lines = [l for l in output.split('\n') if l]
    has_caption = any("Table caption" in line for line in lines)
    print(f"Table(caption='Table caption') has_caption={has_caption}")

    print("\n=== Table column count ===")

    table = Table()
    table.add_column("A")
    table.add_column("B")
    table.add_column("C")
    table.add_row("1", "2", "3")
    output = render_table(table, 60)
    lines = [l for l in output.split('\n') if l]
    # Count column separators in data row
    for line in lines:
        if "1" in line and "2" in line and "3" in line:
            sep_count = line.count("│")
            print(f"  data row has {sep_count} separators")
            break

    print("\n=== Table with expand ===")

    table = Table(expand=True)
    table.add_column("Col")
    table.add_row("X")
    output = render_table(table, 50)
    lines = [l for l in output.split('\n') if l]
    if lines:
        # Expanded table should fill width
        max_len = max(len(l) for l in lines)
        print(f"Table(expand=True) at width=50: max_line_len={max_len}")


if __name__ == "__main__":
    main()
