#!/bin/bash

# Benchmark script for Leibniz formula for π
# Compares C, Rust, OtterLang, Python, and Nim performance

set -e

echo "=== Benchmarking Leibniz Formula for π ==="
echo "Calculating π with 100,000,000 iterations"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BENCH_DIR="$(pwd)"
PROJECT_ROOT="$(cd ../.. && pwd)"
OTTER_BIN="$PROJECT_ROOT/target/release/otter"
CACHE_DIR="$BENCH_DIR/.otter_cache"
export OTTER_CACHE_DIR="$CACHE_DIR"
export OTTER_STDLIB_DIR="$PROJECT_ROOT/stdlib/otter"

if [ ! -x "$OTTER_BIN" ]; then
    echo "Error: Otter binary not found at $OTTER_BIN. Run 'cargo build --release' first."
    exit 1
fi


# Check for required tools
if ! command -v python3 &> /dev/null; then
    echo -e "${YELLOW}Warning: python3 not found. Python benchmark will be skipped.${NC}"
    SKIP_PYTHON=1
else
    SKIP_PYTHON=0
fi

if ! command -v nim &> /dev/null; then
    echo -e "${YELLOW}Warning: nim not found. Nim benchmark will be skipped.${NC}"
    SKIP_NIM=1
else
    SKIP_NIM=0
fi

# Clean previous builds
echo -e "${BLUE}Cleaning previous builds...${NC}"
rm -f pi_leibniz_c pi_leibniz_rust pi_leibniz_otter pi_leibniz_nim
rm -f pi_leibniz.o pi_leibniz.ll
rm -rf "$CACHE_DIR"
mkdir -p "$CACHE_DIR"

# Compile C
echo -e "${BLUE}Compiling C...${NC}"
gcc -O3 pi_leibniz.c -o pi_leibniz_c -lm
if [ ! -f pi_leibniz_c ]; then
    echo "Error: Failed to compile C"
    exit 1
fi

# Compile Rust
echo -e "${BLUE}Compiling Rust...${NC}"
rustc -O pi_leibniz.rs -o pi_leibniz_rust
if [ ! -f pi_leibniz_rust ]; then
    echo "Error: Failed to compile Rust"
    exit 1
fi

# Compile OtterLang (clean build, no cache)
echo -e "${BLUE}Compiling OtterLang...${NC}"
if cd "$PROJECT_ROOT" && "$OTTER_BIN" build "$BENCH_DIR/pi_leibniz.ot" -o "$BENCH_DIR/pi_leibniz_otter" --release --no-cache 2>&1 && [ -f "$BENCH_DIR/pi_leibniz_otter" ]; then
    cd "$BENCH_DIR"
    SKIP_OTTER=0
    echo -e "${GREEN}✅ OtterLang compiled successfully${NC}"
else
    cd "$BENCH_DIR"
    SKIP_OTTER=1
    echo -e "${YELLOW}⚠️  OtterLang compilation failed. Skipping OtterLang benchmark.${NC}"
fi

# Compile Nim (if available)
if [ "$SKIP_NIM" -eq 0 ]; then
    echo -e "${BLUE}Compiling Nim...${NC}"
    if ! nim c -d:release -o:pi_leibniz_nim pi_leibniz.nim 2>/dev/null; then
        echo -e "${YELLOW}Warning: Failed to compile Nim. Skipping Nim benchmark.${NC}"
        SKIP_NIM=1
    fi
fi

echo ""
echo -e "${GREEN}Running benchmarks (5 runs each)...${NC}"
echo ""

# Benchmark C
echo -e "${YELLOW}C (gcc -O3):${NC}"
C_TIMES=()
echo "  Warm-up run (not timed)..."
./pi_leibniz_c >/dev/null 2>&1
for i in {1..5}; do
    TIME=$(/usr/bin/time -p ./pi_leibniz_c 2>&1 | grep "^real" | awk '{print $2}')
    C_TIMES+=($TIME)
    echo "  Run $i: ${TIME}s"
done

# Benchmark Rust
echo -e "${YELLOW}Rust (rustc -O):${NC}"
RUST_TIMES=()
echo "  Warm-up run (not timed)..."
./pi_leibniz_rust >/dev/null 2>&1
for i in {1..5}; do
    TIME=$(/usr/bin/time -p ./pi_leibniz_rust 2>&1 | grep "^real" | awk '{print $2}')
    RUST_TIMES+=($TIME)
    echo "  Run $i: ${TIME}s"
done

