use crate::IntoValue;
use crate::{Agent, Value};
use std::collections::{HashMap, VecDeque};
use std::iter::Peekable;
use std::ops::{Div, Mul, Rem, Sub};
use std::str::Chars;

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Pow,
    PowAssign,
    Div,
    DivAssign,
    Mod,
    ModAssign,
    LeftShift,
    RightShift,
    GreaterThan,
    GreaterThanOrEqual,
    Not,
    LessThan,
    LessThanOrEqual,
    BitwiseAND,
    BitwiseOR,
    BitwiseXOR,
    BitwiseNOT,
    LogicalAND,
    LogicalOR,
    Assign,
    Equal,
    NotEqual,
    Typeof,
    Void,
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    NumberLiteral(f64),
    StringLiteral(String),
    Identifier(String),
    Operator(Operator),
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Null,
    True,
    False,
    This,
    Function,
    Arrow,
    Class,
    New,
    Let,
    Const,
    Semicolon,
    Colon,
    Question,
    Dot,
    BackQuote,
    At,
    Comma,
    Throw,
    Break,
    Continue,
    Try,
    Catch,
    Finally,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Import,
    Export,
    Default,
    From,
    Async,
    Await,
    Gen,
    Yield,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    NullLiteral,
    TrueLiteral,
    FalseLiteral,
    NumberLiteral(f64),
    StringLiteral(String),
    SymbolLiteral(String),
    RegexLiteral(String),
    TemplateLiteral(Vec<String>, Vec<Node>), // quasis, expressions
    Initializer(String, Box<Node>),          // name, value
    ObjectLiteral(Vec<Node>),                // initialiers
    ObjectInitializer(Box<Node>, Box<Node>), // name, value
    ArrayLiteral(Vec<Node>),
    TupleLiteral(Vec<Node>), // items
    Identifier(String),
    BlockStatement(Vec<Node>, HashMap<String, bool>, bool), // nodes, declarations, top
    ReturnStatement(Box<Node>),
    ThrowStatement(Box<Node>),
    IfStatement(Box<Node>, Box<Node>), // test, consequent
    IfElseStatement(Box<Node>, Box<Node>, Box<Node>), // test, consequent, alternative
    WhileStatement(Box<Node>, Box<Node>), // test, body
    ForStatement(bool, String, Box<Node>, Box<Node>), // async, binding, target, body
    BreakStatement,
    ContinueStatement,
    TryStatement(
        Box<Node>,         // try clause
        Option<String>,    // catch binding
        Option<Box<Node>>, // catch clause
        Option<Box<Node>>, // finally clause
    ),
    ExpressionStatement(Box<Node>),
    NewExpression(Box<Node>),
    MemberExpression(Box<Node>, String), // base, property
    ComputedMemberExpression(Box<Node>, Box<Node>), // base, property expression
    ThisExpression,
    CallExpression(Box<Node>, Vec<Node>), // callee, arguments
    TailCallExpression(Box<Node>, Vec<Node>), // callee, arguments
    UnaryExpression(Operator, Box<Node>), // op x
    BinaryExpression(Box<Node>, Operator, Box<Node>), // x op y
    ConditionalExpression(Box<Node>, Box<Node>, Box<Node>), // test, consequent, alternative
    FunctionDeclaration(String, Vec<Node>, Box<Node>, FunctionKind), // name, args, body
    FunctionExpression(Option<String>, Vec<Node>, Box<Node>, FunctionKind), // name, args, body
    ArrowFunctionExpression(Vec<Node>, Box<Node>, FunctionKind), // args, body
    ParenthesizedExpression(Box<Node>),   // expr
    AwaitExpression(Box<Node>),
    YieldExpression(Option<Box<Node>>),
    LexicalInitialization(String, Box<Node>), // identifier, initial value
    ImportDeclaration(String),                // specifier
    ImportNamedDeclaration(String, Vec<String>), // specifier, bindings
    ImportDefaultDeclaration(String, String), // specifier, binding
    ImportStandardDeclaration(String, Vec<String>), // namespace, bindings
    ExportDeclaration(Box<Node>),
}

#[derive(Debug)]
pub enum Error {
    NormalEOF,
    UnexpectedEOF,
    UnexpectedToken,
    DuplicateBinding,
}

impl IntoValue for Error {
    fn into_value(&self, agent: &Agent) -> Value {
        Value::new_error(agent, "parsing error")
    }
}

