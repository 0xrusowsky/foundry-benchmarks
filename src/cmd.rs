pub use clap::{ArgAction, Parser};
use clap::{Args, Subcommand};
use eyre::{Result, eyre};
use std::collections::HashMap;

use crate::{Source, config::ConfigFile, utils::{ProjectConfig, JsonProjectConfig}};

pub type Verbosity = u8;

#[derive(Parser, Debug)]
#[clap(author, version, about = "A CLI tool to benchmark Foundry projects.")]
pub struct Cli {
    /// Specifies the list of repository URLs to benchmark.
    /// Can be provided multiple times or as a comma-separated list (e.g., --repos url1,url2 or --repos url1 --repos url2).
    /// If not provided, a default list of projects will be used.
    #[clap(short, long, env = "BENCHMARK_REPOS", num_args = 1.., value_delimiter = ',', global = true)]
    pub repos: Option<Vec<String>>,

    /// Path to TOML configuration file for custom project settings
    #[clap(short = 'c', long, global = true)]
    pub config: Option<String>,

    /// Per-project configuration in format "repo:json" or just "repo"
    /// Example: --repo 'owner/repo:{"dependencies":["forge-std"],"remappings":["@std/=lib/forge-std/"]}'
    #[clap(long, conflicts_with = "repos", global = true)]
    pub repo: Option<Vec<String>>,

    /// Dependencies to install (comma-separated, applies to all repos except those using --repo)
    #[clap(long, value_delimiter = ',', global = true)]
    pub deps: Option<Vec<String>>,

    /// Remappings for the project (comma-separated, applies to all repos except those using --repo)
    #[clap(long, value_delimiter = ',', global = true)]
    pub remappings: Option<Vec<String>>,

    /// Environment variables (comma-separated KEY=VALUE pairs, applies to all repos except those using --repo)
    #[clap(long, value_delimiter = ',', global = true)]
    pub env: Option<Vec<String>>,

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
    /// Priority order:
    /// 1. --repo flag with per-project JSON configs
    /// 2. --repos flag with global config flags
    /// 3. TOML config file (custom)
    /// 4. TOML config file (default)
    pub fn get_repos(&self) -> Result<Vec<ProjectConfig>> {
        if let Some(config) = &self.repo {
            return self.parse_project_config(config);
        }

        let mut configs: HashMap<String, ProjectConfig> = HashMap::new();

        let config_path = self.config.as_deref().unwrap_or("benchmarks.toml");
        let file_config = ConfigFile::load(config_path)?;

        let has_cli_overrides = self.repos.is_some()
            || self.deps.is_some()
            || self.remappings.is_some()
            || self.env.is_some();

        let use_custom = file_config.has_custom_config() && !has_cli_overrides;

        for project_config in file_config.to_project_configs(use_custom) {
            configs.insert(project_config.name.clone(), project_config);
        }

        // Handle --repos flag with global overrides
        if let Some(repo_names) = &self.repos {
            let mut selected_configs = Vec::new();

            for repo_name in repo_names {
                let mut config = configs
                    .get(repo_name)
                    .cloned()
                    .unwrap_or_else(|| ProjectConfig::new(repo_name));

                // Apply global CLI overrides
                if let Some(deps) = &self.deps {
                    config.config.dependencies = Some(deps.clone());
                }
                if let Some(remappings) = &self.remappings {
                    config.config.remappings = Some(remappings.clone());
                }
                if let Some(env_pairs) = &self.env {
                    config.config.env_vars = Some(parse_env_pairs(env_pairs)?);
                }

                selected_configs.push(config);
            }

            return Ok(selected_configs);
        }

        Ok(configs.into_values().collect())
    }

    /// Parse project specifications in format "repo" or "repo:json"
    fn parse_project_config(&self, specs: &[String]) -> Result<Vec<ProjectConfig>> {
        let mut file_configs: HashMap<String, ProjectConfig> = HashMap::new();

        let config_path = self.config.as_deref().unwrap_or("benchmarks.toml");
        let file_config = ConfigFile::load(config_path)?;

        // For --repo flag, we always use defaults since it's an explicit CLI override
        let use_custom = false;

        for project_config in file_config.to_project_configs(use_custom) {
            file_configs.insert(project_config.name.clone(), project_config);
        }

        let mut result = Vec::new();

        for spec in specs {
            let config = if let Some(colon_pos) = spec.find(':') {
                let repo_name = &spec[..colon_pos];
                let json_str = &spec[colon_pos + 1..];

                // Start with existing config or create new
                let base_config = file_configs
                    .get(repo_name)
                    .cloned()
                    .unwrap_or_else(|| ProjectConfig::new(repo_name));

                // Parse JSON config
                let json_config: JsonProjectConfig = serde_json::from_str(json_str)
                    .map_err(|e| eyre!("Failed to parse JSON config for '{}': {}", repo_name, e))?;

                // Merge configs: JSON overrides base
                let merged_config = JsonProjectConfig {
                    dependencies: json_config.dependencies.or(base_config.config.dependencies),
                    remappings: json_config.remappings.or(base_config.config.remappings),
                    env_vars: json_config.env_vars.or(base_config.config.env_vars),
                };

                ProjectConfig {
                    name: base_config.name,
                    config: merged_config,
                }
            } else {
                file_configs
                    .get(spec)
                    .cloned()
                    .unwrap_or_else(|| ProjectConfig::new(spec))
            };

            result.push(config);
        }

        Ok(result)
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

/// Parse environment variable pairs
fn parse_env_pairs(pairs: &[String]) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    for pair in pairs {
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(eyre!(
                "Invalid environment variable format: '{}'. Expected KEY=VALUE",
                pair
            ));
        }
        env_vars.insert(parts[0].to_string(), parts[1].to_string());
    }

