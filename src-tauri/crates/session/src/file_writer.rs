//! JSONL file writer/reader for `.session` files.
//!
//! Each session has a file at `{sessions_dir}/{conversation_id}.session`.
//! Records are appended one JSON object per line. The format is:
//! ```jsonl
//! {"ts":"...","seq":1,"role":"user","content":"..."}
//! {"ts":"...","seq":2,"role":"assistant","content":"...","tokens":{"input":10,"output":20}}
//! {"ts":"...","seq":3,"role":"tool","tool_call_id":"c_1","tool":"bash","tool_input":{...},"content":"..."}
//! ```

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::record::SessionRecord;

/// Append-only writer for a session file. Thread-safe via Mutex.
pub struct SessionFileWriter {
    path: PathBuf,
    file: Mutex<File>,
}

impl SessionFileWriter {
    /// Open (or create) the session file for appending. Also creates the
    /// parent directory if needed.
    pub fn open(sessions_dir: &PathBuf, conversation_id: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(sessions_dir)?;
        let path = sessions_dir.join(format!("{}.session", conversation_id));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }

    /// Append a record as one JSON line.
    pub fn append(&self, record: &SessionRecord) -> std::io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        line.push('\n');
        let mut file = self.file.lock().expect("session file mutex poisoned");
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    /// Path of the underlying file (for debugging / inspection).
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Reader for a session file. Reads records line-by-line.
pub struct SessionFileReader;

impl SessionFileReader {
    /// Read the entire file and return all records in order.
    pub fn read_all(sessions_dir: &PathBuf, conversation_id: &str) -> std::io::Result<Vec<SessionRecord>> {
        let path = sessions_dir.join(format!("{}.session", conversation_id));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionRecord>(&line) {
                Ok(r) => records.push(r),
                Err(e) => {
                    tracing::warn!("Skipping malformed session record: {}", e);
                }
            }
        }
        Ok(records)
    }

    /// Read only the last N records. More efficient than read_all for large
    /// files because it reads from the end.
    pub fn read_last(
        sessions_dir: &PathBuf,
        conversation_id: &str,
        limit: usize,
    ) -> std::io::Result<Vec<SessionRecord>> {
        let mut all = Self::read_all(sessions_dir, conversation_id)?;
        if all.len() <= limit {
            Ok(all)
        } else {
            Ok(all.split_off(all.len() - limit))
        }
    }
}
