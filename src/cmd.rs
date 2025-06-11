pub use clap::{ArgAction, Parser};
use clap::{Args, Subcommand};
use eyre::{Result, eyre};

use crate::{
    Source,
    utils::{ProjectConfig, default_repos},
};

pub type Verbosity = u8;

#[derive(Parser, Debug)]
#[clap(author, version, about = "A CLI tool to benchmark Foundry projects.")]
pub struct Cli {
    /// Specifies the list of repository URLs to benchmark.
    /// Can be provided multiple times or as a comma-separated list (e.g., --repos url1,url2 or --repos url1 --repos url2).
    /// If not provided, a default list of projects will be used.
    #[clap(short, long, env = "BENCHMARK_REPOS", num_args = 1.., value_delimiter = ',', global = true)]
    pub repos: Option<Vec<String>>,

    /// Optional: Number of test runs for each project to average results. 10 by default.
    #[clap(
        long,
        default_value_t = 10,
        global = true,
        help = "Number of test runs per project to average the results"
    )]
    pub num_runs: usize,

    /// Verbosity level of the log messages.
    ///
    /// Pass multiple times to increase the verbosity (e.g. -v, -vv, -vvv).
    ///
    /// Depending on the context the verbosity levels have different meanings.
    ///
    /// For example, the verbosity levels of the EVM are:
    /// - 2 (-vv): Print logs for all tests.
    /// - 3 (-vvv): Print execution traces for failing tests.
    /// - 4 (-vvvv): Print execution traces for all tests, and setup traces for failing tests.
    /// - 5 (-vvvvv): Print execution and setup traces for all tests, including storage changes.
    #[arg(
        help_heading = "Display options",
        global = true,
        short,
        long,
        verbatim_doc_comment,
        action = ArgAction::Count
    )]
    pub verbosity: Verbosity,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Benchmark a diff between two Foundry versions built from specified branches.
    #[clap(name = "diff")]
    Diff(DiffConfig),
}

/// Struct for reference Foundry source choice (version or branch)
#[derive(Args, Debug)]
#[group(id = "reference_source_group", required = true, multiple = false)]
struct ReferenceSource {
    #[clap(
        long = "reference-version",
        visible_alias = "ref-version",
        value_name = "REF_VERSION",
        help = "Reference Foundry version (e.g., 'stable')"
    )]
    ref_version: Option<String>,

    #[clap(
        long = "reference-branch",
        visible_alias = "ref-branch",
        value_name = "REF_BRANCH",
        help = "Reference Foundry branch (e.g., 'master')"
    )]
    ref_branch: Option<String>,
}

/// Struct for comparison Foundry source choice (version or branch)
#[derive(Args, Debug)]
#[group(id = "comparison_source_group", required = true, multiple = false)]
struct ComparisonSource {
    #[clap(
        long = "comparison-version",
        visible_alias = "vs-version",
        value_name = "VS_VERSION",
        help = "Comparison Foundry version to test (e.g., 'v1.2.0-rc')"
    )]
    vs_version: Option<String>,

    #[clap(
        long = "comparison-branch",
        visible_alias = "vs-branch",
        value_name = "VS_BRANCH",
        help = "Comparison Foundry branch to test (e.g., 'my-perf-optimization')"
    )]
    vs_branch: Option<String>,
}

#[derive(Args, Debug)]
struct DiffConfig {
    #[clap(flatten)]
    reference_source: ReferenceSource,

    #[clap(flatten)]
    comparison_source: ComparisonSource,

    /// Optional: Git repository for building Foundry from source.
    /// Defaults to the official Foundry repository if not provided.
    #[clap(
        long,
        value_name = "FOUNDRY_REPOSITORY",
        default_value = "foundry-rs/foundry",
        help = "Git repository for building Foundry from source"
    )]
    foundry_repo: String,
}

impl Cli {
    /// Returns the list of projects to benchmark.
    ///
    /// * If specific repos are passed via the CLI, checks if they match any
    /// default configurations. Otherwise, creates a new standard config.
    /// * If no repos are passed, returns the full default list.
    pub fn get_repos(&self) -> Vec<ProjectConfig> {
        let default_repos = default_repos();

        if let Some(repo_names) = &self.repos {
            return repo_names
                .iter()
                .map(|repo_name| {
                    default_repos
                        .iter()
                        .find(|default| default.name == *repo_name)
                        .cloned()
                        .unwrap_or_else(|| ProjectConfig::new(repo_name.clone()))
                })
                .collect();
        }

        default_repos
    }

    pub fn get_cmd(&self) -> Result<Option<(&String, Source, Source)>> {
        if let Some(Commands::Diff(config)) = self.command.as_ref() {
            let baseline = match (
                &config.reference_source.ref_version,
                &config.reference_source.ref_branch,
            ) {
                (Some(version), None) => Source::Version(version),
                (None, Some(branch)) => Source::Branch(branch),
                _ => {
                    return Err(eyre!("(single) Foundry reference source is required"));
                }
            };

            let comparison = match (
                &config.comparison_source.vs_version,
                &config.comparison_source.vs_branch,
            ) {
                (Some(version), None) => Source::Version(version),
                (None, Some(branch)) => Source::Branch(branch),
                _ => {
                    return Err(eyre!("(single) Foundry comparison source is required"));
                }
            };

            return Ok(Some((&config.foundry_repo, baseline, comparison)));
        }

        Ok(None)
    }
}
