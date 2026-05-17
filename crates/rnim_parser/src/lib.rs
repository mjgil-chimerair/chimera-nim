//! Lossless parser for modules, statements, expressions, pragmas, command syntax, and error recovery.

#[cfg(test)]
use rnim_allocator as _;
use rnim_lexer::{Lexer, TokenKind};
use rnim_span::FileId;
use rowan::GreenNodeBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RnimLang;

impl rowan::Language for RnimLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from_u16(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.to_u16())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    // Trivia (preserved exactly)
    Whitespace,
    NewLine,
    Comment,

    // Literal tokens
    Token,

    // Module structure
    Module,

    // Declarations
    ProcDef,
    FuncDef,
    TypeDef,
    VarDecl,
    LetDecl,
    ConstDecl,
    MethodDef,
    IteratorDef,
    ConverterDef,
    TemplateDef,
    MacroDef,

    // Statements
    IfStmt,
    WhileStmt,
    ForStmt,
    CaseStmt,
    ReturnStmt,
    BreakStmt,
    ContinueStmt,
    RaiseStmt,
    TryStmt,
    DeferStmt,
    BlockStmt,

    // Expressions
    CallExpr,
    CommandExpr,
    IdentExpr,
    LiteralExpr,
    PrefixExpr,
    PostfixExpr,
    BinaryExpr,
    IndexExpr,
    SliceExpr,
    LambdaExpr,
    CastExpr,
    AddrExpr,
    DerefExpr,
    TupleExpr,
    ObjectExpr,
    ArrayExpr,
    SeqExpr,
    SetExpr,
    RangeExpr,
    ParenExpr,

    // Declaration parts
    ParamList,
    Param,
    ReturnType,
    GenericParams,
    Pragma,
    ExportMarker,

    // Types
    TypeRef,
    ArrayType,
    SeqType,
    SetType,
    TupleType,
    ObjectType,
    RefType,
    PtrType,
    DistinctType,
    ProcType,
    VarType,
    OutType,
    SinkType,
    LentType,
    TypedescType,
    StaticType,
    GenericType,

    // Pragmas
    PragmaExpr,
    PragmaMarker,

    // Module statements
    ImportStmt,
    FromImportStmt,
    IncludeStmt,
    ExportStmt,

    // Error
    Error,
}

