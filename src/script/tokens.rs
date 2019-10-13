#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    CommandIdentifier,
    Identifier,
    Number,
    String,
    Keyword,

    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    EqualEqual,
    Plus,
    Minus,
    Asterisk,
    AsteriskAsterisk,
    ForwardSlash,
    // TODO Add support for +=, -=, etc.
    Not, // Supports both ! and ~
    NotEqual, // Supports both != and ~=
    Caret,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Whitespace,
    NewLine,
}

#[derive(Debug, Clone, Copy)]
pub enum Keyword {
    If, Then,
    Elseif, Else,

}

#[derive(Debug, Clone)]
pub enum TokenData {
    Keyword(Keyword),
    Identifier(String),
    CommandIdentifier(String),
    Number(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Token(TokenType, Option<TokenData>, usize);

impl Token {
    pub fn new(ttype: TokenType, line: usize) -> Token {
        Token(ttype, None, line)
    }

    pub fn with_data(ttype: TokenType, tdata: TokenData, line: usize) -> Token {
        Token(ttype, Some(tdata), line)
    }

    pub fn token_type(&self) -> &TokenType {
        &self.0
    }

    pub fn token_data(&self) -> Option<&TokenData> {
        self.1.as_ref()
    }

    pub fn line(&self) -> &usize {
        &self.2
    }
}