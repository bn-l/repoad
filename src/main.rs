use std::{
    collections::HashSet,
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgAction, Parser, ValueHint};
use content_inspector::{inspect, ContentType};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use tar::Archive;
use tempfile::TempDir;
use walkdir::{DirEntry, WalkDir};

/// Extracts text files from either a GitHub repo **or** an npm package
#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// GitHub `owner/repo[/sub/path]` **or** npm `package`
    #[arg(value_hint = ValueHint::Other)]
    target: String,

    /// Comma-separated list of file extensions to include (e.g. rs,md,txt)
    #[arg(short, long, value_delimiter = ',')]
    extensions: Vec<String>,

    /// Treat `target` as an npm package name instead of a GitHub repo
    #[arg(short = 'n', long = "npm-mode", action = ArgAction::SetTrue)]
    npm_mode: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // ---------------- acquire sources into a temp dir ------------------------
    let tmp = TempDir::new().context("failed to create temporary directory")?;
    let (root, title) = if args.npm_mode {
        download_npm_package(&args.target, tmp.path())?
    } else {
        clone_github_repo(&args.target, tmp.path())?
    };

    // ---------------- walk files & build markdown ---------------------------
    let allowed_exts: HashSet<_> = args.extensions.iter().map(String::as_str).collect();
    let mut md = format!("# {title}\n\n");

    WalkDir::new(&root)
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

            md.push_str(&format!("## {}\n\n```{}\n", rel.display(), lang));
            md.push_str(&src);
            if !src.ends_with('\n') {
                md.push('\n');
            }
            md.push_str("```\n\n");
            println!("Added {rel:?}");
            Ok(())
        })?;

    // ---------------- write output ------------------------------------------
    let outfile = PathBuf::from(format!("{}.md", title.replace('/', "-")));
    fs::write(&outfile, md).with_context(|| format!("writing {outfile:?}"))?;
    println!("Written {}", outfile.display());
    Ok(())
}

// ---------- acquisition helpers ---------------------------------------------

/// Clone GitHub repo just like before
fn clone_github_repo(spec: &str, dest: &Path) -> Result<(PathBuf, String)> {
    let (owner, repo, sub_path) = parse_repo_path(spec)?;
    let repo_url = format!("https://github.com/{owner}/{repo}.git");
    println!("Cloning {repo_url} …");

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options({
        let mut f = git2::FetchOptions::new();
        f.depth(1);
        f
    });
    builder
        .clone(&repo_url, dest)
        .with_context(|| format!("failed to clone {repo_url}"))?;

    let root = dest.join(&sub_path);
    anyhow::ensure!(
        root.exists(),
        "path '{}' does not exist in repository",
        sub_path.display()
    );
    Ok((root, format!("{owner}/{repo}")))
}

/// Download and unpack an npm package into `dest`
fn download_npm_package(pkg: &str, dest: &Path) -> Result<(PathBuf, String)> {
    println!("Resolving npm package '{pkg}' …");
    let client = Client::builder()
        .user_agent("md-extract/0.1")
        .build()?;

    // 1. registry metadata
    let meta: Value = client
        .get(format!("https://registry.npmjs.org/{pkg}"))
        .send()?
        .error_for_status()?
        .json()?;

    // 2. pick latest version tarball
    let latest = meta["dist-tags"]["latest"]
        .as_str()
        .ok_or_else(|| anyhow!("no 'latest' dist-tag"))?;
    let tar_url = meta["versions"][latest]["dist"]["tarball"]
        .as_str()
        .ok_or_else(|| anyhow!("missing tarball url"))?;

    println!("Downloading {tar_url} …");
    let bytes = client.get(tar_url).send()?.error_for_status()?.bytes()?;

    // 3. unpack
    let gz = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;

    // npm packages unpack into `package/…`
    Ok((dest.join("package"), pkg.to_owned()))
}

// ---------- misc helpers (unchanged) ----------------------------------------

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
