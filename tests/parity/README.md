# Parity Tests

This directory contains equivalent Python and Rust programs that exercise the same functionality, allowing side-by-side comparison to verify the Rust implementation matches Python Rich behavior.

## Prerequisites

- Python 3.8+ with `rich` installed (`pip install rich`)
- Rust toolchain (cargo)

## Usage

Run all tests for a phase:
```bash
./run_parity.sh phase1
```

Run a specific module:
```bash
./run_parity.sh phase1 color
./run_parity.sh phase1 cells
./run_parity.sh phase1 style
./run_parity.sh phase1 segment
./run_parity.sh phase1 measure
```

## Output

- `[PASS]` - Python and Rust outputs match exactly
- `[DIFF]` - Outputs differ, with diff shown

## Structure

```
tests/parity/
├── README.md
├── run_parity.sh
└── phase1/
    ├── python/
    │   ├── test_color.py
    │   ├── test_cells.py
    │   ├── test_style.py
    │   ├── test_segment.py
    │   └── test_measure.py
    └── rust/
        ├── Cargo.toml
        └── src/
            ├── main.rs
            ├── color.rs
            ├── cells.rs
            ├── style.rs
            ├── segment.rs
            └── measure.rs
```

## Design Principles

1. **Identical output format**: Both Python and Rust print results in the same format
2. **Deterministic**: No random IDs, timestamps, or terminal detection
3. **Self-contained**: Each test file exercises one module
4. **Diffable**: Output is designed for easy `diff` comparison
