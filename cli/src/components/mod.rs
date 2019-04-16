use failure::Error;
use crate::file_upload::FileUpload;

pub mod wasm;
pub mod pwa;
pub mod icon;
pub mod splashscreen;
pub mod landing_page;


/// Implement this trait to extend an AppBuilder to include additional
/// files. See examples in this directory.
pub trait AppComponent {
    /// Returns a collection of file uploads to be added to be added
    /// to the application. Ordering does not matter.
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error>;
}