    Ok(env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_env_pairs() {
        let pairs = vec!["KEY1=value1".to_string(), "KEY2=value2".to_string()];

        let result = parse_env_pairs(&pairs).unwrap();
        assert_eq!(result.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(result.get("KEY2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_parse_env_pairs_with_equals_in_value() {
        let pairs = vec!["KEY=value=with=equals".to_string()];

        let result = parse_env_pairs(&pairs).unwrap();
        assert_eq!(result.get("KEY"), Some(&"value=with=equals".to_string()));
    }

    #[test]
    fn test_parse_env_pairs_invalid_format() {
        let pairs = vec!["INVALID_FORMAT".to_string()];

        let result = parse_env_pairs(&pairs);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_with_repos_flag() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");

        // Create empty config to avoid loading default benchmarks.toml
        let config_content = r#"
[defaults]
[[project]]
name = "default/project"
"#;
        fs::write(&config_path, config_content).unwrap();

        let cli = Cli {
            repos: Some(vec!["owner/repo1".to_string(), "owner/repo2".to_string()]),
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "owner/repo1");
        assert_eq!(repos[1].name, "owner/repo2");
    }

    #[test]
    fn test_cli_with_global_flags() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");
        fs::write(&config_path, "[defaults]\n").unwrap();

        let cli = Cli {
            repos: Some(vec!["test/repo".to_string()]),
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: Some(vec!["forge-std".to_string(), "openzeppelin".to_string()]),
            remappings: Some(vec!["@std/=lib/".to_string()]),
            env: Some(vec!["RPC_URL=https://test.rpc".to_string()]),
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);

        let repo = &repos[0];
        assert_eq!(repo.name, "test/repo");
        assert_eq!(
            repo.dependencies().unwrap(),
            &vec!["forge-std", "openzeppelin"]
        );
        assert_eq!(repo.remappings().unwrap(), &vec!["@std/=lib/"]);
        assert_eq!(
            repo.env_vars().unwrap().get("RPC_URL"),
            Some(&"https://test.rpc".to_string())
        );
    }

    #[test]
    fn test_cli_with_repo_json_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");
        fs::write(&config_path, "[defaults]\n").unwrap();

        let cli = Cli {
            repos: None,
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: Some(vec![
                r#"test/repo1:{"dependencies":["dep1","dep2"]}"#.to_string(),
                r#"test/repo2:{"remappings":["@lib/=lib/"],"env_vars":{"KEY":"value"}}"#
                    .to_string(),
                "test/repo3".to_string(),
            ]),
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 3);

        assert_eq!(repos[0].name, "test/repo1");
        assert_eq!(
            repos[0].dependencies().unwrap(),
            &vec!["dep1", "dep2"]
        );

        assert_eq!(repos[1].name, "test/repo2");
        assert_eq!(repos[1].remappings().unwrap(), &vec!["@lib/=lib/"]);
        assert_eq!(
            repos[1].env_vars().unwrap().get("KEY"),
            Some(&"value".to_string())
        );

        assert_eq!(repos[2].name, "test/repo3");
        assert!(repos[2].dependencies().is_none());
    }

    #[test]
    fn test_cli_with_config_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.toml");

        let config_content = r#"
[defaults]
env_vars = { DEFAULT_VAR = "default_value" }

[[project]]
name = "config/repo1"
dependencies = ["forge-std"]
"#;

        fs::write(&config_path, config_content).unwrap();

        let cli = Cli {
            repos: None,
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "config/repo1");
        assert_eq!(repos[0].dependencies().unwrap(), &vec!["forge-std"]);
        assert_eq!(
            repos[0].env_vars().unwrap().get("DEFAULT_VAR"),
            Some(&"default_value".to_string())
        );
    }

    #[test]
    fn test_cli_priority_repo_over_repos() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");
        fs::write(&config_path, "[defaults]\n").unwrap();

        let cli = Cli {
            repos: Some(vec!["should/be/ignored".to_string()]),
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: Some(vec!["actual/repo".to_string()]),
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "actual/repo");
    }

