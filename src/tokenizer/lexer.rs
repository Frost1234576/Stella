use std::fmt::Display;
use std::iter::Peekable;
use std::str::Chars;
use std::sync::Arc;

// ==========================================
// 1. DATA STRUCTURES (The "What")
// ==========================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    // Single-char tokens
    LeftParen, RightParen, LeftBrace, RightBrace,
    LeftBracket, RightBracket, Comma, Dot, Minus, 
    Plus, Semicolon, Slash, QuestionMark, Caret,
    Octothorpe, Tilde,

    // One or two char tokens
    Bang, BangEqual,
    Equal, EqualEqual,
    Greater, GreaterEqual,
    Less, LessEqual, Arrow,
    Colon, DoubleDescriptor, 
    Star, DoubleStar,
    Bar, DoubleBar,
    Ampersand, DoubleAmpersand,

    // Literals
    Identifier, String,
    Int, Long, Float, Double,

    // Keywords
    Class, Else, False, Fun, For, If, Nil,
    Print, Return, Super, This, True, Var, While, Implement, Struct, Import,
    Public, Private, Static, As, PrimitiveVarType, Break, Continue,
    
    // Method Definitions
    Function, Hook, Event, Command, Lambda,

    // End of input
    EOF,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File{
    pub filename: String,
    pub path: String,
    pub ext: String,
    pub contents: String,
}

impl File{
    pub fn new(path: &str) -> Self{
        Self {
            filename: path.to_string(),
            path: path.to_string(),
            ext: path.split('.').last().unwrap_or("").to_string(),
            contents: "".to_string(),
        }
    }

    pub fn read(&mut self) -> std::io::Result<()> {
        self.contents = std::fs::read_to_string(&self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,      // The actual text, e.g., "var" or "123.45"
    pub line: usize,
	pub literal: Option<String>,
    pub file: Arc<File>,
}

pub struct Lexer {
    pos: usize,
    tokens: Vec<Token>,
    curr_line: usize,
    file: Arc<File>,
}

impl Lexer {
    pub fn new(file: Arc<File>) -> Self {
        Self {
            file,
            pos: 0,
            tokens: Vec::new(),
            curr_line: 1,
        }
    }

    fn peek(&self) -> Option<char> { 
        self.file.contents.chars().nth(self.pos) 
    }
    
    fn peek_next(&self) -> Option<char> {
        self.file.contents.chars().nth(self.pos + 1)
    }
    
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        self.pos += 1;
        ch
    }


    /// The main entry point to turn string -> Vec<Token>
    pub fn tokenize(mut self) -> Vec<Token> {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\r' | '\t' => { self.advance(); } // Ignore whitespace
                '\n' => {
                    self.curr_line += 1;
                    self.advance();
                }
                '(' => self.add_token(TokenType::LeftParen, "("),
                ')' => self.add_token(TokenType::RightParen, ")"),
                '{' => self.add_token(TokenType::LeftBrace, "{"),
                '}' => self.add_token(TokenType::RightBrace, "}"),
                '[' => self.add_token(TokenType::LeftBracket, "["),
                ']' => self.add_token(TokenType::RightBracket, "]"),
                ',' => self.add_token(TokenType::Comma, ","),
                '.' => self.add_token(TokenType::Dot, "."),
                '-' => self.match_token('>', TokenType::Arrow, TokenType::Minus),
                '+' => self.add_token(TokenType::Plus, "+"),
                ';' => self.add_token(TokenType::Semicolon, ";"),
                '*' => self.match_token('*', TokenType::DoubleStar, TokenType::Star),
                ':' => self.match_token(':', TokenType::DoubleDescriptor, TokenType::Colon),
                '?' => self.add_token(TokenType::QuestionMark, "?"),
                '^' => self.add_token(TokenType::Caret, "^"),
                '|' => self.match_token('|', TokenType::DoubleBar, TokenType::Bar),
                '&' => self.match_token('&', TokenType::DoubleAmpersand, TokenType::Ampersand),
                '#' => self.add_token(TokenType::Octothorpe, "#"),
                '~' => self.add_token(TokenType::Tilde, "~"),

                // Double character operators
                '!' => self.match_token('=', TokenType::BangEqual, TokenType::Bang),
                '=' => self.match_token('=', TokenType::EqualEqual, TokenType::Equal),
                '<' => self.match_token('=', TokenType::LessEqual, TokenType::Less),
                '>' => self.match_token('=', TokenType::GreaterEqual, TokenType::Greater),

                // Slash or Comment?
                '/' => {
                    self.advance(); // consume first slash
                    if let Some('/') = self.peek() {
                        // It's a comment loop until newline
                        while let Some(ch) = self.peek() {
                            if ch == '\n' { break; }
                            self.advance();
                        }
                    } else {
                        self.push_token(TokenType::Slash, "/".to_string(), None);
                    }
                }

                '"' => self.string(),
                c if c.is_ascii_digit() => self.number(),
                c if c.is_alphabetic() || c == '_' => self.identifier(),

                _ => {
                    eprintln!("Unexpected character on line {}: {}", self.curr_line, c);
                    self.advance();
                }
            }
        }

        // Append EOF token
        self.tokens.push(Token {
            token_type: TokenType::EOF,
            lexeme: "".to_string(),
            literal: None,
            line: self.curr_line,
            file: self.file.clone(),
        });

