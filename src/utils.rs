use dotenvy::dotenv;
use std::{collections::HashMap, env};

pub const GITHUB_URL: &str = "https://github.com";

/// Represents the configuration for a benchmarkable project.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub dependencies: Option<Vec<String>>,
    pub remappings: Option<Vec<String>>,
    pub env_vars: Option<HashMap<String, String>>,
}

impl ProjectConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependencies: None,
            remappings: None,
            env_vars: None,
        }
    }

    pub fn with_deps(mut self, deps: Vec<impl Into<String>>) -> Self {
        self.dependencies = Some(deps.into_iter().map(|d| d.into()).collect());
        self
    }

    pub fn with_remappings(mut self, remappings: Vec<impl Into<String>>) -> Self {
        self.remappings = Some(remappings.into_iter().map(|r| r.into()).collect());
        self
    }

    /// Sets environment variables from two separate vectors of names and values.
    ///
    /// # Panics
    /// This method will panic if the number of names does not match the number of values.
    pub fn with_env_vars(
        mut self,
        names: Vec<impl Into<String>>,
        values: Vec<impl Into<String>>,
    ) -> Self {
        assert_eq!(
            names.len(),
            values.len(),
            "The number of environment variable names must match the number of values."
        );

        let env_vars_map: HashMap<String, String> = names
            .into_iter()
            .map(|n| n.into())
            .zip(values.into_iter().map(|v| v.into()))
            .collect();

        self.env_vars = Some(env_vars_map);
        self
    }

    pub fn url(&self) -> String {
        format!("{GITHUB_URL}/{name}", name = self.name)
    }

    pub fn label(&self) -> String {
        format!("[{name}]", name = self.name)
    }
}

/// Returns the default list of repositories to benchmark.
pub fn default_repos() -> Vec<ProjectConfig> {
    dotenv().ok();

    // Load required env vars
    const MAINNET_RPC_URL: &str = "MAINNET_RPC_URL";
    let mainnet_rpc_url = env::var(MAINNET_RPC_URL).expect("env var 'MAINNET_RPC_URL' must be set");

    // Initialize default project repos
    vec![
        ProjectConfig::new("uniswap/v4-core"),
        ProjectConfig::new("sparkdotfi/spark-psm"),
        ProjectConfig::new("morpho-org/morpho-blue"),
        ProjectConfig::new("sablier-labs/lockup")
            .with_deps(vec![
                "install",
                "foundry-rs/forge-std",
                "OpenZeppelin/openzeppelin-contracts@v5.0.2",
                "PaulRBerg/prb-math@v4.1.0",
                "vectorized/solady",
                "evmcheb/solarray",
            ])
            .with_remappings(vec![
                "forge-std/src/=lib/forge-std/src/",
                "solarray/src/=lib/solarray/src/",
                "solady/src/=lib/solady/src/",
                "@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/",
                "@prb/math/=lib/prb-math/",
                "node_modules/=lib/",
            ])
            .with_env_vars(vec![MAINNET_RPC_URL], vec![&mainnet_rpc_url]),
        ProjectConfig::new("vectorized/solady"),
        // ProjectConfig::new("euler-xyz/ethereum-vault-connector"), // TODO: figure out why testing fails,
    ]
}
