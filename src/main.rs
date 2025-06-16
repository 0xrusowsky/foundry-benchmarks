mod benchmark;
use benchmark::{Benchmarks, Source};

mod cmd;
use cmd::{Cli, Parser};

mod config;
mod ui;
mod utils;

use eyre::Result;
use std::process::Command;
use yansi::Paint;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let repos = cli.get_repos()?;

    match cli.get_cmd()? {
        None => {
            let tested_projects = benchmark::run_pipeline(&repos, cli.num_runs, cli.verbosity)?;
            ui::banner(Some("BENCHMARK SUMMARY"));

            for project in tested_projects {
                println!(
                    " * {} ({})",
                    Paint::primary(&project.name).bold(),
                    Paint::cyan(&project.url)
                );
                println!("   - build time: {:.2}s", project.build_time);
                println!(
                    "   - test time:  {:.2}s (avg for {} runs)",
                    project.avg_test_time, project.runs
                );
            }
            ui::banner(None);
        }
        Some((foundry_repo, baseline, comparison)) => {
            ui::big_banner("FOUNDRY BENCHMARKS");

            println!("Foundry Repo URL       {foundry_repo}");
            println!(
                "Baseline source        {}: {}",
                baseline.ty(),
                baseline.name()
            );
            println!(
                "Comparison source      {}: {}",
                comparison.ty(),
                comparison.name()
            );
            println!("Number of test runs    {}", cli.num_runs);
            println!("Test verbosity         {}", cli.verbosity);

            ui::big_banner(&format!(
                "FOUNDRYUP --> baseline ({}: {})",
                baseline.ty(),
                baseline.name()
            ));
            let status = Command::new("foundryup")
                .arg("-r")
                .arg(foundry_repo)
                .arg(baseline.short())
                .arg(baseline.name())
                .status();
            if status.is_err() {
                return Err(eyre::eyre!(
                    "{} Failed to run 'foundry up -r {} -b {}' successfully.",
                    Paint::red("ERROR:").bold(),
                    baseline.short(),
                    baseline.name()
                ));
            };
            let ref_tests = benchmark::run_pipeline(&repos, cli.num_runs, cli.verbosity)?;

            ui::big_banner(&format!(
                "FOUNDRYUP --> comparison ({}: {})",
                comparison.ty(),
                comparison.name()
            ));
            let status = Command::new("foundryup")
                .arg("-r")
                .arg(foundry_repo)
                .arg(comparison.short())
                .arg(comparison.name())
                .status();
            if status.is_err() {
                return Err(eyre::eyre!(
                    "{} Failed to run 'foundry up -r {} {} {}' successfully.",
                    Paint::red("ERROR:").bold(),
                    &foundry_repo,
                    comparison.short(),
                    comparison.name()
                ));
            };
            let vs_tests = benchmark::run_pipeline(&repos, cli.num_runs, cli.verbosity)?;

            let benchmarks = Benchmarks {
                foundry_repo,
                verbosity: if cli.verbosity != 0 {
                    format!("-{}", "v".repeat(cli.verbosity as usize))
                } else {
                    String::new()
                },
                ref_tests,
                ref_source: baseline,
                vs_tests,
                vs_source: comparison,
            };

            ui::log_test_table(&benchmarks);
        }
    }

    Ok(())
}
