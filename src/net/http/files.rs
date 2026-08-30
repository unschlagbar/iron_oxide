use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{Method, Request, Response, Status, mime};

/// Serves files from a root directory.
pub struct StaticFiles {
    root: PathBuf,
    /// File served when the path resolves to a directory.
    index: Option<String>,
}

impl StaticFiles {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            index: Some("index.html".to_string()),
        }
    }

    pub fn index(mut self, name: impl Into<String>) -> Self {
        self.index = Some(name.into());
        self
    }

    pub fn no_index(mut self) -> Self {
        self.index = None;
        self
    }

    pub fn serve(&self, req: &Request) -> Response {
        if !matches!(req.method, Method::Get | Method::Head) {
            return Response::error(Status::MethodNotAllowed).with_header("Allow", "GET, HEAD");
        }

        let Some(path) = self.resolve(req.path()) else {
            return Response::error(Status::NotFound);
        };

        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Response::error(Status::NotFound),
        };

        let path = if meta.is_dir() {
            let Some(index) = &self.index else {
                return Response::error(Status::Forbidden);
            };
            let candidate = path.join(index);
            if !candidate.is_file() {
                return Response::error(Status::NotFound);
            }
            candidate
        } else {
            path
        };

        match fs::read(&path) {
            Ok(content) => Response::bytes(Status::Ok, mime::from_path(&path), content),
            Err(_) => Response::error(Status::InternalServerError),
        }
    }

    /// Maps a request path onto a file below `root`.
    ///
    /// Returns `None` as soon as the path would escape the root directory.
    /// This is the only place preventing directory traversal, so nothing is
    /// waved through here.
    fn resolve(&self, request_path: &str) -> Option<PathBuf> {
        let mut out = self.root.clone();

        for segment in request_path.split('/') {
            match segment {
                "" | "." => continue,
                ".." => {
                    // Never ascend past the root.
                    if !out.pop() || !out.starts_with(&self.root) {
                        return None;
                    }
                }
                seg => {
                    // A segment must be neither a separator nor a root, or
                    // `/C:/` and `//etc` could escape.
                    let p = Path::new(seg);
                    if p.components().count() != 1
                        || !matches!(p.components().next(), Some(Component::Normal(_)))
                    {
                        return None;
                    }
                    // A NUL byte would truncate the path at the syscall.
                    if seg.contains('\0') {
                        return None;
                    }
                    out.push(seg);
                }
            }
        }

        // After resolving symlinks the path must still be below the root.
        // The comparison uses the canonicalized root because `root` itself may
        // be relative or a symlink.
        let real_root = fs::canonicalize(&self.root).ok()?;
        let real_path = fs::canonicalize(&out).ok()?;
        if !real_path.starts_with(&real_root) {
            return None;
        }

        Some(real_path)
    }
}
