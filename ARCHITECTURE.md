# Newsship Architecture & Implementation Plan

**Version:** 1.0
**Date:** 2025-11-08
**Status:** Planning / Pre-Implementation

---

## Executive Summary

Newsship is a standalone tool that adds AI-generated RSS feeds to newsboat, configured via natural language prompts. Users continue using their existing newsboat installation, and AI feeds appear alongside traditional RSS feeds with zero additional configuration beyond setting API keys via environment variables.

**Implementation Strategy:** Tier 1 External Script Integration
- Uses newsboat's existing `exec:` URL mechanism
- Standalone script handles all AI interactions
- Zero C++ modifications to newsboat
- Clean separation of concerns
- Low risk, fast iteration

---

## 1. User Experience Specification

### 1.1 First-Time User Journey

**Installation**
1. User installs newsship binary (standalone tool)
2. User sets API key as environment variable:
   ```bash
   export OPENAI_API_KEY="sk-..."
   # or in ~/.bashrc, ~/.zshrc, etc.
   ```
3. That's it - newsship works with zero config files needed

**First AI Feed Setup**
1. User edits `~/.newsboat/urls` and adds:
   ```
   exec:~/.newsship/newsship tech-news
   ```

2. User creates `~/.newsship/feeds.conf` (auto-created with defaults if missing):
   ```
   feed tech-news
     prompt "Find 10 recent articles about AI breakthroughs and emerging technology"
   ```

3. User launches `newsboat` normally

