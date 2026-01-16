#!/usr/bin/env python3
"""Parity test for measure module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.measure import Measurement


def main():
    print("=== Measurement Creation ===")

    m = Measurement(5, 10)
    print(f'Measurement(5, 10) -> minimum={m.minimum}, maximum={m.maximum}')

    m = Measurement(0, 0)
    print(f'Measurement(0, 0) -> minimum={m.minimum}, maximum={m.maximum}')

    print("\n=== span ===")

    m = Measurement(5, 10)
    print(f'Measurement(5, 10).span -> {m.span}')

    m = Measurement(5, 5)
    print(f'Measurement(5, 5).span -> {m.span}')

    m = Measurement(0, 100)
    print(f'Measurement(0, 100).span -> {m.span}')

    print("\n=== normalize ===")

    m = Measurement(5, 10)
    n = m.normalize()
    print(f'Measurement(5, 10).normalize() -> ({n.minimum}, {n.maximum})')

    m = Measurement(10, 5)
    n = m.normalize()
    print(f'Measurement(10, 5).normalize() -> ({n.minimum}, {n.maximum})')

    m = Measurement(-5, 10)
    n = m.normalize()
    print(f'Measurement(-5, 10).normalize() -> ({n.minimum}, {n.maximum})')

    m = Measurement(-10, -5)
    n = m.normalize()
    print(f'Measurement(-10, -5).normalize() -> ({n.minimum}, {n.maximum})')

    print("\n=== with_maximum ===")

    m = Measurement(5, 10)
    n = m.with_maximum(7)
    print(f'Measurement(5, 10).with_maximum(7) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.with_maximum(3)
    print(f'Measurement(5, 10).with_maximum(3) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.with_maximum(15)
    print(f'Measurement(5, 10).with_maximum(15) -> ({n.minimum}, {n.maximum})')

    print("\n=== with_minimum ===")

    m = Measurement(5, 10)
    n = m.with_minimum(7)
    print(f'Measurement(5, 10).with_minimum(7) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.with_minimum(3)
    print(f'Measurement(5, 10).with_minimum(3) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.with_minimum(15)
    print(f'Measurement(5, 10).with_minimum(15) -> ({n.minimum}, {n.maximum})')

    print("\n=== clamp ===")

    m = Measurement(5, 10)
    n = m.clamp(min_width=7)
    print(f'Measurement(5, 10).clamp(min_width=7) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.clamp(max_width=7)
    print(f'Measurement(5, 10).clamp(max_width=7) -> ({n.minimum}, {n.maximum})')

    m = Measurement(5, 10)
    n = m.clamp(min_width=6, max_width=8)
    print(f'Measurement(5, 10).clamp(min_width=6, max_width=8) -> ({n.minimum}, {n.maximum})')


if __name__ == "__main__":
    main()
