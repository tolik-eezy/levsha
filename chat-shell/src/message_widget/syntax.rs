//! Lightweight syntax highlighting using simple token scanning.
//!
//! Produces Pango markup `<span>` tags with foreground colors that match
//! the warm parchment theme of Levsha OS.

/// Syntax highlight colors (warm-toned, matching the parchment theme).
mod colors {
    pub const KEYWORD: &str = "#9B6A4A"; // dark copper/ochre
    pub const STRING: &str = "#6B8E5A"; // olive green
    pub const COMMENT: &str = "#A89B8C"; // muted gray-brown
    pub const NUMBER: &str = "#B5694A"; // terracotta
    pub const FUNCTION: &str = "#7A6B8A"; // muted purple
    pub const TYPE: &str = "#5A7A8A"; // slate blue
    pub const OPERATOR: &str = "#6B5D4F"; // secondary text
}

/// Highlight source code and return Pango markup.
///
/// The input `code` must NOT be XML-escaped yet; this function handles
/// escaping internally so that the `<span>` tags it inserts remain valid.
pub fn highlight_code(language: &str, code: &str) -> String {
    let lang = language.to_lowercase();
    let tokens = tokenize(&lang, code);
    tokens_to_pango(&tokens)
}

// ── Token Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Operator,
    Plain,
}

#[derive(Debug)]
struct Token {
    kind: TokenKind,
    text: String,
}

// ── Language Keyword Lists ─────────────────────────────────────────

fn keywords_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" | "py" => &[
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
            "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
            "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
            "return", "try", "while", "with", "yield",
        ],
        "rust" | "rs" => &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while",
        ],
        "bash" | "sh" | "shell" | "zsh" => &[
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "in",
            "function", "return", "local", "export", "source", "exit", "break", "continue",
            "shift", "unset", "readonly", "declare", "typeset", "set",
        ],
        "javascript" | "js" | "typescript" | "ts" => &[
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "let",
            "new",
            "null",
            "of",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "yield",
        ],
        "go" | "golang" => &[
            "break",
            "case",
            "chan",
            "const",
            "continue",
            "default",
            "defer",
            "else",
            "fallthrough",
            "for",
            "func",
            "go",
            "goto",
            "if",
            "import",
            "interface",
            "map",
            "package",
            "range",
            "return",
            "select",
            "struct",
            "switch",
            "type",
            "var",
        ],
        _ => &[],
    }
}

fn builtin_types_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" | "rs" => &[
            "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
            "u32", "u64", "u128", "usize", "str", "String", "Vec", "Option", "Result", "Box", "Rc",
            "Arc", "HashMap", "HashSet", "BTreeMap",
        ],
        "python" | "py" => &[
            "int",
            "float",
            "str",
            "bool",
            "list",
            "dict",
            "tuple",
            "set",
            "bytes",
            "type",
            "object",
            "range",
            "frozenset",
            "complex",
        ],
        "go" | "golang" => &[
            "bool",
            "byte",
            "complex64",
            "complex128",
            "error",
            "float32",
            "float64",
            "int",
            "int8",
            "int16",
            "int32",
            "int64",
            "rune",
            "string",
            "uint",
            "uint8",
            "uint16",
            "uint32",
            "uint64",
            "uintptr",
        ],
        "javascript" | "js" | "typescript" | "ts" => &[
            "Array",
            "Boolean",
            "Date",
            "Error",
            "Function",
            "Map",
            "Number",
            "Object",
            "Promise",
            "RegExp",
            "Set",
            "String",
            "Symbol",
            "undefined",
        ],
        _ => &[],
    }
}

fn builtins_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" | "py" => &[
            "print",
            "len",
            "range",
            "enumerate",
            "zip",
            "map",
            "filter",
            "sorted",
            "reversed",
            "input",
            "open",
            "isinstance",
            "issubclass",
            "hasattr",
            "getattr",
            "setattr",
            "super",
            "property",
        ],
        "bash" | "sh" | "shell" | "zsh" => &[
            "echo", "cd", "ls", "cat", "grep", "sed", "awk", "find", "mkdir", "rm", "cp", "mv",
            "chmod", "chown", "curl", "wget", "tar", "git", "docker", "sudo", "apt", "dnf", "yum",
            "pip",
        ],
        _ => &[],
    }
}

