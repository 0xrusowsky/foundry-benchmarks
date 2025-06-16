# Foundry Benchmarks CLI

A command-line tool designed to benchmark the performance of [Foundry](https://github.com/foundry-rs/foundry) projects. It measures build and test times, allowing you to run standalone benchmarks or compare the performance impact of changes between two different Foundry versions or branches.

The tool is pre-configured with a list of popular, open-source repositories but can be pointed at any public Git repository containing a Foundry project.

## TODO

- [ ] Support euler and OZ contracts (currently failing due to revm revert where timestamps must be <= u64)

## Features

- **Benchmark Build & Test Times**: Get concrete performance metrics for any Foundry project.
- **Compare Foundry Versions**: Run A/B performance tests between different Foundry versions or branches (e.g., `master` vs. your feature branch).
- **Flexible Repository Targeting**: Use the default list of projects or provide your own list of repositories.
- **Customizable Test Runs**: Configure the number of test runs to average results for more stable metrics.
- **Markdown-Ready Output**: Generates a clean, shareable markdown table summarizing comparison results.
- **Custom Project Configurations**: Support for dependencies, remappings, and environment variables via TOML files or CLI flags.
- **Per-Project Settings**: Configure each project individually with different dependencies and settings.
- **Parallel Processing**: Clones and builds projects in parallel for faster benchmarking.
- **Environment Variable Expansion**: Supports `${VAR_NAME}` syntax in configuration files.

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

### Custom Project Configuration

The tool supports three flexible ways to configure project settings: TOML configuration files, global CLI flags, and per-project JSON configurations.

#### Using a TOML Configuration File

The tool uses a `benchmarks.toml` file (default) that supports both custom and default configurations. The file the following sections:

- `[custom]`: Your custom environment variables (takes precedence when no CLI args are provided)
- `[defaults]`: Default environment variables (used as fallback)

Where each section can have individual subsections:
- `[[project]]`: Individual project configurations

See `benchmarks.toml` for the complete configuration including all default repositories.

**Config Priority Logic:**
1. CLI arguments always take precedence
2. If `[custom]` section has content and no CLI args are provided, it will be used
3. Otherwise, `[defaults]` section will be used

Example custom configuration:
```toml
# benchmarks.toml
[custom]
env_vars = { MAINNET_RPC_URL = "https://my-custom-rpc.com" }

[[project]]
name = "my-org/my-project"
dependencies = ["install", "foundry-rs/forge-std@v1.8.0"]
remappings = ["forge-std/=lib/forge-std/"]
env_vars = { CUSTOM_VAR = "value" }
```

Run with default `benchmarks.toml` file:
```sh
cargo run
```

Or specify a custom config file:
```sh
cargo run -- --config my-config.toml
```

#### Using CLI Flags

Apply the same configuration to all specified repositories:
```sh
cargo run -- --repos project-a,project-b \
  --deps "install,forge-std" \
  --remappings "forge-std/=lib/forge-std/" \
  --env "MAINNET_RPC_URL=https://...,OTHER_VAR=value"
```

Note: When using CLI flags, they apply to all repositories specified with `--repos`.

#### Per-Project JSON Configuration

Configure each project individually with different settings:
```sh
cargo run -- \
  --repo 'my-org/project-a:{"dependencies":["forge-std"]}' \
  --repo 'my-org/project-b:{"remappings":["@oz/=lib/oz/"]}' \
  --repo my-org/project-c
```

The JSON configuration supports:
- `dependencies`: Array of forge dependencies to install
- `remappings`: Array of import remappings
- `env_vars`: Object with environment variable key-value pairs

#### Configuration Priority

When multiple configuration methods are used, they are applied in this order (highest to lowest priority):
1. `--repo` flag with per-project JSON configuration
2. `--repos` flag with global CLI flags (`--deps`, `--remappings`, `--env`)
3. TOML configuration file (`--config` or default `benchmarks.toml`): `[custom]` section.
4. TOML configuration file (`--config` or default `benchmarks.toml`): `[default]` section.

## Output Example

When running the `diff` command, the tool generates a markdown table that's perfect for pasting into GitHub pull requests or issues.

### Diff Mode Output

```
## benchmarks `forge test`

| Project | Before [master](https://github.com/foundry-rs/foundry/tree/master) | After [my-perf-opt](https://github.com/foundry-rs/foundry/tree/my-perf-opt) | Relative Diff |
|---------|-----------|-------|-----------|
| [uniswap/v4-core](https://github.com/uniswap/v4-core) | 12.45s | 10.23s | -17.8% |
| [morpho-org/morpho-blue](https://github.com/morpho-org/morpho-blue) | 8.67s | 8.54s | -1.5% |
| [sablier-labs/lockup](https://github.com/sablier-labs/lockup) | 15.23s | 14.89s | -2.2% |

note: the reported times are the average of XX runs.
```

### Standard Mode Output

```
BENCHMARK SUMMARY
------------------------------------------------------------------------
 * uniswap/v4-core (https://github.com/uniswap/v4-core)
   - build time: 4.56s
   - test time:  12.45s (avg for XX runs)
 * morpho-org/morpho-blue (https://github.com/morpho-org/morpho-blue)
   - build time: 3.21s
   - test time:  8.67s (avg for XX runs)
------------------------------------------------------------------------
```

## Default Projects

The tool comes pre-configured with several popular Foundry projects:

- `uniswap/v4-core` - Uniswap V4 Core contracts
- `sparkdotfi/spark-psm` - Spark Protocol PSM module
- `morpho-org/morpho-blue` - Morpho Blue lending protocol
- `vectorized/solady` - Gas-optimized Solidity snippets
- `ithacaxyz/account` - Ithaca account abstraction
- `sablier-labs/lockup` - Sablier V2 lockup streaming

See `benchmarks.toml` for the full list and their configurations.
