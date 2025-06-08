use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{arg, Parser, ValueHint};
use content_inspector::{inspect, ContentType};
// use git2::Repository;
use tempfile::TempDir;
use walkdir::{DirEntry, WalkDir};

/// Extracts text files from a GitHub repo path (owner/repo[/sub/path])
#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Repository in `owner/repo[/sub/path]` form
    #[arg(value_hint = ValueHint::Other)]
    repo: String,

    /// Comma-separated list of file extensions to include (e.g. rs,md,txt)
    #[arg(short, long, value_delimiter = ',')]
    extensions: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (owner, repo, sub_path) = parse_repo_path(&args.repo)?;

    // --- Clone into a temp dir that cleans itself up automatically -----------
    let tmp = TempDir::new().context("failed to create temporary directory")?;
    let repo_url = format!("https://github.com/{owner}/{repo}.git");
    println!("Cloning {repo_url} …");
    
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options({
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.depth(1);
        fetch_opts
    });
    builder.clone(&repo_url, tmp.path())
        .with_context(|| format!("failed to clone {repo_url}"))?;

    // --- Walk files ----------------------------------------------------------
    let target_root = tmp.path().join(&sub_path);
    anyhow::ensure!(
        target_root.exists(),
        "path '{}' does not exist in repository", sub_path.display()
    );

    let allowed_exts: HashSet<_> = args.extensions.iter().map(String::as_str).collect();
    let mut markdown = String::new();
    markdown.push_str(&format!("# {owner}/{repo}\n\n"));

    WalkDir::new(&target_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(skip_git)
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .try_for_each(|entry| -> Result<()> {
            let path = entry.path();
            let rel = path.strip_prefix(tmp.path()).unwrap();

            // extension filter
            if !allowed_exts.is_empty() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if !allowed_exts.contains(ext) {
                    return Ok(());
                }
            }

            if !is_likely_text(path) {
                println!("Skipping binary {rel:?}");
                return Ok(());
            }

            let src = match fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    println!("Skipping {rel:?}: {e}");
                    return Ok(());
                }
            };

            let lang = lang_for_ext(path.extension().and_then(|s| s.to_str()).unwrap_or(""));

            markdown.push_str(&format!("## {}\n\n```{}\n", rel.display(), lang));
            markdown.push_str(&src);
            if !src.ends_with('\n') {
                markdown.push('\n');
            }
            markdown.push_str("```\n\n");
            println!("Added {rel:?}");
            Ok(())
        })?;

    // --- Write output --------------------------------------------------------
    let outfile = PathBuf::from(format!("{}.md", args.repo.replace('/', "-")));
    fs::write(&outfile, markdown).with_context(|| format!("writing {outfile:?}"))?;
    println!("Written {}", outfile.display());
    Ok(())
}

// ---------- helpers ----------------------------------------------------------

fn parse_repo_path(s: &str) -> Result<(String, String, PathBuf)> {
    let mut parts = s.trim_matches('/').splitn(3, '/');
    let owner = parts.next().context("missing owner")?.to_owned();
    let repo = parts.next().context("missing repo")?.to_owned();
    let sub = parts.next().unwrap_or("").to_owned();
    Ok((owner, repo, PathBuf::from(sub)))
}

fn skip_git(entry: &DirEntry) -> bool {
    !entry
        .path()
        .components()
        .any(|c| c.as_os_str() == ".git")
}

fn is_likely_text(path: &Path) -> bool {
    match fs::read(path) {
        Ok(buf) => !matches!(inspect(&buf), ContentType::BINARY),
        Err(_) => false,
    }
}

fn lang_for_ext(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "cpp" | "cc" | "cxx" => "cpp",
        "c" | "h" | "hpp" => "c",
        "java" => "java",
        "go" => "go",
        "sh" => "bash",
        "ps1" => "powershell",
        "bat" | "cmd" => "batch",
        "json" => "json",
        "xml" => "xml",
        "html" => "html",
        "css" => "css",
        "md" => "markdown",
        "yml" | "yaml" => "yaml",
        "toml" => "toml",
        _ => "",
    }
} 