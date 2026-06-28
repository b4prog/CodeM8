use std::cmp::Ordering;
use std::fs;

use rayon::prelude::*;
use rust_code_analysis::{get_from_ext, get_function_spaces, FuncSpace, SpaceKind};

use crate::error::{CodeM8Error, Result};
use crate::model::{FunctionComplexity, SourceFile};
use crate::paths::format_path;

const ANONYMOUS_FUNCTION_NAME: &str = "<anonymous>";

pub fn complexity_supported_file_extensions(extensions: &[String]) -> Vec<String> {
    extensions
        .iter()
        .filter(|extension| get_from_ext(extension).is_some())
        .cloned()
        .collect()
}

pub fn detect_complex_functions(
    files: &[SourceFile],
    max_cognitive_complexity: u32,
    max_cyclomatic_complexity: u32,
) -> Result<Vec<FunctionComplexity>> {
    let mut functions = files
        .par_iter()
        .map(|file| {
            detect_file_complex_functions(file, max_cognitive_complexity, max_cyclomatic_complexity)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    functions.sort_by(compare_function_complexity);
    Ok(functions)
}

fn detect_file_complex_functions(
    file: &SourceFile,
    max_cognitive_complexity: u32,
    max_cyclomatic_complexity: u32,
) -> Result<Vec<FunctionComplexity>> {
    let Some(language) = get_from_ext(&file.extension) else {
        return Ok(Vec::new());
    };
    let source = fs::read(&file.path)
        .map_err(|error| CodeM8Error::io(&file.display_path, "read file", &error))?;
    let Some(root_space) = get_function_spaces(&language, source, &file.path, None) else {
        return Ok(Vec::new());
    };
    let mut functions = Vec::new();
    collect_complex_functions(
        file,
        &root_space,
        max_cognitive_complexity,
        max_cyclomatic_complexity,
        &mut functions,
    );
    Ok(functions)
}

fn collect_complex_functions(
    file: &SourceFile,
    space: &FuncSpace,
    max_cognitive_complexity: u32,
    max_cyclomatic_complexity: u32,
    functions: &mut Vec<FunctionComplexity>,
) {
    if space.kind == SpaceKind::Function {
        push_complex_function(
            file,
            space,
            max_cognitive_complexity,
            max_cyclomatic_complexity,
            functions,
        );
    }
    for child in &space.spaces {
        collect_complex_functions(
            file,
            child,
            max_cognitive_complexity,
            max_cyclomatic_complexity,
            functions,
        );
    }
}

fn push_complex_function(
    file: &SourceFile,
    space: &FuncSpace,
    max_cognitive_complexity: u32,
    max_cyclomatic_complexity: u32,
    functions: &mut Vec<FunctionComplexity>,
) {
    let cognitive_complexity = space.metrics.cognitive.cognitive();
    let cyclomatic_complexity = space.metrics.cyclomatic.cyclomatic();
    if cognitive_complexity <= f64::from(max_cognitive_complexity)
        && cyclomatic_complexity <= f64::from(max_cyclomatic_complexity)
    {
        return;
    }
    functions.push(FunctionComplexity {
        file_path: file.display_path.clone(),
        function_name: space
            .name
            .clone()
            .unwrap_or_else(|| ANONYMOUS_FUNCTION_NAME.to_string()),
        start_line: space.start_line,
        end_line: space.end_line,
        cognitive_complexity,
        cyclomatic_complexity,
    });
}

fn compare_function_complexity(left: &FunctionComplexity, right: &FunctionComplexity) -> Ordering {
    format_path(&left.file_path)
        .cmp(&format_path(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.end_line.cmp(&right.end_line))
        .then_with(|| left.function_name.cmp(&right.function_name))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn source_file(extension: &str, contents: &str) -> SourceFile {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "codem8-complexity-detection-{}-{id}.{extension}",
            std::process::id()
        ));
        fs::write(&path, contents).expect("write source file");
        SourceFile {
            path,
            display_path: PathBuf::from(format!("sample.{extension}")),
            extension: extension.to_string(),
        }
    }

    #[test]
    fn filters_unsupported_extensions() {
        let extensions = complexity_supported_file_extensions(&[
            "rs".to_string(),
            "rb".to_string(),
            "ts".to_string(),
        ]);
        assert_eq!(extensions, ["rs", "ts"]);
    }

    #[test]
    fn detects_functions_over_either_limit() {
        let file = source_file(
            "rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n",
        );
        let functions =
            detect_complex_functions(std::slice::from_ref(&file), 1, 1).expect("detect");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].file_path, PathBuf::from("sample.rs"));
        assert!(functions[0].function_name.contains("risky"));
        assert!(functions[0].cognitive_complexity > 1.0);
        assert!(functions[0].cyclomatic_complexity > 1.0);
        fs::remove_file(file.path).expect("cleanup");
    }
}
