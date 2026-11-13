use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use directories::BaseDirs;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct SessionSummary {
    pub id: String,
    pub cwd: PathBuf,
    pub title: String,
    pub updated: SystemTime,
    pub path: PathBuf,
}

#[derive(Deserialize)]
struct SessionHeader {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Default, Deserialize)]
struct SessionMeta {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    model: Option<String>,
}

pub fn sessions_root() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.home_dir().join(".commandcode").join("projects"))
}

pub fn scan_sessions() -> Vec<SessionSummary> {
    let Some(root) = sessions_root() else {
        return Vec::new();
    };
    let Ok(projects) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut sessions = Vec::new();
    for project in projects.flatten() {
        let Ok(file_type) = project.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(project.path()) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.ends_with(".jsonl") || name.ends_with(".checkpoints.jsonl") {
                continue;
            }
            if let Some(summary) = parse_session_file(&path) {
                sessions.push(summary);
            }
        }
    }

    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated));
    sessions
}

pub fn parse_session_file(path: &Path) -> Option<SessionSummary> {
    let header = read_header(path)?;
    if header.kind != "session" {
        return None;
    }
    let id = header.id.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string()
    });
    let cwd = header.cwd?;
    let meta = read_meta(&path.with_extension("meta.json")).unwrap_or_default();
    let title = meta
        .title
        .or(header.name)
        .or_else(|| first_user_text(path))
        .unwrap_or_else(|| format!("Session {}", short_id(&id)));
    let updated = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    Some(SessionSummary {
        id,
        cwd,
        title,
        updated,
        path: path.to_path_buf(),
    })
}

fn read_header(path: &Path) -> Option<SessionHeader> {
    let file = File::open(path).ok()?;
    let mut lines = BufReader::new(file).lines();
    let line = lines.next()?.ok()?;
    serde_json::from_str(&line).ok()
}

fn read_meta(path: &Path) -> Option<SessionMeta> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn first_user_text(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(200) {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(|kind| kind.as_str()) != Some("message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        if message.get("role").and_then(|role| role.as_str()) != Some("user") {
            continue;
        }
        let Some(content) = message
            .get("content")
            .and_then(|content| content.as_array())
        else {
            continue;
        };
        for block in content {
            if block.get("type").and_then(|kind| kind.as_str()) != Some("text") {
                continue;
            }
            if let Some(text) = block.get("text").and_then(|text| text.as_str()) {
                let text = text.trim();
                if !text.is_empty() {
                    return Some(truncate_title(text));
                }
            }
        }
    }
    None
}

fn truncate_title(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let max_chars = 72;
    if single_line.chars().count() <= max_chars {
        return single_line;
    }
    let mut truncated: String = single_line.chars().take(max_chars).collect();
    truncated.push_str("...");
    truncated
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "commandcode-gui-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_header_meta_and_title_fallback() {
        let dir = temp_dir("catalog");
        let path = dir.join("abc.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"abc","timestamp":"2026-01-01T00:00:00.000Z","cwd":"/tmp/project"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"1","parentId":null,"timestamp":"2026-01-01T00:00:01.000Z","message":{{"role":"user","content":[{{"type":"text","text":"hello catalog"}}]}}}}"#
        )
        .unwrap();
        drop(file);

        let summary = parse_session_file(&path).unwrap();
        assert_eq!(summary.id, "abc");
        assert_eq!(summary.cwd, PathBuf::from("/tmp/project"));
        assert_eq!(summary.title, "hello catalog");

        fs::write(dir.join("abc.meta.json"), r#"{"title":"Named session"}"#).unwrap();
        let summary = parse_session_file(&path).unwrap();
        assert_eq!(summary.title, "Named session");

        let _ = fs::remove_dir_all(dir);
    }
}
