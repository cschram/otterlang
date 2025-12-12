#!/usr/bin/env bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CLEAR='\033[0m'

# Iterate over example files
failures=()
for file in examples/**/*.ot; do
    echo -e -n "Running example ${BLUE}$file${CLEAR}..."
    output="$(target/debug/otter run $file 2>&1)"
    if [ $? -ne 0 ]; then
        echo -e "${RED}failed.${CLEAR}"
        failures+=("$file" "$output")
    else
        echo -e "${GREEN}success.${CLEAR}"
    fi
done
failure_count=$((${#failures[@]} / 2))
if [ $failure_count -ne 0 ]; then
    echo -e "${RED}${failure_count} examples failed:${CLEAR}"
    for ((i=0; i<${#failures[@]}; i+=2)); do
        echo -e "${failures[i]}"
        echo -e "${failures[i+1]}"
    done
    exit 1
else
    echo -e "${GREEN}All examples ran successfully.${CLEAR}"
fi