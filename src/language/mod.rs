mod classification;
mod patterns;
mod registry;

pub use classification::{classify_line, hash_normalized_line};
pub use registry::supported_file_extensions;