# Benchmark OtterLang (if compiled successfully)
if [ "$SKIP_OTTER" -eq 0 ]; then
    echo -e "${YELLOW}OtterLang (otter --release):${NC}"
    OTTER_TIMES=()
    echo "  Warm-up run (not timed)..."
    ./pi_leibniz_otter >/dev/null 2>&1
    for i in {1..5}; do
        # Capture all output, then filter for the time line
        OUTPUT=$(/usr/bin/time -p ./pi_leibniz_otter 2>&1)
        TIME=$(echo "$OUTPUT" | grep -E "^real " | awk '{print $2}')
        if [ -z "$TIME" ]; then
            # Fallback: try to find real on any line
            TIME=$(echo "$OUTPUT" | grep "real" | tail -1 | awk '{print $NF}')
        fi
        if [ -z "$TIME" ] || [ "$TIME" = "0.00" ]; then
            # If time is 0, try using a more precise timer
            TIME=$(command time -p ./pi_leibniz_otter 2>&1 | grep "^real" | awk '{print $2}')
        fi
        OTTER_TIMES+=($TIME)
        echo "  Run $i: ${TIME}s"
    done
fi

# Benchmark Python (if available)
if [ "$SKIP_PYTHON" -eq 0 ]; then
    echo -e "${YELLOW}Python (python3):${NC}"
    PYTHON_TIMES=()
    echo "  Warm-up run (not timed)..."
    python3 pi_leibniz.py >/dev/null 2>&1
    for i in {1..5}; do
        TIME=$(/usr/bin/time -p python3 pi_leibniz.py 2>&1 | grep "^real" | awk '{print $2}')
        PYTHON_TIMES+=($TIME)
        echo "  Run $i: ${TIME}s"
    done
fi

# Benchmark Nim (if available)
if [ "$SKIP_NIM" -eq 0 ]; then
    echo -e "${YELLOW}Nim (nim c -d:release):${NC}"
    NIM_TIMES=()
    echo "  Warm-up run (not timed)..."
    ./pi_leibniz_nim >/dev/null 2>&1
    for i in {1..5}; do
        TIME=$(/usr/bin/time -p ./pi_leibniz_nim 2>&1 | grep "^real" | awk '{print $2}')
        NIM_TIMES+=($TIME)
        echo "  Run $i: ${TIME}s"
    done
fi

# Calculate averages
C_AVG=$(printf '%s\n' "${C_TIMES[@]}" | awk '{sum+=$1; count++} END {printf "%.3f", sum/count}')
RUST_AVG=$(printf '%s\n' "${RUST_TIMES[@]}" | awk '{sum+=$1; count++} END {printf "%.3f", sum/count}')

# Calculate ratios
RUST_RATIO=$(echo "scale=2; $RUST_AVG / $C_AVG" | bc)

if [ "$SKIP_OTTER" -eq 0 ]; then
    OTTER_AVG=$(printf '%s\n' "${OTTER_TIMES[@]}" | awk '{sum+=$1; count++} END {printf "%.3f", sum/count}')
    OTTER_RATIO=$(echo "scale=2; $OTTER_AVG / $C_AVG" | bc)
fi

if [ "$SKIP_PYTHON" -eq 0 ]; then
    PYTHON_AVG=$(printf '%s\n' "${PYTHON_TIMES[@]}" | awk '{sum+=$1; count++} END {printf "%.3f", sum/count}')
    PYTHON_RATIO=$(echo "scale=2; $PYTHON_AVG / $C_AVG" | bc)
fi

if [ "$SKIP_NIM" -eq 0 ]; then
    NIM_AVG=$(printf '%s\n' "${NIM_TIMES[@]}" | awk '{sum+=$1; count++} END {printf "%.3f", sum/count}')
    NIM_RATIO=$(echo "scale=2; $NIM_AVG / $C_AVG" | bc)
fi

echo ""
echo "=== Results ==="
echo ""
printf "%-15s %-20s %-20s %-15s\n" "Language" "Compiler" "Avg Time (5 runs)" "Relative to C"
echo "------------------------------------------------------------------------"
printf "%-15s %-20s %-20s %-15s\n" "C" "gcc -O3" "${C_AVG}s" "1.00x (baseline)"
printf "%-15s %-20s %-20s %-15s\n" "Rust" "rustc -O" "${RUST_AVG}s" "${RUST_RATIO}x"
if [ "$SKIP_OTTER" -eq 0 ]; then
    printf "%-15s %-20s %-20s %-15s\n" "OtterLang" "otter --release" "${OTTER_AVG}s" "${OTTER_RATIO}x"
fi
if [ "$SKIP_PYTHON" -eq 0 ]; then
    printf "%-15s %-20s %-20s %-15s\n" "Python" "python3" "${PYTHON_AVG}s" "${PYTHON_RATIO}x"
fi
if [ "$SKIP_NIM" -eq 0 ]; then
    printf "%-15s %-20s %-20s %-15s\n" "Nim" "nim c -d:release" "${NIM_AVG}s" "${NIM_RATIO}x"
fi
echo ""
echo "Note: These benchmarks are run with a warm-up execution and may not be 100% accurate."
echo "Results can vary based on system load, CPU throttling, and other factors."
echo ""
echo "IMPORTANT: This benchmark tests compute-heavy code with NO allocations."
echo "GC overhead is not included. For code that allocates frequently, GC overhead"
echo "would apply and may affect performance differently."
echo ""

