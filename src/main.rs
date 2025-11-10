mod ai;
mod cache;
mod config;
mod error;
mod rss;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info, warn};
use std::fs;
use std::path::PathBuf;

/// AI-generated RSS feeds for newsboat
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Feed name to generate (when no subcommand specified)
    #[arg(value_name = "FEED_NAME", global = true)]
    feed_name: Option<String>,

    /// Config file path
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    /// Force refresh, ignore cache
    #[arg(short, long, global = true)]
    force_refresh: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install natty-lang-feeder with newsboat integration
    Install {
        /// Installation directory (default: ~/.natty-lang-feeder)
        #[arg(long)]
        install_dir: Option<PathBuf>,

        /// Skip updating newsboat configuration
        #[arg(long)]
        skip_newsboat: bool,
    },

    /// Uninstall natty-lang-feeder
    Uninstall {
        /// Installation directory to remove (default: ~/.natty-lang-feeder)
        #[arg(long)]
        install_dir: Option<PathBuf>,

        /// Also remove newsboat configuration entries
        #[arg(long)]
        remove_newsboat: bool,
    },

    /// Add a new feed to configuration
    AddFeed {
        /// Feed name
        name: String,

        /// Feed prompt (natural language query)
        prompt: String,

        /// Also add to newsboat URLs
        #[arg(long)]
        add_to_newsboat: bool,
    },

    /// List all configured feeds
    ListFeeds,

    /// Generate wrapper script for newsboat
    GenerateWrapper {
        /// Output path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate RSS feed (explicit command)
    Generate {
        /// Feed name to generate
        feed_name: String,

        /// Force refresh, ignore cache
        #[arg(short, long)]
        force_refresh: bool,
    },
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
        std::env::var("NATTY_LANG_FEEDER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&log_level))
        .target(env_logger::Target::Stderr)
        .init();

    // Route to appropriate command handler
    match args.command {
        Some(Commands::Install { install_dir, skip_newsboat }) => {
            cmd_install(install_dir, skip_newsboat).await
        }
        Some(Commands::Uninstall { install_dir, remove_newsboat }) => {
            cmd_uninstall(install_dir, remove_newsboat).await
        }
        Some(Commands::AddFeed { name, prompt, add_to_newsboat }) => {
            cmd_add_feed(&name, &prompt, add_to_newsboat, args.config).await
        }
        Some(Commands::ListFeeds) => {
            cmd_list_feeds(args.config).await
        }
        Some(Commands::GenerateWrapper { output }) => {
            cmd_generate_wrapper(output).await
        }
        Some(Commands::Generate { feed_name, force_refresh }) => {
            cmd_generate(&feed_name, args.config, force_refresh).await
        }
        None => {
            // Backward compatibility: if no subcommand but feed_name is provided, generate the feed
            if let Some(feed_name) = args.feed_name {
                cmd_generate(&feed_name, args.config, args.force_refresh).await
            } else {
                anyhow::bail!("No feed name provided. Use --help for usage information.");
            }
        }
    }
}

/// Generate RSS feed for a given feed name
async fn cmd_generate(feed_name: &str, config_path: Option<PathBuf>, force_refresh: bool) -> Result<()> {
    info!("Generating feed: {}", feed_name);

    // Load configuration
    let config_path = config_path.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
            .join("feeds.conf")
    });

    let config = config::Config::load(&config_path)?;
    let feed_config = config.get_feed(feed_name)?;

    // Check cache first
    if !force_refresh {
        if let Some(cached_rss) = cache::get_cached_feed(feed_name, &config)? {
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
    let rss_xml = rss::build_rss(feed_name, &articles)?;

    // Cache the result
    cache::write_cache(feed_name, &rss_xml, &feed_config, &config)?;

    // Output to stdout
    println!("{}", rss_xml);

    Ok(())
}

/// Install natty-lang-feeder
async fn cmd_install(install_dir: Option<PathBuf>, skip_newsboat: bool) -> Result<()> {
    let install_dir = install_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
    });

    println!("📦 Installing natty-lang-feeder to {}", install_dir.display());

    // Create installation directory
    fs::create_dir_all(&install_dir)
        .context("Failed to create installation directory")?;

    // Get the current executable path
    let current_exe = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Copy binary
    let binary_dest = install_dir.join("natty-lang-feeder");
    fs::copy(&current_exe, &binary_dest)
        .context("Failed to copy binary")?;

    println!("✓ Binary installed to {}", binary_dest.display());

    // Generate and install wrapper script
    let wrapper_path = install_dir.join("natty-lang-feeder-wrapper.sh");
    let wrapper_content = generate_wrapper_content(&binary_dest);
    fs::write(&wrapper_path, wrapper_content)
        .context("Failed to write wrapper script")?;

    // Make wrapper executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper_path, perms)?;
    }

    println!("✓ Wrapper script installed to {}", wrapper_path.display());

    // Create cache directory
    let cache_dir = install_dir.join("cache");
    fs::create_dir_all(&cache_dir)
        .context("Failed to create cache directory")?;
    println!("✓ Cache directory created at {}", cache_dir.display());

    // Copy example config if doesn't exist
    let config_path = install_dir.join("feeds.conf");
    if !config_path.exists() {
        let example_config = include_str!("../examples/feeds.conf");
        fs::write(&config_path, example_config)
            .context("Failed to write example configuration")?;
        println!("✓ Example configuration created at {}", config_path.display());
        println!("  → Edit this file to configure your feeds");
    } else {
        println!("⚠ Configuration already exists at {}", config_path.display());
    }

    println!("\n✨ Installation complete!");
    println!("\nNext steps:");
    println!("1. Set your API key:");
    println!("   export OPENAI_API_KEY='your-key-here'");
    println!("   (Add to ~/.bashrc or ~/.zshrc for persistence)\n");
    println!("2. Edit your feed configuration:");
    println!("   {}\n", config_path.display());

    if !skip_newsboat {
        println!("3. Add feeds to newsboat (~/.newsboat/urls):");
        println!("   exec:{} tech-news", wrapper_path.display());
        println!("   exec:{} security-news\n", wrapper_path.display());
        println!("4. Configure newsboat (~/.newsboat/config):");
        println!("   auto-reload no\n");
    }

    println!("5. Test your feed:");
    println!("   {} tech-news", wrapper_path.display());

    Ok(())
}

