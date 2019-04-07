/// Represents a file to be uploaded. File contents are held in memory
/// as a vector of bytes. This may not be desireable for very large
/// files...
pub struct FileUpload {
    pub filename: String,
    pub mimetype: String,
    pub bytes: Vec<u8>
}

impl FileUpload {
    pub fn new(filename: String, mimetype: String, bytes: Vec<u8>) -> Self {
        Self {filename, mimetype, bytes}
    }
}