**Zero-Config Default Behavior**
- If `OPENAI_API_KEY` is set, OpenAI is used automatically
- If `ANTHROPIC_API_KEY` is set (and OpenAI key isn't), Claude is used as fallback
- Sensible defaults for refresh intervals, article counts, etc.

### 1.2 Daily Usage Workflow

**Opening newsship**
```
$ newsboat
```

**Feed List View**
```
  N     Tech News                                         (15)
        Hacker News                                       (42)
  N     Security Updates                                  (8)
        Ars Technica                                      (23)
```

*Note: AI-generated feeds appear alongside traditional RSS feeds with no visual distinction.*

**Reloading Feeds**
- User presses `r` to reload all feeds
- Traditional RSS feeds fetch normally
- AI feeds show: `"Generating AI feed 'tech-news'..."`
- Generated content appears within 5-15 seconds (depending on API)
- User can continue using newsboat while AI feeds generate

**Reading AI Articles**
- Article list shows AI-generated summaries as titles
- Opening an article shows:
  ```
  Title: New Breakthrough in Quantum Computing Achieves Error Correction

  Summary:
  Researchers at Google Quantum AI have demonstrated a major advancement
  in quantum error correction, reducing error rates by 89% using a new
  surface code implementation...

  Sources:
  - https://www.nature.com/articles/quantum-breakthrough-2025
  - https://techcrunch.com/2025/11/google-quantum-error-correction
  - https://arstechnica.com/science/2025/11/quantum-computing-milestone

  Read more: [opens browser to primary source]
  ```

### 1.3 Error Handling UX

**Missing API Key**
- Newsboat launches normally
- AI feeds show error: `"Error: OPENAI_API_KEY not set"`
- Error logged to `~/.newsship/error.log`
- Traditional RSS feeds work normally
- User can still browse cached AI articles from previous successful runs

**Rate Limited**
```
[Warning] AI feed 'tech-news' rate limited. Showing cached version from 2 hours ago.
```
- Cached articles remain visible
- Next reload attempt uses exponential backoff

**Garbage/Invalid Output**
- If AI returns unparseable or poor content, display it as-is
- User sees raw output and can adjust prompt
- Error logged to `~/.newsship/error.log` for debugging

**Network Failures**
- Same behavior as traditional RSS feed network failures
- Shows cached version with timestamp
- Retries on next reload

### 1.4 Configuration Philosophy

**Hybrid Approach with Smart Defaults**

Simple (90% of use cases):
```
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs"
```

Advanced (power users):
```
feed security-news
  prompt "Latest CVEs and security vulnerabilities"
  provider openai
  model gpt-4o
  refresh 1800
  max-articles 15
  temperature 0.2
```

Inline overrides:
```
feed quick-news
  prompt "Tech news from last 6 hours"
  model gpt-4o-mini  # Faster, cheaper for quick updates
```

---

## 2. Technical Architecture (Tier 1: External Script)

### 2.1 System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         newsboat                            │
│  (Unmodified - standard newsboat binary)                    │
│                                                              │
│  - Reads ~/.newsboat/urls                                   │
│  - Detects exec: prefix                                     │
│  - Executes external command                                │
│  - Parses RSS XML from stdout                               │
│  - Caches in ~/.newsboat/cache.db                           │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        │ exec: trigger
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                 ~/.newsship/newsship                        │
│                  (Rust binary)                               │
│                                                              │
│  1. Parse command-line args (feed name)                     │
│  2. Read ~/.newsship/feeds.conf                             │
│  3. Load feed configuration                                 │
│  4. Check cache (if not expired, return cached XML)         │
│  5. Call AI API (OpenAI/Claude)                             │
│  6. Generate RSS 2.0 XML                                    │
│  7. Write to cache                                          │
│  8. Output XML to stdout                                    │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        │ HTTP API calls
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    AI Services                               │
│                                                              │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  OpenAI API      │         │  Claude API      │         │
│  │  (Primary)       │         │  (Fallback)      │         │
│  │                  │         │                  │         │
│  │  - Chat API      │         │  - Web search    │         │
│  │  - GPT-4o        │         │  - Native cites  │         │
│  └──────────────────┘         └──────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Component Specifications

#### 2.2.1 Newsboat Integration (No Changes)

- **Location:** Standard newsboat installation
- **Configuration:** `~/.newsboat/urls`
- **Entry Format:**
  ```
  exec:~/.newsship/newsship tech-news
  exec:~/.newsship/newsship security-news
  ```
- **Behavior:**
  - Newsboat executes the script during reload cycle
  - Captures stdout as RSS XML
  - Parses using existing RSS parser
  - Caches in SQLite database
  - Uses GUIDs for deduplication

#### 2.2.2 Feed Generator Binary

**Name:** `newsship` (Rust binary)
**Location:** `~/.newsship/newsship`
**Language:** Rust
**Dependencies:**
- `reqwest` - HTTP client with async support
- `serde_json` - JSON parsing for API responses
- `quick-xml` - RSS XML generation
- `tokio` - Async runtime
- `sha2` - GUID generation
- `chrono` - Timestamp handling

**Command-Line Interface:**
```bash
~/.newsship/newsship <feed-name> [options]

Arguments:
  <feed-name>           Feed identifier from feeds.conf

Options:
  --config <path>       Config file path (default: ~/.newsship/feeds.conf)
  --force-refresh       Ignore cache, regenerate feed
  --debug               Enable debug logging
  --help                Show help
```

**Execution Flow:**
```rust
fn main() {
    // 1. Parse CLI args
    let args = parse_args();

    // 2. Load configuration
    let config = load_config(&args.config_path)?;

    // 3. Get feed definition
    let feed = config.get_feed(&args.feed_name)?;

    // 4. Check cache
    if let Some(cached) = check_cache(&feed) {
        if !args.force_refresh && !cache_expired(cached, &feed) {
            println!("{}", cached.xml);
            return Ok(());
        }
    }

    // 5. Generate feed via AI
    let articles = generate_articles(&feed, &config)?;

    // 6. Build RSS XML
    let rss_xml = build_rss(&feed, &articles)?;

    // 7. Update cache
    write_cache(&feed, &rss_xml)?;

    // 8. Output to stdout
    println!("{}", rss_xml);
}
```

#### 2.2.3 Configuration File Format

**Location:** `~/.newsship/feeds.conf`
**Format:** Custom INI-like format with sections

```conf
# Global settings (optional - all have defaults)
default-provider openai          # openai | claude
cache-dir ~/.newsship/cache
log-level info                   # error | warn | info | debug

# Global prompt prefix (applied to all feeds)
global-prompt "You are an expert news curator. Provide accurate, concise summaries with source citations."

# Feed definitions
feed tech-news
  prompt "Find 10 recent articles about AI breakthroughs and emerging technology from the last 48 hours"
  # Optional overrides (all have smart defaults)
  # provider openai
  # model gpt-4o
  # refresh 3600
  # max-articles 10
  # temperature 0.3

feed security-news
  prompt "Find the latest cybersecurity vulnerabilities, CVEs, and security incidents"
  model gpt-4o-mini              # Faster/cheaper for frequent updates
  refresh 1800                   # Update every 30 minutes

feed custom-topic
  prompt "Articles about Rust programming language and systems programming"
  provider claude                # Override default provider
  model claude-sonnet-4-5
  max-articles 5
  temperature 0.2
```

**Smart Defaults:**
- `provider`: Uses OpenAI if `OPENAI_API_KEY` set, else Claude if `ANTHROPIC_API_KEY` set
- `model`: `gpt-4o` for OpenAI, `claude-sonnet-4-5` for Claude
- `refresh`: 3600 seconds (1 hour)
- `max-articles`: 10
- `temperature`: 0.3 (deterministic but creative)

**Configuration Parsing:**
```rust
struct GlobalConfig {
    default_provider: Provider,
    cache_dir: PathBuf,
    log_level: LogLevel,
    global_prompt: Option<String>,
}

struct FeedConfig {
    name: String,
    prompt: String,
    provider: Option<Provider>,    // None = use default
    model: Option<String>,          // None = use provider default
    refresh: Option<u64>,           // None = use 3600
    max_articles: Option<u8>,       // None = use 10
    temperature: Option<f32>,       // None = use 0.3
}
```

#### 2.2.4 AI Service Integration

**Provider Interface:**
```rust
#[async_trait]
trait AIProvider {
    async fn generate_articles(
        &self,
        prompt: &str,
        max_articles: u8,
    ) -> Result<Vec<Article>, AIError>;

    fn name(&self) -> &str;
    fn cost_per_call(&self) -> f32;
}

struct Article {
    title: String,
    summary: String,
    sources: Vec<Source>,
    date: DateTime<Utc>,
    guid: String,
}

struct Source {
    url: String,
    title: String,
    cited_text: Option<String>,
}
```

**Claude Implementation:**
```rust
struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    async fn generate_articles(&self, prompt: &str, max_articles: u8)
        -> Result<Vec<Article>, AIError>
    {
        // 1. Build request with web_search tool enabled
        let request = json!({
            "model": self.model,
            "max_tokens": 4096,
            "tools": [{"type": "web_search_20250305"}],
            "messages": [{
                "role": "user",
                "content": format!(
                    "{}\\n\\nFind exactly {} recent articles. \
                     For each article provide: title (max 100 chars), \
                     summary (max 500 chars), and source URLs.",
                    prompt, max_articles
                )
            }]
        });

        // 2. Call API
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        // 3. Parse response and extract citations
        let articles = self.parse_response(response).await?;

        // 4. Generate GUIDs
        for article in &mut articles {
            article.guid = generate_guid(&article.title, &article.summary);
        }

        Ok(articles)
    }
}
```

**OpenAI Implementation:**
```rust
struct OpenAIProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    async fn generate_articles(&self, prompt: &str, max_articles: u8)
        -> Result<Vec<Article>, AIError>
    {
        // 1. Call OpenAI Chat API with structured output
        // 2. Since OpenAI lacks native search, prompt must be clear:
        //    "Search the web and find..." (relies on user using ChatGPT web)
        //    OR integrate with Serper/Tavily for actual search
        // 3. Parse JSON response into Article structs
        // 4. Generate GUIDs

        // Note: Full implementation similar to Claude but with
        // different API endpoint and auth header format
    }
}
```

**Error Handling:**
```rust
enum AIError {
    NetworkError(reqwest::Error),
    RateLimited { retry_after: u64 },
    AuthenticationFailed,
    InvalidResponse(String),
    QuotaExceeded,
}

impl AIProvider {
    async fn call_with_retry(&self, ...) -> Result<Response, AIError> {
        let mut delay = 1000; // Start with 1 second

        for attempt in 0..3 {
            match self.call_api().await {
                Ok(response) => return Ok(response),
                Err(AIError::RateLimited { retry_after }) => {
                    eprintln!("Rate limited, retrying in {}s", retry_after);
                    sleep(Duration::from_secs(retry_after)).await;
                }
                Err(e) => {
                    if attempt == 2 {
                        return Err(e);
                    }
                    sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }
}
```

#### 2.2.5 RSS Generation

**Format:** RSS 2.0
**Encoding:** UTF-8
**Validation:** Must pass standard RSS validators

**XML Structure:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Tech News</title>
    <link>https://newsship.local/tech-news</link>
    <description>AI-generated feed: Recent AI breakthroughs</description>
    <language>en-us</language>
    <lastBuildDate>Fri, 08 Nov 2025 12:00:00 GMT</lastBuildDate>
    <ttl>60</ttl>
    <generator>newsship/0.1.0</generator>

    <item>
      <title>New Breakthrough in Quantum Computing Achieves Error Correction</title>
      <link>https://www.nature.com/articles/quantum-breakthrough-2025</link>
      <guid isPermaLink="false">tag:newsship.local,2025:tech-news-a3f8b91c</guid>
      <pubDate>Thu, 07 Nov 2025 15:30:00 GMT</pubDate>
      <description><![CDATA[
        <p>Researchers at Google Quantum AI have demonstrated a major advancement
        in quantum error correction, reducing error rates by 89% using a new
        surface code implementation...</p>

        <p><strong>Sources:</strong></p>
        <ul>
          <li><a href="https://www.nature.com/articles/quantum-breakthrough-2025">Nature: Quantum Computing Breakthrough</a></li>
          <li><a href="https://techcrunch.com/2025/11/google-quantum-error-correction">TechCrunch: Google's Quantum Milestone</a></li>
        </ul>
      ]]></description>
    </item>

    <!-- More items... -->
  </channel>
</rss>
```

**GUID Generation Strategy:**
```rust
fn generate_guid(title: &str, summary: &str) -> String {
    use sha2::{Sha256, Digest};

    // Combine title + summary for content hash
    let content = format!("{}{}", title, &summary[..200.min(summary.len())]);

    // Hash to ensure deterministic GUID
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    // TAG URI scheme: tag:domain,date:specific-id
    format!(
        "tag:newsship.local,2025:{}",
        hex::encode(&hash[..8]) // First 16 hex chars
    )
}
```

**Key Requirements:**
- GUIDs must be stable across regenerations if content is identical
- Dates in RFC 822 format for `pubDate`
- HTML entities properly escaped or wrapped in CDATA
- `<ttl>` set to refresh interval (in minutes)
- Multiple sources formatted as HTML list in description

#### 2.2.6 Caching System

**Cache Location:** `~/.newsship/cache/`
**Cache Structure:**
```
~/.newsship/cache/
  tech-news.xml           # Cached RSS XML
  tech-news.meta          # Metadata (timestamp, TTL)
  security-news.xml
  security-news.meta
```

**Cache Metadata Format:**
```json
{
  "generated_at": "2025-11-08T12:00:00Z",
  "ttl_seconds": 3600,
  "expires_at": "2025-11-08T13:00:00Z",
  "article_count": 10,
  "provider": "claude",
  "model": "claude-sonnet-4-5"
}
```

**Cache Logic:**
```rust
fn check_cache(feed: &FeedConfig) -> Option<CachedFeed> {
    let cache_path = feed.cache_path();
    let meta_path = feed.meta_path();

    if !cache_path.exists() || !meta_path.exists() {
        return None;
    }

    let meta: CacheMeta = read_json(&meta_path)?;

    if Utc::now() > meta.expires_at {
        return None; // Expired
    }

    let xml = fs::read_to_string(&cache_path)?;
    Some(CachedFeed { xml, meta })
}

fn write_cache(feed: &FeedConfig, xml: &str) -> Result<()> {
    let meta = CacheMeta {
        generated_at: Utc::now(),
        ttl_seconds: feed.refresh.unwrap_or(3600),
        expires_at: Utc::now() + Duration::seconds(feed.refresh.unwrap_or(3600)),
        article_count: count_articles(xml),
        provider: feed.provider.to_string(),
        model: feed.model.clone(),
    };

    fs::write(feed.cache_path(), xml)?;
    fs::write(feed.meta_path(), serde_json::to_string_pretty(&meta)?)?;

    Ok(())
}
```

---

## 3. Implementation Plan

### Phase 1: Minimal Viable Product (Week 1-2)

**Goal:** Working AI feed generation with OpenAI

**Deliverables:**
- [ ] Rust project scaffolding
- [ ] CLI argument parsing
- [ ] Basic config file parser (feed name + prompt only)
- [ ] OpenAI API integration
  - [ ] Authentication
  - [ ] Chat completion request
  - [ ] Response parsing
- [ ] RSS 2.0 XML generation
- [ ] GUID generation algorithm
- [ ] Output to stdout
- [ ] Manual testing with newsboat

**Success Criteria:**
- `exec:~/.newsship/newsship tech-news` produces valid RSS XML
- Newsboat parses and displays AI-generated articles
- Articles include summaries and source links

### Phase 2: Production Features (Week 3-4)

**Goal:** Robust, production-ready script

**Deliverables:**
- [ ] Full configuration file support
  - [ ] Global settings
  - [ ] Per-feed overrides
  - [ ] Smart defaults
- [ ] Caching system
  - [ ] Read/write cache
  - [ ] TTL expiration
  - [ ] Force refresh flag
- [ ] Error handling
  - [ ] API key validation
  - [ ] Rate limiting detection
  - [ ] Network error recovery
  - [ ] Invalid response handling
- [ ] Logging system
  - [ ] Debug mode
  - [ ] Error logs to file
  - [ ] Stdout remains clean RSS
- [ ] Claude provider implementation (fallback)
- [ ] Provider fallback logic

**Success Criteria:**
- Script handles missing API keys gracefully
- Cache prevents redundant API calls
- Errors logged but don't crash newsboat
- Works with both OpenAI and Claude

### Phase 3: Distribution & Documentation (Week 5-6)

**Goal:** Easy installation and comprehensive docs

**Deliverables:**
- [ ] Build system
  - [ ] Cross-platform compilation (Linux, macOS, BSD)
  - [ ] Release binaries
  - [ ] Installation script
- [ ] Installation guide
  - [ ] Dependencies
  - [ ] Build from source
  - [ ] Binary installation
  - [ ] Configuration setup
- [ ] User documentation
  - [ ] Quick start guide
  - [ ] Configuration reference
  - [ ] Prompt engineering tips
  - [ ] Troubleshooting guide
- [ ] Developer documentation
  - [ ] Architecture overview
  - [ ] Adding new providers
  - [ ] Contributing guide
- [ ] Example configurations
  - [ ] Tech news
  - [ ] Security updates
  - [ ] Academic papers
  - [ ] Domain-specific feeds

**Success Criteria:**
- New user can install and configure in < 10 minutes
- Documentation answers common questions
- Examples demonstrate key features

### Phase 4: Polish & Community (Week 7-8)

**Goal:** Production release ready for users

**Deliverables:**
- [ ] Performance optimization
  - [ ] Async/parallel feed generation
  - [ ] Connection pooling
  - [ ] Response streaming
- [ ] Advanced features
  - [ ] Custom date ranges in prompts
  - [ ] Domain allowlist/blocklist
  - [ ] Article deduplication across feeds
- [ ] Testing
  - [ ] Unit tests for core functions
  - [ ] Integration tests with mock APIs
  - [ ] Real-world usage testing
- [ ] Packaging
  - [ ] Debian/Ubuntu packages
  - [ ] Arch AUR
  - [ ] Homebrew formula (macOS)
  - [ ] Docker image
- [ ] Community engagement
  - [ ] GitHub repository setup
  - [ ] Issue templates
  - [ ] PR guidelines
  - [ ] Announcement post

**Success Criteria:**
- Installation available via package managers
- Test coverage > 70%
- At least 10 beta testers providing feedback
- Zero critical bugs

---

## 4. Configuration Reference

### 4.1 Environment Variables

**Required (at least one):**
- `OPENAI_API_KEY` - OpenAI API key (primary)
- `ANTHROPIC_API_KEY` - Claude API key (fallback)

**Optional:**
- `NEWSSHIP_CONFIG` - Custom config file path (default: `~/.newsship/feeds.conf`)
- `NEWSSHIP_CACHE_DIR` - Custom cache directory (default: `~/.newsship/cache`)
- `NEWSSHIP_LOG_LEVEL` - Logging verbosity: `error|warn|info|debug` (default: `info`)

### 4.2 Newsboat URLs File

**Location:** `~/.newsboat/urls`

**Format:**
```
# Traditional RSS feeds
https://news.ycombinator.com/rss
https://arstechnica.com/feed/

# AI-generated feeds
exec:~/.newsship/newsship tech-news
exec:~/.newsship/newsship security-news
exec:~/.newsship/newsship rust-weekly
```

**Optional tags:**
```
exec:~/.newsship/newsship tech-news "Tech News" ai tech
```

### 4.3 Feed Configuration File

**Location:** `~/.newsship/feeds.conf`

**Sections:**
1. **Global settings** (optional)
2. **Feed definitions** (one or more)

**Full example:**
```conf
# ============================================
# Global Settings (all optional)
# ============================================

default-provider openai
cache-dir ~/.newsship/cache
log-level info

# Applied to all prompts as a prefix
global-prompt "You are an expert news curator. Provide accurate, concise summaries."

# ============================================
# Feed Definitions
# ============================================

feed tech-news
  prompt "Find 10 recent articles about AI, machine learning, and emerging tech from the last 48 hours"

feed security-news
  prompt "Latest cybersecurity incidents, CVEs, and vulnerability disclosures"
  model gpt-4o-mini
  refresh 1800
  max-articles 15

feed rust-weekly
  prompt "Recent articles about Rust programming, including libraries, tutorials, and community updates"
  provider claude
  model claude-sonnet-4-5
  temperature 0.2

feed academic-ai
  prompt "Recent AI research papers from arXiv, focusing on LLMs and neural architectures"
  max-articles 5
  refresh 7200
```

### 4.4 Configuration Validation

**On script execution, validate:**
1. At least one API key is set
2. Feed name from CLI arg exists in config
3. All specified models are valid
4. Refresh intervals are reasonable (300-86400 seconds)
5. Temperature is 0.0-2.0
6. Max articles is 1-50

**If validation fails:**
- Print clear error message to stderr
- Output minimal RSS with error in description to stdout
- Exit with code 1

---

## 5. Success Criteria

### 5.1 Functional Requirements

**Must Have:**
- ✅ Generate valid RSS 2.0 XML from AI prompts
- ✅ Work with unmodified newsboat via `exec:` mechanism
- ✅ Support OpenAI and Claude providers
- ✅ Cache generated feeds to minimize API calls
- ✅ Generate stable GUIDs for deduplication
- ✅ Handle missing API keys gracefully
- ✅ Show multiple sources per article
- ✅ Work with zero config (environment variables only)

**Should Have:**
- ✅ Smart defaults for all settings
- ✅ Provider fallback (OpenAI → Claude)
- ✅ Rate limiting detection and retry
- ✅ Debug logging to file
- ✅ Force refresh option
- ✅ Custom cache TTL per feed

**Nice to Have:**
- ⚪ Parallel feed generation
- ⚪ Custom date range filtering
- ⚪ Domain allowlist/blocklist
- ⚪ Article deduplication across feeds
- ⚪ Usage statistics tracking

### 5.2 Performance Requirements

- **RSS Generation Time:** < 15 seconds for 10 articles
- **Cache Hit:** < 100ms to return cached feed
- **Memory Usage:** < 50MB per feed generation
- **Startup Time:** < 1 second to parse config and check cache

### 5.3 Quality Requirements

- **RSS Validity:** 100% pass rate on W3C RSS validator
- **GUID Stability:** Same content = same GUID across runs
- **Error Rate:** < 5% failure rate under normal operation
- **Cache Hit Rate:** > 80% during typical usage (hourly reloads)

### 5.4 User Experience Requirements

- **Installation:** < 10 minutes from zero to working feed
- **Configuration:** Single file, < 20 lines for typical setup
- **Documentation:** Every feature documented with examples
- **Error Messages:** Clear, actionable, no technical jargon

---

## 6. Risk Assessment & Mitigations

### 6.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|---------|------------|
| AI returns invalid/unparseable data | Medium | High | Validate response schema, retry with error feedback, fallback to cached |
| Rate limiting blocks updates | Medium | Medium | Implement exponential backoff, show cached version, set conservative TTL |
| GUID collision or instability | Low | High | Use content hash + feed name, test thoroughly with edge cases |
| newsboat `exec:` output buffer limits | Low | Medium | Test with large feeds, implement pagination if needed |
| API key exposure in process list | Low | High | Use env vars only, never CLI args or config files |

### 6.2 Operational Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|---------|------------|
| Unexpected API costs | Medium | Medium | Document pricing clearly, recommend conservative refresh intervals |
| Provider API changes | Low | High | Version-pin API requests, monitor changelogs, add integration tests |
| Cache corruption | Low | Medium | Validate cache on read, regenerate if invalid, log errors |

### 6.3 Community Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|---------|------------|
| Users expect native newsboat integration | Medium | Low | Document clearly this is external script approach, plan Tier 2 if demand exists |
| Prompt engineering too complex | Medium | Medium | Provide examples, templates, and best practices guide |
| Maintenance burden too high | Low | Medium | Keep codebase simple, comprehensive tests, clear documentation |

---

## 7. Future Enhancements (Post-MVP)

### Tier 2: Native Integration (Optional)

**If Tier 1 proves successful and popular:**
- Implement `ai:` URL scheme directly in newsboat
- Rust library with C++ FFI using `cxx` crate
- Tighter integration with newsboat's config system
- Progress indicators during feed generation
- Better error reporting in UI

**Timeline:** +6-8 weeks after Tier 1 completion

### Advanced Features (Community-Driven)

- **Custom LLM support:** OpenRouter, local models (Ollama)
- **Smart scheduling:** Refresh based on feed volatility
- **Article quality scoring:** Filter low-quality AI summaries
- **Multi-language support:** Non-English news feeds
- **Citation ranking:** Prioritize authoritative sources

---

## 8. Design Decisions (Resolved)

**Key decisions made during planning:**

1. **Binary name:** `newsship` - simple, memorable, matches project name

2. **Primary provider:** OpenAI (with Claude as fallback)
   - Default model: GPT-4o for quality, GPT-4o-mini for cost savings
   - Users can override per-feed

3. **Visual indicators:** None - AI feeds appear identical to traditional RSS feeds

4. **Installation strategy:** Standalone tool that works with stock newsboat
   - No fork required, no upstream compatibility issues
   - Users continue using their existing newsboat installation

5. **Prompt configuration:** User-written prompts only
   - No built-in template library (keeps it simple)
   - Sample prompts provided in example config file

6. **Error handling:** Errors logged to `~/.newsship/error.log` only
   - Clean stdout for RSS output
   - Users can tail log file for debugging

---

## Next Steps

**Before implementation begins:**

1. ✅ Review and approve this architecture document
2. ✅ Resolve design decisions
3. ⏳ Set up GitHub repository structure
4. ⏳ Create initial Rust project scaffold
5. ⏳ Begin Phase 1 implementation

**Status:** Ready for implementation approval

**Estimated time to first working prototype:** 2 weeks from approval

---

## Appendix A: Example Use Cases

### Use Case 1: Daily Tech News
```conf
feed tech-digest
  prompt "Find 10 important tech news stories from the last 24 hours, focusing on product launches, funding rounds, and industry trends"
  refresh 3600
```

### Use Case 2: Security Monitoring
```conf
feed security-alerts
  prompt "Latest CVEs, security vulnerabilities, and data breaches from the last 12 hours"
  model gpt-4o-mini
  refresh 1800
  max-articles 20
```

### Use Case 3: Research Papers
```conf
feed ml-papers
  prompt "Recent machine learning papers from arXiv, focusing on transformer architectures and LLM improvements"
  refresh 7200
  max-articles 5
```

### Use Case 4: Local News
```conf
feed sf-bay-news
  prompt "Local news from San Francisco Bay Area in the last 24 hours, including city politics, transportation, and housing"
  temperature 0.2
```

---

## Appendix B: Troubleshooting Guide

### Problem: Feed shows "Error: OPENAI_API_KEY not set"

**Solution:**
```bash
export OPENAI_API_KEY="sk-..."
# Add to ~/.bashrc or ~/.zshrc for persistence
```

### Problem: Feed not updating / showing old articles

**Possible causes:**
1. Cache not expired yet (check TTL in feeds.conf)
2. newsboat not reloading (press `r` or check `reload-time`)
3. API rate limited (check logs)

**Solution:**
```bash
# Force refresh (bypass cache)
~/.newsship/newsship tech-news --force-refresh

# Check logs
tail -f ~/.newsship/error.log

# Verify cache status
ls -lh ~/.newsship/cache/
```

### Problem: AI returns poor quality summaries

**Solution:**
1. Refine prompt to be more specific
2. Add examples in prompt
3. Lower temperature (0.1-0.2 for more deterministic)
4. Try different model (GPT-4o for quality, Claude Sonnet for alternative perspective)

### Problem: "Rate limited" errors

**Solution:**
1. Increase refresh interval (3600 → 7200)
2. Reduce max-articles (10 → 5)
3. Check API usage dashboard
4. Consider upgrading API tier

---

**End of Architecture Document**
