// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! AST node definitions for C++ code

use crate::util::{InternedString, Span};

/// Root of the AST for a translation unit (a single source file)
#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub file_id: crate::util::FileId,
    pub includes: Vec<IncludeDirective>,
    pub declarations: Vec<Declaration>,
    pub errors: Vec<ParseError>,
}

/// Include directive (#include)
#[derive(Debug, Clone)]
pub struct IncludeDirective {
    pub path: String,
    pub is_system: bool,  // <header> vs "header.h"
    pub span: Span,
}

/// Parse error information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

/// Top-level declarations in a translation unit
#[derive(Debug, Clone)]
pub enum Declaration {
    Namespace(NamespaceDecl),
    Class(ClassDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Function(FunctionDecl),
    Variable(VariableDecl),
    TypeAlias(TypeAliasDecl),
    Template(TemplateDecl),
    Using(UsingDecl),
    UsingDirective(UsingDirective),
    // UE-specific declarations
    UClass(UClassDecl),
    UStruct(UStructDecl),
    UEnum(UEnumDecl),
}

/// Namespace declaration
#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub name: Option<InternedString>, // None for anonymous namespace
    pub span: Span,
    pub declarations: Vec<Declaration>,
}

/// Class declaration
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: InternedString,
    pub span: Span,
    pub access: AccessSpecifier,
    pub bases: Vec<BaseClass>,
    pub members: Vec<ClassMember>,
    pub is_struct: bool, // true if declared with 'struct' keyword
    pub template_params: Option<Vec<TemplateParam>>,
    pub doc_comment: Option<String>,
}

/// Base class specification
#[derive(Debug, Clone)]
pub struct BaseClass {
    pub type_path: TypePath,
    pub access: AccessSpecifier,
    pub is_virtual: bool,
    pub span: Span,
}

/// Access specifier for members
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSpecifier {
    Public,
    Protected,
    Private,
}

/// Class member
#[derive(Debug, Clone)]
pub enum ClassMember {
    Field(FieldDecl),
    Method(FunctionDecl),
    Constructor(ConstructorDecl),
    Destructor(DestructorDecl),
    TypeAlias(TypeAliasDecl),
    NestedClass(ClassDecl),
    NestedEnum(EnumDecl),
    UProperty(UPropertyDecl),
    UFunction(UFunctionDecl),
}

/// Struct declaration (equivalent to class with public default access)
pub type StructDecl = ClassDecl;

/// Enum declaration
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: InternedString,
    pub span: Span,
    pub is_class: bool, // true for enum class
    pub underlying_type: Option<Type>,
    pub variants: Vec<EnumVariant>,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: InternedString,
    pub span: Span,
    pub value: Option<Expr>,
}

/// Function declaration
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: InternedString,
    pub span: Span,
    pub return_type: Type,
    pub parameters: Vec<Parameter>,
    pub is_const: bool,
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_inline: bool,
    pub body: Option<FunctionBody>,
    pub doc_comment: Option<String>,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Option<InternedString>,
    pub ty: Type,
    pub default_value: Option<Expr>,
    pub span: Span,
}

/// Function body
#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub span: Span,
    pub statements: Vec<Stmt>,
}

/// Constructor declaration
#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub span: Span,
    pub parameters: Vec<Parameter>,
    pub initializer_list: Vec<MemberInitializer>,
    pub body: Option<FunctionBody>,
    pub is_explicit: bool,
    pub is_default: bool,
    pub is_delete: bool,
}

/// Member initializer in constructor
#[derive(Debug, Clone)]
pub struct MemberInitializer {
    pub member: InternedString,
    pub init_expr: Expr,
    pub span: Span,
}

/// Destructor declaration
#[derive(Debug, Clone)]
pub struct DestructorDecl {
    pub span: Span,
    pub body: Option<FunctionBody>,
    pub is_virtual: bool,
    pub is_default: bool,
    pub is_delete: bool,
}

/// Field declaration
#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: InternedString,
    pub ty: Type,
    pub span: Span,
    pub access: AccessSpecifier,
    pub is_static: bool,
    pub is_mutable: bool,
    pub initializer: Option<Expr>,
}

