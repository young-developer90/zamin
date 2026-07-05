use std::fmt;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TokenKind {
    // Literals
    Identifier(String),
    IntLiteral(i64),
    UIntLiteral(u64),
    FloatLiteral(f64),
    StringLiteral(String),
    FStringLiteral(String),

    // Keywords
    Let, Const, Func, Export, Import, From, As,
    If, Else, Elif, While, For, In, Return,
    True, False, Nil,
    Match, Throw, Try, Catch,
    Break, Continue,
    And, Or, Not, Struct,

    // Operators
    Plus, Minus, Star, Slash, Percent,
    DoubleStar, DoubleSlash,
    PlusEq, MinusEq, StarEq, SlashEq,
    Increment, Decrement,
    Eq, Ne, Lt, Gt, Le, Ge,
    Assign,
    Dot, DotDot, Ellipsis,
    Arrow,
    Pipe,
    Colon, Semicolon, Question, Comma,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    FatArrow,

    // Special
    Newline,
    Eof,
    Error(String),
}

impl TokenKind {
    pub fn keyword_name(&self) -> Option<&'static str> {
        match self {
            TokenKind::Let => Some("let"),
            TokenKind::Const => Some("const"),
            TokenKind::Func => Some("func"),
            TokenKind::Export => Some("export"),
            TokenKind::Import => Some("import"),
            TokenKind::From => Some("from"),
            TokenKind::As => Some("as"),
            TokenKind::If => Some("if"),
            TokenKind::Else => Some("else"),
            TokenKind::Elif => Some("elif"),
            TokenKind::While => Some("while"),
            TokenKind::For => Some("for"),
            TokenKind::In => Some("in"),
            TokenKind::Return => Some("return"),
            TokenKind::True => Some("true"),
            TokenKind::False => Some("false"),
            TokenKind::Nil => Some("nil"),
            TokenKind::Match => Some("match"),
            TokenKind::Throw => Some("throw"),
            TokenKind::Try => Some("try"),
            TokenKind::Catch => Some("catch"),
            TokenKind::Break => Some("break"),
            TokenKind::Continue => Some("continue"),
            TokenKind::And => Some("and"),
            TokenKind::Or => Some("or"),
            TokenKind::Not => Some("not"),
            TokenKind::Struct => Some("struct"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    start_line: usize,
    start_col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            start_line: 1,
            start_col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '*' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }
    }

    fn read_string(&mut self, quote: char) -> Result<String, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err("unterminated string".to_string()),
                Some(c) if c == quote => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\"') => s.push('\"'),
                        Some('\\') => s.push('\\'),
                        Some('0') => s.push('\0'),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => return Err("unterminated string escape".to_string()),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn read_multiline_string(&mut self) -> Result<String, String> {
        let mut s = String::new();
        let mut quote_count = 0;
        loop {
            match self.advance() {
                None => return Err("unterminated multiline string".to_string()),
                Some('\"') => {
                    quote_count += 1;
                    if quote_count == 3 {
                        break;
                    }
                }
                Some(c) => {
                    for _ in 0..quote_count {
                        s.push('\"');
                    }
                    quote_count = 0;
                    s.push(c);
                }
            }
        }
        if s.starts_with('\n') {
            s.remove(0);
        }
        Ok(s)
    }

