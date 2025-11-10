# Newsship

AI-generated RSS feeds for newsboat. Turn any natural language query into an RSS feed.

## Overview

Newsship is a standalone tool that integrates with [newsboat](https://newsboat.org/) to provide AI-generated RSS feeds. Simply describe what news you want, and newsship will use AI to find and summarize relevant articles.

## Features

- ✅ Works with standard newsboat (no fork required)
- ✅ OpenAI and Claude API support
- ✅ Natural language feed prompts
- ✅ Smart caching to minimize API costs
- ✅ Zero-config defaults (just set API key)
- ✅ Configurable refresh intervals, models, and more

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- newsboat
- OpenAI API key or Anthropic API key

### Build from Source

```bash
# Clone the repository
git clone https://github.com/wwlorey/newsship
cd newsship

# Build the release binary
cargo build --release

# Copy binary to your PATH
cp target/release/newsship ~/.newsship/newsship

# Or install globally
cargo install --path .
```

## Quick Start

**Note:** See the `examples/` directory for complete configuration files.

### 1. Set API Key

```bash
# For OpenAI (primary)
export OPENAI_API_KEY="sk-..."

# Or for Claude (fallback)
export ANTHROPIC_API_KEY="sk-ant-..."

# Add to your shell config (~/.bashrc, ~/.zshrc, etc.) for persistence
echo 'export OPENAI_API_KEY="sk-..."' >> ~/.bashrc
```

### 2. Create Feed Configuration

Create `~/.newsship/feeds.conf` (see `examples/feeds.conf` for a complete example):

```conf
# Global settings (optional)
default-provider openai
log-level info

# Feed definitions with 24-hour cache to minimize costs
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs and emerging technology"
  refresh 86400    # 24 hours - only regenerate once per day

feed security-news
  prompt "Latest CVEs and security vulnerabilities"
  model gpt-4o-mini
  refresh 86400
```

### 3. Add to Newsboat

Edit `~/.newsboat/urls` (see `examples/newsboat-urls` for a complete example):

```
# Traditional RSS feeds
https://news.ycombinator.com/rss

# AI-generated feeds
exec:~/.newsship/newsship tech-news
exec:~/.newsship/newsship security-news
```

And configure `~/.newsboat/config` to prevent auto-reloads (see `examples/newsboat-config`):

```conf
# Only reload when you press 'r', not automatically
auto-reload no
```

### 4. Use Newsboat Normally

```bash
newsboat
```

Press `r` to reload feeds. AI feeds will be generated alongside traditional RSS feeds.

## Configuration

### Feed Configuration Format

```conf
# Global settings
default-provider openai          # openai | claude
cache-dir ~/.newsship/cache
log-level info                   # error | warn | info | debug

# Optional global prompt prefix
global-prompt "You are an expert news curator."

# Feed definition
feed <name>
  prompt "Your natural language query"
  provider openai                # Optional: override default
  model gpt-4o                   # Optional: specific model
  refresh 3600                   # Optional: seconds (default: 3600)
  max-articles 10                # Optional: article count (default: 10)
  temperature 0.3                # Optional: AI temperature (default: 0.3)
```

### Environment Variables

**Required (at least one):**
- `OPENAI_API_KEY` - OpenAI API key (primary)
- `ANTHROPIC_API_KEY` - Claude API key (fallback)

**Optional:**
- `NEWSSHIP_CONFIG` - Custom config file path (default: `~/.newsship/feeds.conf`)
- `NEWSSHIP_CACHE_DIR` - Custom cache directory (default: `~/.newsship/cache`)
- `NEWSSHIP_LOG_LEVEL` - Logging verbosity: `error|warn|info|debug`

### Command-Line Options

```bash
newsship <feed-name> [OPTIONS]

Arguments:
  <FEED_NAME>  Feed identifier from feeds.conf

Options:
  -c, --config <FILE>     Config file path
  -f, --force-refresh     Ignore cache, regenerate feed
  -d, --debug             Enable debug logging
  -h, --help              Print help
  -V, --version           Print version
```

## Example Use Cases

### Daily Tech News
```conf
feed tech-digest
  prompt "Find 10 important tech news stories from the last 24 hours, focusing on product launches, funding rounds, and industry trends"
  refresh 3600
```

### Security Monitoring
```conf
feed security-alerts
  prompt "Latest CVEs, security vulnerabilities, and data breaches from the last 12 hours"
  model gpt-4o-mini
  refresh 1800
  max-articles 20
```

### Research Papers
```conf
feed ml-papers
  prompt "Recent machine learning papers from arXiv, focusing on transformer architectures and LLM improvements"
  refresh 7200
  max-articles 5
```

### Local News
```conf
feed sf-bay-news
  prompt "Local news from San Francisco Bay Area in the last 24 hours"
  temperature 0.2
```

## How It Works

1. **newsboat** detects `exec:` URLs and runs the newsship binary
2. **newsship** reads your feed configuration and checks the cache
3. If cache is expired, it calls the AI API with your prompt
4. AI returns articles with titles, summaries, and source URLs
5. **newsship** generates valid RSS 2.0 XML and outputs to stdout
6. **newsboat** parses the RSS and displays articles normally

## Caching

Newsship caches generated feeds to minimize API costs:

- Cache location: `~/.newsship/cache/`
- Default TTL: 1 hour (configurable per feed)
- Force refresh: `newsship <feed> --force-refresh`

Cache files:
- `<feed-name>.xml` - Cached RSS XML
- `<feed-name>.meta` - Metadata (timestamp, TTL, provider)

## Cost Estimates

Typical costs for 3-5 AI feeds with 2-4 reloads per day:

**OpenAI GPT-4o:**
- ~$6-10/month

**OpenAI GPT-4o-mini:**
- ~$1-3/month (cheaper, faster)

**Claude Sonnet:**
- ~$15-25/month (includes web search)

**Tips to reduce costs:**
- Increase refresh intervals (3600+ seconds)
- Use gpt-4o-mini for frequent updates
- Reduce max-articles count

## Controlling When AI Articles Are Generated

To avoid multiple AI API calls per day (and unnecessary costs), you need to configure **both** newsship and newsboat properly.

### Understanding the Cache System

Newsship only calls the AI API when:
1. No cache exists for a feed
2. The cache has expired (based on the `refresh` TTL)
3. You use `--force-refresh` flag

**The cache TTL controls how often the AI can be called.** If you set `refresh 86400` (24 hours), newsship will only call the AI once per day maximum, regardless of how many times newsboat reloads.

### Recommended Configuration for Minimal API Calls

**1. Set Long Cache TTLs in newsship** (`~/.newsship/feeds.conf`):

```conf
# AI feeds - generate once per day
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs"
  refresh 86400     # 24 hours - only regenerate once per day

feed security-news
  prompt "Latest security vulnerabilities"
  refresh 86400     # 24 hours

# Or even longer for less time-sensitive content
feed weekly-digest
  prompt "Important tech stories from the past week"
  refresh 604800    # 7 days
```

**2. Configure newsboat to Only Reload on Startup** (`~/.newsboat/config`):

```conf
# Only reload feeds when newsboat starts, not automatically
auto-reload no

# Optional: reload feeds in the background on startup
reload-only-visible-feeds no
reload-threads 4
```

### Usage Pattern

With this configuration:
1. **Open newsboat** → Feeds reload (but use cache if not expired)
2. **AI feeds cached?** → Return instantly, no API call
3. **AI feeds expired?** → Call AI API, cache for 24 hours
4. **Work in newsboat** → No automatic reloads, no API calls
5. **Close newsboat** → No API calls
6. **Reopen later same day** → Uses cache, no API calls
7. **Next day** → Cache expired, fresh AI generation

### Cost Example

With this setup for 5 AI feeds:
- **Worst case:** 5 API calls/day (if you open newsboat once per day)
- **Typical case:** 2-3 API calls/day (some feeds still cached)
- **Monthly cost:** ~$2-5 with gpt-4o-mini, ~$5-10 with gpt-4o

### Manual Refresh When Needed

If you want fresh articles before the cache expires:

```bash
# Force refresh a specific feed
newsship tech-news --force-refresh

# Or press 'r' in newsboat and wait for all feeds to reload
# (will respect cache TTL unless you force-refresh manually)
```

### Alternative: Reload Only on Explicit Request

If you want even more control, keep newsboat open and only reload manually:

```conf
# In ~/.newsboat/config
auto-reload no
```

Then only press `r` in newsboat when you actively want to check for new articles.

## Troubleshooting

### Feed shows "Error: OPENAI_API_KEY not set"

```bash
export OPENAI_API_KEY="sk-..."
# Add to ~/.bashrc or ~/.zshrc for persistence
```

### Feed not updating

1. Check cache TTL: `ls -lh ~/.newsship/cache/`
2. Force refresh: `newsship <feed> --force-refresh`
3. Check logs: `~/.newsship/error.log`

### Poor quality summaries

1. Make prompt more specific
2. Lower temperature (0.1-0.2)
3. Try different model (gpt-4o vs gpt-4o-mini)

### Rate limited

1. Increase refresh interval
2. Reduce max-articles count
3. Check API usage dashboard

## Development

### Project Structure

```
newsship/
├── src/
│   ├── main.rs       # CLI entry point
│   ├── config.rs     # Configuration parsing
│   ├── ai/
│   │   ├── mod.rs    # AI provider trait
│   │   ├── openai.rs # OpenAI implementation
│   │   └── claude.rs # Claude implementation
│   ├── rss.rs        # RSS generation
│   ├── cache.rs      # Caching system
│   └── error.rs      # Error types
├── Cargo.toml        # Dependencies
└── README.md
```

### Running Tests

```bash
cargo test
```

### Debug Mode

```bash
newsship tech-news --debug
# Or
NEWSSHIP_LOG_LEVEL=debug newsship tech-news
```

## Contributing

Contributions welcome! Please:

1. Follow existing code style
2. Add tests for new features
3. Update documentation
4. Create detailed pull requests

## License

MIT License - See LICENSE file for details

## Acknowledgments

- [newsboat](https://newsboat.org/) - Excellent RSS reader
- [OpenAI](https://openai.com/) - GPT models
- [Anthropic](https://anthropic.com/) - Claude models

## Support

- GitHub Issues: https://github.com/wwlorey/newsship/issues
- Documentation: See ARCHITECTURE.md for technical details
