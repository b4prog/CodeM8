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
    pending_nesting_blocks: u32,
    score: u32,
}

impl<'a> JavaCognitiveScanner<'a> {
    const fn new(tokens: &'a [String]) -> Self {
        Self {
            tokens,
            index: 0,
            nesting: 0,
            pending_nesting_blocks: 0,
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
        match self.current() {
            "if" if self.previous_is("else") => {
                self.add_condition_complexity();
                self.pending_nesting_blocks += 1;
            }
            "if" | "for" | "while" | "switch" | "catch" => {
                self.score += self.nesting + 1;
                self.add_condition_complexity();
                self.pending_nesting_blocks += 1;
            }
            "else" => self.score += 1,
            "do" | "?" => self.score += self.nesting + 1,
            "&&" | "||" => self.add_boolean_sequence_complexity(),
            "{" => self.open_block(),
            "}" => self.nesting = self.nesting.saturating_sub(1),
            _ => {}
        }
    }

    fn current(&self) -> &str {
        self.tokens[self.index].as_str()
    }

    fn previous_is(&self, token: &str) -> bool {
        self.index > 0 && self.tokens[self.index - 1] == token
    }

    const fn open_block(&mut self) {
        if self.pending_nesting_blocks > 0 {
            self.nesting += 1;
            self.pending_nesting_blocks -= 1;
        }
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
            self.score += 1;
        }
    }

    fn is_inside_condition_header(&self) -> bool {
        let mut depth = 0usize;
        for index in (0..self.index).rev() {
            match self.tokens[index].as_str() {
                ")" => depth += 1,
                "(" if depth == 0 => {
                    return is_control_token(self.tokens.get(index.wrapping_sub(1)))
                }
                "(" => depth = depth.saturating_sub(1),
                "{" | ";" if depth == 0 => return false,
                _ => {}
            }
        }
        false
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