    fn read_fstring(&mut self) -> Result<String, String> {
        let mut s = String::new();
        let mut depth = 0;
        loop {
            match self.advance() {
                None => return Err("unterminated f-string".to_string()),
                Some('\"') if depth == 0 => break,
                Some('{') => {
                    depth += 1;
                    s.push('{');
                }
                Some('}') => {
                    depth -= 1;
                    s.push('}');
                }
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\"') => s.push('\"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => return Err("unterminated f-string escape".to_string()),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn read_number(&mut self, first: char) -> TokenKind {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    s.push(c);
                }
                self.advance();
            } else if c == '.' && !is_float {
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        is_float = true;
                        s.push('.');
                        self.advance();
                        continue;
                    }
                }
                break;
            } else {
                break;
            }
        }

        if is_float {
            TokenKind::FloatLiteral(s.parse::<f64>().unwrap_or(0.0))
        } else {
            let n: i64 = s.parse().unwrap_or(0);
            TokenKind::IntLiteral(n)
        }
    }

    fn read_identifier(&mut self, first: char) -> TokenKind {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "func" => TokenKind::Func,
            "export" => TokenKind::Export,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "elif" => TokenKind::Elif,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "match" => TokenKind::Match,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "struct" => TokenKind::Struct,
            _ => TokenKind::Identifier(s),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let c = match self.advance() {
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        line: self.line,
                        col: self.col,
                    });
                    return tokens;
                }
                Some(c) => c,
            };

            self.start_line = self.line;
            self.start_col = self.col;

            let token = match c {
                ' ' | '\t' | '\r' => continue,
                '\n' => {
                    Some(Token {
                        kind: TokenKind::Newline,
                        line: self.line - 1,
                        col: self.col,
                    })
                }
                '/' if self.peek() == Some('/') => {
                    self.skip_line_comment();
                    continue;
                }
                '/' if self.peek() == Some('*') => {
                    self.advance();
                    self.skip_block_comment();
                    continue;
                }
                '"' => {
                    if self.peek() == Some('"') && self.peek_next() == Some('"') {
                        self.advance();
                        self.advance();
                        match self.read_multiline_string() {
                            Ok(s) => Some(Token {
                                kind: TokenKind::StringLiteral(s),
                                line: self.start_line,
                                col: self.start_col,
                            }),
                            Err(e) => Some(Token {
                                kind: TokenKind::Error(e),
                                line: self.start_line,
                                col: self.start_col,
                            }),
                        }
                    } else {
                        match self.read_string('"') {
                            Ok(s) => Some(Token {
                                kind: TokenKind::StringLiteral(s),
                                line: self.start_line,
                                col: self.start_col,
                            }),
                            Err(e) => Some(Token {
                                kind: TokenKind::Error(e),
                                line: self.start_line,
                                col: self.start_col,
                            }),
                        }
                    }
                }
                'f' if self.peek() == Some('"') => {
                    self.advance();
                    match self.read_fstring() {
                        Ok(s) => Some(Token {
                            kind: TokenKind::FStringLiteral(s),
                            line: self.start_line,
                            col: self.start_col,
                        }),
                        Err(e) => Some(Token {
                            kind: TokenKind::Error(e),
                            line: self.start_line,
                            col: self.start_col,
                        }),
                    }
                }
                c if c.is_ascii_digit() => Some(Token {
                    kind: self.read_number(c),
                    line: self.start_line,
                    col: self.start_col,
                }),
                c if c.is_alphabetic() || c == '_' => Some(Token {
                    kind: self.read_identifier(c),
                    line: self.start_line,
                    col: self.start_col,
                }),
                '+' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::PlusEq,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('+') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Increment,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Plus,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '-' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::MinusEq,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('-') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Decrement,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('>') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Arrow,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Minus,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '*' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::StarEq,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('*') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::DoubleStar,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Star,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '/' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::SlashEq,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('/') => {
                        self.skip_line_comment();
                        continue;
                    }
                    Some('*') => {
                        self.advance();
                        self.skip_block_comment();
                        continue;
                    }
                    _ => Some(Token {
                        kind: TokenKind::Slash,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '%' => Some(Token {
                    kind: TokenKind::Percent,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '=' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Eq,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    Some('>') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::FatArrow,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Assign,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '!' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Ne,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Error("unexpected '!'".to_string()),
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '<' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Le,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Lt,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '>' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        Some(Token {
                            kind: TokenKind::Ge,
                            line: self.start_line,
                            col: self.start_col,
                        })
                    }
                    _ => Some(Token {
                        kind: TokenKind::Gt,
                        line: self.start_line,
                        col: self.start_col,
                    }),
                },
                '.' => match self.peek() {
                    Some('.') => {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            Some(Token {
                                kind: TokenKind::Ellipsis,
                                line: self.start_line,
                                col: self.start_col,
                            })
                        } else {
                            Some(Token {
                                kind: TokenKind::DotDot,
                                line: self.start_line,
                                col: self.start_col,
                            })
                        }
                    }
                    _ => Some(Token {
                        kind: TokenKind::Dot,
                        line: self.start_line,
                        col: self.start_col,
                    })
                },
                '|' => Some(Token {
                    kind: TokenKind::Pipe,
                    line: self.start_line,
                    col: self.start_col,
                }),
                ':' => Some(Token {
                    kind: TokenKind::Colon,
                    line: self.start_line,
                    col: self.start_col,
                }),
                ';' => Some(Token {
                    kind: TokenKind::Semicolon,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '?' => Some(Token {
                    kind: TokenKind::Question,
                    line: self.start_line,
                    col: self.start_col,
                }),
                ',' => Some(Token {
                    kind: TokenKind::Comma,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '(' => Some(Token {
                    kind: TokenKind::LParen,
                    line: self.start_line,
                    col: self.start_col,
                }),
                ')' => Some(Token {
                    kind: TokenKind::RParen,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '{' => Some(Token {
                    kind: TokenKind::LBrace,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '}' => Some(Token {
                    kind: TokenKind::RBrace,
                    line: self.start_line,
                    col: self.start_col,
                }),
                '[' => Some(Token {
                    kind: TokenKind::LBracket,
                    line: self.start_line,
                    col: self.start_col,
                }),
                ']' => Some(Token {
                    kind: TokenKind::RBracket,
                    line: self.start_line,
                    col: self.start_col,
                }),
                _ => Some(Token {
                    kind: TokenKind::Error(format!("unexpected character '{}'", c)),
                    line: self.start_line,
                    col: self.start_col,
                }),
            };

            if let Some(tok) = token {
                tokens.push(tok);
            }
        }
    }
}
