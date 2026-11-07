//! A case's directories: its working directory with the case's files, a home, and the
//! snapshot of what the case left behind.

use std::path::{Path, PathBuf};

use crate::case::Case;

/// Files larger than this are recorded by size and hash only.
pub const FILE_LIMIT: usize = 64 * 1024;
/// At most this many entries are recorded per case.
pub const ENTRY_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    File(Vec<u8>),
    /// A file over [`FILE_LIMIT`]: its size and FNV-1a 64 hash.
    BigFile {
        size: u64,
        hash: u64,
    },
    Dir,
    Link(Vec<u8>),
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileEntry {
    /// Relative to the working directory, `/`-separated.
    pub path: String,
    pub entry: Entry,
}

/// Spawning a process holds this for reading, writing a case's files for writing. A
/// child forked while another thread has a file open for writing inherits that fd until
/// it execs; executing the file then fails with "Text file busy" (ETXTBSY). The lock
/// keeps forks and open written files apart.
pub static SPAWN_LOCK: std::sync::RwLock<()> = std::sync::RwLock::new(());

pub struct Sandbox {
    pub dir: PathBuf,
    pub cwd: PathBuf,
    pub home: PathBuf,
}

impl Sandbox {
    /// Creates `<base>/<n>/{case,home}` and writes the case's files.
    pub fn create(base: &Path, n: usize, case: &Case) -> std::io::Result<Sandbox> {
        let dir = base.join(n.to_string());
        let cwd = dir.join("case");
        let home = dir.join("home");
        std::fs::create_dir_all(&cwd)?;
        std::fs::create_dir_all(&home)?;
        let _guard = SPAWN_LOCK.write().unwrap_or_else(|e| e.into_inner());
        for d in &case.dirs {
            std::fs::create_dir_all(cwd.join(d))?;
        }
        for f in &case.files {
            let path = cwd.join(&f.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &f.contents)?;
            #[cfg(unix)]
            if f.executable {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(perms.mode() | 0o111);
                std::fs::set_permissions(&path, perms)?;
            }
        }
        Ok(Sandbox { dir, cwd, home })
    }

    /// What the case created or changed in its working directory, sorted by path:
    /// the case's own unchanged files (and the directories that hold them) are left out.
    pub fn snapshot(&self, case: &Case) -> Vec<FileEntry> {
        let mut out = Vec::new();
        walk(&self.cwd, "", &mut out);
        out.retain(|e| {
            if let Some(f) = case.files.iter().find(|f| f.path == e.path) {
                return e.entry != Entry::File(f.contents.clone());
            }
            if e.entry == Entry::Dir {
                let prefix = format!("{}/", e.path);
                return !case.files.iter().any(|f| f.path.starts_with(&prefix))
                    && !case
                        .dirs
                        .iter()
                        .any(|d| *d == e.path || d.starts_with(&prefix));
            }
            true
        });
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.truncate(ENTRY_LIMIT);
        out
    }

    pub fn remove(self) {
        make_writable(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<FileEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let full = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&full) else {
            continue;
        };
        let ft = meta.file_type();
        // A git repository's internals (`.git`, bare `*.git`: index mtimes, objects, the
        // git version's and the file system's defaults) are git's, not the shell's, and
        // differ per run and per git: the agent cases (L3) compare what git prints (0031).
        if ft.is_dir() && path.rsplit('/').next().is_some_and(|n| n.ends_with(".git")) {
            continue;
        }
        let e = if ft.is_symlink() {
            let target = std::fs::read_link(&full).unwrap_or_default();
            Entry::Link(target.to_string_lossy().into_owned().into_bytes())
        } else if ft.is_dir() {
            walk(&full, &path, out);
            Entry::Dir
        } else if ft.is_file() {
            match std::fs::read(&full) {
                Ok(data) if data.len() <= FILE_LIMIT => Entry::File(data),
                Ok(data) => Entry::BigFile {
                    size: data.len() as u64,
                    hash: fnv1a64(&data),
                },
                Err(_) => Entry::Other,
            }
        } else {
            Entry::Other
        };
        out.push(FileEntry { path, entry: e });
    }
}

/// Cases may leave read-only directories behind; make everything removable.
fn make_writable(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::symlink_metadata(dir)
            && meta.is_dir()
        {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o700);
            let _ = std::fs::set_permissions(dir, perms);
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    make_writable(&e.path());
                }
            }
        }
    }
    #[cfg(not(unix))]
    let _ = dir;
}
