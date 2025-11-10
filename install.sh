#!/bin/bash
# Installation script for natty-lang-feeder
# This script builds the binary and installs it with the newsboat wrapper

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== natty-lang-feeder Installation ===${NC}"
echo

# Check for Rust/Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo not found. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

# Build the binary
echo -e "${BLUE}Building natty-lang-feeder...${NC}"
cargo build --release

if [ ! -f "target/release/natty-lang-feeder" ]; then
    echo -e "${RED}Error: Build failed. Binary not found at target/release/natty-lang-feeder${NC}"
    exit 1
fi

# Set installation directory
INSTALL_DIR="${HOME}/.natty-lang-feeder"
echo -e "${BLUE}Installing to ${INSTALL_DIR}${NC}"

# Create installation directory
mkdir -p "${INSTALL_DIR}"

# Copy binary
echo "Copying binary..."
cp target/release/natty-lang-feeder "${INSTALL_DIR}/"

# Copy and setup wrapper script
echo "Installing wrapper script for newsboat compatibility..."
cp natty-lang-feeder-wrapper.sh "${INSTALL_DIR}/"
chmod +x "${INSTALL_DIR}/natty-lang-feeder-wrapper.sh"

# Create example config if it doesn't exist
if [ ! -f "${INSTALL_DIR}/feeds.conf" ]; then
    echo "Creating example configuration..."
    cp examples/feeds.conf "${INSTALL_DIR}/"
    echo -e "${GREEN}✓ Example configuration created at ${INSTALL_DIR}/feeds.conf${NC}"
    echo "  Please edit this file to configure your feeds"
fi

# Create cache directory
mkdir -p "${INSTALL_DIR}/cache"

echo
echo -e "${GREEN}=== Installation Complete! ===${NC}"
echo
echo "Next steps:"
echo "1. Set your API key:"
echo "   export OPENAI_API_KEY='your-key-here'"
echo "   (Add to ~/.bashrc or ~/.zshrc for persistence)"
echo
echo "2. Edit your feed configuration:"
echo "   ${INSTALL_DIR}/feeds.conf"
echo
echo "3. Add feeds to newsboat (${HOME}/.newsboat/urls):"
echo "   exec:${INSTALL_DIR}/natty-lang-feeder-wrapper.sh tech-news"
echo "   exec:${INSTALL_DIR}/natty-lang-feeder-wrapper.sh security-news"
echo
echo "4. Configure newsboat (${HOME}/.newsboat/config):"
echo "   auto-reload no"
echo
echo "5. Test your feed:"
echo "   ${INSTALL_DIR}/natty-lang-feeder-wrapper.sh tech-news"
echo
echo -e "${BLUE}Note: The wrapper script is required for newsboat compatibility.${NC}"
echo -e "${BLUE}Always use 'natty-lang-feeder-wrapper.sh' in your newsboat URLs.${NC}"
