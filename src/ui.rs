use yansi::Paint;

use crate::Benchmarks;

/// Assumes `ref_benchmarks[i]` corresponds to `vs_benchmarks[i]`
pub fn log_test_table(b: &Benchmarks) {
    println!("\n## benchmarks `forge test {}`\n", b.verbosity);

    println!(
        "| Project | Before [{}]({}) | After [{}]({}) | Relative Diff |",
        b.ref_source.name(),
        b.ref_source.github_url(b.foundry_repo),
        b.vs_source.name(),
        b.vs_source.github_url(b.foundry_repo),
    );
    println!("|--------|----------|------|-----------|");

    for (before_project, after_project) in b.ref_tests.iter().zip(b.vs_tests.iter()) {
        let project_link = format!("[{}]({})", before_project.name, before_project.url);

        let before_time = before_project.avg_test_time;
        let after_time = after_project.avg_test_time;

        let overhead = if before_time == 0.0 {
            if after_time == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            (after_time - before_time) / before_time * 100.0
        };

        println!(
            "| {} | {:.2}s | {:.2}s | {:.1}% |",
            project_link, before_time, after_time, overhead
        );
    }

    println!(
        "\nnote: the reported times are the average of {} runs.",
        b.ref_tests[0].runs
    );
}

const BASE_BANNER: &str =
    "------------------------------------------------------------------------";
fn print_banner(text: Option<&str>, with_line_break: bool) {
    let banner = match text {
        Some(text) => {
            let num_chars = BASE_BANNER.len().saturating_sub(text.len() + 4);
            format!("-- {text} {repeat}", repeat = "-".repeat(num_chars))
        }
        None => BASE_BANNER.into(),
    };

    println!(
        "{}{}",
        if with_line_break { "\n" } else { "" },
        Paint::new(banner).bold()
    );
}

pub fn banner(text: Option<&str>) {
    print_banner(text, true);
}

pub fn big_banner(text: &str) {
    print_banner(None, true);
    println!("{}", Paint::new(text).bold());
    print_banner(None, false);
}

/// Helper function to print output errors from external commands.
pub fn log_cmd_error(bytes: &[u8], msg: &str) {
    eprintln!("{msg}");

    let content = String::from_utf8_lossy(bytes);
    content
        .lines()
        .for_each(|line| eprintln!("{}", Paint::red(line).dim()));
}
