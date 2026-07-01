use rust_code_analysis::FuncSpace;

const JAVA_EXTENSION: &str = "java";

pub(super) fn cognitive_complexity(
    extension: &str,
    source: &[u8],
    space: &FuncSpace,
) -> Option<f64> {
    (extension == JAVA_EXTENSION).then(|| {
        let contents = String::from_utf8_lossy(source);
        let source = function_source(&contents, space);
        let tokens = tokenize_java(source);
        JavaCognitiveScanner::new(&tokens).scan()
    })
}

fn function_source<'a>(source: &'a str, space: &FuncSpace) -> &'a str {
    let start_line = space.start_line.saturating_sub(1);
    let line_count = space.end_line.saturating_sub(start_line);
    let start = byte_index_for_line(source, start_line);
    let end = byte_index_for_line(source, start_line + line_count);
    &source[start..end]
}

fn byte_index_for_line(source: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    source
        .match_indices('\n')
        .nth(line.saturating_sub(1))
        .map_or(source.len(), |(index, _)| index + 1)
}

fn tokenize_java(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(token) = read_java_token(ch, &mut chars) {
            tokens.push(token);
        }
    }
    tokens
}

fn read_java_token(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Option<String> {
    if skip_comment_or_quote(ch, chars) {
        return None;
    }
    read_operator_or_symbol(ch, chars).or_else(|| read_identifier_token(ch, chars))
}

fn skip_comment_or_quote(ch: char, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    match ch {
        '/' if chars.peek() == Some(&'/') => {
            skip_line_comment(chars);
            true
        }
        '/' if chars.peek() == Some(&'*') => {
            skip_block_comment(chars);
            true
        }
        '"' | '\'' => {
            skip_quoted(ch, chars);
            true
        }
        _ => false,
    }
}

fn read_operator_or_symbol(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Option<String> {
    match ch {
        '&' if chars.peek() == Some(&'&') => Some(read_double_char_operator("&&", chars)),
        '|' if chars.peek() == Some(&'|') => Some(read_double_char_operator("||", chars)),
        '?' | '{' | '}' | '(' | ')' | ';' => Some(ch.to_string()),
        _ => None,
    }
}

fn read_double_char_operator(
    token: &str,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> String {
    chars.next();
    token.to_string()
}

fn read_identifier_token(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Option<String> {
    (ch.is_alphabetic() || ch == '_').then(|| read_identifier(ch, chars))
}

fn skip_line_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    chars.next();
    for ch in chars.by_ref() {
        if ch == '\n' {
            break;
        }
    }
}

fn skip_block_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    chars.next();
    let mut previous = '\0';
    for ch in chars.by_ref() {
        if previous == '*' && ch == '/' {
            break;
        }
        previous = ch;
    }
}

fn skip_quoted(quote: char, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let mut escaped = false;
    for ch in chars.by_ref() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            break;
        }
    }
}