/// Variable declaration
#[derive(Debug, Clone)]
pub struct VariableDecl {
    pub name: InternedString,
    pub ty: Type,
    pub span: Span,
    pub is_const: bool,
    pub is_static: bool,
    pub initializer: Option<Expr>,
}

/// Type alias (typedef or using)
#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub name: InternedString,
    pub target: Type,
    pub span: Span,
}

/// Template declaration
#[derive(Debug, Clone)]
pub struct TemplateDecl {
    pub parameters: Vec<TemplateParam>,
    pub declaration: Box<Declaration>,
    pub span: Span,
}

/// Template parameter
#[derive(Debug, Clone)]
pub enum TemplateParam {
    Type {
        name: InternedString,
        default: Option<Type>,
    },
    Value {
        name: InternedString,
        ty: Type,
        default: Option<Expr>,
    },
    Template {
        name: InternedString,
        params: Vec<TemplateParam>,
    },
}

/// Using declaration
#[derive(Debug, Clone)]
pub struct UsingDecl {
    pub path: TypePath,
    pub span: Span,
}

/// Using directive
#[derive(Debug, Clone)]
pub struct UsingDirective {
    pub namespace: TypePath,
    pub span: Span,
}

/// Type representation
#[derive(Debug, Clone)]
pub enum Type {
    Void,
    Primitive(PrimitiveType),
    Named(TypePath),
    Pointer(Box<Type>, PointerKind),
    Reference(Box<Type>, ReferenceKind),
    Array(Box<Type>, Option<usize>),
    TemplateInstantiation {
        template: TypePath,
        args: Vec<TemplateArg>,
    },
    Function {
        return_type: Box<Type>,
        params: Vec<Type>,
    },
    Auto, // auto keyword
    Decltype(Box<Expr>),
    Error, // Placeholder for parse errors
}

/// Primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    Char,
    SignedChar,
    UnsignedChar,
    WChar,
    Char16,
    Char32,
    Short,
    UnsignedShort,
    Int,
    UnsignedInt,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
    Float,
    Double,
    LongDouble,
}

/// Pointer kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    Raw,
    Member, // pointer to member
}

/// Reference kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    LValue, // &
    RValue, // &&
}

/// Qualified type path (e.g., std::vector, UE::FVector)
#[derive(Debug, Clone)]
pub struct TypePath {
    pub segments: Vec<InternedString>,
    pub is_global: bool, // starts with ::
}

impl TypePath {
    pub fn simple(name: InternedString) -> Self {
        Self {
            segments: vec![name],
            is_global: false,
        }
    }
}

/// Template argument
#[derive(Debug, Clone)]
pub enum TemplateArg {
    Type(Type),
    Expr(Expr),
}

/// Simplified expression (for Phase 1, we just track the span)
#[derive(Debug, Clone)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    // Literals
    Literal(Literal),

    // Names and paths
    Identifier(InternedString),
    QualifiedName(TypePath),

    // Function calls and invocations
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    // Member access
    MemberAccess {
        object: Box<Expr>,
        member: InternedString,
        is_arrow: bool, // true for ->, false for .
    },

    // Operators
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    // Increment/decrement
    PostIncrement(Box<Expr>),
    PostDecrement(Box<Expr>),
    PreIncrement(Box<Expr>),
    PreDecrement(Box<Expr>),

    // Assignment
    Assign {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    CompoundAssign {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // Conditional
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    // Type operations
    Cast {
        ty: Type,
        expr: Box<Expr>,
    },
    SizeOf(Box<Type>),
    SizeOfExpr(Box<Expr>),
    AlignOf(Box<Type>),
    TypeId(Box<Type>),

    // Array/subscript
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },

    // Lambda
    Lambda {
        captures: Vec<LambdaCapture>,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Vec<Stmt>,
    },

    // Constructors
    NewExpr {
        ty: Type,
        args: Vec<Expr>,
        is_array: bool,
    },
    DeleteExpr {
        expr: Box<Expr>,
        is_array: bool,
    },

    // Initializer list
    InitializerList(Vec<Expr>),

    // Parenthesized expression
    Paren(Box<Expr>),

    // Comma operator
    Comma(Vec<Expr>),

    // This pointer
    This,

    // Error placeholder
    Error,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Nullptr,
}

