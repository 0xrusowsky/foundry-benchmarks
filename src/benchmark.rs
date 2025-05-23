use eyre::{Context, Result};
use rayon::prelude::*;
use std::path::PathBuf;
use std::{io::Write, process::Command, time::Instant};
use tempfile::TempDir;
use yansi::Paint;

use crate::cmd::Verbosity;
use crate::ui;
use crate::utils::GITHUB_URL;

/// Foundry source. Either a tagged version, or a branch.
#[derive(Debug, Clone)]
pub enum Source<'url> {
    Version(&'url String),
    Branch(&'url String),
}

impl<'url> Source<'url> {
    pub fn short(&self) -> &'static str {
        match self {
            Self::Version(_) => "-v",
            Self::Branch(_) => "-b",
        }
    }

    pub fn ty(&self) -> &'static str {
        match self {
            Self::Branch(_) => "branch",
            Self::Version(_) => "version",
        }
    }

    pub fn name(&self) -> &'url str {
        match self {
            Self::Branch(b) => b,
            Self::Version(v) => v,
        }
    }

    pub fn github_url(&self, foundry_repo: &str) -> String {
        match self {
            Self::Branch(b) => format!("{GITHUB_URL}/{foundry_repo}/tree/{b}"),
            Self::Version(v) => format!("{GITHUB_URL}/{foundry_repo}/releases/tag/{v}"),
        }
    }
}

/// Basic information of a benchmarked project
pub struct ProjectDetails<'url> {
    pub name: String,
    pub url: &'url str,
    pub repo_label: String,
}

impl<'url> ProjectDetails<'url> {
    pub fn new(url: &'url str) -> std::result::Result<Self, String> {
        let chunks = url.rsplit('/').collect::<Vec<&str>>();
        let repo_name = if chunks.len() >= 2 {
            format!("{}/{}", chunks[1], chunks[0]).to_lowercase()
        } else {
            return Err(format!(
                "Invalid URL (could not extract repo name): {}",
                url
            ));
        };
        let repo_label = format!("[{}]", repo_name);
        let repo_label = Paint::cyan(&repo_label).bold().to_string();

        Ok(ProjectDetails {
            name: repo_name,
            url,
            repo_label,
        })
    }
}

/// State of a project after it has been successfully cloned.
/// The `temp_dir` field owns the temporary directory, ensuring cleanup on drop.
pub struct Ready<'url> {
    pub details: ProjectDetails<'url>,
    pub path: PathBuf,
    pub _temp_dir: TempDir,
}

/// State of a project after it has been successfully built.
pub struct Built<'url> {
    pub state: Ready<'url>,
    pub build_time: f64,
}

/// Final state of a project after successful testing.
pub struct Tested<'url> {
    pub name: String,
    pub url: &'url str,
    pub build_time: f64,
    pub avg_test_time: f64,
    pub runs: usize,
}

impl<'url> Tested<'url> {
    fn new(built_state: Built<'url>, tests_times: Vec<f64>, runs: usize) -> Self {
        Tested {
            name: built_state.state.details.name.clone(),
            url: built_state.state.details.url,
            build_time: built_state.build_time,
            avg_test_time: if runs > 0 {
                tests_times.iter().sum::<f64>() / runs as f64
            } else {
                0.0
            },
            runs,
        }
    }
}

/// Helper struct to aggregate all the requires data to compute benchmark diffs.
pub struct Benchmarks<'url> {
    pub foundry_repo: &'url str,
    pub verbosity: String,
    pub ref_source: Source<'url>,
    pub ref_tests: Vec<Tested<'url>>,
    pub vs_source: Source<'url>,
    pub vs_tests: Vec<Tested<'url>>,
}

