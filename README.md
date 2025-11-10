# Natty Lang Feeder

Turn any natural language query into an RSS feed using AI.

## What is Natty Lang Feeder?

Instead of hunting for the perfect RSS feed, just describe what you want in plain English. Natty Lang Feeder uses AI (OpenAI or Anthropic) to find relevant articles and generate a custom RSS feed for you.

**Example:** "Find recent AI breakthroughs and emerging technology" → Get a daily RSS feed with exactly that content.

## Using with Newsboat

Natty Lang Feeder works seamlessly with [newsboat](https://newsboat.org/), a popular terminal RSS reader.

**Quick Setup:**

1. Install natty-lang-feeder and set your API key
2. Create a feed configuration describing what news you want
3. Add it to newsboat using `exec:` URLs
4. Use newsboat normally - your AI feeds appear alongside regular RSS feeds

**In your `~/.newsboat/urls` file:**
```
# Traditional RSS feeds
https://news.ycombinator.com/rss

# AI-generated feeds
exec:~/.natty-lang-feeder/natty-lang-feeder tech-news
exec:~/.natty-lang-feeder/natty-lang-feeder security-news
```

That's it. When newsboat loads, it runs natty-lang-feeder, which generates your custom RSS feed on demand. Articles are cached daily to minimize API costs.

**Benefits:**
- No need to find and subscribe to dozens of RSS feeds
- Get personalized news curated by AI
- Works with your existing newsboat installation
- Smart caching minimizes costs (~$2-10/month)

## Features

- ✅ Works with standard newsboat (no fork required)
- ✅ OpenAI and Anthropic API support
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
git clone https://github.com/wwlorey/natty-lang-feeder
cd natty-lang-feeder

# Build the release binary
cargo build --release

# Copy binary to your PATH
cp target/release/natty-lang-feeder ~/.natty-lang-feeder/natty-lang-feeder

# Or install globally
cargo install --path .
```

## Quick Start

**Note:** See the `examples/` directory for complete configuration files.

### 1. Set API Key

```bash
# For OpenAI (primary)
export OPENAI_API_KEY="sk-..."

# Or for Anthropic (fallback)
export ANTHROPIC_API_KEY="sk-ant-..."

# Add to your shell config (~/.bashrc, ~/.zshrc, etc.) for persistence
echo 'export OPENAI_API_KEY="sk-..."' >> ~/.bashrc
```

### 2. Create Feed Configuration

**Option A: Use the sample configuration (recommended)**

```bash
# Copy and customize the sample configuration
cp feeds.conf.sample ~/.natty-lang-feeder/feeds.conf
nano ~/.natty-lang-feeder/feeds.conf
```

The `feeds.conf.sample` file includes 10+ example feeds with detailed comments explaining all configuration options, prompt writing tips, and cost optimization strategies. See also `examples/feeds.conf` for day-based caching examples.

**Option B: Create a minimal configuration**

Create `~/.natty-lang-feeder/feeds.conf`:

```conf
# Global settings (optional)
default-provider openai
log-level info

# Feed definitions (day-based caching enabled by default)
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs and emerging technology"

feed security-news
  prompt "Latest CVEs and security vulnerabilities"
  model gpt-4o-mini
```

### 3. Add to Newsboat

Edit `~/.newsboat/urls` (see `examples/newsboat-urls` for a complete example):

```
# Traditional RSS feeds
https://news.ycombinator.com/rss

# AI-generated feeds
exec:~/.natty-lang-feeder/natty-lang-feeder tech-news
exec:~/.natty-lang-feeder/natty-lang-feeder security-news
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
default-provider openai          # openai | anthropic
cache-dir ~/.natty-lang-feeder/cache
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
- `ANTHROPIC_API_KEY` - Anthropic API key (fallback)

**Optional:**
- `NATTY_LANG_FEEDER_CONFIG` - Custom config file path (default: `~/.natty-lang-feeder/feeds.conf`)
- `NATTY_LANG_FEEDER_CACHE_DIR` - Custom cache directory (default: `~/.natty-lang-feeder/cache`)
- `NATTY_LANG_FEEDER_LOG_LEVEL` - Logging verbosity: `error|warn|info|debug`

### Command-Line Options

```bash
natty-lang-feeder <feed-name> [OPTIONS]

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

1. **newsboat** detects `exec:` URLs and runs the natty-lang-feeder binary
2. **natty-lang-feeder** reads your feed configuration and checks the cache
3. If cache is expired, it calls the AI API with your prompt
4. AI returns articles with titles, summaries, and source URLs
5. **natty-lang-feeder** generates valid RSS 2.0 XML and outputs to stdout
6. **newsboat** parses the RSS and displays articles normally

## Caching

Natty Lang Feeder caches generated feeds to minimize API costs:

- Cache location: `~/.natty-lang-feeder/cache/`
- Default TTL: 1 hour (configurable per feed)
- Force refresh: `natty-lang-feeder <feed> --force-refresh`

Cache files:
- `<feed-name>.xml` - Cached RSS XML
- `<feed-name>.meta` - Metadata (timestamp, TTL, provider)

## Cost Estimates

Typical costs for 3-5 AI feeds with 2-4 reloads per day:

**OpenAI GPT-4o:**
- ~$6-10/month

**OpenAI GPT-4o-mini:**
- ~$1-3/month (cheaper, faster)

**Anthropic Sonnet:**
- ~$15-25/month (includes web search)

**Tips to reduce costs:**
- Increase refresh intervals (3600+ seconds)
- Use gpt-4o-mini for frequent updates
- Reduce max-articles count

## Controlling When AI Articles Are Generated

Natty Lang Feeder is designed to **minimize API costs** by generating AI articles only once per day (based on calendar day boundaries in UTC).

### Understanding the Day-Based Cache System

Natty Lang Feeder only calls the AI API when:
1. No cache exists for a feed
2. The cache was generated on a previous day (UTC)
3. You use `--force-refresh` flag

**The cache is valid for the entire calendar day it was generated.** If you generate articles at 11pm on Monday, they'll regenerate when you open newsboat on Tuesday (even if it's 8am - less than 24 hours later). This ensures you always get fresh content when starting your day, without multiple API calls throughout the day.

### Recommended Configuration for Minimal API Calls

**1. Configure natty-lang-feeder feeds** (`~/.natty-lang-feeder/feeds.conf`):

```conf
# AI feeds - automatically cached per calendar day
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs"

feed security-news
  prompt "Latest security vulnerabilities"
  model gpt-4o-mini

# Note: The 'refresh' parameter is optional and not required
# Articles automatically regenerate once per calendar day
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
1. **Monday 8am: Open newsboat** → First time today, AI generates articles
2. **Monday 9am: Reopen newsboat** → Uses cached articles from 8am, no API call
3. **Monday 11pm: Reopen newsboat** → Still Monday, uses cache, no API call
4. **Tuesday 7am: Open newsboat** → New day detected, AI generates fresh articles
5. **Tuesday noon: Reopen newsboat** → Uses cached articles from 7am, no API call

**Result:** Maximum of 1 API call per feed per day, only when you actually open newsboat on a new day.

### Cost Example

With this setup for 5 AI feeds:
- **Daily cost:** Exactly 5 API calls/day (one per feed, only on first newsboat open each day)
- **If you skip a day:** 0 API calls (natty-lang-feeder only runs when newsboat opens)
- **Monthly cost:** ~$2-5 with gpt-4o-mini, ~$5-10 with gpt-4o

### Manual Refresh When Needed

If you want fresh articles during the same day (bypassing the day-based cache):

```bash
# Force refresh a specific feed
natty-lang-feeder tech-news --force-refresh

# Or use it directly in newsboat by temporarily modifying the URL
```

**Note:** With `auto-reload no` in newsboat, pressing `r` will reload all feeds and respect the day-based cache (only calling AI if it's a new day). AI feeds cached today will return instantly without API calls.

## Troubleshooting

### Feed shows "Error: OPENAI_API_KEY not set"

```bash
export OPENAI_API_KEY="sk-..."
# Add to ~/.bashrc or ~/.zshrc for persistence
```

### Feed not updating

1. Check cache TTL: `ls -lh ~/.natty-lang-feeder/cache/`
2. Force refresh: `natty-lang-feeder <feed> --force-refresh`
3. Check logs: `~/.natty-lang-feeder/error.log`

### Poor quality summaries

1. Make prompt more specific
2. Lower temperature (0.1-0.2)
3. Try different model (gpt-4o vs gpt-4o-mini vs anthropic)

### Rate limited

1. Increase refresh interval
2. Reduce max-articles count
3. Check API usage dashboard

## Development

### Project Structure

```
natty-lang-feeder/
├── src/
│   ├── main.rs       # CLI entry point
│   ├── config.rs     # Configuration parsing
│   ├── ai/
│   │   ├── mod.rs    # AI provider trait
│   │   ├── openai.rs # OpenAI implementation
│   │   └── claude.rs # Anthropic implementation
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
natty-lang-feeder tech-news --debug
# Or
NATTY_LANG_FEEDER_LOG_LEVEL=debug natty-lang-feeder tech-news
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
- [OpenAI](https://openai.com/) - AI models
- [Anthropic](https://anthropic.com/) - AI models

## Support

- GitHub Issues: https://github.com/wwlorey/natty-lang-feeder/issues
- Documentation: See ARCHITECTURE.md for technical details