/// Lambda capture
#[derive(Debug, Clone)]
pub struct LambdaCapture {
    pub name: InternedString,
    pub kind: CaptureKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    ByValue,
    ByReference,
    ByMove,
}

/// Statement
#[derive(Debug, Clone)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    // Declarations
    VarDecl {
        name: InternedString,
        ty: Option<Type>,
        init: Option<Expr>,
    },

    // Expression statement
    Expr(Expr),

    // Control flow
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    DoWhile {
        body: Vec<Stmt>,
        condition: Expr,
    },
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: Vec<Stmt>,
    },
    RangeFor {
        var: InternedString,
        ty: Option<Type>,
        range: Expr,
        body: Vec<Stmt>,
    },
    Switch {
        condition: Expr,
        cases: Vec<SwitchCase>,
    },

    // Jump statements
    Return(Option<Expr>),
    Break,
    Continue,
    Goto(InternedString),
    Label(InternedString),

    // Exception handling
    Try {
        body: Vec<Stmt>,
        catch_clauses: Vec<CatchClause>,
    },
    Throw(Option<Expr>),

    // Block
    Block(Vec<Stmt>),

    // Empty statement
    Empty,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub pattern: Option<Expr>, // None for default case
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub exception_type: Option<Type>,
    pub name: Option<InternedString>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // Pointer-to-member
    PtrToMember,      // .*
    PtrToMemberArrow, // ->*
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitwiseNot,
    Deref,
    AddressOf,
}

// ============================================================================
// UE5-Specific AST Nodes
// ============================================================================

/// UCLASS declaration (class with UCLASS() macro)
#[derive(Debug, Clone)]
pub struct UClassDecl {
    pub class_decl: ClassDecl,
    pub specifiers: UClassSpecifiers,
}

/// UCLASS specifiers extracted from the macro
#[derive(Debug, Clone, Default)]
pub struct UClassSpecifiers {
    pub blueprintable: bool,
    pub blueprint_type: bool,
    pub abstract_class: bool,
    pub config: Option<InternedString>,
    pub category: Option<String>,
    pub meta: Vec<(InternedString, Option<String>)>,
}

/// USTRUCT declaration
#[derive(Debug, Clone)]
pub struct UStructDecl {
    pub struct_decl: StructDecl,
    pub specifiers: UStructSpecifiers,
}

#[derive(Debug, Clone, Default)]
pub struct UStructSpecifiers {
    pub blueprintable: bool,
    pub atomic: bool,
}

/// UENUM declaration
#[derive(Debug, Clone)]
pub struct UEnumDecl {
    pub enum_decl: EnumDecl,
    pub specifiers: UEnumSpecifiers,
}

#[derive(Debug, Clone, Default)]
pub struct UEnumSpecifiers {
    pub blueprintable: bool,
}

/// UPROPERTY declaration
#[derive(Debug, Clone)]
pub struct UPropertyDecl {
    pub field: FieldDecl,
    pub specifiers: UPropertySpecifiers,
}

#[derive(Debug, Clone, Default)]
pub struct UPropertySpecifiers {
    pub edit_anywhere: bool,
    pub edit_default_only: bool,
    pub edit_instance_only: bool,
    pub visible_anywhere: bool,
    pub blueprint_read_write: bool,
    pub blueprint_read_only: bool,
    pub replicated: bool,
    pub category: Option<String>,
    pub meta: Vec<(InternedString, Option<String>)>,
}

/// UFUNCTION declaration
#[derive(Debug, Clone)]
pub struct UFunctionDecl {
    pub function: FunctionDecl,
    pub specifiers: UFunctionSpecifiers,
}

#[derive(Debug, Clone, Default)]
pub struct UFunctionSpecifiers {
    pub blueprint_callable: bool,
    pub blueprint_pure: bool,
    pub blueprint_implementable_event: bool,
    pub blueprint_native_event: bool,
    pub exec: bool,
    pub server: bool,
    pub client: bool,
    pub reliable: bool,
    pub unreliable: bool,
    pub category: Option<String>,
    pub meta: Vec<(InternedString, Option<String>)>,
}
