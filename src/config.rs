use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

use crate::utils::ProjectConfig;

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
            .wrap_err_with(|| format!("Failed to read config file: {}", path))?;
        let mut config: ConfigFile = toml::from_str(&contents)
            .wrap_err_with(|| format!("Failed to parse TOML config file: {}", path))?;

        // Expand environment variables in config
        config.expand_env_vars();
        Ok(config)
    }

    fn expand_env_vars(&mut self) {
        // Expand custom env vars
        if let Some(env_vars) = &mut self.custom.env_vars {
            for (_, value) in env_vars.iter_mut() {
                *value = expand_env_var(value);
            }
        }

        // Expand defaults env vars
        if let Some(env_vars) = &mut self.defaults.env_vars {
            for (_, value) in env_vars.iter_mut() {
                *value = expand_env_var(value);
            }
        }

        // Expand project env vars
        for project in &mut self.project {
            if let Some(env_vars) = &mut project.env_vars {
                for (_, value) in env_vars.iter_mut() {
                    *value = expand_env_var(value);
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
                let mut config = ProjectConfig::new(&proj.name);

                // Apply dependencies
                if let Some(deps) = &proj.dependencies {
                    config = config.with_deps(deps.clone());
                }

                // Apply remappings
                if let Some(remappings) = &proj.remappings {
                    config = config.with_remappings(remappings.clone());
                }

                // Apply env vars (merge with global)
                let mut env_vars = global_env_vars.clone();
                if let Some(proj_env_vars) = &proj.env_vars {
                    env_vars.extend(proj_env_vars.clone());
                }
                if !env_vars.is_empty() {
                    let (names, values): (Vec<_>, Vec<_>) = env_vars.into_iter().unzip();
                    config = config.with_env_vars(names, values);
                }

                config
            })
            .collect()
    }
}

/// Expands environment variables in format ${VAR_NAME} or $VAR_NAME
fn expand_env_var(value: &str) -> String {
    let mut result = value.to_string();

    // Handle ${VAR_NAME} format
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start + 2..].find('}') {
            let var_name = &result[start + 2..start + 2 + end];
            let var_value = env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name));
            result.replace_range(start..start + 3 + end, &var_value);
        } else {
            break;
        }
    }

    // Handle $VAR_NAME format (simple word boundaries)
    let mut i = 0;
    while i < result.len() {
        if result.chars().nth(i) == Some('$') && i + 1 < result.len() {
            let rest = &result[i + 1..];
            let var_end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if var_end > 0 {
                let var_name = &rest[..var_end];
                if let Ok(var_value) = env::var(var_name) {
                    result.replace_range(i..i + 1 + var_end, &var_value);
                    i += var_value.len();
                    continue;
                }
            }
        }
        i += 1;
    }

    result
}
