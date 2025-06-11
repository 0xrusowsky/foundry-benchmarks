# Foundry Benchmarks CLI

A command-line tool designed to benchmark the performance of [Foundry](https://github.com/foundry-rs/foundry) projects. It measures build and test times, allowing you to run standalone benchmarks or compare the performance impact of changes between two different Foundry versions or branches.

The tool is pre-configured with a list of popular, open-source repositories but can be pointed at any public Git repository containing a Foundry project.

## TODO

- [ ] Support euler contracts (figure out why tests fail)
- [ ] Support custom config via CLI (for non-default projects)
- [ ] Add OZ and Ithaca contracts as default benchmarks


## Features

- **Benchmark Build & Test Times**: Get concrete performance metrics for any Foundry project.
- **Compare Foundry Versions**: Run A/B performance tests between different Foundry versions or branches (e.g., `master` vs. your feature branch).
- **Flexible Repository Targeting**: Use the default list of projects or provide your own list of repositories.
- **Customizable Test Runs**: Configure the number of test runs to average results for more stable metrics.
- **Markdown-Ready Output**: Generates a clean, shareable markdown table summarizing comparison results.
- **Handles Custom Setups**: Automatically installs special dependencies for projects that require them.

## Prerequisites

Before you begin, ensure you have the following installed on your system:

- **Rust & Cargo**: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
- **Foundry (via `foundryup`)**: `foundryup` is required to switch between different Foundry versions for comparison. [Installation Guide](https://book.getfoundry.sh/getting-started/installation).

## Installation

1.  Clone this repository:
    ```sh
    git clone https://github.com/0xrusowsky/foundry-benchmarks
    cd foundry-benchmarks
    ```

2.  Build the project:
    ```sh
    cargo build --release
    ```
    The final executable will be available at `target/release/foundry-benchmarks`.

## Usage

The CLI offers two main modes: a simple benchmark run and a `diff` mode for comparing two Foundry sources.

### Basic Benchmarking

If you run the tool without any subcommands, it performs a basic benchmark using the currently active Foundry version.

#### Running on Default Repositories

Execute the tool with no arguments to run benchmarks on the pre-configured list of repositories.

```sh
cargo run
```

#### Running on Specific Repositories

Use the `--repos` flag to specify one or more repositories. You can provide short-form `owner/repo` names, which will be resolved to full GitHub URLs.

**Run on a single repository:**
```sh
cargo run -- --repos sablier-labs/lockup
```

**Run on a list of repositories:**
```sh
cargo run -- --repos uniswap/v4-core,morpho-org/morpho-blue
```

#### Controlling Test Runs and Verbosity

-   `--num-runs`: Controls how many times `forge test` is executed to average the results. Defaults to 10.
-   `-v`, `--verbosity`: Increases the verbosity of the `forge` commands. Can be repeated for higher levels (e.g., `-vv`, `-vvv`).

```sh
# Run 50 test iterations for solady with high verbosity
cargo run -- --repos vectorized/solady --num-runs 50 -vvv
```

### Comparing Foundry Versions with `diff`

The `diff` subcommand is the most powerful feature. It installs two different versions of Foundry, runs the full benchmark pipeline on each, and presents a comparison table.

The reference source (baseline) is specified with `--reference-version`/`--reference-branch`, and the comparison source is specified with `--comparison-version`/`--comparison-branch`.

#### Comparing Two Branches

This is useful for checking the performance impact of a feature branch against the main branch.

```sh
# Alternatively, you could use `--ref-branch` and `vs-branch`
cargo run -- diff --reference-branch master --comparison-branch my-perf-optimization
```

#### Comparing Two Versions

This is useful for comparing a release candidate or a specific version tag against the stable version.

```sh
# Alternatively, you could use `--ref-version` and `vs-version`
cargo run -- diff --reference-version nightly --comparison-version v1.2.0-rc
```

#### Using a Custom Foundry Repository

If you are working with a fork of Foundry, you can specify it using the `--foundry-repo` flag.

```sh
cargo run -- diff \
  --foundry-repo your-github/foundry \
  --reference-branch master \
  --comparison-branch your-feature-branch
```

## Output Example

When running the `diff` command, the tool generates a markdown table that's perfect for pasting into GitHub pull requests or issues.

| Project | Before [master](https://github.com/foundry-rs/foundry/tree/master) | After [my-perf-opt](https://github.com/foundry-rs/foundry/tree/my-perf-opt) | Relative Diff |
|---|---|---|---|
| [uniswap/v4-core](https://github.com/uniswap/v4-core) | 5,43s | 5,11s | -5,9% |
| [morpho-org/morpho-blue](https://github.com/morpho-org/morpho-blue) | 1,21s | 1,15s | -5,0% |
| [sablier-labs/lockup](https://github.com/sablier-labs/lockup) | 2,56s | 2,60s | +1,6% |

> note: the reported times are the average of 10 runs.
