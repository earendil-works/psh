//! The case model and the loaders that turn the three layers' files into cases.
//!
//! - L1 `oils`: `tests/oils/spec/*.test.sh`; the code is fed to the shell on stdin, as
//!   Oils' runner does.
//! - L2 `brush`: `tests/brush/brush-shell/tests/cases/**/*.yaml`; the shell gets the
//!   case's `args` and its `stdin`, as brush's harness does (minus brush's own default
//!   flags, which change nothing for a non-interactive bash).
//! - L3 `agent`: `tests/agent/*.test.sh` in the Oils format; the code runs as
//!   `SHELL -c CODE`, the way agents' commands run.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::oils;
use crate::yaml::{self, Yaml};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Oils,
    Brush,
    Agent,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Oils, Layer::Brush, Layer::Agent];

    pub fn name(self) -> &'static str {
        match self {
            Layer::Oils => "oils",
            Layer::Brush => "brush",
            Layer::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<Layer> {
        Layer::ALL.into_iter().find(|l| l.name() == s)
    }

    /// Where the layer's sources live, relative to the repository root.
    pub fn source_dir(self) -> &'static str {
        match self {
            Layer::Oils => "tests/oils/spec",
            Layer::Brush => "tests/brush/brush-shell/tests/cases",
            Layer::Agent => "tests/agent",
        }
    }

    /// The ratchet file, relative to the repository root.
    pub fn ratchet_path(self) -> String {
        format!("tests/{}/expected-failures.txt", self.name())
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone)]
pub struct CaseFile {
    /// Relative to the case's working directory, `/`-separated.
    pub path: String,
    pub contents: Vec<u8>,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct Case {
    /// `layer/file/index`, e.g. `oils/alias/3` or `brush/compat/arrays/12`.
    pub id: String,
    pub layer: Layer,
    /// The source file's id within its layer, e.g. `alias` or `compat/arrays`.
    pub file: String,
    /// 0-based position in its file.
    pub index: usize,
    pub name: String,
    /// 1-based line of the case in its source file (0 if unknown).
    pub line: usize,
    /// The script text: what `bash -n` checks (L0).
    pub script: String,
    /// Arguments after the shell's name.
    pub args: Vec<String>,
    /// Bytes on stdin; `None` is an empty stdin (`/dev/null`).
    pub stdin: Option<Vec<u8>>,
    /// Case-specific environment, applied over the layer's base environment.
    pub env: Vec<(String, String)>,
    /// Files created in the working directory before the run.
    pub files: Vec<CaseFile>,
    /// Directories created in the working directory before the run.
    pub dirs: Vec<String>,
    /// `HOME`, relative to the working directory, when the case sets it.
    pub home_dir: Option<String>,
    pub timeout: Duration,
    /// Labels that group rules can match (`pty`, `interactive`, ...).
    pub tags: Vec<String>,
    /// Brush's judgement that stdout is not comparable (e.g. it lists the host).
    pub ignore_stdout: bool,
    pub ignore_whitespace: bool,
}

/// The source file of a case's file id.
pub fn source_path(root: &Path, layer: Layer, file: &str) -> PathBuf {
    let ext = match layer {
        Layer::Brush => "yaml",
        _ => "test.sh",
    };
    root.join(layer.source_dir()).join(format!("{file}.{ext}"))
}

pub const OILS_TIMEOUT: Duration = Duration::from_secs(10);
pub const BRUSH_TIMEOUT: Duration = Duration::from_secs(15);

/// Loads every case of a layer, in file order (files sorted by id).
pub fn load_layer(root: &Path, layer: Layer) -> Result<Vec<Case>, String> {
    let dir = root.join(layer.source_dir());
    let ext = match layer {
        Layer::Brush => ".yaml",
        _ => ".test.sh",
    };
    let mut files = Vec::new();
    collect_files(&dir, &dir, ext, &mut files)?;
    files.sort();
    let mut cases = Vec::new();
    for file in files {
        let path = source_path(root, layer, &file);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let loaded = match layer {
            Layer::Oils | Layer::Agent => load_oils(layer, &file, &text),
            Layer::Brush => load_brush(&file, &text, path.parent().unwrap()),
        };
        cases.extend(loaded.map_err(|e| format!("{}: {e}", path.display()))?);
    }
    Ok(cases)
}

fn collect_files(base: &Path, dir: &Path, ext: &str, out: &mut Vec<String>) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, ext, out)?;
        } else if let Some(name) = path.to_str().and_then(|p| p.strip_suffix(ext)) {
            let rel = Path::new(name).strip_prefix(base).unwrap();
            let id: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(id.join("/"));
        }
    }
    Ok(())
}

