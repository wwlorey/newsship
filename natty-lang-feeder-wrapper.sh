#!/bin/sh
# Wrapper script for newsboat compatibility
# Newsboat's exec: mechanism requires a shell script with a shebang

# Determine the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Execute the actual binary with all arguments passed through
exec "${SCRIPT_DIR}/natty-lang-feeder" "$@"
