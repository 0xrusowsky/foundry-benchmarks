use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::utils::{JsonProjectConfig, ProjectConfig};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub custom: CustomConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub project: Vec<ProjectConfigToml>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CustomConfig {
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct DefaultsConfig {
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfigToml {
    pub name: String,
    pub dependencies: Option<Vec<String>>,
    pub remappings: Option<Vec<String>>,
    pub env_vars: Option<HashMap<String, String>>,
}

impl ConfigFile {
    pub fn load(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read config file: {path}"))?;
        let mut config: ConfigFile = toml::from_str(&contents)
            .wrap_err_with(|| format!("Failed to parse TOML config file: {path}"))?;

        // Expand environment variables in config
        config.expand_env_vars();
        Ok(config)
    }

    fn expand_env_vars(&mut self) {
        // Expand custom env vars
        if let Some(env_vars) = &mut self.custom.env_vars {
            for (_, value) in env_vars.iter_mut() {
                // shellexpand::env returns the original string if expansion fails
                *value = shellexpand::env(value)
                    .unwrap_or_else(|_| value.as_str().into())
                    .into_owned();
            }
        }

        // Expand defaults env vars
        if let Some(env_vars) = &mut self.defaults.env_vars {
            for (_, value) in env_vars.iter_mut() {
                *value = shellexpand::env(value)
                    .unwrap_or_else(|_| value.as_str().into())
                    .into_owned();
            }
        }

        // Expand project env vars
        for project in &mut self.project {
            if let Some(env_vars) = &mut project.env_vars {
                for (_, value) in env_vars.iter_mut() {
                    *value = shellexpand::env(value)
                        .unwrap_or_else(|_| value.as_str().into())
                        .into_owned();
                }
            }
        }
    }

    /// Check if the custom section has any configuration
    pub fn has_custom_config(&self) -> bool {
        self.custom.env_vars.is_some()
    }

    pub fn to_project_configs(&self, use_custom: bool) -> Vec<ProjectConfig> {
        let global_env_vars = if use_custom && self.custom.env_vars.is_some() {
            self.custom.env_vars.clone()
        } else {
            self.defaults.env_vars.clone()
        }
        .unwrap_or_default();

        self.project
            .iter()
            .map(|proj| {
                // Apply env vars (merge with global)
                let mut env_vars = global_env_vars.clone();
                if let Some(proj_env_vars) = &proj.env_vars {
                    env_vars.extend(proj_env_vars.clone());
                }

                let json_config = JsonProjectConfig {
                    dependencies: proj.dependencies.clone(),
                    remappings: proj.remappings.clone(),
                    env_vars: if env_vars.is_empty() {
                        None
                    } else {
                        Some(env_vars)
                    },
                };

                ProjectConfig {
                    name: proj.name.clone(),
                    config: json_config,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_shellexpand_behavior() {
        // Test basic shellexpand behavior
        let expanded = shellexpand::env("simple string").unwrap();
        assert_eq!(expanded.as_ref(), "simple string");

        // Test with missing env var - shellexpand returns error, but we handle it
        let result = shellexpand::env("$MISSING_VAR_UNIQUE_12345");
        assert!(result.is_err());

        // Test with braces
        let result = shellexpand::env("${MISSING_VAR_UNIQUE_12345}");
        assert!(result.is_err());

        // Test fallback behavior
        let value = "$MISSING_VAR_UNIQUE_12345";
        let expanded = shellexpand::env(value).unwrap_or_else(|_| value.into());
        assert_eq!(expanded.as_ref(), "$MISSING_VAR_UNIQUE_12345");
    }

    #[test]
    fn test_config_file_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.toml");

        let config_content = r#"
[custom]
env_vars = { CUSTOM_RPC = "https://custom.rpc" }

[defaults]
env_vars = { DEFAULT_RPC = "https://default.rpc" }

[[project]]
name = "test/project1"
dependencies = ["forge-std"]
remappings = ["@std/=lib/forge-std/"]
env_vars = { PROJECT_VAR = "project_value" }

[[project]]
name = "test/project2"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = ConfigFile::load(config_path.to_str().unwrap()).unwrap();

        assert!(config.has_custom_config());
        assert_eq!(
            config.custom.env_vars.as_ref().unwrap().get("CUSTOM_RPC"),
            Some(&"https://custom.rpc".to_string())
        );
        assert_eq!(
            config
                .defaults
                .env_vars
                .as_ref()
                .unwrap()
                .get("DEFAULT_RPC"),
            Some(&"https://default.rpc".to_string())
        );
        assert_eq!(config.project.len(), 2);
        assert_eq!(config.project[0].name, "test/project1");
        assert_eq!(
            config.project[0].dependencies.as_ref().unwrap(),
            &vec!["forge-std"]
        );
    }

    #[test]
    fn test_config_file_structure() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.toml");

        // Test config file with literal values (no env var expansion needed)
        let config_content = r#"
[custom]
env_vars = { RPC_URL = "https://custom.rpc" }

[defaults]
env_vars = { DEFAULT_RPC = "https://default.rpc/endpoint" }

[[project]]
name = "test/project"
env_vars = { PROJECT_RPC = "https://project.rpc/v1" }
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = ConfigFile::load(config_path.to_str().unwrap()).unwrap();

        assert_eq!(
            config.custom.env_vars.as_ref().unwrap().get("RPC_URL"),
            Some(&"https://custom.rpc".to_string())
        );
        assert_eq!(
            config
                .defaults
                .env_vars
                .as_ref()
                .unwrap()
                .get("DEFAULT_RPC"),
            Some(&"https://default.rpc/endpoint".to_string())
        );
        assert_eq!(
            config.project[0]
                .env_vars
                .as_ref()
                .unwrap()
                .get("PROJECT_RPC"),
            Some(&"https://project.rpc/v1".to_string())
        );
    }

    #[test]
    fn test_config_file_with_env_var_syntax() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.toml");

        // Test that env var syntax is preserved when vars don't exist
        let config_content = r#"
[custom]
env_vars = { RPC_URL = "${NONEXISTENT_RPC_URL}" }

[defaults]
env_vars = { DEFAULT_RPC = "$NONEXISTENT_RPC_URL/endpoint" }

[[project]]
name = "test/project"
env_vars = { PROJECT_RPC = "${NONEXISTENT_RPC_URL}/v1" }
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = ConfigFile::load(config_path.to_str().unwrap()).unwrap();

        // Verify that non-existent env vars are preserved as-is
        assert_eq!(
            config.custom.env_vars.as_ref().unwrap().get("RPC_URL"),
            Some(&"${NONEXISTENT_RPC_URL}".to_string())
        );
        assert_eq!(
            config
                .defaults
                .env_vars
                .as_ref()
                .unwrap()
                .get("DEFAULT_RPC"),
            Some(&"$NONEXISTENT_RPC_URL/endpoint".to_string())
        );
        assert_eq!(
            config.project[0]
                .env_vars
                .as_ref()
                .unwrap()
                .get("PROJECT_RPC"),
            Some(&"${NONEXISTENT_RPC_URL}/v1".to_string())
        );
    }

    #[test]
    fn test_to_project_configs_use_custom() {
        let mut config = ConfigFile::default();

        config.custom.env_vars = Some(HashMap::from([(
            "CUSTOM_VAR".to_string(),
            "custom_value".to_string(),
        )]));

        config.defaults.env_vars = Some(HashMap::from([(
            "DEFAULT_VAR".to_string(),
            "default_value".to_string(),
        )]));

        config.project.push(ProjectConfigToml {
            name: "test/project".to_string(),
            dependencies: Some(vec!["dep1".to_string()]),
            remappings: None,
            env_vars: Some(HashMap::from([(
                "PROJECT_VAR".to_string(),
                "project_value".to_string(),
            )])),
        });

        let projects = config.to_project_configs(true);

        assert_eq!(projects.len(), 1);
        let project = &projects[0];

        assert_eq!(project.name, "test/project");
        assert_eq!(project.dependencies().unwrap(), &vec!["dep1"]);

        let env_vars = project.env_vars().unwrap();
        assert_eq!(
            env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            env_vars.get("PROJECT_VAR"),
            Some(&"project_value".to_string())
        );
        assert_eq!(env_vars.get("DEFAULT_VAR"), None);
    }

    #[test]
    fn test_to_project_configs_use_defaults() {
        let mut config = ConfigFile::default();

        config.custom.env_vars = Some(HashMap::from([(
            "CUSTOM_VAR".to_string(),
            "custom_value".to_string(),
        )]));

        config.defaults.env_vars = Some(HashMap::from([(
            "DEFAULT_VAR".to_string(),
            "default_value".to_string(),
        )]));

        config.project.push(ProjectConfigToml {
            name: "test/project".to_string(),
            dependencies: None,
            remappings: Some(vec!["@std/=lib/".to_string()]),
            env_vars: None,
        });

        let projects = config.to_project_configs(false);

        assert_eq!(projects.len(), 1);
        let project = &projects[0];

        assert_eq!(project.remappings().unwrap(), &vec!["@std/=lib/"]);

        let env_vars = project.env_vars().unwrap();
        assert_eq!(
            env_vars.get("DEFAULT_VAR"),
            Some(&"default_value".to_string())
        );
        assert_eq!(env_vars.get("CUSTOM_VAR"), None);
    }

    #[test]
    fn test_has_custom_config() {
        let mut config = ConfigFile::default();
        assert!(!config.has_custom_config());

        config.custom.env_vars = Some(HashMap::new());
        assert!(config.has_custom_config());
    }
}
