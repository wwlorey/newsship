#!/bin/bash
# Test RSS output generation using a simple Rust program

cat > /tmp/test_rss.rs << 'EOF'
use chrono::Utc;

#[path = "src/ai/mod.rs"]
mod ai;

#[path = "src/rss.rs"]
mod rss;

#[path = "src/error.rs"]
mod error;

fn main() {
    let articles = vec![
        ai::Article {
            title: "Test Article 1".to_string(),
            summary: "This is a test summary for article 1. It contains sample text to verify RSS generation works correctly.".to_string(),
            sources: vec![
                ai::Source {
                    url: "https://example.com/article1".to_string(),
                    title: "Example News".to_string(),
                },
                ai::Source {
                    url: "https://test.com/article1".to_string(),
                    title: "Test Source".to_string(),
                }
            ],
            date: Utc::now(),
            guid: rss::generate_guid("Test Article 1", "This is a test summary for article 1"),
        },
        ai::Article {
            title: "Test Article 2 with Special <Chars> & \"Quotes\"".to_string(),
            summary: "Testing HTML escaping & special characters".to_string(),
            sources: vec![
                ai::Source {
                    url: "https://example.com/article2".to_string(),
                    title: "Example Source".to_string(),
                }
            ],
            date: Utc::now(),
            guid: rss::generate_guid("Test Article 2", "Testing HTML escaping"),
        },
    ];

    let rss_xml = rss::build_rss("test-feed", &articles).unwrap();
    println!("{}", rss_xml);
}
EOF

echo "=== Testing RSS XML Generation ==="
echo "Building test program..."
rustc --edition 2021 \
  --extern chrono=/home/user/newsship/target/release/deps/libchrono-*.rlib \
  --extern sha2=/home/user/newsship/target/release/deps/libsha2-*.rlib \
  --extern hex=/home/user/newsship/target/release/deps/libhex-*.rlib \
  --extern log=/home/user/newsship/target/release/deps/liblog-*.rlib \
  -L /home/user/newsship/target/release/deps \
  /tmp/test_rss.rs -o /tmp/test_rss 2>&1 | head -20

if [ -f /tmp/test_rss ]; then
    echo "Running test..."
    /tmp/test_rss | head -50
    echo ""
    echo "Validating XML structure..."
    /tmp/test_rss | grep -c "<rss version=\"2.0\">" && echo "✓ RSS tag found"
    /tmp/test_rss | grep -c "<item>" && echo "✓ Items found"
    /tmp/test_rss | grep -c "<guid" && echo "✓ GUIDs found"
    /tmp/test_rss | grep -c "tag:natty-lang-feeder.local" && echo "✓ TAG URI format correct"
else
    echo "Failed to build test program, skipping..."
fi
