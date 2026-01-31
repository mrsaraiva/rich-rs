#!/usr/bin/env python3
"""Parity test for tree module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.tree import Tree
from io import StringIO


def render_tree(tree, width=60):
    """Render a tree to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(tree, end="")
    return output.getvalue()


def main():
    print("=== Single node tree ===")

    tree = Tree("Root")
    output = render_tree(tree, 40)
    lines = [l for l in output.split('\n') if l]
    print(f"Tree('Root') lines={len(lines)}")
    print(f"  first line: '{lines[0] if lines else ''}'")

    print("\n=== Tree with children ===")

    tree = Tree("Parent")
    tree.add("Child 1")
    tree.add("Child 2")
    output = render_tree(tree, 40)
    lines = [l for l in output.split('\n') if l]
    print(f"Tree with 2 children lines={len(lines)}")
    for i, line in enumerate(lines):
        has_branch = "├" in line or "└" in line
        print(f"  line[{i}]: has_branch={has_branch}")

    print("\n=== Nested tree ===")

    tree = Tree("Root")
    branch1 = tree.add("Branch 1")
    branch1.add("Leaf 1.1")
    branch1.add("Leaf 1.2")
    tree.add("Branch 2")
    output = render_tree(tree, 40)
    lines = [l for l in output.split('\n') if l]
    print(f"Nested tree lines={len(lines)}")
    # Count indentation levels
    for i, line in enumerate(lines):
        indent = len(line) - len(line.lstrip())
        print(f"  line[{i}]: indent={indent}")

    print("\n=== Tree guide characters ===")

    tree = Tree("Root")
    tree.add("Child 1")
    tree.add("Child 2")
    tree.add("Child 3")
    output = render_tree(tree, 40)
    lines = [l for l in output.split('\n') if l]
    # Check that last child uses └ and others use ├
    for i, line in enumerate(lines):
        if "├" in line:
            print(f"  line[{i}]: uses ├── (branch)")
        elif "└" in line:
            print(f"  line[{i}]: uses └── (end)")


if __name__ == "__main__":
    main()
