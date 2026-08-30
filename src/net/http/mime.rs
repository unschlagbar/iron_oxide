use std::path::Path;

/// Content-Type from a file extension. Text types get a charset parameter
/// appended, otherwise browsers guess the encoding.
pub fn from_path(path: &Path) -> &'static str {
    let Some(ext) = path.extension() else {
        return "application/octet-stream";
    };

    // to_ascii_lowercase is not available on OsStr directly, so go via bytes.
    let ext = ext.as_encoded_bytes().to_ascii_lowercase();

    match ext.as_slice() {
        b"html" | b"htm" => "text/html; charset=utf-8",
        b"css" => "text/css; charset=utf-8",
        b"js" | b"mjs" => "text/javascript; charset=utf-8",
        b"json" => "application/json",
        b"xml" => "application/xml",
        b"txt" | b"md" => "text/plain; charset=utf-8",
        b"csv" => "text/csv; charset=utf-8",
        b"svg" => "image/svg+xml",
        b"png" => "image/png",
        b"apng" => "image/apng",
        b"jpg" | b"jpeg" => "image/jpeg",
        b"gif" => "image/gif",
        b"webp" => "image/webp",
        b"avif" => "image/avif",
        b"ico" => "image/vnd.microsoft.icon",
        b"bmp" => "image/bmp",
        b"mp3" => "audio/mpeg",
        b"wav" => "audio/wav",
        b"ogg" => "audio/ogg",
        b"opus" => "audio/opus",
        b"flac" => "audio/flac",
        b"mp4" => "video/mp4",
        b"webm" => "video/webm",
        b"woff" => "font/woff",
        b"woff2" => "font/woff2",
        b"ttf" => "font/ttf",
        b"otf" => "font/otf",
        b"wasm" => "application/wasm",
        b"pdf" => "application/pdf",
        b"zip" => "application/zip",
        b"gz" => "application/gzip",
        b"tar" => "application/x-tar",
        _ => "application/octet-stream",
    }
}
