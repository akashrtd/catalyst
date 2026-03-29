use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Unknown,
}

impl ProjectLanguage {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyFile {
    pub path: String,
    pub purpose: String,
}

pub fn detect_key_files(dir: &Path, language: &ProjectLanguage) -> Vec<KeyFile> {
    let mut files = Vec::new();

    let universal = [
        ("README.md", "Project documentation"),
        ("README.txt", "Project documentation"),
        ("LICENSE", "License file"),
        ("LICENSE.md", "License file"),
        (".gitignore", "Git ignore rules"),
        (".env.example", "Environment variable template"),
        ("Makefile", "Build automation"),
        ("Dockerfile", "Container build config"),
        ("docker-compose.yml", "Container orchestration"),
    ];

    for (name, purpose) in &universal {
        if dir.join(name).exists() {
            files.push(KeyFile {
                path: name.to_string(),
                purpose: purpose.to_string(),
            });
        }
    }

    let language_specific: Vec<(&str, &str)> = match language {
        ProjectLanguage::Rust => vec![
            ("Cargo.toml", "Workspace/package manifest"),
            ("Cargo.lock", "Dependency lockfile"),
            ("src/lib.rs", "Library entry point"),
            ("src/main.rs", "Binary entry point"),
            ("tests/", "Integration tests"),
            ("benches/", "Benchmarks"),
            ("clippy.toml", "Clippy configuration"),
            ("rustfmt.toml", "Formatter configuration"),
        ],
        ProjectLanguage::TypeScript | ProjectLanguage::JavaScript => vec![
            ("package.json", "Package manifest"),
            ("package-lock.json", "Dependency lockfile"),
            ("tsconfig.json", "TypeScript configuration"),
            ("src/index.ts", "Library entry point"),
            ("src/index.tsx", "Library entry point"),
            ("src/index.js", "Library entry point"),
            ("src/main.ts", "Application entry point"),
            ("next.config.js", "Next.js configuration"),
            ("vite.config.ts", "Vite configuration"),
            (".eslintrc.js", "Linter configuration"),
        ],
        ProjectLanguage::Python => vec![
            ("pyproject.toml", "Project configuration"),
            ("setup.py", "Package setup"),
            ("requirements.txt", "Dependencies"),
            ("src/__init__.py", "Package marker"),
            ("tests/", "Test directory"),
            ("tox.ini", "Test automation"),
            ("pytest.ini", "Pytest configuration"),
        ],
        ProjectLanguage::Go => vec![
            ("go.mod", "Module definition"),
            ("go.sum", "Dependency checksums"),
            ("main.go", "Application entry point"),
            ("cmd/", "Command binaries"),
            ("internal/", "Internal packages"),
        ],
        ProjectLanguage::Unknown => vec![],
    };

    for (name, purpose) in &language_specific {
        if name.ends_with('/') {
            if dir.join(name).is_dir() {
                files.push(KeyFile {
                    path: name.to_string(),
                    purpose: purpose.to_string(),
                });
            }
        } else if dir.join(name).exists() {
            files.push(KeyFile {
                path: name.to_string(),
                purpose: purpose.to_string(),
            });
        }
    }

    files
}

pub fn detect_language(dir: &Path) -> ProjectLanguage {
    if dir.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }
    if dir.join("go.mod").exists() {
        return ProjectLanguage::Go;
    }
    if dir.join("package.json").exists() {
        let ts_config = dir.join("tsconfig.json");
        if ts_config.exists() {
            return ProjectLanguage::TypeScript;
        }
        return ProjectLanguage::JavaScript;
    }
    if dir.join("requirements.txt").exists() || dir.join("pyproject.toml").exists() {
        return ProjectLanguage::Python;
    }
    ProjectLanguage::Unknown
}

const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".next",
    "dist",
    "build",
    "__pycache__",
    ".cache",
    "vendor",
    ".venv",
    "venv",
];

pub fn build_file_tree(dir: &Path, max_depth: usize, max_lines: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    build_tree_recursive(dir, "", max_depth, 0, &mut lines, max_lines);
    lines.join("\n")
}

