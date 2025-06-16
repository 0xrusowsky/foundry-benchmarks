use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GITHUB_URL: &str = "https://github.com";

/// Represents the configuration for a benchmarkable project.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub config: JsonProjectConfig,
}

/// JSON configuration for a project (excludes `name`)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct JsonProjectConfig {
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
            config: JsonProjectConfig::default(),
        }
    }

    #[cfg(test)]
    pub fn with_config(mut self, config: JsonProjectConfig) -> Self {
        self.config = config;
        self
    }

    #[cfg(test)]
    pub fn with_deps(mut self, deps: Vec<impl Into<String>>) -> Self {
        self.config.dependencies = Some(deps.into_iter().map(|d| d.into()).collect());
        self
    }

    #[cfg(test)]
    pub fn with_remappings(mut self, remappings: Vec<impl Into<String>>) -> Self {
        self.config.remappings = Some(remappings.into_iter().map(|r| r.into()).collect());
        self
    }

    /// Sets environment variables from two separate vectors of names and values.
    ///
    /// # Panics
    /// This method will panic if the number of names does not match the number of values.
    #[cfg(test)]
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

        self.config.env_vars = Some(env_vars_map);
        self
    }

    pub fn url(&self) -> String {
        format!("{GITHUB_URL}/{name}", name = self.name)
    }

    pub fn label(&self) -> String {
        format!("[{name}]", name = self.name)
    }

    // Convenience getters that delegate to config
    pub fn dependencies(&self) -> Option<&Vec<String>> {
        self.config.dependencies.as_ref()
    }

    pub fn remappings(&self) -> Option<&Vec<String>> {
        self.config.remappings.as_ref()
    }

    pub fn env_vars(&self) -> Option<&HashMap<String, String>> {
        self.config.env_vars.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_new() {
        let config = ProjectConfig::new("test/repo");
        assert_eq!(config.name, "test/repo");
        assert!(config.dependencies().is_none());
        assert!(config.remappings().is_none());
        assert!(config.env_vars().is_none());
    }

    #[test]
    fn test_project_config_with_deps() {
        let config = ProjectConfig::new("test/repo").with_deps(vec!["dep1", "dep2"]);

        assert_eq!(config.dependencies().unwrap(), &vec!["dep1", "dep2"]);
    }

    #[test]
    fn test_project_config_with_remappings() {
        let config =
            ProjectConfig::new("test/repo").with_remappings(vec!["@lib/=lib/", "@std/=lib/std/"]);

        assert_eq!(
            config.remappings().unwrap(),
            &vec!["@lib/=lib/", "@std/=lib/std/"]
        );
    }

    #[test]
    fn test_project_config_with_env_vars() {
        let config = ProjectConfig::new("test/repo")
            .with_env_vars(vec!["VAR1", "VAR2"], vec!["value1", "value2"]);

        let env_vars = config.env_vars().unwrap();
        assert_eq!(env_vars.get("VAR1"), Some(&"value1".to_string()));
        assert_eq!(env_vars.get("VAR2"), Some(&"value2".to_string()));
    }

    #[test]
    #[should_panic(
        expected = "The number of environment variable names must match the number of values"
    )]
    fn test_project_config_with_env_vars_mismatch() {
        ProjectConfig::new("test/repo").with_env_vars(
            vec!["VAR1", "VAR2"],
            vec!["value1"], // Missing value2
        );
    }

    #[test]
    fn test_project_config_builder_chain() {
        let config = ProjectConfig::new("test/repo")
            .with_deps(vec!["forge-std", "openzeppelin"])
            .with_remappings(vec!["@std/=lib/forge-std/"])
            .with_env_vars(vec!["RPC_URL"], vec!["https://test.rpc"]);

        assert_eq!(config.name, "test/repo");
        assert_eq!(config.dependencies().unwrap().len(), 2);
        assert_eq!(config.remappings().unwrap().len(), 1);
        assert_eq!(config.env_vars().unwrap().len(), 1);
    }

    #[test]
    fn test_project_config_url() {
        let config = ProjectConfig::new("owner/repo");
        assert_eq!(config.url(), "https://github.com/owner/repo");
    }

    #[test]
    fn test_project_config_label() {
        let config = ProjectConfig::new("owner/repo");
        assert_eq!(config.label(), "[owner/repo]");
    }

    #[test]
    fn test_json_project_config_serde() {
        let json_config = JsonProjectConfig {
            dependencies: Some(vec!["dep1".to_string()]),
            remappings: Some(vec!["@lib/=lib/".to_string()]),
            env_vars: Some(HashMap::from([("KEY".to_string(), "value".to_string())])),
        };

        let json = serde_json::to_string(&json_config).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"dependencies\""));
        assert!(json.contains("\"remappings\""));
        assert!(json.contains("\"env_vars\""));

        // Deserialize back
        let deserialized: JsonProjectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.dependencies.as_ref().unwrap(), &vec!["dep1"]);
        assert_eq!(
            deserialized.remappings.as_ref().unwrap(),
            &vec!["@lib/=lib/"]
        );
        assert_eq!(
            deserialized.env_vars.as_ref().unwrap().get("KEY"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_json_project_config_partial() {
        // Test that partial JSON works (as used in --repo flag)
        let json = r#"{"dependencies":["dep1"]}"#;

        let config: JsonProjectConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.dependencies.as_ref().unwrap(), &vec!["dep1"]);
        assert!(config.remappings.is_none());
        assert!(config.env_vars.is_none());
    }

    #[test]
    fn test_project_config_with_config() {
        let json_config = JsonProjectConfig {
            dependencies: Some(vec!["dep1".to_string()]),
            remappings: Some(vec!["@lib/=lib/".to_string()]),
            env_vars: Some(HashMap::from([("KEY".to_string(), "value".to_string())])),
        };

        let config = ProjectConfig::new("test/repo").with_config(json_config);

        assert_eq!(config.name, "test/repo");
        assert_eq!(config.dependencies().unwrap(), &vec!["dep1"]);
        assert_eq!(config.remappings().unwrap(), &vec!["@lib/=lib/"]);
        assert_eq!(
            config.env_vars().unwrap().get("KEY"),
            Some(&"value".to_string())
        );
    }
}
