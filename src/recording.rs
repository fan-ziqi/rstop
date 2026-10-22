use crate::model::Snapshot;
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
pub struct Recorder {
    path: PathBuf,
    max: u64,
    file: File,
    size: u64,
}
impl Recorder {
    pub fn new(path: &Path, max_mib: u64) -> Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            path: path.into(),
            max: max_mib * 1024 * 1024,
            file,
            size,
        })
    }
    pub fn write(&mut self, snapshot: &Snapshot) -> Result<()> {
        let mut data = serde_json::to_vec(snapshot)?;
        data.push(b'\n');
        if self.size > 0 && self.size + data.len() as u64 > self.max {
            for index in (1..=3).rev() {
                let from = if index == 1 {
                    self.path.clone()
                } else {
                    PathBuf::from(format!("{}.{}", self.path.display(), index - 1))
                };
                let to = PathBuf::from(format!("{}.{}", self.path.display(), index));
                if from.exists() {
                    if to.exists() {
                        fs::remove_file(&to)?;
                    }
                    fs::rename(from, to)?;
                }
            }
            self.file = File::create(&self.path)?;
            self.size = 0;
        }
        self.file.write_all(&data)?;
        self.file.flush()?;
        self.size += data.len() as u64;
        Ok(())
    }
}
pub fn replay(path: &Path) -> Result<Vec<Snapshot>> {
    let file = File::open(path)?;
    anyhow::ensure!(
        file.metadata()?.len() <= 512 * 1024 * 1024,
        "Replay exceeds 512 MiB; split the recording first"
    );
    let reader = BufReader::new(file);
    let mut result = vec![];
    let mut bytes = 0usize;
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        bytes += line.len();
        anyhow::ensure!(
            bytes <= 512 * 1024 * 1024,
            "Replay exceeds 512 MiB; split the recording first"
        );
        if line.trim().is_empty() {
            continue;
        }
        result.push(
            serde_json::from_str(&line)
                .with_context(|| format!("Invalid recording line {}", index + 1))?,
        );
    }
    anyhow::ensure!(!result.is_empty(), "Recording contains no snapshots");
    Ok(result)
}
pub fn csv(snapshot: &Snapshot, writer: &mut impl Write, header: bool) -> Result<()> {
    if header {
        writeln!(writer, "timestamp,path,value")?;
    }
    fn visit(v: &serde_json::Value, path: &str, time: u64, w: &mut impl Write) -> Result<()> {
        match v {
            serde_json::Value::Object(object) => {
                for (k, v) in object {
                    visit(v, &format!("{path}.{k}"), time, w)?;
                }
            }
            serde_json::Value::Array(array) => {
                for (i, v) in array.iter().enumerate() {
                    visit(v, &format!("{path}[{i}]"), time, w)?;
                }
            }
            _ => {
                let value = v
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| v.to_string());
                writeln!(
                    w,
                    "{time},\"{}\",\"{}\"",
                    path.replace('"', "\"\""),
                    value.replace('"', "\"\"")
                )?;
            }
        }
        Ok(())
    }
    visit(
        &serde_json::to_value(snapshot)?,
        "system",
        snapshot.timestamp,
        writer,
    )
}