fn build_tree_recursive(
    dir: &Path,
    prefix: &str,
    max_depth: usize,
    current_depth: usize,
    lines: &mut Vec<String>,
    max_lines: usize,
) {
    if current_depth >= max_depth || lines.len() >= max_lines {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut sorted_entries: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str())
        })
        .collect();

    sorted_entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let total = sorted_entries.len();
    for (i, entry) in sorted_entries.iter().enumerate() {
        if lines.len() >= max_lines {
            lines.push(format!("{}  ... (truncated)", prefix));
            return;
        }

        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            lines.push(format!("{}{}{}/", prefix, connector, name));
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            build_tree_recursive(
                &entry.path(),
                &child_prefix,
                max_depth,
                current_depth + 1,
                lines,
                max_lines,
            );
        } else {
            lines.push(format!("{}{}{}", prefix, connector, name));
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitContext {
    pub branch: String,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub recent_commits: Vec<GitCommit>,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
}

pub fn detect_git_context(dir: &Path) -> Option<GitContext> {
    if !dir.join(".git").exists() {
        return None;
    }

    let branch = run_git(dir, &["branch", "--show-current"])?;
    let status = run_git(dir, &["status", "--porcelain"])?;
    let log = run_git(dir, &["log", "--oneline", "-5"])?;

    let mut modified_files = Vec::new();
    let mut staged_files = Vec::new();

    for line in status.lines() {
        if line.len() < 4 {
            continue;
        }
        let status_code = &line[..2];
        let path = line[3..].to_string();
        if status_code
            .chars()
            .next()
            .is_some_and(|c| c != ' ' && c != '?')
        {
            staged_files.push(path.clone());
        }
        if status_code.chars().nth(1).is_some_and(|c| c != ' ') {
            modified_files.push(path);
        }
    }

    let recent_commits = log
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let (hash, message) = line.split_once(' ')?;
            Some(GitCommit {
                hash: hash.to_string(),
                message: message.to_string(),
            })
        })
        .collect();

    Some(GitContext {
        branch,
        modified_files,
        staged_files,
        recent_commits,
    })
}

fn run_git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::Rust);
    }

    #[test]
    fn test_detect_typescript() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        fs::write(dir.path().join("tsconfig.json"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::TypeScript);
    }

    #[test]
    fn test_detect_javascript() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::JavaScript);
    }

    #[test]
    fn test_detect_python() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::Python);
    }

    #[test]
    fn test_detect_python_pyproject() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::Python);
    }

    #[test]
    fn test_detect_go() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("go.mod"), "").unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::Go);
    }

    #[test]
    fn test_detect_unknown() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_language(dir.path()), ProjectLanguage::Unknown);
    }

    #[test]
    fn test_language_display_name() {
        assert_eq!(ProjectLanguage::Rust.display_name(), "Rust");
        assert_eq!(ProjectLanguage::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn test_file_tree_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        let tree = build_file_tree(dir.path(), 3, 200);
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("src/"));
        assert!(tree.contains("lib.rs"));
    }

    #[test]
    fn test_file_tree_skips_hidden() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".hidden")).unwrap();
        fs::write(dir.path().join("visible.txt"), "hello").unwrap();

        let tree = build_file_tree(dir.path(), 3, 200);
        assert!(tree.contains("visible.txt"));
        assert!(!tree.contains(".hidden"));
    }

    #[test]
    fn test_file_tree_max_lines() {
        let dir = TempDir::new().unwrap();
        for i in 0..50 {
            fs::write(dir.path().join(format!("file_{:02}.txt", i)), "").unwrap();
        }

        let tree = build_file_tree(dir.path(), 3, 5);
        let line_count = tree.lines().count();
        assert!(line_count <= 6);
    }

    #[test]
    fn test_file_tree_depth_limit() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("a/b/c/d")).unwrap();
        fs::write(dir.path().join("a/b/c/d/deep.txt"), "").unwrap();
        fs::write(dir.path().join("a/shallow.txt"), "").unwrap();

        let tree = build_file_tree(dir.path(), 2, 200);
        assert!(tree.contains("shallow.txt"));
        assert!(!tree.contains("deep.txt"));
    }

    #[test]
    fn test_git_context_no_git() {
        let dir = TempDir::new().unwrap();
        assert!(detect_git_context(dir.path()).is_none());
    }

    #[test]
    fn test_key_files_rust() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        let files = detect_key_files(dir.path(), &ProjectLanguage::Rust);
        assert!(files.iter().any(|f| f.path == "Cargo.toml"));
        assert!(files.iter().any(|f| f.path == "README.md"));
        assert!(files.iter().any(|f| f.path == "src/lib.rs"));
    }

    #[test]
    fn test_key_files_typescript() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        fs::write(dir.path().join("tsconfig.json"), "").unwrap();

        let files = detect_key_files(dir.path(), &ProjectLanguage::TypeScript);
        assert!(files.iter().any(|f| f.path == "package.json"));
        assert!(files.iter().any(|f| f.path == "tsconfig.json"));
    }

    #[test]
    fn test_key_files_missing_files() {
        let dir = TempDir::new().unwrap();
        let files = detect_key_files(dir.path(), &ProjectLanguage::Rust);
        assert!(files.is_empty());
    }

    #[test]
    fn test_key_files_unknown_language() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("README.md"), "").unwrap();
        let files = detect_key_files(dir.path(), &ProjectLanguage::Unknown);
        assert!(files.iter().any(|f| f.path == "README.md"));
        assert!(!files.iter().any(|f| f.path == "Cargo.toml"));
    }

    #[test]
    fn test_key_files_purpose() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let files = detect_key_files(dir.path(), &ProjectLanguage::Rust);
        let cargo = files.iter().find(|f| f.path == "Cargo.toml").unwrap();
        assert!(cargo.purpose.contains("manifest"));
    }
}
