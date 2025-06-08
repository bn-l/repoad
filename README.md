<p align="center">
  <img src="logo.webp" alt="repoad logo" width="400" />
</p>

Turn any GitHub repository into a single, LLM-friendly markdown file.

E.g.

```shell
repoad charmbracelet/soft-serve
```

That will copy all the text files from the repo into a nicely formatted markdown file to fill the context of an LLM with.

Each file's content is enclosed in a fenced code block with appropriate language identifiers for syntax highlighting, and prefixed with a heading containing the file path.

## Usage

```text
Extracts text files from a GitHub repo path (owner/repo[/sub/path])

USAGE:
    repoad [OPTIONS] <REPO>

ARGS:
    <REPO>
        Repository in `owner/repo[/sub/path]` form

OPTIONS:
    -e, --extensions <EXTENSIONS>
        Comma-separated list of file extensions to include (e.g. rs,md,txt)

    -h, --help
        Print help information

    -V, --version
        Print version information
```

## Examples

### Process a whole repository

This will process all text files in the `charmbracelet/soft-serve` repository.

```bash
repoad charmbracelet/soft-serve
```
*   `charmbracelet`: The GitHub username or organization.
*   `soft-serve`: The repository name.

The output will be a single file named `charmbracelet-soft-serve.md`.

### Process a specific directory

This will process all text files in the `cmd` directory of the `charmbracelet/soft-serve` repository.

```bash
repoad charmbracelet/soft-serve/cmd
```
*   `cmd`: The specific folder within the repository to process.

The output will be a single file named `charmbracelet-soft-serve-cmd.md`.

### Filter by file extension

This will process only `.go` and `.md` files from the `charmbracelet/soft-serve` repository.

```bash
repoad charmbracelet/soft-serve -e go,md
```
*   `-e go,md`: The `-e` (or `--extensions`) flag allows you to specify a comma-separated list of file extensions to include.

## Output Example

The generated markdown file will look something like this:

```markdown
# charmbracelet/soft-serve

## soft-serve.go

```go
// ... (file content) ...
```

## cmd/soft/main.go

```go
// ... (file content) ...
```
```

## Features

- **Fast**: Built in Rust for performance. It uses a shallow git clone (`--depth=1`) to only download the latest version of the repository, saving time and bandwidth.
- **Flexible**: Process a whole repository or just a subdirectory. Filter by file extensions.
- **Simple**: Does one thing well.

## Installation

1.  Ensure you have the [Rust toolchain](https://rustup.rs/) installed.
2.  Clone this project.
3.  Run `cargo build --release`.
4.  The executable will be located at `target/release/repoad`.
