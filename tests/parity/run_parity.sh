#!/bin/bash

# Parity test runner - compares Python Rich output with Rust rich-rs output

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PHASE="${1:-phase1}"
MODULE="${2:-all}"
RICH_VERSION="${RICH_VERSION:-14.3.2}"
PARITY_AUTO_INSTALL="${PARITY_AUTO_INSTALL:-0}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

PHASE_DIR="$SCRIPT_DIR/$PHASE"

if [ ! -d "$PHASE_DIR" ]; then
    echo -e "${RED}Error: Phase directory not found: $PHASE_DIR${NC}"
    exit 1
fi

# Build Rust crate once
echo -e "${CYAN}Building Rust parity crate...${NC}"
(cd "$PHASE_DIR/rust" && {
    if [ "${PARITY_OFFLINE:-0}" = "1" ]; then
        cargo build --release --quiet --offline
    else
        cargo build --release --quiet || cargo build --release --quiet --offline
    fi
})
RUST_BIN="$PHASE_DIR/rust/target/release/parity-${PHASE}"

if [ ! -x "$RUST_BIN" ]; then
    # Backward compatibility for older phases that hard-coded the binary name.
    if [ -x "$PHASE_DIR/rust/target/release/parity-phase1" ]; then
        RUST_BIN="$PHASE_DIR/rust/target/release/parity-phase1"
    fi
fi

run_test() {
    local module="$1"
    local python_file="$PHASE_DIR/python/test_${module}.py"
    local tmp_python=$(mktemp)
    local tmp_rust=$(mktemp)
    local module_title
    module_title="$(printf "%s" "$module" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')"

    echo -e "\n${CYAN}=== ${module_title} Tests ===${NC}"

    if [ ! -f "$python_file" ]; then
        echo -e "${YELLOW}[SKIP] Python test not found: $python_file${NC}"
        return
    fi

    # Run Python
    echo -n "Running Python... "
    python3 "$python_file" > "$tmp_python" 2>&1 || {
        echo -e "${RED}FAILED${NC}"
        cat "$tmp_python"
        rm -f "$tmp_python" "$tmp_rust"
        return 1
    }
    echo "done"

    # Run Rust
    echo -n "Running Rust... "
    "$RUST_BIN" "$module" > "$tmp_rust" 2>&1 || {
        echo -e "${RED}FAILED${NC}"
        cat "$tmp_rust"
        rm -f "$tmp_python" "$tmp_rust"
        return 1
    }
    echo "done"

    # Compare outputs
    if diff -q "$tmp_python" "$tmp_rust" > /dev/null 2>&1; then
        echo -e "${GREEN}[PASS]${NC} Outputs match"
    else
        echo -e "${RED}[DIFF]${NC} Outputs differ:"
        diff --color=always -u "$tmp_python" "$tmp_rust" | head -50 || true
    fi

    rm -f "$tmp_python" "$tmp_rust"
}

check_rich_version() {
    local installed
    installed="$(python3 - <<'PY'
import importlib.util
spec = importlib.util.find_spec("rich")
if spec is None:
    print("")
else:
    try:
        from importlib import metadata
    except Exception:
        import importlib_metadata as metadata  # type: ignore
    print(metadata.version("rich"))
PY
)"

    if [ -z "$installed" ]; then
        if [ "$PARITY_AUTO_INSTALL" = "1" ]; then
            echo -e "${YELLOW}rich not installed; installing rich==$RICH_VERSION...${NC}"
            python3 -m pip install --quiet "rich==${RICH_VERSION}"
            return
        fi
        echo -e "${RED}Error:${NC} Python rich is not installed."
        echo -e "Install with: python3 -m pip install \"rich==${RICH_VERSION}\""
        exit 1
    fi

    if [ "$installed" != "$RICH_VERSION" ]; then
        if [ "$PARITY_AUTO_INSTALL" = "1" ]; then
            echo -e "${YELLOW}rich version ${installed} != ${RICH_VERSION}; installing expected version...${NC}"
            python3 -m pip install --quiet "rich==${RICH_VERSION}"
            return
        fi
        echo -e "${RED}Error:${NC} Python rich version mismatch."
        echo -e "Expected: ${RICH_VERSION}"
        echo -e "Installed: ${installed}"
        echo -e "Install with: python3 -m pip install \"rich==${RICH_VERSION}\""
        exit 1
    fi
}

# Run tests
if [ "$MODULE" = "all" ]; then
    check_rich_version
    modules=()
    for python_file in "$PHASE_DIR/python"/test_*.py; do
        if [ ! -f "$python_file" ]; then
            continue
        fi
        base="$(basename "$python_file")"
        mod="${base#test_}"
        mod="${mod%.py}"
        modules+=("$mod")
    done

    if [ "${#modules[@]}" -eq 0 ]; then
        echo -e "${YELLOW}[SKIP] No Python tests found under $PHASE_DIR/python${NC}"
        exit 0
    fi

    IFS=$'\n' sorted=($(printf "%s\n" "${modules[@]}" | sort))
    unset IFS
    for mod in "${sorted[@]}"; do
        run_test "$mod"
    done
else
    check_rich_version
    run_test "$MODULE"
fi

echo -e "\n${CYAN}Done.${NC}"
