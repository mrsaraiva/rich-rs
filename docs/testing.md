# Testing

This repo uses two layers of tests:

1. Rust unit and doc tests
   - Run with: `CARGO_INCREMENTAL=0 cargo test`
   - The `CARGO_INCREMENTAL=0` flag avoids filesystem lock issues on some mounts.

2. Parity tests (Rust vs Python Rich)
   - These live under `tests/parity/` and are excluded from the published crate.
   - They compare deterministic outputs from Python Rich against `rich-rs`.

## Parity tests

### Prerequisites

- Python 3.8+
- Python `rich` installed (pinned to the version expected by the harness)

### Run all parity phases

```sh
/opt/homebrew/bin/bash tests/parity/run_parity.sh phase1
/opt/homebrew/bin/bash tests/parity/run_parity.sh phase2
/opt/homebrew/bin/bash tests/parity/run_parity.sh phase3
/opt/homebrew/bin/bash tests/parity/run_parity.sh phase4
/opt/homebrew/bin/bash tests/parity/run_parity.sh phase5
```

### Rich version pinning

The runner validates the installed Python Rich version before executing tests.
You can override the expected version with:

```sh
RICH_VERSION=14.3.2 /opt/homebrew/bin/bash tests/parity/run_parity.sh phase1
```

To auto-install the expected version into the current Python environment:

```sh
PARITY_AUTO_INSTALL=1 /opt/homebrew/bin/bash tests/parity/run_parity.sh phase1
```
