use std::{fs, io::Cursor, path::PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;


pub struct HTTPS {
    pub http_verion: Option<String>,
    pub host: Option<String>,
    pub status: Option<String>
}

impl HTTPS {
    pub fn format(content_type: &[u8], content: &[u8]) -> Vec<u8> {
        let content_len = content.len();
        let mut format = Vec::with_capacity(content_len + 100);
        format.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
        format.extend_from_slice(b"Content-Type: ");
        format.extend_from_slice(content_type);
        format.extend_from_slice(b"\r\ncharset=UTF-8\r\nContent-Length: ");
        format.extend_from_slice(content_len.to_string().as_bytes());
        format.extend_from_slice(b"\r\n\r\n");
        format.extend_from_slice(content);
        format
    }

    pub fn format_content(path: PathBuf) -> Option<Vec<u8>> {

        if path.is_dir() {
            match Self::zip_directory(&path) {
                Ok(zip_data) => {
                    let zip_filename = format!("{}.zip", path.file_name()?.to_string_lossy());
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\n\
                        Content-Type: application/zip\r\n\
                        Content-Disposition: attachment; filename=\"{}\"\r\n\
                        Content-Length: {}\r\n\r\n",
                        zip_filename,
                        zip_data.len()
                    );

                    let mut response = headers.into_bytes();
                    response.extend_from_slice(&zip_data);
                    return Some(response);
                }
                Err(_) => return None,
            }
        }

        let file_content = fs::read(&path);
        match file_content {
            Ok(content) => {
                let content_type: &[u8] = match path.extension() {
                    Some(extention) => match extention.as_encoded_bytes() {
                        b"apng" => b"image/apng",
                        b"png" => b"image/png",
                        b"webp" => b"image/webp",
                        b"gif" => b"image/gif",
                        b"jpeg" => b"image/jpeg",
                        b"svh" => b"image/svg+xml",
                        b"avif" => b"image/avif",
                        b"zip" => b"application/zip",
                        b"json" => b"text/json",
                        b"js" => b"text/javascript",
                        b"wasm" => b"application/wasm",
                        b"html" => b"text/html",
                        b"pdf" => b"application/pdf",
                        b"mp3" => b"audio/mpeg",
                        b"mp4" => b"audio/mp4",
                        b"ogg" => b"audio/ogg",
                        b"wav" => b"audio/wav",
                        b"ico" => b"image/vnd.microsoft.icon",
                        _ => b"text/plain",
                    },
                    None => b"text/html",
                };
                return Some(HTTPS::format(content_type, &content));
            }
            Err(_) => return None,
        }
    }
    
    /// Verzeichnis rekursiv in eine ZIP-Datei packen
    fn zip_directory(dir: &PathBuf) -> Result<Vec<u8>, std::io::Error> {
        let mut zip_data = Vec::new();
        let cursor = Cursor::new(&mut zip_data);
        let mut zip = ZipWriter::new(cursor);

        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        fn add_dir_to_zip(
            zip: &mut ZipWriter<Cursor<&mut Vec<u8>>>,
            dir: &PathBuf,
            base_path: &PathBuf,
            options: FileOptions<()>,
        ) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let relative_path = path.strip_prefix(base_path).unwrap();

                if path.is_dir() {
                    zip.add_directory(relative_path.to_str().unwrap(), options)?;
                    add_dir_to_zip(zip, &path, base_path, options)?;
                } else {
                    let mut file = fs::File::open(&path)?;
                    zip.start_file(relative_path.to_str().unwrap(), options)?;
                    std::io::copy(&mut file, zip)?;
                }
            }
            Ok(())
        }

        add_dir_to_zip(&mut zip, dir, dir, options)?;
        zip.finish()?;
        Ok(zip_data)
    }
}