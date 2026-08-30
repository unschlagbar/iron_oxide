/// A header list with case-insensitive lookup.
///
/// Header names are case-insensitive per RFC, but the original order and
/// spelling are preserved. A Vec is enough: real requests rarely carry more
/// than a few dozen headers, where linear search beats a HashMap.
#[derive(Debug, Default, Clone)]
pub struct Headers {
    entries: Vec<(String, String)>,
}

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            entries: Vec::with_capacity(cap),
        }
    }

    /// The first value for this name, matched case-insensitively.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// All values for this name. Some headers (Set-Cookie) may appear more
    /// than once and must not be folded together.
    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.entries
            .iter()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Checks whether a comma-separated header contains a token,
    /// e.g. `Connection: keep-alive, Upgrade`.
    pub fn contains_token(&self, name: &str, token: &str) -> bool {
        self.get_all(name).any(|v| {
            v.split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(token))
        })
    }

    /// Appends a header without removing existing ones of the same name.
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.entries.push((name.into(), value.into()));
    }

    /// Sets a header, removing any previous ones of the same name.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        self.entries.retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
        self.entries.push((name, value.into()));
    }

    pub fn remove(&mut self, name: &str) {
        self.entries.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Parses a value as a number, e.g. for Content-Length.
    pub fn get_usize(&self, name: &str) -> Option<usize> {
        self.get(name)?.trim().parse().ok()
    }
}