/// Does this language use `#` for line comments?
fn hash_comments(lang: &str) -> bool {
    matches!(
        lang,
        "python" | "py" | "bash" | "sh" | "shell" | "zsh" | "yaml" | "yml" | "toml"
    )
}

/// Does this language use `//` for line comments?
fn slash_comments(lang: &str) -> bool {
    matches!(
        lang,
        "rust"
            | "rs"
            | "javascript"
            | "js"
            | "typescript"
            | "ts"
            | "go"
            | "golang"
            | "c"
            | "cpp"
            | "java"
    )
}

// ── Tokenizer ──────────────────────────────────────────────────────

fn tokenize(lang: &str, code: &str) -> Vec<Token> {
    // JSON and YAML get special handling.
    if lang == "json" {
        return tokenize_json(code);
    }
    if lang == "yaml" || lang == "yml" {
        return tokenize_yaml(code);
    }

    let keywords = keywords_for(lang);
    let types = builtin_types_for(lang);
    let builtins = builtins_for(lang);
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        // Line comments.
        if hash_comments(lang) && chars[i] == '#' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        if slash_comments(lang) && i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Strings (double-quoted and single-quoted).
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }
            tokens.push(Token {
                kind: TokenKind::String,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Numbers.
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            // Hex prefix.
            if chars[i] == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                while i < len && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                    i += 1;
                }
                // Suffix like `f32`, `u64`, `i32`.
                if i < len && chars[i].is_ascii_alphabetic() {
                    while i < len && chars[i].is_ascii_alphanumeric() {
                        i += 1;
                    }
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Identifiers / keywords.
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            // Check if next non-whitespace is `(` to detect function calls.
            let next_meaningful = chars[i..].iter().position(|c| !c.is_whitespace());
            let is_call = next_meaningful
                .map(|offset| chars[i + offset] == '(')
                .unwrap_or(false);

            let kind = if keywords.contains(&word.as_str()) {
                TokenKind::Keyword
            } else if types.contains(&word.as_str()) {
                TokenKind::Type
            } else if builtins.contains(&word.as_str()) || is_call {
                TokenKind::Function
            } else {
                TokenKind::Plain
            };
            tokens.push(Token { kind, text: word });
            continue;
        }

        // Operators.
        if is_operator(chars[i]) {
            let start = i;
            // Consume multi-char operators like ==, !=, >=, ->, =>, ::, etc.
            i += 1;
            if i < len && is_operator_continuation(chars[i - 1], chars[i]) {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Whitespace and other characters.
        tokens.push(Token {
            kind: TokenKind::Plain,
            text: chars[i].to_string(),
        });
        i += 1;
    }

    tokens
}

fn is_operator(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '^' | '~' | ':'
    )
}

fn is_operator_continuation(prev: char, cur: char) -> bool {
    matches!(
        (prev, cur),
        ('=' | '!' | '<' | '>', '=')
            | ('-', '>')
            | ('=', '>')
            | ('&', '&')
            | ('|', '|')
            | (':', ':')
            | ('<', '<')
            | ('>', '>')
    )
}

// ── JSON Tokenizer ─────────────────────────────────────────────────

fn tokenize_json(code: &str) -> Vec<Token> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        // Strings.
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();

            // Check if this string is a key (followed by `:` ignoring whitespace).
            let next = chars[i..].iter().position(|c| !c.is_whitespace());
            let is_key = next.map(|off| chars[i + off] == ':').unwrap_or(false);

            tokens.push(Token {
                kind: if is_key {
                    TokenKind::Type
                } else {
                    TokenKind::String
                },
                text,
            });
            continue;
        }

        // Numbers.
        if chars[i].is_ascii_digit()
            || (chars[i] == '-' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if chars[i] == '-' {
                i += 1;
            }
            while i < len
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || chars[i] == 'e'
                    || chars[i] == 'E'
                    || chars[i] == '+'
                    || chars[i] == '-')
            {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Keywords: true, false, null.
        if chars[i].is_ascii_alphabetic() {
            let start = i;
            while i < len && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = match word.as_str() {
                "true" | "false" | "null" => TokenKind::Keyword,
                _ => TokenKind::Plain,
            };
            tokens.push(Token { kind, text: word });
            continue;
        }

        // Everything else.
        tokens.push(Token {
            kind: TokenKind::Plain,
            text: chars[i].to_string(),
        });
        i += 1;
    }

    tokens
}

