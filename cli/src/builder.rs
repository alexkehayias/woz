use std::mem;
use std::sync::{Arc, Mutex};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use rusoto_s3::*;
use rusoto_core::ByteStream;
use failure::Error;
use failure::ResultExt;
use flate2::Compression;
use flate2::write::GzEncoder;
use futures::prelude::*;
use tokio::prelude::*;

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
        let release_flag = match env {
            Environment::Production => " --release",
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
        // In order to get errors out of tokio they need to be share
        // the data in a thread safe way
        let failures = Arc::new(Mutex::new(0));
        // This clone allows us to check the failures after the event
        // loop runs
        let fail_count = Arc::clone(&failures);

        let work = stream::iter_ok(self.files.clone()).for_each(move |f| {
            // References the outer failures. This will get moved into
            // the scope of the async task closure
            let fails_ref = Arc::clone(&failures);

            let FileUpload {filename, mimetype, bytes} = f;
            let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
            gzip.write_all(&bytes).expect("Failed to gzip encode bytes");
            let compressed_bytes = gzip.finish().expect("Failed to gzip file");

            let req = PutObjectRequest {
                bucket: String::from(S3_BUCKET_NAME),
                key: filename.to_owned(),
                body: Some(ByteStream::from(compressed_bytes)),
                content_type: Some(mimetype.to_owned()),
                content_encoding: Some(String::from("gzip")),
                ..Default::default()
            };

            // Add the task so the event loop will pick it up
            tokio::spawn(
                client.put_object(req)
                    // Tokio runtime expects futures to return ()
                    .map(move |_| () )
                    // Tokio runtime expects futures errors to be ()
                    .map_err(move |e| {
                        // Swap out the current count with our new
                        // count of failures.
                        let mut fails = *fails_ref.lock().unwrap();
                        let mut new_fails = fails + 1;
                        mem::swap(&mut fails, &mut new_fails);
                        println!("File upload error: {}", e);
                    })
            )
        });

        // Actually execute the async tasks, blocking the main thread
        // until idle (completed all spawned tasks)
        tokio::run(work);

        if *fail_count.lock().unwrap() > 0 {
            Err(format_err!("Failed to upload app to S3"))
        } else {
            Ok(())
        }

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
