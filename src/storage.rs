//! Crash-safe files: whole files replaced atomically, and append-only logs of
//! checksummed records.

use std::{
    fs::{self, File, OpenOptions},
    io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write},
    path::Path,
};

/// Replaces `path` with `data` so a crash leaves either the old or the new file, never a mix.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_dir(path)
}

/// Makes a rename or a new file in `path`'s directory survive a power cut.
#[cfg(unix)]
fn sync_dir(path: &Path) -> Result<()> {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => File::open(dir)?.sync_all(),
        _ => Ok(()),
    }
}

/// Windows has no handle to sync a directory with; its rename is journaled.
#[cfg(not(unix))]
fn sync_dir(_: &Path) -> Result<()> {
    Ok(())
}

/// An append-only file of records, each framed as `len: u32, crc32: u32, payload`.
pub struct Log {
    file: File,
}

impl Log {
    /// Opens or creates the log and returns its records. A torn or corrupt tail,
    /// which only a crash during `append` leaves, is cut off.
    pub fn open(path: &Path) -> Result<(Self, Vec<Vec<u8>>)> {
        let created = !path.exists();
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        if created {
            sync_dir(path)?;
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut records = Vec::new();
        let mut pos = 0;
        while let Some(record) = frame(&data[pos..]) {
            pos += 8 + record.len();
            records.push(record.to_vec());
        }

        if pos < data.len() {
            file.set_len(pos as u64)?;
            file.sync_all()?;
        }
        file.seek(SeekFrom::End(0))?;
        Ok((Self { file }, records))
    }

    /// Writes one record and returns once it is on the disk.
    pub fn append(&mut self, payload: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(8 + payload.len());
        Self::frame(payload, &mut buf)?;
        self.file.write_all(&buf)?;
        self.file.sync_data()
    }

    /// One record as `append` writes it, onto `out`: for writing a whole log at once.
    pub fn frame(payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
        let len = u32::try_from(payload.len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "record too large"))?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
        out.extend_from_slice(payload);
        Ok(())
    }
}

/// The payload of the first frame in `data`, if it is complete and intact.
fn frame(data: &[u8]) -> Option<&[u8]> {
    let len = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?) as usize;
    let crc = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
    let payload = data.get(8..8 + len)?;
    (crc32fast::hash(payload) == crc).then_some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("iron_oxide_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn log_keeps_records_and_cuts_a_torn_tail() {
        let path = dir("log").join("log");
        {
            let (mut log, records) = Log::open(&path).unwrap();
            assert!(records.is_empty());
            log.append(b"first").unwrap();
            log.append(b"second").unwrap();
        }

        // A crash halfway through the third append.
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[9, 0, 0, 0, 1, 2]).unwrap();
        drop(file);

        let (mut log, records) = Log::open(&path).unwrap();
        assert_eq!(records, [b"first".to_vec(), b"second".to_vec()]);
        log.append(b"third").unwrap();

        let (_, records) = Log::open(&path).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[2], b"third");
    }

    #[test]
    fn atomic_write_replaces() {
        let path = dir("atomic").join("file");
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!path.with_extension("tmp").exists());
    }
}