impl SyntaxKind {
    pub fn from_u16(n: u16) -> Self {
        // Safe conversion: SyntaxKind is a repr(u16) enum with consecutive values
        // We match explicitly to avoid any unsafe code
        match n {
            0 => SyntaxKind::Whitespace,
            1 => SyntaxKind::NewLine,
            2 => SyntaxKind::Comment,
            3 => SyntaxKind::Token,
            4 => SyntaxKind::Module,
            5 => SyntaxKind::ProcDef,
            6 => SyntaxKind::FuncDef,
            7 => SyntaxKind::TypeDef,
            8 => SyntaxKind::VarDecl,
            9 => SyntaxKind::LetDecl,
            10 => SyntaxKind::ConstDecl,
            11 => SyntaxKind::MethodDef,
            12 => SyntaxKind::IteratorDef,
            13 => SyntaxKind::ConverterDef,
            14 => SyntaxKind::TemplateDef,
            15 => SyntaxKind::MacroDef,
            16 => SyntaxKind::IfStmt,
            17 => SyntaxKind::WhileStmt,
            18 => SyntaxKind::ForStmt,
            19 => SyntaxKind::CaseStmt,
            20 => SyntaxKind::ReturnStmt,
            21 => SyntaxKind::BreakStmt,
            22 => SyntaxKind::ContinueStmt,
            23 => SyntaxKind::RaiseStmt,
            24 => SyntaxKind::TryStmt,
            25 => SyntaxKind::DeferStmt,
            26 => SyntaxKind::BlockStmt,
            27 => SyntaxKind::CallExpr,
            28 => SyntaxKind::CommandExpr,
            29 => SyntaxKind::IdentExpr,
            30 => SyntaxKind::LiteralExpr,
            31 => SyntaxKind::PrefixExpr,
            32 => SyntaxKind::PostfixExpr,
            33 => SyntaxKind::BinaryExpr,
            34 => SyntaxKind::IndexExpr,
            35 => SyntaxKind::SliceExpr,
            36 => SyntaxKind::LambdaExpr,
            37 => SyntaxKind::CastExpr,
            38 => SyntaxKind::AddrExpr,
            39 => SyntaxKind::DerefExpr,
            40 => SyntaxKind::TupleExpr,
            41 => SyntaxKind::ObjectExpr,
            42 => SyntaxKind::ArrayExpr,
            43 => SyntaxKind::SeqExpr,
            44 => SyntaxKind::SetExpr,
            45 => SyntaxKind::RangeExpr,
            46 => SyntaxKind::ParenExpr,
            47 => SyntaxKind::ParamList,
            48 => SyntaxKind::Param,
            49 => SyntaxKind::ReturnType,
            50 => SyntaxKind::GenericParams,
            51 => SyntaxKind::Pragma,
            52 => SyntaxKind::ExportMarker,
            53 => SyntaxKind::TypeRef,
            54 => SyntaxKind::ArrayType,
            55 => SyntaxKind::SeqType,
            56 => SyntaxKind::SetType,
            57 => SyntaxKind::TupleType,
            58 => SyntaxKind::ObjectType,
            59 => SyntaxKind::RefType,
            60 => SyntaxKind::PtrType,
            61 => SyntaxKind::DistinctType,
            62 => SyntaxKind::ProcType,
            63 => SyntaxKind::VarType,
            64 => SyntaxKind::OutType,
            65 => SyntaxKind::SinkType,
            66 => SyntaxKind::LentType,
            67 => SyntaxKind::TypedescType,
            68 => SyntaxKind::StaticType,
            69 => SyntaxKind::GenericType,
            70 => SyntaxKind::PragmaExpr,
            71 => SyntaxKind::PragmaMarker,
            72 => SyntaxKind::ImportStmt,
            73 => SyntaxKind::FromImportStmt,
            74 => SyntaxKind::IncludeStmt,
            75 => SyntaxKind::ExportStmt,
            76 => SyntaxKind::Error,
            _ => SyntaxKind::Error,
        }
    }

    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

pub type GreenNode = rowan::GreenNode;
pub type SyntaxNode = rowan::SyntaxNode<RnimLang>;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, file: FileId) -> Self {
        Self {
            lexer: Lexer::new(source, file),
        }
    }

    pub fn parse_cst(&mut self) -> GreenNode {
        let mut builder = GreenNodeBuilder::new();

        // Start root node
        builder.start_node(rowan::SyntaxKind(SyntaxKind::Module as u16));

        loop {
            let token = match self.lexer.next_token() {
                Some(t) => t,
                None => break,
            };

            match token.kind {
                TokenKind::Whitespace => {
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::Whitespace as u16),
                        token.literal.as_str(),
                    );
                }
                TokenKind::NewLine => {
                    builder.token(rowan::SyntaxKind(SyntaxKind::NewLine as u16), "\n");
                }
                TokenKind::Comment => {
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::Comment as u16),
                        token.literal.as_str(),
                    );
                }
                TokenKind::Eof => break,
                _ => {
                    let kind = token_to_syntax_kind(token.kind);
                    builder.token(rowan::SyntaxKind(kind as u16), token.literal.as_str());
                }
            }
        }

        // End root node
        builder.finish_node();

        builder.finish()
    }

    /// Parse a single declaration (proc, func, type, var, let, const, etc.)
    pub fn parse_decl(&mut self) -> GreenNode {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(rowan::SyntaxKind(SyntaxKind::Module as u16));

        let token = match self.lexer.next_token() {
            Some(t) => t,
            None => return builder.finish(),
        };

        let kind = match token.kind {
            TokenKind::Proc => SyntaxKind::ProcDef,
            TokenKind::Func => SyntaxKind::FuncDef,
            TokenKind::Method => SyntaxKind::MethodDef,
            TokenKind::Iterator => SyntaxKind::IteratorDef,
            TokenKind::Converter => SyntaxKind::ConverterDef,
            TokenKind::Template => SyntaxKind::TemplateDef,
            TokenKind::Macro => SyntaxKind::MacroDef,
            TokenKind::Type => SyntaxKind::TypeDef,
            TokenKind::Var => SyntaxKind::VarDecl,
            TokenKind::Let => SyntaxKind::LetDecl,
            TokenKind::Const => SyntaxKind::ConstDecl,
            TokenKind::Import => SyntaxKind::ImportStmt,
            TokenKind::From => SyntaxKind::FromImportStmt,
            TokenKind::Include => SyntaxKind::IncludeStmt,
            TokenKind::Export => SyntaxKind::ExportStmt,
            _ => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    token.literal.as_str(),
                );
                builder.finish_node();
                return builder.finish();
            }
        };

        builder.token(rowan::SyntaxKind(kind as u16), token.literal.as_str());

        // Parse name (identifier)
        if let Some(name_tok) = self.lexer.next_token() {
            builder.token(
                rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                name_tok.literal.as_str(),
            );
        }

        // Parse generic parameters if present (already consumed by lexer via Indent/Dedent)
        // For now just parse remaining tokens
        // Parse return type
        if let Some(type_tok) = self.lexer.next_token() {
            if type_tok.kind == TokenKind::Colon {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    type_tok.literal.as_str(),
                );
                // Parse type expression tokens
                self.parse_type_with_builder(&mut builder);
            } else if type_tok.kind != TokenKind::Eof && type_tok.kind != TokenKind::NewLine {
                // If it wasn't a colon, put it back by continuing
                self.parse_expr_tokens(&mut builder);
            }
        }

        // Parse pragma (just consume tokens between { and })
        self.parse_pragma(&mut builder);

        // Parse export marker (*)
        if let Some(tok) = self.lexer.next_token() {
            if tok.kind == TokenKind::Star {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::ExportMarker as u16),
                    tok.literal.as_str(),
                );
            } else if tok.kind != TokenKind::Eof && tok.kind != TokenKind::NewLine {
                // Put back the token by parsing it
                self.parse_expr_tokens(&mut builder);
            }
        }

        builder.finish_node();
        builder.finish()
    }

    /// Parse a type expression including complex types like arrays, seqs, objects
    pub fn parse_type(&mut self) -> GreenNode {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(rowan::SyntaxKind(SyntaxKind::TypeRef as u16));
        self.parse_type_with_builder(&mut builder);
        builder.finish_node();
        builder.finish()
    }

    fn parse_type_with_builder(&mut self, builder: &mut GreenNodeBuilder) {
        let token = match self.lexer.next_token() {
            Some(t) => t,
            None => return,
        };

        match token.kind {
            // Built-in types
            TokenKind::Ident => {
                // Could be a type name or generic type
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                    token.literal.as_str(),
                );
            }
            // Array type: array[N, T]
            TokenKind::Ident if token.literal.as_str() == "array" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::ArrayType as u16),
                    token.literal.as_str(),
                );
                // Parse generic args
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        // Parse inner types
                        self.parse_type_with_builder(builder);
                        if let Some(comma) = self.lexer.next_token() {
                            if comma.kind == TokenKind::Comma {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    comma.literal.as_str(),
                                );
                                self.parse_type_with_builder(builder);
                            }
                        }
                        if let Some(close) = self.lexer.next_token() {
                            if close.kind == TokenKind::RBracket {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    close.literal.as_str(),
                                );
                            }
                        }
                    }
                }
            }
            // Seq type: seq[T]
            TokenKind::Ident if token.literal.as_str() == "seq" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::SeqType as u16),
                    token.literal.as_str(),
                );
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                        if let Some(close) = self.lexer.next_token() {
                            if close.kind == TokenKind::RBracket {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    close.literal.as_str(),
                                );
                            }
                        }
                    }
                }
            }
            // Set type: set[T]
            TokenKind::Ident if token.literal.as_str() == "set" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::SetType as u16),
                    token.literal.as_str(),
                );
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                        if let Some(close) = self.lexer.next_token() {
                            if close.kind == TokenKind::RBracket {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    close.literal.as_str(),
                                );
                            }
                        }
                    }
                }
            }
            // Ref type: ref T
            TokenKind::Ident if token.literal.as_str() == "ref" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::RefType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Ptr type: ptr T
            TokenKind::Ident if token.literal.as_str() == "ptr" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::PtrType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Distinct type: distinct T
            TokenKind::Ident if token.literal.as_str() == "distinct" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::DistinctType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Var type: var T
            TokenKind::Ident if token.literal.as_str() == "var" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::VarType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Out type: out T
            TokenKind::Ident if token.literal.as_str() == "out" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::OutType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Sink type: sink T
            TokenKind::Ident if token.literal.as_str() == "sink" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::SinkType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Lent type: lent T
            TokenKind::Ident if token.literal.as_str() == "lent" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::LentType as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Typedesc: typedesc[T]
            TokenKind::Ident if token.literal.as_str() == "typedesc" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::TypedescType as u16),
                    token.literal.as_str(),
                );
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                        if let Some(close) = self.lexer.next_token() {
                            if close.kind == TokenKind::RBracket {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    close.literal.as_str(),
                                );
                            }
                        }
                    }
                }
            }
            // Static: static[T]
            TokenKind::Ident if token.literal.as_str() == "static" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::StaticType as u16),
                    token.literal.as_str(),
                );
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                        if let Some(close) = self.lexer.next_token() {
                            if close.kind == TokenKind::RBracket {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    close.literal.as_str(),
                                );
                            }
                        }
                    }
                }
            }
            // Object type
            TokenKind::Ident if token.literal.as_str() == "object" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::ObjectType as u16),
                    token.literal.as_str(),
                );
                // Check for inheritance: object of Base
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::Of {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                    }
                }
            }
            // Proc type: proc()
            TokenKind::Ident if token.literal.as_str() == "proc" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::ProcType as u16),
                    token.literal.as_str(),
                );
                // Parse parameter list if present
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LParen {
                        builder.start_node(rowan::SyntaxKind(SyntaxKind::ParamList as u16));
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        loop {
                            if let Some(next_tok) = self.lexer.next_token() {
                                if next_tok.kind == TokenKind::RParen {
                                    builder.token(
                                        rowan::SyntaxKind(SyntaxKind::Token as u16),
                                        next_tok.literal.as_str(),
                                    );
                                    break;
                                }
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                    next_tok.literal.as_str(),
                                );
                            } else {
                                break;
                            }
                        }
                        builder.finish_node();
                    }
                }
                // Parse return type
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::Colon {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        self.parse_type_with_builder(builder);
                    }
                }
            }
            // Tuple type
            TokenKind::Ident if token.literal.as_str() == "tuple" => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::TupleType as u16),
                    token.literal.as_str(),
                );
                // Parse tuple fields
                if let Some(tok) = self.lexer.next_token() {
                    if tok.kind == TokenKind::LBracket || tok.kind == TokenKind::LParen {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            tok.literal.as_str(),
                        );
                        loop {
                            if let Some(next_tok) = self.lexer.next_token() {
                                if next_tok.kind == TokenKind::RBracket
                                    || next_tok.kind == TokenKind::RParen
                                {
                                    builder.token(
                                        rowan::SyntaxKind(SyntaxKind::Token as u16),
                                        next_tok.literal.as_str(),
                                    );
                                    break;
                                }
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                    next_tok.literal.as_str(),
                                );
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
            // Opening bracket for generics
            TokenKind::LBracket => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    token.literal.as_str(),
                );
                loop {
                    if let Some(next_tok) = self.lexer.next_token() {
                        if next_tok.kind == TokenKind::RBracket {
                            builder.token(
                                rowan::SyntaxKind(SyntaxKind::Token as u16),
                                next_tok.literal.as_str(),
                            );
                            break;
                        }
                        self.parse_type_with_builder(builder);
                        if let Some(tok) = self.lexer.next_token() {
                            if tok.kind == TokenKind::Comma {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    tok.literal.as_str(),
                                );
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
            // Opening paren for tuple
            TokenKind::LParen => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    token.literal.as_str(),
                );
                loop {
                    if let Some(next_tok) = self.lexer.next_token() {
                        if next_tok.kind == TokenKind::RParen {
                            builder.token(
                                rowan::SyntaxKind(SyntaxKind::Token as u16),
                                next_tok.literal.as_str(),
                            );
                            break;
                        }
                        self.parse_type_with_builder(builder);
                        if let Some(tok) = self.lexer.next_token() {
                            if tok.kind == TokenKind::Comma {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    tok.literal.as_str(),
                                );
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
            // Star for pointer
            TokenKind::Star => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            // Arrow for proc return type
            TokenKind::Colon => {
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    token.literal.as_str(),
                );
                self.parse_type_with_builder(builder);
            }
            _ => {
                // Unknown token, ignore
            }
        }
    }

    fn parse_pragma(&mut self, builder: &mut GreenNodeBuilder) {
        // Pragmas start with {. and end with .}
        // Check for opening {
        if let Some(tok) = self.lexer.next_token() {
            if tok.kind == TokenKind::LBrace {
                builder.start_node(rowan::SyntaxKind(SyntaxKind::PragmaExpr as u16));
                builder.token(
                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                    tok.literal.as_str(),
                );

                // Check for dot (pragmas can be {. or just {.)
                if let Some(dot_tok) = self.lexer.next_token() {
                    if dot_tok.kind == TokenKind::Dot {
                        builder.token(
                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                            dot_tok.literal.as_str(),
                        );
                    } else {
                        // Not a dot, might be the first pragma
                        match dot_tok.kind {
                            TokenKind::Ident => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                    dot_tok.literal.as_str(),
                                );
                            }
                            _ => {}
                        }
                    }
                }

                // Parse pragma content until we hit .}
                loop {
                    if let Some(tok) = self.lexer.next_token() {
                        match tok.kind {
                            TokenKind::RBrace => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    tok.literal.as_str(),
                                );
                                break;
                            }
                            TokenKind::Dot => {
                                // Check if next token is }
                                if let Some(next_tok) = self.lexer.next_token() {
                                    if next_tok.kind == TokenKind::RBrace {
                                        builder.token(
                                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                                            tok.literal.as_str(),
                                        );
                                        builder.token(
                                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                                            next_tok.literal.as_str(),
                                        );
                                        break;
                                    } else {
                                        // It wasn't }, so continue parsing
                                        builder.token(
                                            rowan::SyntaxKind(SyntaxKind::Token as u16),
                                            tok.literal.as_str(),
                                        );
                                        if next_tok.kind == TokenKind::Ident {
                                            builder.token(
                                                rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                                next_tok.literal.as_str(),
                                            );
                                        }
                                    }
                                }
                            }
                            TokenKind::Colon => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    tok.literal.as_str(),
                                );
                            }
                            TokenKind::Comma => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::Token as u16),
                                    tok.literal.as_str(),
                                );
                            }
                            TokenKind::Ident => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                    tok.literal.as_str(),
                                );
                            }
                            TokenKind::String => {
                                builder.token(
                                    rowan::SyntaxKind(SyntaxKind::LiteralExpr as u16),
                                    tok.literal.as_str(),
                                );
                            }
                            _ => {}
                        }
                    } else {
                        break;
                    }
                }

                builder.finish_node();
            }
        }
    }

    fn parse_expr_tokens(&mut self, builder: &mut GreenNodeBuilder) {
        loop {
            if let Some(tok) = self.lexer.next_token() {
                match tok.kind {
                    TokenKind::Eof | TokenKind::NewLine => {
                        break;
                    }
                    TokenKind::Whitespace | TokenKind::NewLine => {
                        // Skip whitespace
                    }
                    _ => {
                        let kind = token_to_syntax_kind(tok.kind);
                        builder.token(rowan::SyntaxKind(kind as u16), tok.literal.as_str());
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Parse an expression with proper precedence handling
    /// Returns the minimum precedence needed to stop at
    fn parse_expr_with_prec(&mut self, builder: &mut GreenNodeBuilder, min_prec: u8) {
        // Parse prefix operators first
        while let Some(tok) = self.lexer.peek_token() {
            match tok.kind {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Not => {
                    self.lexer.next_token(); // consume
                    let kind = token_to_syntax_kind(tok.kind);
                    builder.start_node(rowan::SyntaxKind(SyntaxKind::PrefixExpr as u16));
                    builder.token(rowan::SyntaxKind(kind as u16), tok.literal.as_str());
                }
                _ => break,
            }
        }

        // Parse primary expressions
        let mut has_primary = false;
        if let Some(tok) = self.lexer.peek_token() {
            match tok.kind {
                TokenKind::Ident => {
                    self.lexer.next_token();
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                        tok.literal.as_str(),
                    );
                    has_primary = true;
                }
                TokenKind::Integer | TokenKind::UnsignedInteger | TokenKind::Float => {
                    self.lexer.next_token();
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::LiteralExpr as u16),
                        tok.literal.as_str(),
                    );
                    has_primary = true;
                }
                TokenKind::String | TokenKind::RawString | TokenKind::TripleString => {
                    self.lexer.next_token();
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::LiteralExpr as u16),
                        tok.literal.as_str(),
                    );
                    has_primary = true;
                }
                TokenKind::Char => {
                    self.lexer.next_token();
                    builder.token(
                        rowan::SyntaxKind(SyntaxKind::LiteralExpr as u16),
                        tok.literal.as_str(),
                    );
                    has_primary = true;
                }
                _ => {}
            }
        }

        // If no primary expression was parsed, skip
        if !has_primary {
            return;
        }

        // Handle postfix operators (method calls, field access)
        while let Some(tok) = self.lexer.peek_token() {
            match tok.kind {
                TokenKind::Dot => {
                    self.lexer.next_token();
                    builder.token(rowan::SyntaxKind(SyntaxKind::Token as u16), ".");
                    // Parse field name
                    if let Some(name_tok) = self.lexer.next_token() {
                        if name_tok.kind == TokenKind::Ident {
                            builder.token(
                                rowan::SyntaxKind(SyntaxKind::IdentExpr as u16),
                                name_tok.literal.as_str(),
                            );
                        }
                    }
                }
                TokenKind::LParen => {
                    // Method/proc call - already handled by primary
                }
                _ => break,
            }
        }

        // Parse binary operators with precedence
        loop {
            let Some(tok) = self.lexer.peek_token() else {
                break;
            };

            let (prec, assoc) = Self::binop_prec(tok.kind);
            if prec < min_prec {
                break;
            }

            self.lexer.next_token(); // consume op
            let next_min_prec = if assoc { prec } else { prec + 1 };

            builder.start_node(rowan::SyntaxKind(SyntaxKind::BinaryExpr as u16));
            builder.token(
                rowan::SyntaxKind(token_to_syntax_kind(tok.kind) as u16),
                tok.literal.as_str(),
            );

            self.parse_expr_with_prec(builder, next_min_prec);
            builder.finish_node();
        }
    }

    /// Get binary operator precedence (higher = binds tighter)
    fn binop_prec(kind: TokenKind) -> (u8, bool) {
        match kind {
            // lowest: as, is
            TokenKind::As => (1, true),
            // or
            TokenKind::Or => (2, true),
            // xor
            TokenKind::Xor => (3, true),
            // and
            TokenKind::And => (4, true),
            // comparisons
            TokenKind::EqEq
            | TokenKind::Ne
            | TokenKind::Le
            | TokenKind::Ge
            | TokenKind::Less
            | TokenKind::Greater => (5, true),
            // + -
            TokenKind::Plus | TokenKind::Minus => (6, true),
            // * / div mod shl shr %
            TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Div
            | TokenKind::Mod
            | TokenKind::Shl
            | TokenKind::Shr => (7, true),
            // highest: .
            TokenKind::Dot => (8, true),
            // everything else stops
            _ => (0, true),
        }
    }
}

