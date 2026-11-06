//! Ratchet files (`tests/<layer>/expected-failures.txt`): the active cases that are
//! allowed to fail. One case id per line, optionally `id: reason`; `#` comments and
//! blank lines at the top are kept. Sorted by file, then case index.

use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct Ratchet {
    /// Leading comment lines, kept verbatim.
    pub header: Vec<String>,
    /// Case id to its reason (possibly empty).
    pub entries: BTreeMap<String, String>,
}

/// Sort key: the id's file part, then its numeric index.
pub fn sort_key(id: &str) -> (String, usize) {
    match id.rsplit_once('/') {
        Some((file, idx)) => (file.to_string(), idx.parse().unwrap_or(usize::MAX)),
        None => (id.to_string(), 0),
    }
}

impl Ratchet {
    pub fn parse(text: &str) -> Ratchet {
        let mut r = Ratchet::default();
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                if r.entries.is_empty() {
                    r.header.push(line.to_string());
                }
                continue;
            }
            let (id, reason) = match t.split_once(": ") {
                Some((id, reason)) => (id.trim(), reason.trim()),
                None => (t.trim_end_matches(':'), ""),
            };
            r.entries.insert(id.to_string(), reason.to_string());
        }
        r
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for h in &self.header {
            out.push_str(h);
            out.push('\n');
        }
        let mut ids: Vec<&String> = self.entries.keys().collect();
        ids.sort_by_key(|id| sort_key(id));
        for id in ids {
            let reason = &self.entries[id];
            if reason.is_empty() {
                out.push_str(id);
            } else {
                out.push_str(&format!("{id}: {reason}"));
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_order() {
        let text = "# header\n\noils/a/10\noils/a/2: reason here\noils/a-b/1\n";
        let r = Ratchet::parse(text);
        assert_eq!(r.entries.len(), 3);
        assert_eq!(r.entries["oils/a/2"], "reason here");
        assert_eq!(
            r.render(),
            "# header\n\noils/a/2: reason here\noils/a/10\noils/a-b/1\n"
        );
    }
}
