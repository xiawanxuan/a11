use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use anyhow::{Result, Context};

pub struct ChunkReader {
    file_path: String,
    chunk_size: usize,
    file_size: u64,
}

impl ChunkReader {
    pub fn new<P: AsRef<Path>>(file_path: P, chunk_size: usize) -> Result<Self> {
        let file_path = file_path.as_ref()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
            .to_string();

        let metadata = std::fs::metadata(&file_path)
            .with_context(|| format!("Failed to read metadata for {}", file_path))?;

        Ok(ChunkReader {
            file_path,
            chunk_size,
            file_size: metadata.len(),
        })
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn chunk_count(&self) -> usize {
        if self.file_size == 0 {
            0
        } else {
            (self.file_size as usize + self.chunk_size - 1) / self.chunk_size
        }
    }

    pub fn read_lines<F>(&self, mut callback: F) -> Result<usize>
    where
        F: FnMut(&str, usize) -> Result<bool>,
    {
        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file: {}", self.file_path))?;
        let reader = BufReader::new(file);
        let mut line_count = 0;

        for (index, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {}", index))?;
            line_count = index + 1;
            if !callback(&line, index)? {
                break;
            }
        }

        Ok(line_count)
    }

    pub fn read_chunk_lines(&self, chunk_index: usize) -> Result<Vec<String>> {
        let start = (chunk_index * self.chunk_size) as u64;
        if start >= self.file_size {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(start + self.chunk_size as u64, self.file_size);
        let mut file = File::open(&self.file_path)?;

        if start > 0 {
            file.seek(SeekFrom::Start(start - 1))?;
            let mut byte = [0u8; 1];
            file.read_exact(&mut byte)?;
            if byte[0] != b'\n' {
                let mut rest = Vec::new();
                let mut byte = [0u8; 1];
                loop {
                    if file.read(&mut byte)? == 0 {
                        break;
                    }
                    if byte[0] == b'\n' {
                        break;
                    }
                    rest.push(byte[0]);
                    if (file.stream_position()? - start) as usize >= self.chunk_size {
                        break;
                    }
                }
            }
        }

        let actual_start = file.stream_position()?;
        let read_size = (end - actual_start) as usize + self.chunk_size / 10;
        let mut buffer = vec![0u8; read_size];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        let content = String::from_utf8_lossy(&buffer);
        let lines: Vec<String> = content.lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        Ok(lines)
    }

    pub fn get_line_count(&self) -> Result<usize> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        Ok(reader.lines().count())
    }

    pub fn read_head(&self, n: usize) -> Result<Vec<String>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::with_capacity(n);

        for line in reader.lines().take(n) {
            lines.push(line?);
        }

        Ok(lines)
    }

    pub fn read_tail(&self, n: usize) -> Result<Vec<String>> {
        let mut file = File::open(&self.file_path)?;
        let file_size = self.file_size;

        if file_size == 0 {
            return Ok(Vec::new());
        }

        let mut pos = file_size;
        let mut lines = Vec::new();
        let mut buffer = Vec::new();
        let chunk = std::cmp::min(8192, file_size as usize);

        while lines.len() < n && pos > 0 {
            let read_size = std::cmp::min(chunk as u64, pos);
            pos -= read_size;
            file.seek(SeekFrom::Start(pos))?;

            let mut chunk_buf = vec![0u8; read_size as usize];
            file.read_exact(&mut chunk_buf)?;

            for &byte in chunk_buf.iter().rev() {
                if byte == b'\n' && !buffer.is_empty() {
                    buffer.reverse();
                    let line = String::from_utf8_lossy(&buffer).to_string();
                    lines.push(line);
                    buffer.clear();
                    if lines.len() >= n {
                        break;
                    }
                } else {
                    buffer.push(byte);
                }
            }

            if lines.len() >= n {
                break;
            }
        }

        if !buffer.is_empty() && lines.len() < n {
            buffer.reverse();
            let line = String::from_utf8_lossy(&buffer).to_string();
            lines.push(line);
        }

        lines.reverse();
        Ok(lines)
    }

    pub fn iter_chunks(&self) -> ChunkIterator<'_> {
        ChunkIterator {
            reader: self,
            current_chunk: 0,
        }
    }
}

pub struct ChunkIterator<'a> {
    reader: &'a ChunkReader,
    current_chunk: usize,
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_chunk >= self.reader.chunk_count() {
            None
        } else {
            let chunk = self.reader.read_chunk_lines(self.current_chunk);
            self.current_chunk += 1;
            Some(chunk)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(lines: usize) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for i in 0..lines {
            writeln!(file, "Line {}: test log message content here", i).unwrap();
        }
        file
    }

    #[test]
    fn test_read_head() {
        let file = create_test_file(100);
        let reader = ChunkReader::new(file.path(), 4096).unwrap();
        let head = reader.read_head(10).unwrap();
        assert_eq!(head.len(), 10);
        assert_eq!(head[0], "Line 0: test log message content here");
    }

    #[test]
    fn test_read_tail() {
        let file = create_test_file(100);
        let reader = ChunkReader::new(file.path(), 4096).unwrap();
        let tail = reader.read_tail(5).unwrap();
        assert_eq!(tail.len(), 5);
        assert!(tail[0].starts_with("Line 95:"));
    }

    #[test]
    fn test_line_count() {
        let file = create_test_file(50);
        let reader = ChunkReader::new(file.path(), 4096).unwrap();
        assert_eq!(reader.get_line_count().unwrap(), 50);
    }
}
