mod duplicate_detection;
mod duplicate_renderer;

pub(crate) use duplicate_detection::detect_duplicate_blocks;
pub use duplicate_renderer::{render_duplicate_report, DuplicateReport, DuplicateReportTimings};