fn load_oils(layer: Layer, file: &str, text: &str) -> Result<Vec<Case>, String> {
    let parsed = oils::parse(text)?;
    let mut tags: Vec<String> = Vec::new();
    let mut dirs = Vec::new();
    for (k, v) in &parsed.meta {
        if k == "tags" {
            tags.extend(v.split_whitespace().map(String::from));
        }
        // Oils creates `_tmp` in the case directory for these files.
        if k == "legacy_tmp_dir" && v == "yes" {
            dirs.push("_tmp".to_string());
        }
    }
    Ok(parsed
        .cases
        .into_iter()
        .enumerate()
        .map(|(index, c)| {
            let (args, stdin) = match layer {
                Layer::Agent => (vec!["-c".to_string(), c.code.clone()], None),
                _ => (Vec::new(), Some(c.code.clone().into_bytes())),
            };
            Case {
                id: format!("{layer}/{file}/{index}"),
                layer,
                file: file.to_string(),
                index,
                name: c.desc,
                line: c.line,
                script: c.code,
                args,
                stdin,
                env: Vec::new(),
                files: Vec::new(),
                dirs: dirs.clone(),
                home_dir: None,
                timeout: OILS_TIMEOUT,
                tags: tags.clone(),
                ignore_stdout: false,
                ignore_whitespace: false,
            }
        })
        .collect())
}

fn yaml_files(node: Option<&Yaml>, dir: &Path) -> Result<Vec<CaseFile>, String> {
    let mut out = Vec::new();
    for f in node.map(Yaml::as_seq).unwrap_or(&[]) {
        let path = f
            .get("path")
            .and_then(Yaml::as_str)
            .ok_or("test file without a path")?;
        let contents = match (f.get("contents"), f.get("source_path")) {
            (_, Some(src)) => {
                let src = dir.join(src.as_str().ok_or("bad source_path")?);
                std::fs::read(&src).map_err(|e| format!("{}: {e}", src.display()))?
            }
            (Some(c), None) => c.as_str().unwrap_or("").as_bytes().to_vec(),
            (None, None) => Vec::new(),
        };
        out.push(CaseFile {
            path: path.to_string(),
            contents,
            executable: f.get("executable").and_then(Yaml::as_bool).unwrap_or(false),
        });
    }
    Ok(out)
}

fn load_brush(file: &str, text: &str, dir: &Path) -> Result<Vec<Case>, String> {
    let doc = yaml::parse(text).map_err(|e| e.to_string())?;
    let common = yaml_files(doc.get("common_test_files"), dir)?;
    let mut cases = Vec::new();
    for (index, c) in doc
        .get("cases")
        .map(Yaml::as_seq)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let flag = |k: &str| c.get(k).and_then(Yaml::as_bool).unwrap_or(false);
        let name = c
            .get("name")
            .and_then(Yaml::as_str)
            .unwrap_or("(unnamed)")
            .to_string();
        let args: Vec<String> = c
            .get("args")
            .map(Yaml::as_seq)
            .unwrap_or(&[])
            .iter()
            .map(|a| a.as_str().unwrap_or("").to_string())
            .collect();
        let stdin = c.get("stdin").and_then(Yaml::as_str).map(str::to_string);
        let mut env = Vec::new();
        if let Some(Yaml::Map(entries)) = c.get("env") {
            for (k, v) in entries {
                env.push((k.clone(), v.as_str().unwrap_or("").to_string()));
            }
        }
        let mut files = common.clone();
        files.extend(yaml_files(c.get("test_files"), dir)?);
        let mut tags = Vec::new();
        if flag("pty") {
            tags.push("pty".to_string());
        }
        if args.iter().any(|a| a == "-i") {
            tags.push("interactive".to_string());
        }
        if flag("skip") {
            tags.push("brush-skip".to_string());
        }
        let timeout = c
            .get("timeout_in_seconds")
            .and_then(Yaml::as_str)
            .and_then(|s| s.parse().ok())
            .map_or(BRUSH_TIMEOUT, Duration::from_secs);
        cases.push(Case {
            id: format!("brush/{file}/{index}"),
            layer: Layer::Brush,
            file: file.to_string(),
            index,
            name,
            line: 0,
            script: stdin.clone().unwrap_or_default(),
            args,
            stdin: Some(stdin.unwrap_or_default().into_bytes()),
            env,
            files,
            dirs: Vec::new(),
            home_dir: c.get("home_dir").and_then(Yaml::as_str).map(str::to_string),
            timeout,
            tags,
            ignore_stdout: flag("ignore_stdout"),
            ignore_whitespace: flag("ignore_whitespace"),
        });
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn every_vendored_oils_file_loads() {
        let cases = load_layer(&repo_root(), Layer::Oils).unwrap();
        // 134 files; the count only moves when the vendored set does.
        let files: std::collections::BTreeSet<_> = cases.iter().map(|c| &c.file).collect();
        assert_eq!(files.len(), 134);
        assert!(cases.len() > 2500, "{} cases", cases.len());
        let alias0 = cases.iter().find(|c| c.id == "oils/alias/0").unwrap();
        assert_eq!(alias0.stdin.as_deref(), Some(alias0.script.as_bytes()));
    }

    #[test]
    fn every_vendored_brush_file_loads() {
        let root = repo_root();
        let cases = load_layer(&root, Layer::Brush).unwrap();
        // Cross-check the YAML reader against a plain count of case headers.
        let mut headers = 0;
        let mut files = Vec::new();
        collect_files(
            &root.join(Layer::Brush.source_dir()),
            &root.join(Layer::Brush.source_dir()),
            ".yaml",
            &mut files,
        )
        .unwrap();
        for f in &files {
            let text = std::fs::read_to_string(source_path(&root, Layer::Brush, f)).unwrap();
            headers += text.lines().filter(|l| l.starts_with("  - name:")).count();
        }
        assert_eq!(cases.len(), headers);
        assert!(cases.iter().all(|c| c.stdin.is_some()));
    }
}