        self.tokens
    }

    // --- Helper Methods ---

    // Helper for single char tokens
    fn add_token(&mut self, token_type: TokenType, lexeme: &str) {
        self.advance();
        self.push_token(token_type, lexeme.to_string(), None);
    }

    // Helper for operators like '!' vs '!='
    fn match_token(&mut self, expected: char, if_match: TokenType, if_not: TokenType) {
        self.advance(); // consume the first char (e.g. '!')
        
        if let Some(c) = self.peek() {
            if c == expected {
                self.advance(); // consume the second char (e.g. '=')
                // Construct the lexeme manually for display
                let lexeme = match if_match {
                    TokenType::BangEqual => "!=",
                    TokenType::EqualEqual => "==",
                    TokenType::LessEqual => "<=",
                    TokenType::GreaterEqual => ">=",
                    TokenType::Arrow => "->",
                    TokenType::DoubleStar => "**",
                    TokenType::DoubleDescriptor => "::",
                    TokenType::DoubleBar => "||",
                    TokenType::DoubleAmpersand => "&&",
                    _ => "??",
                };
                self.push_token(if_match, lexeme.to_string(), None);
            } else {
                let lexeme = match if_not {
                    TokenType::Bang => "!",
                    TokenType::Equal => "=",
                    TokenType::Less => "<",
                    TokenType::Greater => ">",
                    TokenType::Minus => "-",
                    TokenType::Star => "*",
                    TokenType::Colon => ":",
                    TokenType::Bar => "|",
                    TokenType::Ampersand => "&",
                    _ => "?",
                };
                self.push_token(if_not, lexeme.to_string(), None);
            }
        } else {
            let lexeme = match if_not {
                TokenType::Bang => "!",
                TokenType::Equal => "=",
                TokenType::Less => "<",
                TokenType::Greater => ">",
                TokenType::Minus => "-",
                TokenType::Star => "*",
                TokenType::Colon => ":",
                TokenType::Bar => "|",
                TokenType::Ampersand => "&",
                _ => "?",
            };
            self.push_token(if_not, lexeme.to_string(), None);
        }
    }

    fn string(&mut self) {
        self.advance(); // consume opening "
        let mut value = String::new();
        
        while let Some(c) = self.peek() {
            if c == '"' { break; }
            if c == '\n' { self.curr_line += 1; }
            value.push(c);
            self.advance();
        }

        if self.peek().is_none() {
            eprintln!("Unterminated string at line {}", self.curr_line);
            return;
        }

        self.advance(); // consume closing "
        self.push_token(TokenType::String, format!("\"{}\"", value), Some(value));
    }

    fn number(&mut self) { // Int, Long, Float, Double,
        let mut value = String::new();
        let mut _type = TokenType::Int;
        
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                value.push(c);
                self.advance();
            } else { 
                match c {
                    'd' | 'D' => {
                        _type = TokenType::Double;
                        self.advance();
                    }
                    'f' | 'F' => {
                        _type = TokenType::Float;
                        self.advance();
                    }
                    'l' | 'L' => {
                        _type = TokenType::Long;
                        self.advance();
                    }
                    _ => {}
                }
                break; 
            }
        }

        // Look for fractional part
        if let Some('.') = self.peek() {
            value.push('.');
            self.advance(); // consume dot
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    value.push(c);
                    self.advance();
                } else { 
                    match c {
                        'd' | 'D' => {
                            _type = TokenType::Double;
                            self.advance();
                        }
                        'f' | 'F' => {
                            _type = TokenType::Float;
                            self.advance();
                        }
                        _ => {}
                    }
                    break; 
                }
            }
        }

        self.push_token(_type, value.clone(), Some(value));
    }

    fn identifier(&mut self) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                text.push(c);
                self.advance();
            } else { break; }
        }

        let token_type = match text.as_str() {
            "class" => TokenType::Class,
            "else" => TokenType::Else,
            "false" => TokenType::False,
            "for" => TokenType::For,
            "fun" => TokenType::Fun,
            "if" => TokenType::If,
            "nil" => TokenType::Nil,
            "print" => TokenType::Print,
            "return" => TokenType::Return,
            "super" => TokenType::Super,
            "this" => TokenType::This,
            "true" => TokenType::True,
            "var" => TokenType::Var,
            "while" => TokenType::While,
            "impl" => TokenType::Implement,
            "struct" => TokenType::Struct,
            "import" => TokenType::Import,
            "as" => TokenType::As,

            "break" => TokenType::Break,
            "continue" => TokenType::Continue,

            // Modifiers
            "public" => TokenType::Public,
            "private" => TokenType::Private,
            "static" => TokenType::Static,
            
            // var types
            "int" | "str" | "float" | "double" | "long" | "bool" | "char" | "void" => TokenType::PrimitiveVarType,

            // Method Definitions

            "fn" => TokenType::Function,
            "hook" => TokenType::Hook,
            "event" => TokenType::Event,
            "command" => TokenType::Command,
            "lambda" => TokenType::Lambda,

            _ => TokenType::Identifier,
        };

        self.push_token(token_type, text, None);
    }

    fn push_token(&mut self, token_type: TokenType, lexeme: String, literal: Option<String>) {
        self.tokens.push(Token {
            token_type,
            lexeme,
            literal,
            line: self.curr_line,
            file: self.file.clone(),
        });
    }
}
