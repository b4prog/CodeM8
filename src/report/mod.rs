mod complexity_detection;
mod complexity_renderer;
mod duplicate_detection;
mod duplicate_renderer;

pub(crate) use complexity_detection::{
    complexity_supported_file_extensions, detect_complex_functions,
};
pub use complexity_renderer::{
    render_complexity_report, ComplexityReport, ComplexityReportTimings,
};
pub(crate) use duplicate_detection::detect_duplicate_blocks;
pub use duplicate_renderer::{render_duplicate_report, DuplicateReport, DuplicateReportTimings};