    #[test]
    fn test_cli_with_custom_config_no_overrides() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.toml");

        let config_content = r#"
[custom]
env_vars = { CUSTOM_VAR = "custom_value" }

[defaults]
env_vars = { DEFAULT_VAR = "default_value" }

[[project]]
name = "test/repo"
"#;

        fs::write(&config_path, config_content).unwrap();

        let cli = Cli {
            repos: None,
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);
        // Should use custom section when no CLI overrides
        assert_eq!(
            repos[0].env_vars().unwrap().get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(repos[0].env_vars().unwrap().get("DEFAULT_VAR"), None);
    }

    #[test]
    fn test_cli_with_custom_config_with_overrides() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.toml");

        let config_content = r#"
[custom]
env_vars = { CUSTOM_VAR = "custom_value" }

[defaults]
env_vars = { DEFAULT_VAR = "default_value" }

[[project]]
name = "test/repo"
"#;

        fs::write(&config_path, config_content).unwrap();

        let cli = Cli {
            repos: Some(vec!["test/repo".to_string()]),
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: Some(vec!["new-dep".to_string()]),
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);
        // Should use defaults section when CLI overrides present
        assert_eq!(
            repos[0].env_vars().unwrap().get("DEFAULT_VAR"),
            Some(&"default_value".to_string())
        );
        assert_eq!(repos[0].env_vars().unwrap().get("CUSTOM_VAR"), None);
        assert_eq!(repos[0].dependencies().unwrap(), &vec!["new-dep"]);
    }

    #[test]
    fn test_get_cmd_diff_config() {
        let cli = Cli {
            repos: None,
            config: None,
            repo: None,
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: Some(Commands::Diff(DiffConfig {
                reference_source: ReferenceSource {
                    ref_version: None,
                    ref_branch: Some("master".to_string()),
                },
                comparison_source: ComparisonSource {
                    vs_version: Some("v1.0.0".to_string()),
                    vs_branch: None,
                },
                foundry_repo: "foundry-rs/foundry".to_string(),
            })),
        };

        let result = cli.get_cmd().unwrap();
        assert!(result.is_some());

        let (repo, ref_source, vs_source) = result.unwrap();
        assert_eq!(repo, "foundry-rs/foundry");

        match ref_source {
            Source::Branch(b) => assert_eq!(b, "master"),
            _ => panic!("Expected branch source"),
        }

        match vs_source {
            Source::Version(v) => assert_eq!(v, "v1.0.0"),
            _ => panic!("Expected version source"),
        }
    }

    #[test]
    fn test_repo_json_merge_with_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.toml");

        // Create config with base settings
        let config_content = r#"
[defaults]
env_vars = { BASE_VAR = "base_value" }

[[project]]
name = "test/repo1"
dependencies = ["base-dep"]
env_vars = { CONFIG_VAR = "config_value" }
"#;

        fs::write(&config_path, config_content).unwrap();

        let cli = Cli {
            repos: None,
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: Some(vec![
                r#"test/repo1:{"dependencies":["override-dep"],"env_vars":{"JSON_VAR":"json_value"}}"#.to_string(),
            ]),
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 1);

        let repo = &repos[0];
        // JSON dependencies override config
        assert_eq!(repo.dependencies().unwrap(), &vec!["override-dep"]);
        // JSON env_vars override config env_vars
        assert_eq!(
            repo.env_vars().unwrap().get("JSON_VAR"),
            Some(&"json_value".to_string())
        );
        // Config env_vars that weren't in JSON are lost (JSON overrides completely)
        assert_eq!(repo.env_vars().unwrap().get("CONFIG_VAR"), None);
        assert_eq!(repo.env_vars().unwrap().get("BASE_VAR"), None);
    }

    #[test]
    fn test_empty_repo_list() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");
        fs::write(&config_path, "[defaults]\n").unwrap();

        let cli = Cli {
            repos: Some(vec![]),
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: None,
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let repos = cli.get_repos().unwrap();
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn test_invalid_json_in_repo_flag() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("empty.toml");
        fs::write(&config_path, "[defaults]\n").unwrap();

        let cli = Cli {
            repos: None,
            config: Some(config_path.to_str().unwrap().to_string()),
            repo: Some(vec![r#"test/repo:invalid-json"#.to_string()]),
            deps: None,
            remappings: None,
            env: None,
            num_runs: 10,
            verbosity: 0,
            command: None,
        };

        let result = cli.get_repos();
        assert!(result.is_err());
    }
}
