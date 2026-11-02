/// What to copy into a directory once it has been created.
///
/// `directory` is an absolute folder path; `filter` holds the target
/// suffixes — only files whose name ends with one of them are copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub directory: String,
    pub filter: Vec<String>,
}
