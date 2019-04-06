use std::path::PathBuf;
use rusoto_s3::*;
use rusoto_core::ByteStream;
use failure::Error;
use failure::ResultExt;
use crate::config::S3_BUCKET_NAME;


/// Represents a file to be uploaded. File contents are held in memory
/// as a vector of bytes. This may not be desireable for very large
/// files...
pub struct FileUpload {
    pub key: String,
    pub mimetype: String,
    pub bytes: Vec<u8>
}

impl FileUpload {
    pub fn new(key: String, mimetype: String, bytes: Vec<u8>) -> Self {
        Self {key, mimetype, bytes}
    }
}

pub trait AppComponent {
    /// Returns a collection of file uploads to be added to be added
    /// to the application. Ordering does not matter.
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error>;
}

/// Builds the application bundle, a collection of files to be
/// uploaded. You can extend the app build by implementing the
/// AppComponent trait and adding it to the build via
/// the `component` method.
pub struct AppBuilder {
    file_prefix: String,
    pub inner: Vec<FileUpload>,
}

impl AppBuilder {
    pub fn new(file_prefix: String) -> Self {
        Self { file_prefix, inner: Vec::new() }
    }
}

impl AppBuilder {
    /// Add the component to the build
    pub fn component<T>(&mut self, component: T) -> &mut Self
    where T:AppComponent {
        let files = component.files(&self.file_prefix).unwrap();
        for f in files.into_iter() {
            self.inner.push(f);
        };
        self
    }

    /// Returns the size in bytes of the overall app file bundle
    pub fn size(&self) -> usize {
        let mut size = 0;
        for FileUpload {bytes, ..} in self.inner.iter() {
            size += bytes.len();
        }
        size
    }

    /// Upload the app file bundle to S3. It will be immediately
    /// available on the public internet.
    pub fn upload(&self, client: S3Client) -> Result<(), Error> {
        for FileUpload {key, mimetype, bytes} in self.inner.iter() {
            let req = PutObjectRequest {
                bucket: String::from(S3_BUCKET_NAME),
                key: key.to_owned(),
                body: Some(ByteStream::from(bytes.to_owned())),
                content_type: Some(mimetype.to_owned()),
                ..Default::default()
            };

            client.put_object(req)
                .sync()
                .context(format!("Failed to upload file to S3: {}", key))?;
        };
        Ok(())
    }

    /// Download the app bundle to disk at the specified directory.
    pub fn download(&self, dir: PathBuf) -> Result<(), Error> {
        unimplemented!("TODO");
    }
}