/// Represents the state of a project during the benchmark pipeline.
pub enum ProjectState<'url> {
    Cloned(Ready<'url>),
    Built(Built<'url>),
    Tested(Tested<'url>),
    Failed {
        name: String,
        stage: &'static str,
        error: String,
    },
}

/// Attempts to clone a project.
pub fn try_clone_project<'url>(details: ProjectDetails<'url>) -> ProjectState<'url> {
    let temp_dir = match TempDir::new() {
        Ok(td) => td,
        Err(e) => {
            let error_msg = format!(
                "Failed to create temp directory for {}. Error: {:?}",
                details.name, e
            );
            eprintln!(
                "{} {} {}",
                &details.repo_label,
                Paint::red("ERROR:").bold(),
                error_msg
            );
            return ProjectState::Failed {
                name: details.name,
                stage: "clone",
                error: error_msg,
            };
        }
    };
    let path = temp_dir.path().to_path_buf();
    let path_str = path.to_string_lossy();

    println!(
        "{} Cloning {} into {}",
        &details.repo_label,
        Paint::cyan(details.url),
        Paint::yellow(&path_str)
    );

    let clone_output = match Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            details.url,
            path.to_str().expect("Path should be valid UTF-8"),
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            let error_msg = format!(
                "Failed to execute git clone for {}. Error: {:?}",
                details.url, e
            );
            eprintln!(
                "{} {} {}",
                &details.repo_label,
                Paint::red("ERROR:").bold(),
                error_msg
            );
            return ProjectState::Failed {
                name: details.name,
                stage: "clone",
                error: error_msg,
            };
        }
    };

    if !clone_output.status.success() {
        let error_msg = format!(
            "Failed to clone {}. Git command exited with: {}.",
            details.url, clone_output.status
        );
        ui::log_cmd_error(
            &clone_output.stderr,
            &format!(
                "{} {} {}",
                &details.repo_label,
                Paint::red("ERROR:").bold(),
                error_msg
            ),
        );
        return ProjectState::Failed {
            name: details.name,
            stage: "clone",
            error: error_msg,
        };
    }
    println!("{} Cloned successfully.", &details.repo_label);

    ProjectState::Cloned(Ready {
        details,
        path,
        _temp_dir: temp_dir,
    })
}

/// Attempts to build a cloned project.
pub fn try_build_project<'url>(cloned_state: Ready<'url>) -> ProjectState<'url> {
    let details = &cloned_state.details;
    let path_str = cloned_state.path.to_string_lossy();

    println!("{} Running 'forge build'", &details.repo_label);
    let start_time = Instant::now();
    let build_process = match Command::new("forge")
        .arg("build")
        .env("FOUNDRY_DISABLE_NIGHTLY_WARNING", "true")
        .current_dir(&cloned_state.path)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            let error_msg = format!(
                "Failed to execute 'forge build' in {} for {}. Error: {:?}",
                path_str, details.name, e
            );
            eprintln!(
                "{} {} {}",
                &details.repo_label,
                Paint::red("ERROR:").bold(),
                error_msg
            );
            return ProjectState::Failed {
                name: details.name.clone(),
                stage: "build",
                error: error_msg,
            };
        }
    };
    let elapsed = start_time.elapsed().as_secs_f64();

    if build_process.status.success() {
        println!(
            "{} {} Elapsed time: {}",
            &details.repo_label,
            Paint::yellow("BUILT!").bold(),
            Paint::yellow(format!("{elapsed:.2}s").as_str()).bold()
        );
        ProjectState::Built(Built {
            state: cloned_state,
            build_time: elapsed,
        })
    } else {
        let error_msg = format!(
            "'forge build' for {} failed with exit code: {:?}.",
            details.name,
            build_process.status.code()
        );
        ui::log_cmd_error(
            &build_process.stderr,
            &format!(
                "{} {} {}",
                &details.repo_label,
                Paint::red("ERROR:").bold(),
                error_msg
            ),
        );
        ProjectState::Failed {
            name: details.name.clone(),
            stage: "build",
            error: error_msg,
        }
    }
}

/// Attempts to run tests for a built project.
pub fn try_test_project<'url>(
    built_state: Built<'url>,
    num_test_runs: usize,
    verbosity: Verbosity,
) -> ProjectState<'url> {
    let details = &built_state.state.details;
    let mut args = vec!["test"];
    let verbosity_flag = format!("-{}", "v".repeat(verbosity as usize));
    if verbosity != 0 {
        args.push(verbosity_flag.as_str());
    }

    let mut test_times = Vec::with_capacity(num_test_runs);
    for i in 0..num_test_runs {
        println!(
            "{} Running 'forge test' ({}/{}) for {}",
            &details.repo_label,
            i + 1,
            num_test_runs,
            details.name
        );

        let start_at = Instant::now();
        let test_process = match Command::new("forge")
            .args(&args)
            .env("FOUNDRY_DISABLE_NIGHTLY_WARNING", "true")
            .current_dir(&built_state.state.path)
            .output()
        {
            Ok(output) => output,
            Err(e) => {
                let error_msg = format!(
                    "Failed to execute 'forge test' for {}. Error: {:?}",
                    details.name, e
                );
                eprintln!(
                    "{} {} {}",
                    &details.repo_label,
                    Paint::red("ERROR:").bold(),
                    error_msg
                );
                return ProjectState::Failed {
                    name: details.name.clone(),
                    stage: "test",
                    error: error_msg,
                };
            }
        };
        let elapsed = start_at.elapsed().as_secs_f64();

        if test_process.status.success() {
            println!(
                "{} {} Elapsed time: {}",
                &details.repo_label,
                Paint::green("PASSED!").bold(),
                Paint::green(format!("{elapsed:.2}s").as_str()).bold()
            );
            test_times.push(elapsed);
        } else {
            let error_msg = format!(
                "'forge test' for {} FAILED with status code: {:?}",
                details.name,
                test_process.status.code()
            );
            ui::log_cmd_error(
                &test_process.stdout,
                &format!(
                    "{} {} {}",
                    &details.repo_label,
                    Paint::red("FAILED:").bold(),
                    error_msg
                ),
            );
            return ProjectState::Failed {
                name: details.name.clone(),
                stage: "test",
                error: error_msg,
            };
        }
    }

    if test_times.len() == num_test_runs {
        ProjectState::Tested(Tested::new(built_state, test_times, num_test_runs))
    } else {
        let error_msg = format!(
            "Incomplete test runs for {} (expected {}, got {}).",
            details.name,
            num_test_runs,
            test_times.len()
        );
        ProjectState::Failed {
            name: details.name.clone(),
            stage: "test",
            error: error_msg,
        }
    }
}

