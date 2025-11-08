mod ai;
mod cache;
mod config;
mod error;
mod rss;

use anyhow::Result;
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;

/// AI-generated RSS feeds for newsboat
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Feed name to generate
    #[arg(value_name = "FEED_NAME")]
    feed_name: String,

    /// Config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Force refresh, ignore cache
    #[arg(short, long)]
    force_refresh: bool,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let result = run().await;

    if let Err(e) = result {
        error!("Fatal error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        "debug".to_string()
    } else {
        std::env::var("NEWSSHIP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&log_level))
        .target(env_logger::Target::Stderr)
        .init();

    info!("Generating feed: {}", args.feed_name);

    // Load configuration
    let config_path = args.config.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".newsship")
            .join("feeds.conf")
    });

    let config = config::Config::load(&config_path)?;
    let feed_config = config.get_feed(&args.feed_name)?;

    // Check cache first
    if !args.force_refresh {
        if let Some(cached_rss) = cache::get_cached_feed(&args.feed_name, &config)? {
            info!("Using cached feed (not expired)");
            println!("{}", cached_rss);
            return Ok(());
        }
    }

    info!("Generating fresh feed via AI");

    // Create AI provider
    let provider = ai::create_provider(&feed_config, &config)?;

    // Generate articles
    let articles = provider.generate_articles(&feed_config).await?;

    info!("Generated {} articles", articles.len());

    // Build RSS XML
    let rss_xml = rss::build_rss(&args.feed_name, &articles)?;

    // Cache the result
    cache::write_cache(&args.feed_name, &rss_xml, &feed_config, &config)?;

    // Output to stdout
    println!("{}", rss_xml);

    Ok(())
}
