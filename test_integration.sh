#!/bin/bash
# Test script to simulate newsboat calling natty-lang-feeder

# Test 1: Verify help works
echo "=== Test 1: Help output ==="
./target/release/natty-lang-feeder --help
echo ""

# Test 2: Verify version works
echo "=== Test 2: Version output ==="
./target/release/natty-lang-feeder --version
echo ""

# Test 3: Test missing feed error
echo "=== Test 3: Missing feed error ==="
./target/release/natty-lang-feeder nonexistent-feed 2>&1 | grep -E "(ERROR|Feed.*not found)"
echo ""

# Test 4: Test missing API key error
echo "=== Test 4: Missing API key error ==="
unset OPENAI_API_KEY ANTHROPIC_API_KEY
./target/release/natty-lang-feeder tech-news 2>&1 | grep -E "(ERROR|API key)"
echo ""

# Test 5: Test cache directory creation
echo "=== Test 5: Cache directory creation ==="
rm -rf ~/.natty-lang-feeder/cache
echo "Cache directory exists before: $([ -d ~/.natty-lang-feeder/cache ] && echo 'YES' || echo 'NO')"
# This will fail due to no API key, but should create cache dir
./target/release/natty-lang-feeder tech-news 2>&1 > /dev/null || true
echo "Cache directory exists after: $([ -d ~/.natty-lang-feeder/cache ] && echo 'YES' || echo 'NO')"
echo ""

# Test 6: Test config file parsing
echo "=== Test 6: Config file parsing ==="
echo "Feeds in config:"
grep "^feed " ~/.natty-lang-feeder/feeds.conf | awk '{print "  -", $2}'
echo ""

echo "=== All integration tests completed ==="