fn read_identifier(first: char, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut identifier = first.to_string();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_alphanumeric() || ch == '_' {
            identifier.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    identifier
}

struct JavaCognitiveScanner<'a> {
    tokens: &'a [String],
    index: usize,
    nesting: u32,
    pending_blocks: Vec<BlockKind>,
    blocks: Vec<BlockKind>,
    previous_non_header_boolean_operator: Option<BooleanOperator>,
    after_do_block: bool,
    score: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Control,
    Do,
    Plain,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BooleanOperator {
    And,
    Or,
}

impl<'a> JavaCognitiveScanner<'a> {
    const fn new(tokens: &'a [String]) -> Self {
        Self {
            tokens,
            index: 0,
            nesting: 0,
            pending_blocks: Vec::new(),
            blocks: Vec::new(),
            previous_non_header_boolean_operator: None,
            after_do_block: false,
            score: 0,
        }
    }

    fn scan(mut self) -> f64 {
        while self.index < self.tokens.len() {
            self.scan_token();
            self.index += 1;
        }
        f64::from(self.score)
    }

    fn scan_token(&mut self) {
        if self.scan_if_after_else()
            || self.scan_control_token()
            || self.scan_else()
            || self.scan_do()
            || self.scan_question_mark()
            || self.scan_boolean_operator()
            || self.scan_block_boundary()
            || self.scan_statement_boundary()
        {
            return;
        }
        self.after_do_block = false;
    }

    fn current(&self) -> &str {
        self.tokens[self.index].as_str()
    }

    fn previous_is(&self, token: &str) -> bool {
        self.index > 0 && self.tokens[self.index - 1] == token
    }

    fn scan_if_after_else(&mut self) -> bool {
        if self.current() != "if" || !self.previous_is("else") {
            return false;
        }
        self.add_condition_complexity();
        self.pending_blocks.push(BlockKind::Control);
        true
    }

    fn scan_control_token(&mut self) -> bool {
        if !is_control_token(Some(&self.tokens[self.index])) {
            return false;
        }
        if self.current() == "while" && self.after_do_block {
            self.add_condition_complexity();
            self.after_do_block = false;
            self.reset_non_header_boolean_sequence();
            return true;
        }
        self.score += self.nesting + 1;
        self.add_condition_complexity();
        self.pending_blocks.push(BlockKind::Control);
        true
    }

    fn scan_else(&mut self) -> bool {
        if self.current() != "else" {
            return false;
        }
        self.score += 1;
        true
    }

    fn scan_do(&mut self) -> bool {
        if self.current() != "do" {
            return false;
        }
        self.score += self.nesting + 1;
        self.pending_blocks.push(BlockKind::Do);
        true
    }

    fn scan_question_mark(&mut self) -> bool {
        if self.current() != "?" {
            return false;
        }
        self.score += self.nesting + 1;
        true
    }

    fn scan_boolean_operator(&mut self) -> bool {
        if !matches!(self.current(), "&&" | "||") {
            return false;
        }
        self.add_boolean_sequence_complexity();
        true
    }

    fn scan_block_boundary(&mut self) -> bool {
        if self.current() == "{" {
            self.open_block();
            return true;
        }
        if self.current() == "}" {
            self.close_block();
            return true;
        }
        false
    }

    fn scan_statement_boundary(&mut self) -> bool {
        if self.current() != ";" || self.is_inside_condition_header() {
            return false;
        }
        self.pending_blocks.clear();
        self.after_do_block = false;
        self.reset_non_header_boolean_sequence();
        true
    }

    fn open_block(&mut self) {
        let block = self.pending_blocks.pop().unwrap_or(BlockKind::Plain);
        if matches!(block, BlockKind::Control | BlockKind::Do) {
            self.nesting += 1;
        }
        self.blocks.push(block);
        self.reset_non_header_boolean_sequence();
    }

    fn close_block(&mut self) {
        if let Some(block) = self.blocks.pop() {
            if matches!(block, BlockKind::Control | BlockKind::Do) {
                self.nesting = self.nesting.saturating_sub(1);
            }
            self.after_do_block = block == BlockKind::Do;
            if matches!(block, BlockKind::Control | BlockKind::Do) {
                self.pending_blocks.clear();
            }
        }
        self.reset_non_header_boolean_sequence();
    }

    fn add_condition_complexity(&mut self) {
        if self.peek() == Some("(") {
            let end = self.matching_paren_index(self.index + 1);
            self.score += boolean_sequence_complexity(&self.tokens[self.index + 1..end]);
        }
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.index + 1).map(String::as_str)
    }

    fn matching_paren_index(&self, start: usize) -> usize {
        let mut depth = 0usize;
        for index in start..self.tokens.len() {
            match self.tokens[index].as_str() {
                "(" => depth += 1,
                ")" if close_paren(&mut depth) => return index,
                _ => {}
            }
        }
        self.tokens.len()
    }

    fn add_boolean_sequence_complexity(&mut self) {
        if !self.is_inside_condition_header() {
            self.add_non_header_boolean_sequence_complexity();
        }
    }

    fn add_non_header_boolean_sequence_complexity(&mut self) {
        let operator = BooleanOperator::from_token(self.current());
        if self.previous_non_header_boolean_operator != Some(operator) {
            self.score += 1;
        }
        self.previous_non_header_boolean_operator = Some(operator);
    }

    const fn reset_non_header_boolean_sequence(&mut self) {
        self.previous_non_header_boolean_operator = None;
    }

    fn is_inside_condition_header(&self) -> bool {
        let mut depth = 0usize;
        for index in (0..self.index).rev() {
            match self.tokens[index].as_str() {
                ")" => depth += 1,
                "(" if depth == 0 => {
                    return self.is_control_header_start(index);
                }
                "(" => depth = depth.saturating_sub(1),
                "{" | ";" if depth == 0 => return false,
                _ => {}
            }
        }
        false
    }

    fn is_control_header_start(&self, open_paren_index: usize) -> bool {
        let mut index = open_paren_index;
        while index > 0 && self.tokens[index - 1] == "(" {
            index -= 1;
        }
        is_control_token(self.tokens.get(index.wrapping_sub(1)))
    }
}

impl BooleanOperator {
    const fn from_token(token: &str) -> Self {
        match token.as_bytes().first() {
            Some(b'&') => Self::And,
            _ => Self::Or,
        }
    }
}

fn boolean_sequence_complexity(tokens: &[String]) -> u32 {
    let mut score = 0;
    let mut previous_operator: Option<&str> = None;
    for token in tokens {
        if (token == "&&" || token == "||") && previous_operator != Some(token.as_str()) {
            score += 1;
            previous_operator = Some(token.as_str());
        }
    }
    score
}

const fn close_paren(depth: &mut usize) -> bool {
    *depth = depth.saturating_sub(1);
    *depth == 0
}

fn is_control_token(token: Option<&String>) -> bool {
    matches!(
        token.map(String::as_str),
        Some("if" | "for" | "while" | "switch" | "catch")
    )
}

#[cfg(test)]
mod tests {
    use super::{tokenize_java, JavaCognitiveScanner};

    fn assert_java_cognitive_score(source: &str, expected: f64) {
        let tokens = tokenize_java(source);
        let actual = JavaCognitiveScanner::new(&tokens).scan();
        assert!((actual - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn keeps_control_nesting_after_nested_plain_block_closes() {
        let source = "void f() { if (a) { { helper(); } if (b) { helper(); } } }";
        assert_java_cognitive_score(source, 3.0);
    }

    #[test]
    fn does_not_leak_braceless_control_nesting_to_unrelated_block() {
        let source = "void f() { if (a) helper(); { if (b) { helper(); } } }";
        assert_java_cognitive_score(source, 2.0);
    }

    #[test]
    fn treats_do_while_as_one_loop() {
        let source = "void f() { do { if (a) { helper(); } } while (b); }";
        assert_java_cognitive_score(source, 3.0);
    }

    #[test]
    fn groups_non_header_boolean_sequences() {
        let source = "void f() { return a && b && c; }";
        assert_java_cognitive_score(source, 1.0);
    }

    #[test]
    fn treats_nested_control_header_parentheses_as_header() {
        let source = "void f() { if ((a && b)) { helper(); } }";
        assert_java_cognitive_score(source, 2.0);
    }
}