#[derive(Debug)]
pub struct SourcePosition {
    pub index: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    peeked: Option<Option<Token>>,
    index: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a str) -> Lexer<'a> {
        Lexer {
            peeked: None,
            chars: code.chars().peekable(),
            index: 0,
            line: 1,
            column: 0,
        }
    }

    #[inline]
    fn next_char(&mut self) -> Option<char> {
        match self.chars.next() {
            Some('\n') | Some('\r') => {
                self.index += 1;
                self.line += 1;
                self.column = 0;
                Some('\n')
            }
            Some(v) => {
                self.index += 1;
                self.column += 1;
                Some(v)
            }
            None => None,
        }
    }

    fn position(&self) -> SourcePosition {
        SourcePosition {
            index: self.index,
            line: self.line,
            column: self.column,
        }
    }

    fn next(&mut self) -> Option<Token> {
        match self.peeked.take() {
            Some(v) => v,
            None => match self.next_char() {
                Some(char) => match char {
                    ' ' | '\t' | '\r' | '\n' => self.next(),
                    '0'...'9' => {
                        let mut str = char.to_string();
                        let mut one_dot = false;
                        while let Some(c) = self.chars.peek() {
                            match c {
                                '0'...'9' => {
                                    str.push(self.next_char().unwrap());
                                }
                                '.' => {
                                    if !one_dot {
                                        one_dot = true;
                                        str.push(self.next_char().unwrap());
                                    } else {
                                        break;
                                    }
                                }
                                _ => break,
                            }
                        }
                        let num = str
                            .parse::<f64>()
                            .unwrap_or_else(|_| panic!("Invalid number {}", str));
                        Some(Token::NumberLiteral(num))
                    }
                    '"' | '\'' => {
                        let mut str = String::new();
                        while let Some(c) = self.chars.peek() {
                            if c == &char {
                                self.next_char();
                                break;
                            }
                            let c = self.next_char().unwrap();
                            match c {
                                '\\' => match self.next_char().unwrap() {
                                    'n' => str.push('\n'),
                                    't' => str.push('\t'),
                                    '"' => str.push('"'),
                                    '\'' => str.push('\''),
                                    '\\' => str.push('\\'),
                                    c => str.push(c),
                                },
                                '\r' | '\n' => {
                                    panic!("unexpected end of string");
                                }
                                c => str.push(c),
                            }
                        }
                        Some(Token::StringLiteral(str))
                    }
                    'a'...'z' | 'A'...'Z' | '_' => {
                        let mut ident = char.to_string();
                        while let Some(c) = self.chars.peek() {
                            match c {
                                'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => {
                                    ident.push(self.next_char().unwrap())
                                }
                                _ => break,
                            }
                        }
                        // UPDATE parse_identifier WHEN YOU ADD TO THIS LIST!!!!!!
                        Some(match ident.as_ref() {
                            "true" => Token::True,
                            "false" => Token::False,
                            "null" => Token::Null,
                            "this" => Token::This,
                            "class" => Token::Class,
                            "function" => Token::Function,
                            "let" => Token::Let,
                            "const" => Token::Const,
                            "throw" => Token::Throw,
                            "return" => Token::Return,
                            "try" => Token::Try,
                            "catch" => Token::Catch,
                            "finally" => Token::Finally,
                            "break" => Token::Break,
                            "continue" => Token::Continue,
                            "if" => Token::If,
                            "else" => Token::Else,
                            "while" => Token::While,
                            "for" => Token::For,
                            "in" => Token::In,
                            "new" => Token::New,
                            "import" => Token::Import,
                            "export" => Token::Export,
                            "default" => Token::Default,
                            "from" => Token::From,
                            "async" => Token::Async,
                            "await" => Token::Await,
                            "gen" => Token::Gen,
                            "yield" => Token::Yield,
                            "typeof" => Token::Operator(Operator::Typeof),
                            "void" => Token::Operator(Operator::Void),
                            _ => Token::Identifier(ident),
                        })
                    }
                    '{' => Some(Token::LeftBrace),
                    '}' => Some(Token::RightBrace),
                    '[' => Some(Token::LeftBracket),
                    ']' => Some(Token::RightBracket),
                    '(' => Some(Token::LeftParen),
                    ')' => Some(Token::RightParen),
                    ':' => Some(Token::Colon),
                    ';' => Some(Token::Semicolon),
                    '?' => Some(Token::Question),
                    '.' => Some(Token::Dot),
                    ',' => Some(Token::Comma),
                    '`' => Some(Token::BackQuote),
                    '+' => Some(match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::AddAssign)
                        }
                        _ => Token::Operator(Operator::Add),
                    }),
                    '-' => Some(match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::SubAssign)
                        }
                        _ => Token::Operator(Operator::Sub),
                    }),
                    '*' => Some(match self.chars.peek() {
                        Some('*') => {
                            self.next_char();
                            match self.chars.peek() {
                                Some('=') => {
                                    self.next_char();
                                    Token::Operator(Operator::PowAssign)
                                }
                                _ => Token::Operator(Operator::Pow),
                            }
                        }
                        _ => match self.chars.peek() {
                            Some('=') => {
                                self.next_char();
                                Token::Operator(Operator::MulAssign)
                            }
                            _ => Token::Operator(Operator::Mul),
                        },
                    }),
                    '/' => match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Some(Token::Operator(Operator::DivAssign))
                        }
                        Some('*') => {
                            loop {
                                if self.chars.peek() == None {
                                    return None; // Err(Error::UnexpectedEOF);
                                }
                                if let Some('*') = self.next_char() {
                                    if let Some('/') = self.next_char() {
                                        break;
                                    }
                                }
                            }
                            self.next()
                        }
                        Some('/') => {
                            loop {
                                if self.chars.peek() == None {
                                    return None; // Err(Error::UnexpectedEOF);
                                }
                                if let Some('\n') = self.next_char() {
                                    break;
                                }
                            }
                            self.next()
                        }
                        _ => Some(Token::Operator(Operator::Div)),
                    },
                    '%' => Some(match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::ModAssign)
                        }
                        _ => Token::Operator(Operator::Mod),
                    }),
                    '<' => Some(match self.chars.peek() {
                        Some('<') => {
                            self.next_char();
                            Token::Operator(Operator::LeftShift)
                        }
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::LessThanOrEqual)
                        }
                        _ => Token::Operator(Operator::LessThan),
                    }),
                    '!' => Some(match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::NotEqual)
                        }
                        _ => Token::Operator(Operator::Not),
                    }),
                    '>' => Some(match self.chars.peek() {
                        Some('>') => {
                            self.next_char();
                            Token::Operator(Operator::RightShift)
                        }
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::GreaterThanOrEqual)
                        }
                        _ => Token::Operator(Operator::GreaterThan),
                    }),
                    '&' => Some(match self.chars.peek() {
                        Some('&') => {
                            self.next_char();
                            Token::Operator(Operator::LogicalAND)
                        }
                        _ => Token::Operator(Operator::BitwiseAND),
                    }),
                    '|' => Some(match self.chars.peek() {
                        Some('|') => {
                            self.next_char();
                            Token::Operator(Operator::LogicalOR)
                        }
                        _ => Token::Operator(Operator::BitwiseOR),
                    }),
                    '^' => Some(Token::Operator(Operator::BitwiseXOR)),
                    '~' => Some(Token::Operator(Operator::BitwiseNOT)),
                    '=' => Some(match self.chars.peek() {
                        Some('=') => {
                            self.next_char();
                            Token::Operator(Operator::Equal)
                        }
                        Some('>') => {
                            self.next_char();
                            Token::Arrow
                        }
                        _ => Token::Operator(Operator::Assign),
                    }),
                    '@' => Some(Token::At),
                    _ => {
                        panic!("unexpected token {}", char);
                    }
                },
                None => None,
            },
        }
    }

    #[inline]
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next());
        }
        match self.peeked {
            Some(Some(ref value)) => Some(value),
            Some(None) => None,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn peek_immutable(&self) -> Option<&Token> {
        if self.peeked.is_none() {
            panic!();
        }
        match self.peeked {
            Some(Some(ref value)) => Some(value),
            Some(None) => None,
            _ => unreachable!(),
        }
    }

    fn skip_hashbang(&mut self) {
        if self.chars.peek() == Some(&'#') {
            self.next_char();
            if self.chars.peek() == Some(&'!') {
                loop {
                    match self.next_char() {
                        Some('\n') | None => break,
                        _ => {}
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum FunctionKind {
    Normal,
    Async,
    Generator,
}

impl From<u8> for FunctionKind {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, FunctionKind>(n) }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
enum ParseScope {
    TopLevel = 0b0000_0001,
    Block = 0b0000_0010,
    Loop = 0b0000_0100,
    Function = 0b0000_1000,
    AsyncFunction = 0b0001_1000,
    GeneratorFunction = 0b0010_1000,
}

macro_rules! binop_production {
    ( $name:ident, $lower:ident, [ $( $op:path ),* ] ) => {
        fn $name(&mut self) -> Result<Node, Error> {
            let start = self.lexer.position();
            let mut lhs = self.$lower()?;
            match self.lexer.peek() {
                Some(Token::Operator(op)) if $( op == &$op )||* => {
                    let op = op.clone();
                    self.lexer.next();
                    let rhs = self.$name()?;
                    lhs = self.build_binary_expression(lhs, op, rhs)?;
                }
                _ => {},
            }
            Ok(self.reg_pos(start, lhs))
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    scope_bits: u8,
    lex_stack: Vec<HashMap<String, bool>>,
    positions: HashMap<*const Node, SourceSpan>,
}

impl<'a> Parser<'a> {
    pub fn parse(code: &'a str) -> Result<(Node, HashMap<*const Node, SourceSpan>), Error> {
        let mut parser = Parser {
            lexer: Lexer::new(code),
            scope_bits: 0,
            lex_stack: Vec::new(),
            positions: HashMap::new(),
        };

        parser.lexer.skip_hashbang();

        if let Node::BlockStatement(items, decls, top) =
            parser.parse_block_statement(ParseScope::TopLevel)?
        {
            if let Some(Node::ExpressionStatement(expr)) = items.last() {
                // if the last item is an expression statement, replace it with the expression
                // so that the value will be left on the stack to inspect in tests
                let mut sliced = items[0..items.len() - 1].to_vec();
                sliced.push(Node::ParenthesizedExpression((*expr).clone()));
                Ok((Node::BlockStatement(sliced, decls, top), parser.positions))
            } else {
                Ok((Node::BlockStatement(items, decls, top), parser.positions))
            }
        } else {
            unreachable!();
        }
    }

    fn reg_pos(&mut self, start: SourcePosition, node: Node) -> Node {
        let s = SourceSpan {
            start,
            end: self.lexer.position(),
        };
        let r = &node as *const Node;
        self.positions.insert(r, s);
        node
    }

    fn scope(&self, scope: ParseScope) -> bool {
        (self.scope_bits & scope as u8) == scope as u8
    }

    fn peek(&mut self, token: Token) -> bool {
        self.lexer.peek() == Some(&token)
    }

    #[inline]
    fn eat(&mut self, token: Token) -> bool {
        if self.peek(token) {
            self.lexer.next();
            true
        } else {
            false
        }
        /*
        match self.lexer.peek() {
            Some(t) if t == &token => {
                self.lexer.next();
                true
            }
            _ => false,
        }
        */
    }

    fn expect(&mut self, token: Token) -> Result<Token, Error> {
        let t = self.lexer.next();
        match t {
            Some(ref t) if t == &token => Ok(token),
            None => Err(Error::UnexpectedEOF),
            _ => Err(Error::UnexpectedToken),
        }
    }

    fn parse_identifier_list(
        &mut self,
        close: Token,
        initializers: bool,
    ) -> Result<Vec<Node>, Error> {
        let mut identifiers = Vec::new();
        let mut first = true;
        while !self.eat(close.clone()) {
            if first {
                first = false;
            } else {
                self.expect(Token::Comma)?;
                if self.eat(close.clone()) {
                    break;
                }
            }
            let start = self.lexer.position();
            let ident = self.parse_identifier(false)?;
            if self.lexer.peek() == Some(&Token::Operator(Operator::Assign)) && initializers {
                self.lexer.next();
                let init = self.parse_expression()?;
                identifiers.push(self.reg_pos(start, Node::Initializer(ident, Box::new(init))));
            } else {
                identifiers.push(self.reg_pos(start, Node::Identifier(ident)));
            }
        }
        Ok(identifiers)
    }

    fn parse_function(&mut self, expression: bool, kind: FunctionKind) -> Result<Node, Error> {
        let start = self.lexer.position();
        let name = if expression {
            if let Some(Token::Identifier(..)) = self.lexer.peek() {
                Some(self.parse_identifier(false)?)
            } else {
                None
            }
        } else {
            Some(self.parse_identifier(false)?)
        };
        self.expect(Token::LeftParen)?;
        let args = self.parse_identifier_list(Token::RightParen, true)?;
        let body = self.parse_block_statement(match kind {
            FunctionKind::Normal => ParseScope::Function,
            FunctionKind::Async => ParseScope::AsyncFunction,
            FunctionKind::Generator => ParseScope::GeneratorFunction,
        })?;
        Ok(if expression {
            self.reg_pos(
                start,
                Node::FunctionExpression(name, args, Box::new(body), kind),
            )
        } else {
            let name = name.unwrap();
            let scope = self.lex_stack.last_mut().unwrap();
            if scope.contains_key(&name) {
                return Err(Error::DuplicateBinding);
            } else {
                scope.insert(name.clone(), false);
            }
            self.reg_pos(
                start,
                Node::FunctionDeclaration(name, args, Box::new(body), kind),
            )
        })
    }

    fn parse_statement_list_item(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        self.lexer.peek();
        match self.lexer.peek_immutable() {
            None => Err(Error::NormalEOF),
            Some(Token::LeftBrace) => self.parse_block_statement(ParseScope::Block),
            Some(Token::Let) | Some(Token::Const) => self.parse_lexical_declaration(),
            Some(Token::Function) => {
                self.lexer.next();
                self.parse_function(false, FunctionKind::Normal)
            }
            Some(Token::At) => {
                let mut decorators = VecDeque::new();
                while self.eat(Token::At) {
                    let d = self.parse_left_hand_side_expression()?;
                    decorators.push_front(d);
                }
                let kind = if self.eat(Token::Async) {
                    self.expect(Token::Function)?;
                    FunctionKind::Async
                } else if self.eat(Token::Gen) {
                    self.expect(Token::Function)?;
                    FunctionKind::Generator
                } else if self.eat(Token::Function) {
                    FunctionKind::Normal
                } else {
                    return Err(Error::UnexpectedToken);
                };
                if let Node::FunctionDeclaration(name, body, args, kind) =
                    self.parse_function(false, kind)?
                {
                    let mut top = Node::FunctionExpression(Some(name.clone()), body, args, kind);
                    for d in decorators {
                        top = Node::CallExpression(Box::new(d), vec![top]);
                    }
                    Ok(Node::LexicalInitialization(name, Box::new(top)))
                } else {
                    unreachable!();
                }
            }
            Some(Token::Async) => {
                self.lexer.next();
                self.expect(Token::Function)?;
                self.parse_function(false, FunctionKind::Async)
            }
            Some(Token::Gen) => {
                self.lexer.next();
                self.expect(Token::Function)?;
                self.parse_function(false, FunctionKind::Generator)
            }
            Some(Token::Return) if self.scope(ParseScope::Function) => {
                self.lexer.next();
                if self.eat(Token::Semicolon) {
                    Ok(self.reg_pos(start, Node::ReturnStatement(Box::new(Node::NullLiteral))))
                } else {
                    let mut expr = self.parse_expression()?;
                    self.expect(Token::Semicolon)?;
                    if let Node::CallExpression(callee, arguments) = expr {
                        expr = Node::TailCallExpression(callee, arguments);
                    }
                    Ok(self.reg_pos(start, Node::ReturnStatement(Box::new(expr))))
                }
            }
            Some(Token::Throw) => {
                self.lexer.next();
                let expr = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(self.reg_pos(start, Node::ThrowStatement(Box::new(expr))))
            }
            Some(Token::Try) => {
                self.lexer.next();
                let try_clause = Box::new(self.parse_block_statement(ParseScope::Block)?);
                if self.eat(Token::Finally) {
                    let finally_clause = Box::new(self.parse_block_statement(ParseScope::Block)?);
                    Ok(self.reg_pos(
                        start,
                        Node::TryStatement(try_clause, None, None, Some(finally_clause)),
                    ))
                } else {
                    self.expect(Token::Catch)?;
                    let mut binding = None;
                    if let Some(Token::Identifier(..)) = self.lexer.peek() {
                        binding = Some(self.parse_identifier(false)?);
                    }
                    let catch_clause = Box::new(self.parse_block_statement(ParseScope::Block)?);
                    if self.eat(Token::Finally) {
                        let finally_clause =
                            Box::new(self.parse_block_statement(ParseScope::Block)?);
                        Ok(self.reg_pos(
                            start,
                            Node::TryStatement(
                                try_clause,
                                binding,
                                Some(catch_clause),
                                Some(finally_clause),
                            ),
                        ))
                    } else {
                        Ok(self.reg_pos(
                            start,
                            Node::TryStatement(try_clause, binding, Some(catch_clause), None),
                        ))
                    }
                }
            }
            Some(Token::If) => {
                self.lexer.next();
                let test = self.parse_expression()?;
                let consequent = self.parse_block_statement(ParseScope::Block)?;
                if self.eat(Token::Else) {
                    let alternative = if self.lexer.peek() == Some(&Token::If) {
                        self.parse_statement_list_item()?
                    } else {
                        self.parse_block_statement(ParseScope::Block)?
                    };
                    if let Some(n) =
                        self.fold_conditional(test.clone(), consequent.clone(), alternative.clone())
                    {
                        return Ok(n);
                    }
                    Ok(self.reg_pos(
                        start,
                        Node::IfElseStatement(
                            Box::new(test),
                            Box::new(consequent),
                            Box::new(alternative),
                        ),
                    ))
                } else {
                    if let Some(n) = self.fold_conditional(
                        test.clone(),
                        consequent.clone(),
                        Node::ExpressionStatement(Box::new(Node::NullLiteral)),
                    ) {
                        return Ok(self.reg_pos(start, n));
                    }
                    Ok(self.reg_pos(
                        start,
                        Node::IfStatement(Box::new(test), Box::new(consequent)),
                    ))
                }
            }
            Some(Token::While) => {
                self.lexer.next();
                let test = self.parse_expression()?;
                let body = self.parse_block_statement(ParseScope::Loop)?;
                if let Some(n) = self.fold_while_loop(test.clone()) {
                    Ok(self.reg_pos(start, n))
                } else {
                    Ok(self.reg_pos(start, Node::WhileStatement(Box::new(test), Box::new(body))))
                }
            }
            Some(Token::For) => {
                self.lexer.next();
                let asyn = self.eat(Token::Await);
                let binding = self.parse_identifier(false)?;
                self.expect(Token::In)?;
                let target = self.parse_assignment_expression()?;
                let body = self.parse_block_statement(ParseScope::Loop)?;
                Ok(self.reg_pos(
                    start,
                    Node::ForStatement(asyn, binding, Box::new(target), Box::new(body)),
                ))
            }
            Some(Token::Break) if self.scope(ParseScope::Loop) => {
                self.lexer.next();
                self.expect(Token::Semicolon)?;
                Ok(self.reg_pos(start, Node::BreakStatement))
            }
            Some(Token::Continue) if self.scope(ParseScope::Loop) => {
                self.lexer.next();
                self.expect(Token::Semicolon)?;
                Ok(self.reg_pos(start, Node::ContinueStatement))
            }
            Some(Token::Export) if self.scope(ParseScope::TopLevel) => {
                self.lexer.next();
                let decl = match self.lexer.peek() {
                    Some(Token::Let) | Some(Token::Const) => self.parse_lexical_declaration(),
                    Some(Token::Function) => {
                        self.lexer.next();
                        self.parse_function(false, FunctionKind::Normal)
                    }
                    _ => Err(Error::UnexpectedToken),
                }?;
                Ok(self.reg_pos(start, Node::ExportDeclaration(Box::new(decl))))
            }
            Some(Token::Import) if self.scope(ParseScope::TopLevel) => {
                self.lexer.next();
                match self.lexer.peek() {
                    // import "specifier";
                    Some(Token::StringLiteral(..)) => {
                        let specifier = match self.lexer.next() {
                            Some(Token::StringLiteral(s)) => s,
                            _ => unreachable!(),
                        };
                        self.expect(Token::Semicolon)?;
                        Ok(self.reg_pos(start, Node::ImportDeclaration(specifier)))
                    }
                    // import { bindings } from "specifier";
                    Some(Token::LeftBrace) => {
                        self.lexer.next();
                        let bindings = self
                            .parse_identifier_list(Token::RightBrace, false)?
                            .iter()
                            .map(|n| match n {
                                Node::Identifier(n) => n.to_string(),
                                _ => unreachable!(),
                            })
                            .collect();
                        self.expect(Token::From)?;
                        match self.lexer.next() {
                            Some(Token::StringLiteral(s)) => {
                                self.expect(Token::Semicolon)?;
                                Ok(self.reg_pos(start, Node::ImportNamedDeclaration(s, bindings)))
                            }
                            Some(Token::Identifier(ref s)) if s == "standard" => {
                                self.expect(Token::Colon)?;
                                let namespace = self.parse_identifier(true)?;
                                self.expect(Token::Semicolon)?;
                                Ok(self.reg_pos(
                                    start,
                                    Node::ImportStandardDeclaration(namespace, bindings),
                                ))
                            }
                            _ => Err(Error::UnexpectedToken),
                        }
                    }
                    // import binding from "specifier";
                    Some(Token::Identifier(..)) => {
                        let binding = self.parse_identifier(false)?;
                        self.expect(Token::From)?;
                        let specifier = match self.lexer.next() {
                            Some(Token::StringLiteral(s)) => s,
                            _ => unreachable!(),
                        };
                        self.expect(Token::Semicolon)?;
                        Ok(self.reg_pos(start, Node::ImportDefaultDeclaration(specifier, binding)))
                    }
                    _ => Err(Error::UnexpectedToken),
                }
            }
            _ => {
                let expr = self.parse_expression_statement()?;
                self.expect(Token::Semicolon)?;
                Ok(expr)
            }
        }
    }

    fn parse_lexical_declaration(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        match self.lexer.peek() {
            Some(Token::Let) | Some(Token::Const) => {
                let decl = self.lexer.next().unwrap();
                let name = self.parse_identifier(false)?;
                self.expect(Token::Operator(Operator::Assign))?;
                let value = self.parse_assignment_expression()?;
                self.expect(Token::Semicolon)?;
                let scope = self.lex_stack.last_mut().unwrap();
                if scope.contains_key(&name) {
                    return Err(Error::DuplicateBinding);
                } else {
                    scope.insert(
                        name.clone(),
                        match decl {
                            Token::Let => true,
                            Token::Const => false,
                            _ => unreachable!(),
                        },
                    );
                }
                Ok(self.reg_pos(start, Node::LexicalInitialization(name, Box::new(value))))
            }
            _ => Err(Error::UnexpectedToken),
        }
    }

    fn parse_block_statement(&mut self, scope: ParseScope) -> Result<Node, Error> {
        let start = self.lexer.position();
        if scope != ParseScope::TopLevel {
            self.expect(Token::LeftBrace)?;
        }
        let saved = self.scope_bits;
        self.scope_bits |= scope as u8;
        self.lex_stack.push(HashMap::new());
        let mut nodes = Vec::new();
        while !self.eat(Token::RightBrace) {
            match self.parse_statement_list_item() {
                Ok(node) => nodes.push(node),
                Err(Error::NormalEOF) if scope == ParseScope::TopLevel => break,
                Err(e) => {
                    self.scope_bits = saved;
                    self.lex_stack.pop();
                    return Err(e);
                }
            }
        }
        self.scope_bits = saved;
        let declarations = self.lex_stack.pop().unwrap();
        Ok(self.reg_pos(
            start,
            Node::BlockStatement(nodes, declarations, scope == ParseScope::TopLevel),
        ))
    }

    fn parse_expression_statement(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        let expression = self.parse_expression()?;
        Ok(self.reg_pos(start, Node::ExpressionStatement(Box::new(expression))))
    }

    fn parse_expression(&mut self) -> Result<Node, Error> {
        self.parse_assignment_expression()
    }

    fn parse_identifier(&mut self, allow_keyword: bool) -> Result<String, Error> {
        match self.lexer.next() {
            Some(Token::Identifier(name)) => Ok(name),
            Some(Token::Throw) if allow_keyword => Ok("throw".to_string()),
            Some(Token::Catch) if allow_keyword => Ok("catch".to_string()),
            Some(Token::True) if allow_keyword => Ok("true".to_string()),
            Some(Token::False) if allow_keyword => Ok("false".to_string()),
            Some(Token::Null) if allow_keyword => Ok("null".to_string()),
            Some(Token::This) if allow_keyword => Ok("this".to_string()),
            Some(Token::Class) if allow_keyword => Ok("class".to_string()),
            Some(Token::Finally) if allow_keyword => Ok("finally".to_string()),
            Some(Token::Function) if allow_keyword => Ok("function".to_string()),
            Some(Token::Let) if allow_keyword => Ok("let".to_string()),
            Some(Token::Const) if allow_keyword => Ok("const".to_string()),
            Some(Token::Throw) if allow_keyword => Ok("throw".to_string()),
            Some(Token::Return) if allow_keyword => Ok("return".to_string()),
            Some(Token::While) if allow_keyword => Ok("while".to_string()),
            Some(Token::For) if allow_keyword => Ok("for".to_string()),
            Some(Token::In) if allow_keyword => Ok("in".to_string()),
            Some(Token::Break) if allow_keyword => Ok("break".to_string()),
            Some(Token::Continue) if allow_keyword => Ok("continue".to_string()),
            Some(Token::Try) if allow_keyword => Ok("try".to_string()),
            Some(Token::Catch) if allow_keyword => Ok("catch".to_string()),
            Some(Token::If) if allow_keyword => Ok("if".to_string()),
            Some(Token::Else) if allow_keyword => Ok("else".to_string()),
            Some(Token::New) if allow_keyword => Ok("new".to_string()),
            Some(Token::Import) if allow_keyword => Ok("import".to_string()),
            Some(Token::Export) if allow_keyword => Ok("export".to_string()),
            Some(Token::Default) if allow_keyword => Ok("default".to_string()),
            Some(Token::From) if allow_keyword => Ok("from".to_string()),
            Some(Token::Async) if allow_keyword => Ok("async".to_string()),
            Some(Token::Await) if allow_keyword => Ok("await".to_string()),
            Some(Token::Gen) if allow_keyword => Ok("gen".to_string()),
            Some(Token::Yield) if allow_keyword => Ok("yield".to_string()),
            Some(Token::Operator(Operator::Typeof)) if allow_keyword => Ok("typeof".to_string()),
            Some(Token::Operator(Operator::Void)) if allow_keyword => Ok("void".to_string()),
            _ => Err(Error::UnexpectedToken),
        }
    }

    fn parse_assignment_expression(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        if self.eat(Token::Yield) && self.scope(ParseScope::GeneratorFunction) {
            match self.lexer.peek() {
                Some(Token::Semicolon)
                | Some(Token::RightBrace)
                | Some(Token::RightBracket)
                | Some(Token::RightParen)
                | Some(Token::Colon)
                | Some(Token::Comma) => {
                    return Ok(self.reg_pos(start, Node::YieldExpression(None)));
                }
                _ => {
                    let exp = self.parse_assignment_expression()?;
                    return Ok(self.reg_pos(start, Node::YieldExpression(Some(Box::new(exp)))));
                }
            }
        }

        let mut lhs = self.parse_conditional_expression()?;

        macro_rules! op_assign {
            ($op:expr) => {{
                // lhs @= rhs;
                // lhs = lhs @ rhs;
                self.lexer.next();
                let rhs = self.parse_assignment_expression()?;
                let rhs = self.build_binary_expression(lhs.clone(), $op, rhs)?;
                lhs = self.build_binary_expression(lhs, Operator::Assign, rhs)?;
            }};
        }

        self.lexer.peek();
        match self.lexer.peek_immutable() {
            Some(Token::Operator(Operator::Assign)) => {
                self.lexer.next();
                let rhs = self.parse_assignment_expression()?;
                lhs = self.build_binary_expression(lhs, Operator::Assign, rhs)?;
            }
            Some(Token::Operator(Operator::AddAssign)) => op_assign!(Operator::Add),
            Some(Token::Operator(Operator::SubAssign)) => op_assign!(Operator::Sub),
            Some(Token::Operator(Operator::MulAssign)) => op_assign!(Operator::Mul),
            Some(Token::Operator(Operator::PowAssign)) => op_assign!(Operator::Pow),
            Some(Token::Operator(Operator::DivAssign)) => op_assign!(Operator::Div),
            Some(Token::Operator(Operator::ModAssign)) => op_assign!(Operator::Mod),
            _ => {}
        }

        Ok(self.reg_pos(start, lhs))
    }

    fn build_binary_expression(
        &self,
        left: Node,
        op: Operator,
        right: Node,
    ) -> Result<Node, Error> {
        match op {
            Operator::Assign
            | Operator::AddAssign
            | Operator::SubAssign
            | Operator::MulAssign
            | Operator::DivAssign
            | Operator::PowAssign => match left {
                Node::CallExpression(..)
                | Node::UnaryExpression(..)
                | Node::NullLiteral
                | Node::TrueLiteral
                | Node::FalseLiteral
                | Node::ArrayLiteral(..)
                | Node::ObjectLiteral(..)
                | Node::NumberLiteral(..)
                | Node::StringLiteral(..) => {
                    return Err(Error::UnexpectedToken);
                }
                _ => {}
            },
            _ => {}
        };

        macro_rules! num_binop_num {
            ($op:expr) => {
                if let Node::NumberLiteral(lnum) = left {
                    if let Node::NumberLiteral(rnum) = right {
                        return Ok(Node::NumberLiteral($op(lnum, rnum)));
                    }
                }
            };
        }

        macro_rules! num_binop_bool {
            ($op:expr) => {
                if let Node::NumberLiteral(lnum) = left {
                    if let Node::NumberLiteral(rnum) = right {
                        if $op(&lnum, &rnum) {
                            return Ok(Node::TrueLiteral);
                        } else {
                            return Ok(Node::FalseLiteral);
                        }
                    }
                }
            };
        }

        match op {
            Operator::Add => match &left {
                Node::NumberLiteral(lnum) => {
                    if let Node::NumberLiteral(rnum) = right {
                        return Ok(Node::NumberLiteral(lnum + rnum));
                    }
                }
                Node::StringLiteral(lstr) => {
                    if let Node::StringLiteral(rstr) = right {
                        return Ok(Node::StringLiteral(format!("{}{}", lstr, rstr)));
                    }
                }
                _ => {}
            },
            Operator::Sub => num_binop_num!(f64::sub),
            Operator::Mul => num_binop_num!(f64::mul),
            Operator::Div => num_binop_num!(f64::div),
            Operator::Mod => num_binop_num!(f64::rem),
            Operator::Pow => num_binop_num!(f64::powf),
            Operator::LeftShift => num_binop_num!(crate::num_util::f64_shl),
            Operator::RightShift => num_binop_num!(crate::num_util::f64_shr),
            Operator::LessThan => num_binop_bool!(f64::lt),
            Operator::GreaterThan => num_binop_bool!(f64::gt),
            Operator::LessThanOrEqual => num_binop_bool!(f64::le),
            Operator::GreaterThanOrEqual => num_binop_bool!(f64::ge),
            _ => {}
        }

        Ok(Node::BinaryExpression(Box::new(left), op, Box::new(right)))
    }

    fn fold_conditional(&self, test: Node, consequent: Node, alternative: Node) -> Option<Node> {
        match test {
            Node::NumberLiteral(n) => {
                if n != 0f64 {
                    Some(consequent)
                } else {
                    Some(alternative)
                }
            }
            Node::StringLiteral(s) => {
                if s.chars().count() > 0 {
                    Some(consequent)
                } else {
                    Some(alternative)
                }
            }
            Node::FalseLiteral | Node::NullLiteral | Node::UnaryExpression(Operator::Void, ..) => {
                Some(alternative)
            }
            Node::TrueLiteral | Node::ArrayLiteral(..) | Node::ObjectLiteral(..) => {
                Some(consequent)
            }
            _ => None,
        }
    }

    fn fold_while_loop(&self, test: Node) -> Option<Node> {
        match test {
            Node::NullLiteral | Node::FalseLiteral | Node::UnaryExpression(Operator::Void, ..) => {
                Some(Node::ExpressionStatement(Box::new(test)))
            }
            Node::NumberLiteral(n) => {
                if n == 0f64 {
                    Some(Node::ExpressionStatement(Box::new(test)))
                } else {
                    None
                }
            }
            Node::StringLiteral(ref s) => {
                if s.chars().count() == 0 {
                    Some(Node::ExpressionStatement(Box::new(test)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_conditional_expression(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        let lhs = self.parse_logical_or_expression()?;
        if self.eat(Token::Question) {
            let consequent = self.parse_assignment_expression()?;
            self.expect(Token::Colon)?;
            let alternative = self.parse_assignment_expression()?;
            if let Some(n) =
                self.fold_conditional(lhs.clone(), consequent.clone(), alternative.clone())
            {
                return Ok(n);
            }
            return Ok(self.reg_pos(
                start,
                Node::ConditionalExpression(
                    Box::new(lhs),
                    Box::new(consequent),
                    Box::new(alternative),
                ),
            ));
        }
        Ok(lhs)
    }

    binop_production!(
        parse_logical_or_expression,
        parse_logical_and_expression,
        [Operator::LogicalOR]
    );

    binop_production!(
        parse_logical_and_expression,
        parse_bitwise_or_expression,
        [Operator::LogicalAND]
    );

    binop_production!(
        parse_bitwise_or_expression,
        parse_bitwise_xor_expression,
        [Operator::BitwiseOR]
    );

    binop_production!(
        parse_bitwise_xor_expression,
        parse_bitwise_and_expression,
        [Operator::BitwiseXOR]
    );

    binop_production!(
        parse_bitwise_and_expression,
        parse_equality_expression,
        [Operator::BitwiseAND]
    );

    binop_production!(
        parse_equality_expression,
        parse_relational_expression,
        [Operator::Equal, Operator::NotEqual]
    );

    binop_production!(
        parse_relational_expression,
        parse_shift_expression,
        [
            Operator::LessThan,
            Operator::GreaterThan,
            Operator::LessThanOrEqual,
            Operator::GreaterThanOrEqual
        ]
    );

    binop_production!(
        parse_shift_expression,
        parse_additive_expression,
        [Operator::LeftShift, Operator::RightShift]
    );

    binop_production!(
        parse_additive_expression,
        parse_multiplicate_expression,
        [Operator::Add, Operator::Sub]
    );

    binop_production!(
        parse_multiplicate_expression,
        parse_exponentiation_expression,
        [Operator::Mul, Operator::Div, Operator::Mod]
    );

    binop_production!(
        parse_exponentiation_expression,
        parse_unary_expression,
        [Operator::Pow]
    );

    fn parse_unary_expression(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        self.lexer.peek();
        match self.lexer.peek_immutable() {
            Some(Token::Operator(Operator::Add)) => {
                self.lexer.next();
                let expr = self.parse_unary_expression()?;
                Ok(self.reg_pos(start, Node::UnaryExpression(Operator::Add, Box::new(expr))))
            }
            Some(Token::Operator(Operator::Sub)) => {
                self.lexer.next();
                let expr = self.parse_unary_expression()?;
                Ok(self.reg_pos(start, Node::UnaryExpression(Operator::Sub, Box::new(expr))))
            }
            Some(Token::Operator(Operator::BitwiseNOT)) => {
                self.lexer.next();
                let expr = self.parse_unary_expression()?;
                Ok(self.reg_pos(
                    start,
                    Node::UnaryExpression(Operator::BitwiseNOT, Box::new(expr)),
                ))
            }
            Some(Token::Operator(Operator::Not)) => {
                self.lexer.next();
                let expr = self.parse_unary_expression()?;
                Ok(self.reg_pos(start, Node::UnaryExpression(Operator::Not, Box::new(expr))))
            }
            Some(Token::Await) if self.scope(ParseScope::AsyncFunction) => {
                self.lexer.next();
                let expr = self.parse_unary_expression()?;
                Ok(self.reg_pos(start, Node::AwaitExpression(Box::new(expr))))
            }
            _ => self.parse_left_hand_side_expression(),
        }
    }

    fn parse_expression_list(&mut self, close: Token) -> Result<Vec<Node>, Error> {
        let mut list = Vec::new();
        let mut first = true;
        while !self.eat(close.clone()) {
            if first {
                first = false;
            } else {
                self.expect(Token::Comma)?;
                if self.eat(close.clone()) {
                    break;
                }
            }
            list.push(self.parse_assignment_expression()?);
        }
        Ok(list)
    }

    fn parse_left_hand_side_expression(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        let mut base = self.parse_primary_expression()?;
        loop {
            if self.eat(Token::Dot) {
                let property = self.parse_identifier(true)?;
                base = Node::MemberExpression(Box::new(base), property);
            } else if self.eat(Token::LeftBracket) {
                let property = self.parse_expression()?;
                self.expect(Token::RightBracket)?;
                base = Node::ComputedMemberExpression(Box::new(base), Box::new(property));
            } else if self.eat(Token::LeftParen) {
                let list = self.parse_expression_list(Token::RightParen)?;
                base = Node::CallExpression(Box::new(base), list);
            } else {
                return Ok(self.reg_pos(start, base));
            }
        }
    }

    fn parse_arrow_function(
        &mut self,
        kind: FunctionKind,
        mut args: Vec<Node>,
    ) -> Result<Node, Error> {
        for item in &mut args {
            match item {
                Node::Identifier(..) | Node::Initializer(..) => {}
                Node::BinaryExpression(left, op, right) if *op == Operator::Assign => {
                    if let Node::Identifier(ident) = &**left {
                        let init =
                            Node::Initializer(ident.to_string(), Box::new((**right).clone()));
                        std::mem::replace(item, init);
                    } else {
                        return Err(Error::UnexpectedToken);
                    }
                }
                _ => return Err(Error::UnexpectedToken),
            }
        }
        let body = if self.peek(Token::LeftBrace) {
            self.parse_block_statement(match kind {
                FunctionKind::Normal => ParseScope::Function,
                FunctionKind::Async => ParseScope::AsyncFunction,
                FunctionKind::Generator => ParseScope::GeneratorFunction,
            })?
        } else {
            let expr = self.parse_assignment_expression()?;
            Node::BlockStatement(
                vec![Node::ReturnStatement(Box::new(expr))],
                HashMap::new(),
                false,
            )
        };
        Ok(Node::ArrowFunctionExpression(args, Box::new(body), kind))
    }

    fn parse_primary_expression(&mut self) -> Result<Node, Error> {
        let start = self.lexer.position();
        let token = self.lexer.next();
        match token {
            Some(t) => match t {
                Token::This => Ok(self.reg_pos(start, Node::ThisExpression)),
                Token::New => {
                    let expr = self.parse_left_hand_side_expression()?;
                    Ok(self.reg_pos(start, Node::NewExpression(Box::new(expr))))
                }
                Token::Null => Ok(self.reg_pos(start, Node::NullLiteral)),
                Token::True => Ok(self.reg_pos(start, Node::TrueLiteral)),
                Token::False => Ok(self.reg_pos(start, Node::FalseLiteral)),
                Token::Colon => {
                    let name = self.parse_identifier(false)?;
                    Ok(self.reg_pos(start, Node::SymbolLiteral(name)))
                }
                Token::Operator(Operator::Typeof) => {
                    let expr = self.parse_unary_expression()?;
                    Ok(self.reg_pos(
                        start,
                        Node::UnaryExpression(Operator::Typeof, Box::new(expr)),
                    ))
                }
                Token::Operator(Operator::Void) => {
                    let expr = self.parse_unary_expression()?;
                    Ok(self.reg_pos(start, Node::UnaryExpression(Operator::Void, Box::new(expr))))
                }
                Token::StringLiteral(v) => Ok(self.reg_pos(start, Node::StringLiteral(v))),
                Token::NumberLiteral(v) => Ok(self.reg_pos(start, Node::NumberLiteral(v))),
                Token::BackQuote => {
                    let mut quasis = Vec::new();
                    let mut expressions = Vec::new();

                    let mut current = String::new();

                    loop {
                        match self.lexer.chars.next() {
                            Some('$') => {
                                if self.lexer.chars.peek() == Some(&'(') {
                                    quasis.push(current);
                                    current = String::new();
                                    self.lexer.chars.next();
                                    let expr = self.parse_expression()?;
                                    expressions.push(expr);
                                    self.expect(Token::RightParen)?;
                                } else {
                                    current.push('$');
                                }
                            }
                            Some('`') => break,
                            Some(c) => {
                                current.push(c);
                            }
                            None => return Err(Error::UnexpectedEOF),
                        }
                    }

                    quasis.push(current);

                    Ok(self.reg_pos(start, Node::TemplateLiteral(quasis, expressions)))
                }
                Token::Identifier(v) => Ok(self.reg_pos(start, Node::Identifier(v))),
                Token::Function => self.parse_function(true, FunctionKind::Normal),
                Token::Async => {
                    if self.eat(Token::Function) {
                        self.parse_function(true, FunctionKind::Async)
                    } else {
                        self.expect(Token::LeftParen)?;
                        let list = self.parse_identifier_list(Token::RightParen, true)?;
                        self.expect(Token::Arrow)?;
                        self.parse_arrow_function(FunctionKind::Async, list)
                    }
                }
                Token::Gen => {
                    if self.eat(Token::Function) {
                        self.parse_function(true, FunctionKind::Generator)
                    } else {
                        self.expect(Token::LeftParen)?;
                        let list = self.parse_identifier_list(Token::RightParen, true)?;
                        self.expect(Token::Arrow)?;
                        self.parse_arrow_function(FunctionKind::Generator, list)
                    }
                }
                Token::LeftParen => {
                    let mut list = self.parse_expression_list(Token::RightParen)?;
                    if self.eat(Token::Arrow) {
                        // ( ... ) =>
                        self.parse_arrow_function(FunctionKind::Normal, list)
                    } else if list.is_empty() {
                        // ( )
                        Err(Error::UnexpectedToken)
                    } else if list.len() == 1 {
                        // ( expr )
                        Ok(Node::ParenthesizedExpression(Box::new(list.pop().unwrap())))
                    } else {
                        // ( expr, expr )
                        Ok(Node::TupleLiteral(list))
                    }
                }
                Token::LeftBracket => {
                    let list = self.parse_expression_list(Token::RightBracket)?;
                    Ok(self.reg_pos(start, Node::ArrayLiteral(list)))
                }
                Token::LeftBrace => {
                    let mut fields = Vec::new();
                    let mut first = true;
                    while !self.eat(Token::RightBrace) {
                        if first {
                            first = false;
                        } else {
                            self.expect(Token::Comma)?;
                            if self.eat(Token::RightBrace) {
                                break;
                            }
                        }
                        let name = if self.eat(Token::LeftBracket) {
                            let name = self.parse_expression()?;
                            self.expect(Token::RightBracket)?;
                            name
                        } else {
                            Node::StringLiteral(self.parse_identifier(true)?)
                        };
                        let mut init;
                        if self.eat(Token::Colon) {
                            init = self.parse_expression()?;
                        } else {
                            init = self.parse_function(true, FunctionKind::Normal)?
                        }
                        fields.push(Node::ObjectInitializer(Box::new(name), Box::new(init)));
                    }
                    Ok(self.reg_pos(start, Node::ObjectLiteral(fields)))
                }
                Token::Operator(Operator::Div) => {
                    let mut pattern = String::new();
                    loop {
                        match self.lexer.chars.next() {
                            Some('/') => break,
                            Some('\\') => {
                                pattern.push('\\');
                                pattern.push(self.lexer.chars.next().unwrap());
                            }
                            Some(c) => {
                                pattern.push(c);
                            }
                            None => return Err(Error::UnexpectedEOF),
                        }
                    }
                    Ok(self.reg_pos(start, Node::RegexLiteral(pattern)))
                }
                _ => Err(Error::UnexpectedToken),
            },
            None => Err(Error::UnexpectedEOF),
        }
    }
}

#[test]
fn test_parser() {
    macro_rules! hashmap(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = ::std::collections::HashMap::new();
                $(
                    m.insert($key.to_string(), $value);
                )+
                m
            }
         };
    );

    assert_eq!(
        Parser::parse(
            r#"
             const a = 1;
             if a { a += 2; }
             if 1 { a += 3; }
             "#
        )
        .unwrap()
        .0,
        Node::BlockStatement(
            vec![
                Node::LexicalInitialization("a".to_string(), Box::new(Node::NumberLiteral(1f64))),
                Node::IfStatement(
                    Box::new(Node::Identifier("a".to_string())),
                    Box::new(Node::BlockStatement(
                        vec![Node::ExpressionStatement(Box::new(Node::BinaryExpression(
                            Box::new(Node::Identifier("a".to_string())),
                            Operator::Assign,
                            Box::new(Node::BinaryExpression(
                                Box::new(Node::Identifier("a".to_string())),
                                Operator::Add,
                                Box::new(Node::NumberLiteral(2f64)),
                            )),
                        )))],
                        HashMap::new(),
                        false,
                    )),
                ),
                Node::BlockStatement(
                    vec![Node::ExpressionStatement(Box::new(Node::BinaryExpression(
                        Box::new(Node::Identifier("a".to_string())),
                        Operator::Assign,
                        Box::new(Node::BinaryExpression(
                            Box::new(Node::Identifier("a".to_string())),
                            Operator::Add,
                            Box::new(Node::NumberLiteral(3f64)),
                        )),
                    )))],
                    HashMap::new(),
                    false,
                ),
            ],
            hashmap! {
                "a" => false
            },
            true,
        ),
    );

    assert_eq!(
        Parser::parse("while false { 1; }").unwrap().0,
        Node::BlockStatement(
            vec![Node::ParenthesizedExpression(Box::new(Node::FalseLiteral))],
            HashMap::new(),
            true,
        ),
    );

    assert_eq!(
        Parser::parse("#! hashbang line\ntrue;").unwrap().0,
        Node::BlockStatement(
            vec![Node::ParenthesizedExpression(Box::new(Node::TrueLiteral))],
            HashMap::new(),
            true,
        ),
    );
}