/// Uninstall natty-lang-feeder
async fn cmd_uninstall(install_dir: Option<PathBuf>, remove_newsboat: bool) -> Result<()> {
    let install_dir = install_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
    });

    println!("🗑️  Uninstalling natty-lang-feeder from {}", install_dir.display());

    if !install_dir.exists() {
        warn!("Installation directory does not exist: {}", install_dir.display());
        return Ok(());
    }

    // Remove installation directory
    fs::remove_dir_all(&install_dir)
        .context("Failed to remove installation directory")?;

    println!("✓ Removed {}", install_dir.display());

    if remove_newsboat {
        println!("\n⚠  Manual step required:");
        println!("Remove natty-lang-feeder entries from ~/.newsboat/urls");
        println!("(Lines starting with 'exec:{}/')", install_dir.display());
    }

    println!("\n✨ Uninstallation complete!");

    Ok(())
}

/// Add a feed to configuration
async fn cmd_add_feed(name: &str, prompt: &str, add_to_newsboat: bool, config_path: Option<PathBuf>) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
            .join("feeds.conf")
    });

    println!("➕ Adding feed: {}", name);

    // Read existing config
    let mut config_content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .context("Failed to read configuration file")?
    } else {
        String::new()
    };

    // Append new feed
    config_content.push_str(&format!("\nfeed {}\n", name));
    config_content.push_str(&format!("  prompt \"{}\"\n", prompt));

    // Write updated config
    fs::write(&config_path, config_content)
        .context("Failed to write configuration file")?;

    println!("✓ Added feed '{}' to {}", name, config_path.display());

    if add_to_newsboat {
        let install_dir = dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder");
        let wrapper_path = install_dir.join("natty-lang-feeder-wrapper.sh");

        if wrapper_path.exists() {
            println!("\n📝 Add this line to ~/.newsboat/urls:");
            println!("   exec:{} {}", wrapper_path.display(), name);
        } else {
            warn!("Wrapper script not found. Run 'natty-lang-feeder install' first.");
        }
    }

    Ok(())
}

/// List all configured feeds
async fn cmd_list_feeds(config_path: Option<PathBuf>) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
            .join("feeds.conf")
    });

    println!("📋 Configured feeds in {}", config_path.display());

    if !config_path.exists() {
        println!("⚠  Configuration file does not exist");
        println!("Run 'natty-lang-feeder install' or create {} manually", config_path.display());
        return Ok(());
    }

    let config = config::Config::load(&config_path)?;
    let feeds = config.list_feeds();

    if feeds.is_empty() {
        println!("⚠  No feeds configured");
        return Ok(());
    }

    println!("\nFound {} feed(s):\n", feeds.len());
    for feed_name in feeds {
        match config.get_feed(&feed_name) {
            Ok(feed) => {
                println!("  • {}", feed_name);
                println!("    Prompt: {}", feed.prompt);
                if let Some(provider) = &feed.provider {
                    println!("    Provider: {}", provider.as_str());
                }
                if let Some(model) = &feed.model {
                    println!("    Model: {}", model);
                }
                println!();
            }
            Err(_) => {
                println!("  • {} (error loading config)", feed_name);
            }
        }
    }

    Ok(())
}

/// Generate wrapper script
async fn cmd_generate_wrapper(output: Option<PathBuf>) -> Result<()> {
    let install_dir = dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".natty-lang-feeder");
    let binary_path = install_dir.join("natty-lang-feeder");

    let wrapper_content = generate_wrapper_content(&binary_path);

    if let Some(output_path) = output {
        fs::write(&output_path, &wrapper_content)
            .context("Failed to write wrapper script")?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&output_path, perms)?;
        }

        println!("✓ Wrapper script written to {}", output_path.display());
    } else {
        print!("{}", wrapper_content);
    }

    Ok(())
}

/// Generate wrapper script content
fn generate_wrapper_content(binary_path: &PathBuf) -> String {
    format!(
        r#"#!/bin/sh
# Wrapper script for newsboat compatibility
# Newsboat's exec: mechanism requires a shell script with a shebang

# Execute the natty-lang-feeder binary with all arguments passed through
exec "{}" "$@"
"#,
        binary_path.display()
    )
}
