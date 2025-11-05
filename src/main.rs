use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use futures::stream::{FuturesUnordered, StreamExt};
use std::io;
use std::sync::Arc;

mod cache;
mod config;
mod http;
mod model;
mod providers;
mod render;

use cache::Cache;
use config::Config;
use model::{LanguageFilter, Provider, ProviderCfg};
use providers::{GitHub, GitLab, Gitea};
use render::{render, OutputFormat};

/// Trending repositories of the day - minimal MOTD CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Maximum repositories per provider
    #[arg(short = 'n', long = "max", value_name = "N", global = true)]
    max_per_provider: Option<usize>,

    /// Enable specific providers (comma-separated: gh,gl,ge)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',', global = true)]
    provider: Option<Vec<String>>,

    /// Filter by language (comma-separated: rust,go)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',', global = true)]
    lang: Option<Vec<String>>,

    /// Disable cache
    #[arg(long, global = true)]
    no_cache: bool,

    /// Output as JSON instead of MOTD
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Minimum star count threshold
    #[arg(long = "min-stars", value_name = "N", global = true)]
    min_stars: Option<u32>,

    /// Exclude GitHub repositories with these topics (comma-separated)
    #[arg(long = "exclude-topics", value_name = "LIST", value_delimiter = ',', global = true)]
    exclude_topics: Option<Vec<String>>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        match command {
            Commands::Completions { shell } => {
                let mut cmd = Args::command();
                let bin_name = cmd.get_name().to_string();
                generate(shell, &mut cmd, bin_name, &mut io::stdout());
                return Ok(());
            }
        }
    }

    let verbose = args.verbose;

    // Load configuration
    let mut config = Config::load().context("Failed to load configuration")?;

    if verbose {
        eprintln!("📋 Config loaded successfully");
    }

    // Apply CLI overrides
    if let Some(max) = args.max_per_provider {
        config.general.max_per_provider = max;
    }

    if let Some(langs) = args.lang {
        config.general.language_filter = langs;
    }

    if let Some(min) = args.min_stars {
        config.general.min_stars = Some(min);
    }

    if let Some(topics) = args.exclude_topics {
        config.github.exclude_topics = topics;
    }

    // Determine output format
    let format = if args.json {
        OutputFormat::Json
    } else {
        OutputFormat::Motd
    };

    // Initialize cache
    let cache = if args.no_cache {
        if verbose {
            eprintln!("🚫 Cache disabled");
        }
        None
    } else {
        let c = Cache::new(config.general.cache_ttl_mins).context("Failed to initialize cache")?;
        if verbose {
            eprintln!("💾 Cache initialized (TTL: {} mins)", config.general.cache_ttl_mins);
        }
        Some(c)
    };

    // Determine enabled providers
    let enabled_providers = if let Some(ref providers) = args.provider {
        // Parse short names: gh -> github, gl -> gitlab, ge -> gitea
        providers
            .iter()
            .map(|p| match p.as_str() {
                "gh" => "github",
                "gl" => "gitlab",
                "ge" => "gitea",
                _ => p.as_str(),
            })
            .collect::<Vec<_>>()
    } else {
        config.enabled_providers()
    };

    if verbose {
        eprintln!("🔌 Enabled providers: {:?}", enabled_providers);
    }

    // Build provider instances
    let mut provider_instances: Vec<(String, Box<dyn Provider>)> = Vec::new();

    for provider_id in enabled_providers {
        match provider_id {
            "github" => match GitHub::new(config.general.github_timeout_secs) {
                Ok(gh) => {
                    if verbose {
                        eprintln!("  ✓ GitHub provider initialized (timeout: {}s)", config.general.github_timeout_secs);
                    }
                    provider_instances.push(("github".to_string(), Box::new(gh)));
                }
                Err(e) => eprintln!("✗ Failed to initialize GitHub provider: {e}"),
            },
            "gitlab" => match GitLab::new(config.general.gitlab_timeout_secs) {
                Ok(gl) => {
                    if verbose {
                        eprintln!("  ✓ GitLab provider initialized (timeout: {}s)", config.general.gitlab_timeout_secs);
                    }
                    provider_instances.push(("gitlab".to_string(), Box::new(gl)));
                }
                Err(e) => eprintln!("✗ Failed to initialize GitLab provider: {e}"),
            },
            "gitea" => match Gitea::new(config.general.gitea_timeout_secs) {
                Ok(ge) => {
                    if verbose {
                        eprintln!("  ✓ Gitea provider initialized (timeout: {}s)", config.general.gitea_timeout_secs);
                    }
                    provider_instances.push(("gitea".to_string(), Box::new(ge)));
                }
                Err(e) => eprintln!("✗ Failed to initialize Gitea provider: {e}"),
            },
            _ => eprintln!("⚠ Unknown provider: {provider_id}"),
        }
    }

    if provider_instances.is_empty() {
        anyhow::bail!("No providers enabled or available");
    }

    // Create language filter
    let lang_filter = LanguageFilter::new(config.general.language_filter.clone());

    if verbose {
        if config.general.language_filter.is_empty() {
            eprintln!("🌐 Language filter: all languages");
        } else {
            eprintln!("🌐 Language filter: {:?}", config.general.language_filter);
        }
        eprintln!("🚀 Fetching repositories...");
    }

    // Fetch repositories in parallel
    let cache_arc = Arc::new(cache);
    let mut futures = FuturesUnordered::new();

    for (provider_id, provider) in provider_instances {
        let cache_ref = Arc::clone(&cache_arc);
        let lang_filter_clone = lang_filter.clone();
        let config_clone = config.clone();
        let verbose_clone = verbose;

        let future = async move {
            // Try cache first
            if let Some(ref cache) = *cache_ref {
                if let Some(cached_repos) = cache.get(&provider_id).await {
                    if verbose_clone {
                        eprintln!("  💾 {} (cached)", provider_id);
                    }
                    return Ok((provider_id.clone(), cached_repos));
                }
            }

            // Build provider config
            let provider_cfg = ProviderCfg {
                timeout_secs: config_clone.general.timeout_secs,
                token: match provider_id.as_str() {
                    "github" => config_clone.auth.github_token.clone(),
                    "gitlab" => config_clone.auth.gitlab_token.clone(),
                    "gitea" => config_clone.auth.gitea_token.clone(),
                    _ => None,
                },
                base_url: if provider_id == "gitea" {
                    Some(config_clone.gitea.base_url.clone())
                } else {
                    None
                },
                exclude_topics: if provider_id == "github" {
                    config_clone.github.exclude_topics.clone()
                } else {
                    vec![]
                },
            };

            // Fetch from provider
            let repos = provider
                .top_today(
                    &provider_cfg,
                    config_clone.get_max_entries(&provider_id),
                    &lang_filter_clone,
                )
                .await?;

            // Cache the result
            if let Some(ref cache) = *cache_ref {
                let _ = cache.set(&provider_id, repos.clone()).await;
            }

            Ok::<_, anyhow::Error>((provider_id, repos))
        };

        futures.push(future);
    }

    // Collect results
    let mut all_repos = Vec::new();
    let mut errors = Vec::new();

    while let Some(result) = futures.next().await {
        match result {
            Ok((provider_id, repos)) => {
                if verbose {
                    eprintln!("  📦 {}: {} repos", provider_id, repos.len());
                }
                if !repos.is_empty() {
                    all_repos.extend(repos);
                } else if format!("{format:?}") == "Motd" {
                    eprintln!("⚠ No repositories found for {provider_id}");
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  ✗ Provider error: {e}");
                }
                errors.push(e);
            }
        }
    }

    // Handle errors
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("✗ Error: {error}");
        }
    }

    // If all providers failed and we have no repos, exit with error
    if all_repos.is_empty() && !errors.is_empty() {
        anyhow::bail!("All providers failed");
    }

    // Apply ASCII-only filter if enabled
    if config.general.ascii_only {
        let before_count = all_repos.len();
        all_repos.retain(|repo| is_mostly_ascii(repo));
        if verbose {
            let filtered_count = before_count - all_repos.len();
            eprintln!("🔤 ASCII filter: removed {filtered_count} non-ASCII repos");
        }
    }

    // Apply minimum star filter if configured
    if let Some(min_stars) = config.general.min_stars {
        let before_count = all_repos.len();
        all_repos.retain(|repo| repo.stars_total.unwrap_or(0) >= min_stars.into());
        if verbose {
            let filtered_count = before_count - all_repos.len();
            eprintln!("⭐ Star filter: removed {filtered_count} repos below {min_stars} stars");
        }
    }

    if verbose {
        eprintln!("📊 Total repositories: {}", all_repos.len());
    }

    // Render output
    render(&all_repos, format);

    Ok(())
}

/// Check if a repository is mostly ASCII (filters out CJK/non-Latin scripts)
fn is_mostly_ascii(repo: &model::Repo) -> bool {
    // Check name - should be primarily ASCII
    let name_ascii_ratio = ascii_ratio(&repo.name);
    if name_ascii_ratio < 0.8 {
        return false;
    }

    // Check description if present
    if let Some(ref desc) = repo.description {
        let desc_ascii_ratio = ascii_ratio(desc);
        if desc_ascii_ratio < 0.7 {
            return false;
        }
    }

    true
}

/// Calculate the ratio of ASCII characters in a string
fn ascii_ratio(s: &str) -> f64 {
    if s.is_empty() {
        return 1.0;
    }
    let total_chars = s.chars().count();
    let ascii_chars = s.chars().filter(|c| c.is_ascii()).count();
    ascii_chars as f64 / total_chars as f64
}
