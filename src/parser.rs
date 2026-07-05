use crate::ast::*;
use crate::lexer::{Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error at {}:{}: {}", self.line, self.col, self.message)
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut stmts = Vec::new();
        while !self.is_eof() {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => return Err(e),
            }
            self.skip_newlines();
        }
        Ok(Program { stmts })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.tokens.get(self.pos), Some(Token { kind: TokenKind::Eof, .. }))
    }

    fn skip_newlines(&mut self) {
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, TokenKind::Newline) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if let Some(tok) = self.peek() {
            self.kind_matches(&tok.kind, kind)
        } else {
            false
        }
    }

    fn kind_matches(&self, a: &TokenKind, b: &TokenKind) -> bool {
        use TokenKind::*;
        match (a, b) {
            (Identifier(_), Identifier(_)) => true,
            (IntLiteral(_), IntLiteral(_)) => true,
            (UIntLiteral(_), UIntLiteral(_)) => true,
            (FloatLiteral(_), FloatLiteral(_)) => true,
            (StringLiteral(_), StringLiteral(_)) => true,
            (FStringLiteral(_), FStringLiteral(_)) => true,
            _ => a == b,
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if let Some(tok) = self.peek() {
            if self.kind_matches(&tok.kind, &kind) {
                return Ok(self.advance());
            }
        }
        let tok = self.tokens.get(self.pos).unwrap_or(&Token {
            kind: TokenKind::Eof,
            line: 0,
            col: 0,
        });
        Err(ParseError {
            message: format!("expected {:?}, got {:?}", kind, tok.kind),
            line: tok.line,
            col: tok.col,
        })
    }

    fn error(&self, msg: String) -> ParseError {
        let tok = self.tokens.get(self.pos).unwrap_or(&Token {
            kind: TokenKind::Eof,
            line: 0,
            col: 0,
        });
        ParseError {
            message: msg,
            line: tok.line,
            col: tok.col,
        }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();
        if self.is_eof() {
            return Err(self.error("unexpected end of file".to_string()));
        }

        let tok = self.peek().unwrap().clone();

        match &tok.kind {
            TokenKind::Let => self.parse_let(false),
            TokenKind::Const => self.parse_let(true),
            TokenKind::Func => self.parse_func_def(false),
            TokenKind::Export => self.parse_export(),
            TokenKind::Import | TokenKind::From => self.parse_import(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Match => self.parse_match(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Throw => self.parse_throw(),
            TokenKind::Try => self.parse_try(),
            TokenKind::Struct => self.parse_struct_def(),
            TokenKind::Break => { self.advance(); Ok(Stmt::Break) }
            TokenKind::Continue => { self.advance(); Ok(Stmt::Continue) }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.skip_newlines();

        if !self.check(&TokenKind::LBrace) {
            return Err(self.error("expected block (use { })".to_string()));
        }

        self.advance(); // consume '{'
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_eof() {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
            self.skip_newlines();
        }

        self.expect(TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_let(&mut self, is_const: bool) -> Result<Stmt, ParseError> {
        self.advance();

        let name = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected identifier, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };

        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            match self.advance() {
                Token { kind: TokenKind::Identifier(t), .. } => Some(t),
                tok => return Err(ParseError {
                    message: format!("expected type, got {:?}", tok.kind),
                    line: tok.line,
                    col: tok.col,
                }),
            }
        } else {
            None
        };

        let value = if self.check(&TokenKind::Assign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.consume_semicolon();

        Ok(Stmt::Let {
            name,
            type_annotation,
            value: value.map(Box::new).unwrap_or(Box::new(Expr::Nil)),
            is_const,
        })
    }

    fn parse_func_def(&mut self, is_export: bool) -> Result<Stmt, ParseError> {
        self.advance();
        let name = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected function name, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };

        let (params, is_vararg) = self.parse_params()?;
        self.skip_newlines();
        let body = self.parse_block()?;

        Ok(Stmt::FuncDef {
            name,
            params,
            is_vararg,
            body,
            is_export,
        })
    }

    fn parse_params(&mut self) -> Result<(Vec<String>, bool), ParseError> {
        let mut params = Vec::new();
        let mut is_vararg = false;

        if !self.check(&TokenKind::LParen) {
            return Ok((params, is_vararg));
        }

        self.advance();
        self.skip_newlines();

        if self.check(&TokenKind::RParen) {
            self.advance();
            return Ok((params, is_vararg));
        }

        loop {
            match self.advance() {
                Token { kind: TokenKind::Identifier(n), .. } => {
                    if n == "..." {
                        is_vararg = true;
                    } else {
                        params.push(n);
                    }
                }
                tok => return Err(ParseError {
                    message: format!("expected parameter name, got {:?}", tok.kind),
                    line: tok.line,
                    col: tok.col,
                }),
            }

            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen)?;
        Ok((params, is_vararg))
    }

    fn parse_export(&mut self) -> Result<Stmt, ParseError> {
        self.advance();

        if self.check(&TokenKind::Func) {
            return self.parse_func_def(true);
        }

        if self.check(&TokenKind::Const) || self.check(&TokenKind::Let) {
            return self.parse_let(self.check(&TokenKind::Const));
        }

        if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut names = Vec::new();
            loop {
                match self.advance() {
                    Token { kind: TokenKind::Identifier(n), .. } => names.push(n),
                    tok => return Err(ParseError {
                        message: format!("expected identifier, got {:?}", tok.kind),
                        line: tok.line,
                        col: tok.col,
                    }),
                }
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            self.consume_semicolon();
            return Ok(Stmt::Export { names });
        }

        Err(self.error("expected func, const, let, or {..} after export".to_string()))
    }

    fn parse_import(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::From) {
            return self.parse_from_import();
        }

        self.advance();

        let module = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected module name, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };

        let alias = if self.check(&TokenKind::As) {
            self.advance();
            match self.advance() {
                Token { kind: TokenKind::Identifier(n), .. } => Some(n),
                tok => return Err(ParseError {
                    message: format!("expected alias name, got {:?}", tok.kind),
                    line: tok.line,
                    col: tok.col,
                }),
            }
        } else {
            None
        };

        self.consume_semicolon();
        Ok(Stmt::Import {
            module,
            alias,
            symbols: Vec::new(),
        })
    }

    fn parse_from_import(&mut self) -> Result<Stmt, ParseError> {
        self.advance();

        let module = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected module name, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };

        self.expect(TokenKind::Import)?;

        let mut symbols = Vec::new();
        loop {
            let name = match self.advance() {
                Token { kind: TokenKind::Identifier(n), .. } => n,
                tok => return Err(ParseError {
                    message: format!("expected identifier, got {:?}", tok.kind),
                    line: tok.line,
                    col: tok.col,
                }),
            };

            let alias = if self.check(&TokenKind::As) {
                self.advance();
                match self.advance() {
                    Token { kind: TokenKind::Identifier(n), .. } => Some(n),
                    tok => return Err(ParseError {
                        message: format!("expected alias name, got {:?}", tok.kind),
                        line: tok.line,
                        col: tok.col,
                    }),
                }
            } else {
                None
            };

            symbols.push((name, alias));

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.consume_semicolon();
        Ok(Stmt::Import {
            module,
            alias: None,
            symbols,
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        self.advance();

        // Optional parentheses around condition
        let has_paren = self.check(&TokenKind::LParen);
        if has_paren {
            self.advance();
        }

        let condition = self.parse_expr()?;

        if has_paren {
            self.expect(TokenKind::RParen)?;
        }
        self.skip_newlines();

        let then_branch = self.parse_block()?;

        let mut elif_branches = Vec::new();
        let mut else_branch = None;

        self.skip_newlines();
        while self.check(&TokenKind::Elif) {
            self.advance();
            let has_paren = self.check(&TokenKind::LParen);
            if has_paren {
                self.advance();
            }
            let cond = self.parse_expr()?;
            if has_paren {
                self.expect(TokenKind::RParen)?;
            }
            self.skip_newlines();
            let body = self.parse_block()?;
            elif_branches.push((cond, body));
            self.skip_newlines();
        }

        if self.check(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            else_branch = Some(self.parse_block()?);
        }

        Ok(Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.advance();

        let has_paren = self.check(&TokenKind::LParen);
        if has_paren {
            self.advance();
        }

        let condition = self.parse_expr()?;

        if has_paren {
            self.expect(TokenKind::RParen)?;
        }
        self.skip_newlines();

        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        let variable = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected variable name, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_match(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        let value = self.parse_expr()?;
        self.skip_newlines();

        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_eof() {
            let pattern = self.parse_expr()?;
            self.expect(TokenKind::FatArrow)?;
            let body = if self.check(&TokenKind::LBrace) {
                self.parse_block()?
            } else {
                vec![self.parse_stmt()?]
            };
            arms.push((pattern, body));
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Match { value, arms })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        if self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) || self.is_eof() {
            self.consume_semicolon();
            Ok(Stmt::Return(None))
        } else {
            let expr = self.parse_expr()?;
            self.consume_semicolon();
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_throw(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        let expr = self.parse_expr()?;
        self.consume_semicolon();
        Ok(Stmt::Throw(expr))
    }

    fn parse_try(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        self.skip_newlines();
        let body = self.parse_block()?;
        self.skip_newlines();

        if !self.check(&TokenKind::Catch) {
            return Err(self.error("expected catch after try".to_string()));
        }
        self.advance();

        let catch_var = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected variable name, got {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        };

        self.skip_newlines();
        let catch_body = self.parse_block()?;

        Ok(Stmt::Try {
            body,
            catch_var,
            catch_body,
        })
    }

    fn parse_struct_def(&mut self) -> Result<Stmt, ParseError> {
        self.advance();
        let name = match self.advance() {
            Token { kind: TokenKind::Identifier(n), .. } => n,
            tok => return Err(ParseError {
                message: format!("expected struct name, got {:?}", tok.kind),
                line: tok.line, col: tok.col,
            }),
        };
        self.skip_newlines();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_eof() {
            methods.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::StructDef { name, methods })
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;
        self.consume_semicolon();
        Ok(Stmt::Expr(expr))
    }

    fn consume_semicolon(&mut self) {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    // Expression parsing

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_or()?;
        if self.check(&TokenKind::Question) {
            self.advance();
            let then_expr = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            expr = Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                op: BinaryOpKind::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                op: BinaryOpKind::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.check(&TokenKind::Eq) || self.check(&TokenKind::Ne) {
            let op = if self.check(&TokenKind::Eq) {
                self.advance();
                BinaryOpKind::Eq
            } else {
                self.advance();
                BinaryOpKind::Ne
            };
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_term()?;
        loop {
            let op = if self.check(&TokenKind::Lt) {
                self.advance();
                BinaryOpKind::Lt
            } else if self.check(&TokenKind::Gt) {
                self.advance();
                BinaryOpKind::Gt
            } else if self.check(&TokenKind::Le) {
                self.advance();
                BinaryOpKind::Le
            } else if self.check(&TokenKind::Ge) {
                self.advance();
                BinaryOpKind::Ge
            } else if self.check(&TokenKind::In) {
                self.advance();
                BinaryOpKind::In
            } else {
                break;
            };
            let right = self.parse_term()?;
            // Check for chained comparison
            if matches!(op, BinaryOpKind::Lt | BinaryOpKind::Gt | BinaryOpKind::Le | BinaryOpKind::Ge)
                && (self.check(&TokenKind::Lt) || self.check(&TokenKind::Gt)
                    || self.check(&TokenKind::Le) || self.check(&TokenKind::Ge))
            {
                let first = Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right.clone()),
                };
                let rest = self.parse_comparison_chain(right)?;
                left = Expr::BinaryOp {
                    op: BinaryOpKind::And,
                    left: Box::new(first),
                    right: Box::new(rest),
                };
            } else {
                left = Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
        }
        Ok(left)
    }

    fn parse_comparison_chain(&mut self, left: Expr) -> Result<Expr, ParseError> {
        let op = if self.check(&TokenKind::Lt) {
            self.advance();
            BinaryOpKind::Lt
        } else if self.check(&TokenKind::Gt) {
            self.advance();
            BinaryOpKind::Gt
        } else if self.check(&TokenKind::Le) {
            self.advance();
            BinaryOpKind::Le
        } else if self.check(&TokenKind::Ge) {
            self.advance();
            BinaryOpKind::Ge
        } else {
            return Err(self.error("expected comparison operator".to_string()));
        };
        let right = self.parse_term()?;
        if self.check(&TokenKind::Lt) || self.check(&TokenKind::Gt)
            || self.check(&TokenKind::Le) || self.check(&TokenKind::Ge)
        {
            let first = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right.clone()),
            };
            let rest = self.parse_comparison_chain(right)?;
            Ok(Expr::BinaryOp {
                op: BinaryOpKind::And,
                left: Box::new(first),
                right: Box::new(rest),
            })
        } else {
            Ok(Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
        }
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_factor()?;
        loop {
            let op = if self.check(&TokenKind::Plus) {
                self.advance();
                BinaryOpKind::Add
            } else if self.check(&TokenKind::Minus) {
                self.advance();
                BinaryOpKind::Sub
            } else {
                break;
            };
            let right = self.parse_factor()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.check(&TokenKind::DoubleStar) {
                self.advance();
                BinaryOpKind::Pow
            } else if self.check(&TokenKind::Star) {
                self.advance();
                BinaryOpKind::Mul
            } else if self.check(&TokenKind::DoubleSlash) {
                self.advance();
                BinaryOpKind::IntDiv
            } else if self.check(&TokenKind::Slash) {
                self.advance();
                BinaryOpKind::Div
            } else if self.check(&TokenKind::Percent) {
                self.advance();
                BinaryOpKind::Mod
            } else {
                break;
            };
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOpKind::Neg,
                operand: Box::new(operand),
            });
        }
        if self.check(&TokenKind::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOpKind::Not,
                operand: Box::new(operand),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LParen) {
                self.advance();
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        // Check for named arg: identifier = expr
                        if self.peek().map(|t| matches!(&t.kind, TokenKind::Identifier(_))).unwrap_or(false) && self.peek_next().map(|t| matches!(t.kind, TokenKind::Assign)).unwrap_or(false) {
                            let name = match self.advance() { Token { kind: TokenKind::Identifier(n), .. } => n, _ => unreachable!() };
                            self.advance(); // consume '='
                            let value = self.parse_expr()?;
                            args.push(Expr::NamedArg { name, value: Box::new(value) });
                        } else {
                            args.push(self.parse_expr()?);
                        }
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    is_method: false,
                };
            } else if self.check(&TokenKind::DotDot) {
                self.advance();
                let middle = self.parse_unary()?;
                if self.check(&TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    let step = match &middle {
                        Expr::Int(n) => *n,
                        _ => return Err(self.error("range step must be an integer literal".to_string())),
                    };
                    expr = Expr::Range {
                        start: Box::new(expr),
                        end: Box::new(end),
                        step,
                    };
                } else {
                    expr = Expr::Range {
                        start: Box::new(expr),
                        end: Box::new(middle),
                        step: 1,
                    };
                }
            } else if self.check(&TokenKind::Dot) {
                self.advance();
                let field_tok = self.advance();
                let field_name = match &field_tok.kind {
                    TokenKind::Identifier(n) => Some(n.clone()),
                    other => other.keyword_name().map(|s| s.to_string()),
                };
                match field_name {
                    Some(name) => {
                        if self.check(&TokenKind::LParen) {
                            self.advance();
                            let mut args = Vec::new();
                            if !self.check(&TokenKind::RParen) {
                                loop {
                                    if self.peek().map(|t| matches!(&t.kind, TokenKind::Identifier(_))).unwrap_or(false) && self.peek_next().map(|t| matches!(t.kind, TokenKind::Assign)).unwrap_or(false) {
                                        let n = match self.advance() { Token { kind: TokenKind::Identifier(n), .. } => n, _ => unreachable!() };
                                        self.advance();
                                        let v = self.parse_expr()?;
                                        args.push(Expr::NamedArg { name: n, value: Box::new(v) });
                                    } else {
                                        args.push(self.parse_expr()?);
                                    }
                                    if self.check(&TokenKind::Comma) {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            self.expect(TokenKind::RParen)?;
                            expr = Expr::Call {
                                callee: Box::new(Expr::Attr {
                                    obj: Box::new(expr),
                                    name,
                                }),
                                args,
                                is_method: true,
                            };
                        } else {
                            expr = Expr::Attr {
                                obj: Box::new(expr),
                                name,
                            };
                        }
                    }
                    None if matches!(&field_tok.kind, TokenKind::IntLiteral(_)) => {
                        let n = match &field_tok.kind { TokenKind::IntLiteral(n) => *n, _ => unreachable!() };
                        expr = Expr::Index {
                            obj: Box::new(expr),
                            index: Box::new(Expr::Int(n)),
                        };
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("expected field name after '.', got {:?}", field_tok.kind),
                            line: field_tok.line,
                            col: field_tok.col,
                        })
                    }
                }
            } else if self.check(&TokenKind::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                expr = Expr::Index {
                    obj: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        if self.check(&TokenKind::Assign) {
            self.advance();
            let value = self.parse_expr()?;
            expr = Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
            };
        } else if self.check(&TokenKind::PlusEq) {
            self.advance();
            let value = self.parse_expr()?;
            expr = Expr::OpAssign {
                op: BinaryOpKind::Add,
                target: Box::new(expr),
                value: Box::new(value),
            };
        } else if self.check(&TokenKind::MinusEq) {
            self.advance();
            let value = self.parse_expr()?;
            expr = Expr::OpAssign {
                op: BinaryOpKind::Sub,
                target: Box::new(expr),
                value: Box::new(value),
            };
        } else if self.check(&TokenKind::StarEq) {
            self.advance();
            let value = self.parse_expr()?;
            expr = Expr::OpAssign {
                op: BinaryOpKind::Mul,
                target: Box::new(expr),
                value: Box::new(value),
            };
        } else if self.check(&TokenKind::SlashEq) {
            self.advance();
            let value = self.parse_expr()?;
            expr = Expr::OpAssign {
                op: BinaryOpKind::Div,
                target: Box::new(expr),
                value: Box::new(value),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.advance();

        match tok.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Int(n)),
            TokenKind::UIntLiteral(n) => Ok(Expr::UInt(n)),
            TokenKind::FloatLiteral(n) => Ok(Expr::Float(n)),
            TokenKind::StringLiteral(s) => Ok(Expr::String(s)),
            TokenKind::FStringLiteral(s) => Ok(self.parse_fstring(&s)),
            TokenKind::True => Ok(Expr::Bool(true)),
            TokenKind::False => Ok(Expr::Bool(false)),
            TokenKind::Nil => Ok(Expr::Nil),
            TokenKind::Identifier(name) => Ok(Expr::Identifier(name)),
            TokenKind::LParen => {
                self.skip_newlines();
                if self.check(&TokenKind::RParen) {
                    self.advance();
                    Ok(Expr::Tuple(Vec::new()))
                } else {
                    let first = self.parse_expr()?;
                    self.skip_newlines();
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                        let mut items = vec![first];
                        while !self.check(&TokenKind::RParen) && !self.is_eof() {
                            items.push(self.parse_expr()?);
                            self.skip_newlines();
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                                self.skip_newlines();
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        Ok(Expr::Tuple(items))
                    } else {
                        self.expect(TokenKind::RParen)?;
                        Ok(first)
                    }
                }
            }
            TokenKind::LBracket => {
                let mut items = Vec::new();
                self.skip_newlines();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        self.skip_newlines();
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::List(items))
            }
            TokenKind::LBrace => {
                self.skip_newlines();
                if self.check(&TokenKind::RBrace) {
                    self.advance();
                    return Ok(Expr::Dict(Vec::new()));
                }
                let first = self.parse_expr()?;
                self.skip_newlines();

                if self.check(&TokenKind::Colon) {
                    self.advance();
                    let second = self.parse_expr()?;
                    let mut entries = vec![(first, second)];
                    self.skip_newlines();
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                        let key = self.parse_expr()?;
                        self.skip_newlines();
                        if self.check(&TokenKind::Colon) {
                            self.advance();
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                        } else {
                            return Err(self.error("expected ':' in dict literal".to_string()));
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace)?;
                    Ok(Expr::Dict(entries))
                } else {
                    let mut items = vec![first];
                    self.skip_newlines();
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                        items.push(self.parse_expr()?);
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace)?;
                    Ok(Expr::Set(items))
                }
            }
            TokenKind::Pipe => {
                let mut params = Vec::new();
                if !self.check(&TokenKind::Pipe) {
                    loop {
                        match self.advance() {
                            Token { kind: TokenKind::Identifier(n), .. } => params.push(n),
                            tok => return Err(ParseError {
                                message: format!("expected parameter name, got {:?}", tok.kind),
                                line: tok.line,
                                col: tok.col,
                            }),
                        }
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                if !self.check(&TokenKind::Pipe) {
                    return Err(self.error("expected '|' after lambda parameters".to_string()));
                }
                self.advance();
                let body = self.parse_expr()?;
                Ok(Expr::Lambda {
                    params,
                    body: Box::new(body),
                })
            }
            TokenKind::Func => {
                let (params, is_vararg) = self.parse_params()?;
                self.skip_newlines();
                let body = self.parse_block()?;
                Ok(Expr::Func {
                    name: None,
                    params,
                    is_vararg,
                    body,
                })
            }
            _ => Err(ParseError {
                message: format!("unexpected token {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
            }),
        }
    }

    fn parse_fstring(&self, s: &str) -> Expr {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if !current.is_empty() {
                    parts.push(Expr::String(current.clone()));
                    current.clear();
                }
                let mut expr_str = String::new();
                let mut depth = 1;
                while let Some(c) = chars.next() {
                    if c == '{' {
                        depth += 1;
                        expr_str.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(c);
                    } else {
                        expr_str.push(c);
                    }
                }
                let mut sub_parser = Parser::new(&expr_str);
                if let Ok(prog) = sub_parser.parse() {
                    if let Some(stmt) = prog.stmts.into_iter().next() {
                        if let Stmt::Expr(e) = stmt {
                            parts.push(e);
                        }
                    }
                }
            } else if c == '}' {
                current.push(c);
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            parts.push(Expr::String(current));
        }

        if parts.is_empty() {
            return Expr::String(String::new());
        }
        if parts.len() == 1 {
            return parts.into_iter().next().unwrap();
        }

        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::BinaryOp {
                op: BinaryOpKind::Concat,
                left: Box::new(result),
                right: Box::new(part),
            };
        }
        result
    }
}
