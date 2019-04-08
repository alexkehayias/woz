use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use rusoto_s3::*;
use rusoto_core::ByteStream;
use failure::Error;
use failure::ResultExt;
use crate::config::S3_BUCKET_NAME;
use crate::file_upload::FileUpload;
use crate::components::AppComponent;
use crate::config::Environment;


/// Builds the application bundle, a collection of files to be
/// uploaded. You can extend the app build by implementing the
/// AppComponent trait and adding it to the build via
/// the `component` method.
pub struct AppBuilder<'a> {
    components: Vec<&'a AppComponent>,
    files: Vec<FileUpload>,
}

impl<'a> AppBuilder<'a> {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            components: Vec::new(),
        }
    }

    /// Adds the component to the build
    pub fn component(&mut self, component: &'a dyn AppComponent) -> &mut Self {
        self.components.push(component);
        self
    }

    /// Returns the size in bytes of the overall app file bundle
    pub fn size(&self) -> usize {
        let mut size = 0;
        for FileUpload {bytes, ..} in self.files.iter() {
            size += bytes.len();
        }
        size
    }

    pub fn build(&mut self, project_path: &PathBuf,
                 file_prefix: &String, env: &Environment) -> Result<(), Error> {
        // Do a cargo build
        // TODO pass in dev or release build
        let release_flag = match env {
            Environment::Release => " --release",
            _ => ""
        };

        let mut build_proc = process::Command::new("sh")
            .current_dir(project_path)
            .arg("-c")
            .arg(format!("cargo build --target wasm32-unknown-unknown{}", release_flag))
            .stdout(process::Stdio::piped())
            .spawn()
            .context("Failed to spawn build")?;
        let exit_code = build_proc.wait().context("Failed to wait for build")?;
        if !exit_code.success() {
            return Err(format_err!("Build failed, please check output above."))
        }

        for cmpnt in self.components.iter() {
            let files = cmpnt.files(file_prefix).unwrap();
            for f in files.into_iter() {
                self.files.push(f);
            };
        };

        Ok(())
    }

    /// Upload the app file bundle to S3. It will be immediately
    /// available on the public internet.
    pub fn upload(&self, client: S3Client) -> Result<(), Error> {
        for FileUpload {filename, mimetype, bytes} in self.files.iter() {
            let req = PutObjectRequest {
                bucket: String::from(S3_BUCKET_NAME),
                key: filename.to_owned(),
                body: Some(ByteStream::from(bytes.to_owned())),
                content_type: Some(mimetype.to_owned()),
                ..Default::default()
            };

            client.put_object(req)
                .sync()
                .context(format!("Failed to upload file to S3: {}", filename))?;
        };
        Ok(())
    }

    /// Download the app bundle to disk
    pub fn download(&self) -> Result<(), Error> {
        for FileUpload {filename, mimetype: _, bytes} in self.files.iter() {
            println!("Downloading file {}", filename);
            let mut dir = PathBuf::from(filename);
            dir.pop();
            fs::create_dir_all(&dir).context("Failed to make directory").ok();

            let mut f = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(PathBuf::from(filename))
                .context("Unable to create or overwrite file")?;

            f.write_all(bytes).context("Unable to write file")?;
        };
        Ok(())
    }
}
