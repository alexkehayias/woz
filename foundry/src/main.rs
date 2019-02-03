use std::path::Path;
use std::fs::{File};
use std::io::{Read};
use std::time::Duration;
use std::error::Error;

use rusoto_core::{Region, ByteStream, RusotoFuture};
use rusoto_s3::{S3, S3Client, PutObjectRequest, PutObjectOutput, PutObjectError};
#[macro_use]
use lambda_runtime::*;

pub mod lambda;


pub enum MimeType {
    HTML,
    CSS,
    PNG,
    JavaScript,
}

impl MimeType {
    pub fn to_string(&self) -> String {
        let s = match self {
            MimeType::HTML => "text/html",
            MimeType::CSS => "text/css",
            MimeType::JavaScript => "application/javascript",
            MimeType::PNG => "image/png",
        };
        String::from(s)
    }
}

#[cfg(test)]
mod mimetype_test {
    use super::*;

    #[test]
    fn to_string() {
        assert_eq!(String::from("text/html"), MimeType::HTML.to_string());
    }
}

/// Returns a future upload to S3
pub fn upload_to_s3(file_path: &Path,
                    mimetype: &MimeType,
                    bucket: &String,
                    key: &String) -> RusotoFuture<PutObjectOutput, PutObjectError> {
    let mut file = File::open(&file_path).expect("File open failed");
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let client = S3Client::new(Region::UsWest2);
    let acl = String::from("private");
    let body = ByteStream::from(buffer);
    let content_type = mimetype.to_string();
    let request = PutObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        body: Some(body),
        acl: Some(acl),
        content_type: Some(content_type),
        ..PutObjectRequest::default()
    };
    client.put_object(request)
        .with_timeout(Duration::from_secs(300))
}

type FilePath = String;
type S3KeyPath = String;

fn build() -> Vec<(FilePath, MimeType, S3KeyPath)> {
    let accum = Vec::new();
    // TODO do stuff
    println!("Building");
    accum
}

fn main() -> Result<(), Box<dyn Error>> {
    // TODO Build the project
    // Return a list of files to be uploaded
    build();

    // TODO deploy
    // let file_path = Path::new("test/file.js");
    // let mimetype = MimeType::JavaScript;
    // // TODO construct this based on environment
    // let bucket = String::from("wasmddev");
    // // TODO come up with a partitioning scheme
    // let key = String::from("user123/file.js");
    // upload_to_s3(&file_path, &mimetype, &bucket, &key)
    //     .sync()
    //     .expect("Failed to upload to S3");
    println!("Done");
    lambda!(lambda::handler);
    Ok(())
}