// ── YAML Tokenizer ─────────────────────────────────────────────────

fn tokenize_yaml(code: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    for line in code.split('\n') {
        if !tokens.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Plain,
                text: "\n".to_string(),
            });
        }

        let trimmed = line.trim_start();

        // Comments.
        if trimmed.starts_with('#') {
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: line.to_string(),
            });
            continue;
        }

        // Key: value pairs.
        if let Some(colon_pos) = trimmed.find(": ") {
            // Leading whitespace.
            let indent = line.len() - trimmed.len();
            if indent > 0 {
                tokens.push(Token {
                    kind: TokenKind::Plain,
                    text: line[..indent].to_string(),
                });
            }
            // Handle list prefix.
            let key_start;
            if trimmed.starts_with("- ") {
                tokens.push(Token {
                    kind: TokenKind::Operator,
                    text: "- ".to_string(),
                });
                key_start = 2;
            } else {
                key_start = 0;
            }
            let key = &trimmed[key_start..colon_pos];
            tokens.push(Token {
                kind: TokenKind::Type,
                text: key.to_string(),
            });
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: ": ".to_string(),
            });
            let value = &trimmed[colon_pos + 2..];
            let vkind = classify_yaml_value(value);
            tokens.push(Token {
                kind: vkind,
                text: value.to_string(),
            });
        } else if trimmed.starts_with("- ") {
            let indent = line.len() - trimmed.len();
            if indent > 0 {
                tokens.push(Token {
                    kind: TokenKind::Plain,
                    text: line[..indent].to_string(),
                });
            }
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: "- ".to_string(),
            });
            let rest = &trimmed[2..];
            tokens.push(Token {
                kind: TokenKind::String,
                text: rest.to_string(),
            });
        } else {
            // Plain line (could be a bare key ending in `:`).
            if trimmed.ends_with(':') {
                let indent = line.len() - trimmed.len();
                if indent > 0 {
                    tokens.push(Token {
                        kind: TokenKind::Plain,
                        text: line[..indent].to_string(),
                    });
                }
                tokens.push(Token {
                    kind: TokenKind::Type,
                    text: trimmed[..trimmed.len() - 1].to_string(),
                });
                tokens.push(Token {
                    kind: TokenKind::Operator,
                    text: ":".to_string(),
                });
            } else {
                tokens.push(Token {
                    kind: TokenKind::Plain,
                    text: line.to_string(),
                });
            }
        }
    }

    tokens
}

fn classify_yaml_value(value: &str) -> TokenKind {
    let v = value.trim();
    if v.is_empty() {
        return TokenKind::Plain;
    }
    if v == "true" || v == "false" || v == "null" || v == "~" {
        return TokenKind::Keyword;
    }
    if v.starts_with('"') || v.starts_with('\'') {
        return TokenKind::String;
    }
    if v.chars()
        .next()
        .map(|c| c.is_ascii_digit() || c == '-')
        .unwrap_or(false)
    {
        if v.parse::<f64>().is_ok() {
            return TokenKind::Number;
        }
    }
    TokenKind::String
}

// ── Pango Markup Output ────────────────────────────────────────────

fn tokens_to_pango(tokens: &[Token]) -> String {
    let mut out = String::with_capacity(tokens.iter().map(|t| t.text.len() + 40).sum());

    for token in tokens {
        let escaped = glib::markup_escape_text(&token.text);
        match token.kind {
            TokenKind::Keyword => {
                out.push_str(&format!(
                    "<span foreground=\"{}\" weight=\"bold\">{}</span>",
                    colors::KEYWORD,
                    escaped
                ));
            }
            TokenKind::String => {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    colors::STRING,
                    escaped
                ));
            }
            TokenKind::Comment => {
                out.push_str(&format!(
                    "<span foreground=\"{}\" style=\"italic\">{}</span>",
                    colors::COMMENT,
                    escaped
                ));
            }
            TokenKind::Number => {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    colors::NUMBER,
                    escaped
                ));
            }
            TokenKind::Function => {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    colors::FUNCTION,
                    escaped
                ));
            }
            TokenKind::Type => {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    colors::TYPE,
                    escaped
                ));
            }
            TokenKind::Operator => {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    colors::OPERATOR,
                    escaped
                ));
            }
            TokenKind::Plain => {
                out.push_str(&escaped);
            }
        }
    }

    out
}