/// Orchestrates the benchmark pipeline for a list of repository URLs.
///
/// Steps:
///  1. Clone repositories from github (in parallel).
///  2. Run `forge build` (in parallel).
///  3. Run `forge test` (sequentially).
pub fn run_benchmark_pipeline<'url>(
    repo_urls: &'url [String],
    num_test_runs: usize,
    verbosity: Verbosity,
) -> Result<Vec<Tested<'url>>> {
    let initial_projects: Vec<ProjectDetails> = repo_urls
        .par_iter()
        .filter_map(|url| match ProjectDetails::new(url) {
            Ok(details) => Some(details),
            Err(e) => {
                eprintln!(
                    "{} Failed to initialize project from URL {}: {}",
                    Paint::red("ERROR:").bold(),
                    url,
                    e
                );
                None
            }
        })
        .collect();

    if initial_projects.is_empty() && !repo_urls.is_empty() {
        return Err(eyre::eyre!(
            "No projects could be initialized. Please check URLs and permissions."
        ));
    }
    if initial_projects.is_empty() && repo_urls.is_empty() {
        println!("No repository URLs provided to benchmark.");
        return Ok(Vec::new());
    }

    ui::banner(Some("CLONE PROJECTS (in parallel)"));
    let cloned_outcomes: Vec<ProjectState> = initial_projects
        .into_par_iter()
        .map(try_clone_project)
        .collect();

    let mut successfully_cloned: Vec<Ready> = Vec::new();
    let mut failed_project_names: Vec<String> = Vec::new();

    for outcome in cloned_outcomes {
        match outcome {
            ProjectState::Cloned(cloned) => successfully_cloned.push(cloned),
            ProjectState::Failed {
                name, stage, error, ..
            } => {
                eprintln!("Project '{}' failed at stage '{}': {}", name, stage, error);
                failed_project_names.push(name);
            }
            _ => unreachable!("Unexpected outcome after cloning stage"),
        }
    }

    ui::banner(Some("BUILD PROJECTS (in parallel)"));
    let built_outcomes: Vec<ProjectState> = successfully_cloned
        .into_par_iter()
        .map(try_build_project)
        .collect();

    let mut successfully_built: Vec<Built> = Vec::new();
    for outcome in built_outcomes {
        match outcome {
            ProjectState::Built(built) => successfully_built.push(built),
            ProjectState::Failed {
                name, stage, error, ..
            } => {
                eprintln!("Project '{}' failed at stage '{}': {}", name, stage, error);
                failed_project_names.push(name);
            }
            _ => unreachable!("Unexpected outcome after building stage"),
        }
    }

    ui::banner(Some("TEST PROJECTS (sequentially per project)"));
    std::io::stdout()
        .flush()
        .wrap_err("Failed to flush stdout")?;

    let mut final_results: Vec<Tested> = Vec::new();
    // `TempDir` is dropped when it goes out of scope at the end of each iteration, or when consumed by `try_test_project`.
    for built_project in successfully_built {
        match try_test_project(built_project, num_test_runs, verbosity) {
            ProjectState::Tested(tested) => final_results.push(tested),
            ProjectState::Failed {
                name, stage, error, ..
            } => {
                eprintln!("Project '{}' failed at stage '{}': {}", name, stage, error);
                failed_project_names.push(name);
            }
            _ => unreachable!("Unexpected outcome after testing stage"),
        }
    }

    if !failed_project_names.is_empty() {
        println!(
            "\n{}",
            Paint::yellow("Summary of projects that failed at some stage:").bold()
        );
        let unique_failed_names: std::collections::HashSet<String> =
            failed_project_names.into_iter().collect();
        for name in unique_failed_names {
            println!(" - {}", name);
        }
    }

    Ok(final_results)
}