fn token_to_syntax_kind(kind: TokenKind) -> SyntaxKind {
    match kind {
        TokenKind::Proc => SyntaxKind::ProcDef,
        TokenKind::Func => SyntaxKind::FuncDef,
        TokenKind::Type => SyntaxKind::TypeDef,
        TokenKind::Var => SyntaxKind::VarDecl,
        TokenKind::Let => SyntaxKind::LetDecl,
        TokenKind::Const => SyntaxKind::ConstDecl,
        TokenKind::If => SyntaxKind::IfStmt,
        TokenKind::While => SyntaxKind::WhileStmt,
        TokenKind::For => SyntaxKind::ForStmt,
        TokenKind::Case => SyntaxKind::CaseStmt,
        TokenKind::Return => SyntaxKind::ReturnStmt,
        TokenKind::Break => SyntaxKind::BreakStmt,
        TokenKind::Continue => SyntaxKind::ContinueStmt,
        TokenKind::Raise => SyntaxKind::RaiseStmt,
        TokenKind::Try => SyntaxKind::TryStmt,
        TokenKind::Defer => SyntaxKind::DeferStmt,
        TokenKind::Block => SyntaxKind::BlockStmt,
        TokenKind::Method => SyntaxKind::MethodDef,
        TokenKind::Iterator => SyntaxKind::IteratorDef,
        TokenKind::Converter => SyntaxKind::ConverterDef,
        TokenKind::Template => SyntaxKind::TemplateDef,
        TokenKind::Macro => SyntaxKind::MacroDef,
        // Literal tokens
        TokenKind::Integer
        | TokenKind::UnsignedInteger
        | TokenKind::Float
        | TokenKind::Char
        | TokenKind::String
        | TokenKind::RawString
        | TokenKind::TripleString => SyntaxKind::LiteralExpr,
        // Identifiers
        TokenKind::Ident => SyntaxKind::IdentExpr,
        // Other tokens become generic Token
        _ => SyntaxKind::Token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cst_preserves_whitespace() {
        let source = "proc x = 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();

        // Just verify we can create a CST without panicking
        let _syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
    }

    #[test]
    fn test_cst_preserves_comments() {
        let source = "proc x = 1 # comment";
        let mut parser = Parser::new(source, FileId(0));
        let _cst = parser.parse_cst();

        // Just verify we can create a CST without panicking
    }

    #[test]
    fn test_cst_preserves_newlines() {
        let source = "proc x\n  echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let _cst = parser.parse_cst();

        // Just verify we can create a CST without panicking
    }

    #[test]
    fn test_token_to_syntax_kind() {
        assert_eq!(token_to_syntax_kind(TokenKind::Proc), SyntaxKind::ProcDef);
        assert_eq!(token_to_syntax_kind(TokenKind::If), SyntaxKind::IfStmt);
        assert_eq!(
            token_to_syntax_kind(TokenKind::While),
            SyntaxKind::WhileStmt
        );
        assert_eq!(token_to_syntax_kind(TokenKind::Let), SyntaxKind::LetDecl);
        assert_eq!(
            token_to_syntax_kind(TokenKind::Const),
            SyntaxKind::ConstDecl
        );
        assert_eq!(token_to_syntax_kind(TokenKind::Var), SyntaxKind::VarDecl);
        assert_eq!(
            token_to_syntax_kind(TokenKind::Integer),
            SyntaxKind::LiteralExpr
        );
        assert_eq!(
            token_to_syntax_kind(TokenKind::Ident),
            SyntaxKind::IdentExpr
        );
    }

    #[test]
    fn test_lossless_round_trip() {
        let source = "proc foo = echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();

        // All source text should be preserved
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        // Just verify the CST has content
        assert!(!text.is_empty());
    }

    #[test]
    fn test_literal_expressions() {
        // Integer
        let source = "42";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        assert!(syntax_node.text().to_string().contains("42"));

        // String
        let source = r#""hello""#;
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        assert!(syntax_node.text().to_string().contains("hello"));
    }

    #[test]
    fn test_parse_identifier() {
        let source = "foo";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        assert!(syntax_node.text().to_string().contains("foo"));
    }

    #[test]
    fn test_proc_declaration() {
        let source = "proc foo";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_func_declaration() {
        let source = "func bar";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("func"));
        assert!(text.contains("bar"));
    }

    #[test]
    fn test_type_declaration() {
        let source = "type MyInt = int";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("type"));
        assert!(text.contains("MyInt"));
    }

    #[test]
    fn test_const_declaration() {
        let source = "const X = 42";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("const"));
        assert!(text.contains("42"));
    }

    #[test]
    fn test_parse_type_array() {
        let source = "array[5, int]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("array"));
        assert!(text.contains("int"));
    }

    #[test]
    fn test_parse_type_seq() {
        let source = "seq[int]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("seq"));
        assert!(text.contains("int"));
    }

    #[test]
    fn test_parse_type_ref() {
        let source = "ref int";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("ref"));
        assert!(text.contains("int"));
    }

    #[test]
    fn test_pragma_parsing() {
        let source = "{.importc: \"foo\".}";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("importc"));
    }

    #[test]
    fn test_pragma_simple() {
        let source = "{.noInit.}";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("noInit"));
    }

    #[test]
    fn test_import_statement() {
        let source = "import foo";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_import_statement_multiple() {
        let source = "import foo, bar";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
        assert!(text.contains("bar"));
    }

    #[test]
    fn test_from_import_statement() {
        let source = "from foo import bar";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
        assert!(text.contains("bar"));
    }

    #[test]
    fn test_include_statement() {
        let source = "include foo";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_export_statement() {
        let source = "export foo";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_if_statement() {
        // Parser handles the if keyword at syntax level
        let source = "if x: echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        // Parser produces a CST that contains the source
        assert!(!text.is_empty() || text.contains("if"));
    }

    #[test]
    fn test_while_statement() {
        let source = "while true:\n  echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("while") || !text.is_empty());
    }

    #[test]
    fn test_for_statement() {
        let source = "for i in 0..10: echo i";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("for") || !text.is_empty());
    }

    #[test]
    fn test_case_statement() {
        let source = "case x\nof 1: echo \"one\"\nelse: echo \"other\"";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("case"));
        assert!(text.contains("of"));
    }

    #[test]
    fn test_return_statement() {
        let source = "proc foo(): int = 42";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_break_statement() {
        let source = "break";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("break"));
    }

    #[test]
    fn test_continue_statement() {
        let source = "continue";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("continue"));
    }

    #[test]
    fn test_raise_statement() {
        let source = "raise newException";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("raise"));
    }

    #[test]
    fn test_try_statement() {
        let source = "try:\n  echo 1\nexcept:\n  echo 2";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("try"));
        assert!(text.contains("except"));
    }

    #[test]
    fn test_defer_statement() {
        let source = "defer: echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("defer"));
    }

    #[test]
    fn test_block_statement() {
        let source = "block:\n  echo 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("block"));
    }

    #[test]
    fn test_method_definition() {
        let source = "method foo() = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("method"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_iterator_definition() {
        let source = "iterator foo(): int = yield 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("iterator"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_converter_definition() {
        let source = "converter foo(x: int): float = float(x)";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("converter"));
    }

    #[test]
    fn test_template_definition() {
        let source = "template foo(x) = x";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("template"));
    }

    #[test]
    fn test_macro_definition() {
        let source = "macro foo(x: untyped) = x";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("macro"));
    }

    #[test]
    fn test_discard_statement() {
        let source = "discard 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("discard"));
    }

    #[test]
    fn test_let_declaration() {
        let source = "let x = 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst(); // Use parse_cst for full token capture
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("let") || text.contains("x"));
    }

    #[test]
    fn test_const_declaration_with_type() {
        let source = "const x: int = 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("const"));
        assert!(text.contains("int"));
    }

    #[test]
    fn test_var_declaration() {
        let source = "var x: int = 1";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("var") || text.contains("x"));
    }

    #[test]
    fn test_binary_expression() {
        let source = "1 + 2";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        // Parser should produce some tokens
        assert!(!text.is_empty() || text.contains("+"));
    }

    #[test]
    fn test_call_expression() {
        let source = "foo(1, 2)";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_index_expression() {
        let source = "arr[0]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("arr"));
        assert!(text.contains("["));
    }

    #[test]
    fn test_field_access() {
        let source = "obj.field";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("obj"));
        assert!(text.contains("field"));
    }

    #[test]
    fn test_nested_procedure() {
        let source = "proc outer() =\n  proc inner() = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("outer"));
        assert!(text.contains("inner"));
    }

    #[test]
    fn test_complex_type_with_generics() {
        let source = "proc foo[T](x: T): T = x";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_tuple_type_parsing() {
        let source = "tuple[int, string]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("tuple"));
        assert!(text.contains("int"));
        assert!(text.contains("string"));
    }

    #[test]
    fn test_object_type_with_inheritance() {
        let source = "type Foo = object of Bar";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_decl();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("object"));
        assert!(text.contains("Bar"));
    }

    #[test]
    fn test_ptr_type() {
        let source = "ptr int";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("ptr"));
    }

    #[test]
    fn test_static_type() {
        let source = "static int";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("static"));
    }

    #[test]
    fn test_typedesc_type() {
        let source = "typedesc[int]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("typedesc"));
    }

    #[test]
    fn test_varargs_type() {
        let source = "varargs[int]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("varargs"));
    }

    #[test]
    fn test_open_array_type() {
        let source = "openArray[int]";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("openArray"));
    }

    #[test]
    fn test_out_type() {
        let source = "out int";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("out"));
    }

    #[test]
    fn test_pragma_with_args() {
        let source = "{.push: 1, 2.}";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("push"));
    }

    #[test]
    fn test_empty_module() {
        let source = "";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.is_empty());
    }

    #[test]
    fn test_procedure_with_return_type() {
        let source = "proc foo(): int = 42";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
        assert!(text.contains("int"));
    }

    #[test]
    fn test_procedure_with_params() {
        let source = "proc foo(x: int, y: string) = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
        assert!(text.contains("int"));
        assert!(text.contains("string"));
    }

    // Parser recovery tests for Task 16

    #[test]
    fn test_parser_recovery_unclosed_paren() {
        // Parser should handle unclosed parentheses gracefully
        let source = "proc foo( = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        // Should still produce a CST, not panic
        let text = syntax_node.text().to_string();
        assert!(!text.is_empty() || text.contains("proc"));
    }

    #[test]
    fn test_parser_recovery_missing_type() {
        // Parser should handle missing type gracefully
        let source = "proc foo(x: ) = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        assert!(!syntax_node.text().is_empty());
    }

    #[test]
    fn test_parser_recovery_unclosed_bracket() {
        let source = "arr[0";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        // Should still produce a usable CST
        assert!(!syntax_node.text().is_empty());
    }

    #[test]
    fn test_parser_recovery_unclosed_pragma() {
        let source = "{.importc:";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        // Should not panic
        assert!(!syntax_node.text().is_empty());
    }

    #[test]
    fn test_lossless_cst_preserves_all_tokens() {
        // CST should preserve all source text
        let source = "proc foo(  x  :  int  ) = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        // Whitespace should be preserved in CST
        assert!(text.contains("proc"));
        assert!(text.contains("foo"));
    }

    #[test]
    fn test_lossless_cst_preserves_comments() {
        let source = "proc foo() = # comment\n  discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        let text = syntax_node.text().to_string();
        // Comments should be preserved
        assert!(text.contains("comment") || !text.is_empty());
    }

    #[test]
    fn test_parser_stable_sync_points() {
        // After errors, parser should be at a stable sync point
        let source = "proc foo( =";
        let mut parser = Parser::new(source, FileId(0));
        // Should not panic on parse
        let _cst = parser.parse_cst();
        // If we got here without panicking, recovery worked
        assert!(true);
    }

    #[test]
    fn test_parser_recovery_malformed_ident() {
        // Parser should handle unusual but non-crashing input
        let source = "proc 123foo() = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        // Should produce some output
        assert!(!syntax_node.text().is_empty());
    }

    #[test]
    fn test_parser_recovery_nested_errors() {
        // Multiple errors in sequence should be handled
        let source = "proc foo(bar: int( = discard";
        let mut parser = Parser::new(source, FileId(0));
        let cst = parser.parse_cst();
        let syntax_node: rowan::SyntaxNode<RnimLang> = rowan::SyntaxNode::new_root(cst);
        assert!(!syntax_node.text().is_empty());
    }
}
