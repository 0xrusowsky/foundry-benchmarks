use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GITHUB_URL: &str = "https://github.com";

/// Represents the configuration for a benchmarkable project.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remappings: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
