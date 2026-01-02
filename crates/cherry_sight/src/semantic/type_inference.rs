// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Type inference for expressions

use crate::ast::*;
use crate::index::{SymbolId, SymbolKind, SymbolTable};
use crate::semantic::name_resolution::NameResolution;
use crate::util::{Interner, InternedString, Span};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Type information for an expression
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub ty: Type,
    pub is_lvalue: bool,
}

/// Type inference engine
pub struct TypeInference {
    symbol_table: Arc<RwLock<SymbolTable>>,
    interner: Arc<RwLock<Interner>>,
    name_resolution: NameResolution,

    /// Map from expression spans to their inferred types
    type_map: HashMap<Span, TypeInfo>,
}

impl TypeInference {
    pub fn new(
        symbol_table: Arc<RwLock<SymbolTable>>,
        interner: Arc<RwLock<Interner>>,
        name_resolution: NameResolution,
    ) -> Self {
        Self {
            symbol_table,
            interner,
            name_resolution,
            type_map: HashMap::new(),
        }
    }

    /// Infer types for a translation unit
    pub fn infer(&mut self, ast: &TranslationUnit) -> HashMap<Span, TypeInfo> {
        for decl in &ast.declarations {
            self.infer_declaration(decl);
        }

        self.type_map.clone()
    }

    fn infer_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Function(func) => {
                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.infer_stmt(stmt);
                    }
                }
            }
            Declaration::Class(cls) | Declaration::Struct(cls) => {
                for member in &cls.members {
                    if let ClassMember::Method(method) = member {
                        if let Some(ref body) = method.body {
                            for stmt in &body.statements {
                                self.infer_stmt(stmt);
                            }
                        }
                    }
                }
            }
            Declaration::Namespace(ns) => {
                for decl in &ns.declarations {
                    self.infer_declaration(decl);
                }
            }
            Declaration::UClass(ucls) => {
                for member in &ucls.class_decl.members {
                    if let ClassMember::Method(method) = member {
                        if let Some(ref body) = method.body {
                            for stmt in &body.statements {
                                self.infer_stmt(stmt);
                            }
                        }
                    } else if let ClassMember::UFunction(ufunc) = member {
                        if let Some(ref body) = ufunc.function.body {
                            for stmt in &body.statements {
                                self.infer_stmt(stmt);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn infer_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Expr(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::VarDecl { init, .. } => {
                if let Some(init) = init {
                    self.infer_expr(init);
                }
            }
            StmtKind::If { condition, then_block, else_block } => {
                self.infer_expr(condition);
                for stmt in then_block {
                    self.infer_stmt(stmt);
                }
                if let Some(else_block) = else_block {
                    for stmt in else_block {
                        self.infer_stmt(stmt);
                    }
                }
            }
            StmtKind::While { condition, body } | StmtKind::DoWhile { body, condition } => {
                self.infer_expr(condition);
                for stmt in body {
                    self.infer_stmt(stmt);
                }
            }
            StmtKind::For { init, condition, increment, body } => {
                if let Some(init) = init {
                    self.infer_stmt(init);
                }
                if let Some(condition) = condition {
                    self.infer_expr(condition);
                }
                if let Some(increment) = increment {
                    self.infer_expr(increment);
                }
                for stmt in body {
                    self.infer_stmt(stmt);
                }
            }
            StmtKind::RangeFor { range, body, .. } => {
                self.infer_expr(range);
                for stmt in body {
                    self.infer_stmt(stmt);
                }
            }
            StmtKind::Switch { condition, cases } => {
                self.infer_expr(condition);
                for case in cases {
                    if let Some(pattern) = &case.pattern {
                        self.infer_expr(pattern);
                    }
                    for stmt in &case.stmts {
                        self.infer_stmt(stmt);
                    }
                }
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.infer_expr(expr);
                }
            }
            StmtKind::Throw(expr) => {
                if let Some(expr) = expr {
                    self.infer_expr(expr);
                }
            }
            StmtKind::Try { body, catch_clauses } => {
                for stmt in body {
                    self.infer_stmt(stmt);
                }
                for clause in catch_clauses {
                    for stmt in &clause.body {
                        self.infer_stmt(stmt);
                    }
                }
            }
            StmtKind::Block(stmts) => {
                for stmt in stmts {
                    self.infer_stmt(stmt);
                }
            }
            _ => {}
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> TypeInfo {
        let type_info = match &expr.kind {
            ExprKind::Literal(lit) => self.infer_literal(lit),

            ExprKind::Identifier(name) => {
                // Look up the symbol this identifier refers to
                if let Some(&symbol_id) = self.name_resolution.references.get(&expr.span) {
                    if let Some(ty) = self.get_symbol_type(symbol_id) {
                        TypeInfo { ty, is_lvalue: true }
                    } else {
                        TypeInfo { ty: Type::Error, is_lvalue: false }
                    }
                } else {
                    TypeInfo { ty: Type::Error, is_lvalue: false }
                }
            }

            ExprKind::QualifiedName(path) => {
                // Look up qualified name
                if let Some(&symbol_id) = self.name_resolution.references.get(&expr.span) {
                    if let Some(ty) = self.get_symbol_type(symbol_id) {
                        TypeInfo { ty, is_lvalue: true }
                    } else {
                        TypeInfo { ty: Type::Error, is_lvalue: false }
                    }
                } else {
                    TypeInfo { ty: Type::Error, is_lvalue: false }
                }
            }

            ExprKind::Call { callee, args } => {
                let callee_type = self.infer_expr(callee);
                for arg in args {
                    self.infer_expr(arg);
                }

                // Extract return type from function type
                match callee_type.ty {
                    Type::Function { return_type, .. } => {
                        TypeInfo { ty: (*return_type).clone(), is_lvalue: false }
                    }
                    _ => {
                        // Try to resolve method call
                        TypeInfo { ty: Type::Error, is_lvalue: false }
                    }
                }
            }

            ExprKind::MemberAccess { object, member, is_arrow } => {
                let object_type = self.infer_expr(object);
                self.infer_member_access(&object_type.ty, *member, *is_arrow)
            }

            ExprKind::Binary { op, left, right } => {
                let left_type = self.infer_expr(left);
                let right_type = self.infer_expr(right);
                self.infer_binary_op(*op, &left_type, &right_type)
            }

            ExprKind::Unary { op, operand } => {
                let operand_type = self.infer_expr(operand);
                self.infer_unary_op(*op, &operand_type)
            }

            ExprKind::PostIncrement(e) | ExprKind::PostDecrement(e)
            | ExprKind::PreIncrement(e) | ExprKind::PreDecrement(e) => {
                let ty = self.infer_expr(e);
                TypeInfo { ty: ty.ty, is_lvalue: false }
            }

            ExprKind::Assign { left, right } => {
                let left_type = self.infer_expr(left);
                self.infer_expr(right);
                TypeInfo { ty: left_type.ty, is_lvalue: false }
            }

            ExprKind::CompoundAssign { left, .. } => {
                let left_type = self.infer_expr(left);
                TypeInfo { ty: left_type.ty, is_lvalue: false }
            }

            ExprKind::Ternary { condition, then_expr, else_expr } => {
                self.infer_expr(condition);
                let then_type = self.infer_expr(then_expr);
                self.infer_expr(else_expr);
                // Return the then type (simplified - should find common type)
                then_type
            }

            ExprKind::Cast { ty, expr } => {
                self.infer_expr(expr);
                TypeInfo { ty: ty.clone(), is_lvalue: false }
            }

            ExprKind::SizeOf(_) | ExprKind::SizeOfExpr(_)
            | ExprKind::AlignOf(_) => {
                TypeInfo {
                    ty: Type::Primitive(PrimitiveType::UnsignedLong),
                    is_lvalue: false,
                }
            }

            ExprKind::TypeId(_) => {
                // Returns std::type_info
                TypeInfo { ty: Type::Error, is_lvalue: false }
            }

            ExprKind::Index { array, index } => {
                let array_type = self.infer_expr(array);
                self.infer_expr(index);

                match array_type.ty {
                    Type::Array(elem_ty, _) => {
                        TypeInfo { ty: (*elem_ty).clone(), is_lvalue: true }
                    }
                    Type::Pointer(elem_ty, _) => {
                        TypeInfo { ty: (*elem_ty).clone(), is_lvalue: true }
                    }
                    _ => TypeInfo { ty: Type::Error, is_lvalue: false },
                }
            }

            ExprKind::Lambda { return_type, params, .. } => {
                let param_types = params.iter().map(|p| p.ty.clone()).collect();
                let ret_ty = return_type.clone().unwrap_or(Type::Auto);

                TypeInfo {
                    ty: Type::Function {
                        return_type: Box::new(ret_ty),
                        params: param_types,
                    },
                    is_lvalue: false,
                }
            }

            ExprKind::NewExpr { ty, .. } => {
                TypeInfo {
                    ty: Type::Pointer(Box::new(ty.clone()), PointerKind::Raw),
                    is_lvalue: false,
                }
            }

            ExprKind::DeleteExpr { .. } => {
                TypeInfo { ty: Type::Void, is_lvalue: false }
            }

            ExprKind::InitializerList(_) => {
                // Would need context to determine type
                TypeInfo { ty: Type::Error, is_lvalue: false }
            }

            ExprKind::Paren(e) => {
                self.infer_expr(e)
            }

            ExprKind::Comma(exprs) => {
                let mut last_type = TypeInfo { ty: Type::Void, is_lvalue: false };
                for e in exprs {
                    last_type = self.infer_expr(e);
                }
                last_type
            }

            ExprKind::This => {
                // Would need to know the enclosing class
                TypeInfo { ty: Type::Error, is_lvalue: true }
            }

            ExprKind::Error => {
                TypeInfo { ty: Type::Error, is_lvalue: false }
            }
        };

        self.type_map.insert(expr.span, type_info.clone());
        type_info
    }

    fn infer_literal(&self, lit: &Literal) -> TypeInfo {
        let ty = match lit {
            Literal::Integer(_) => Type::Primitive(PrimitiveType::Int),
            Literal::Float(_) => Type::Primitive(PrimitiveType::Double),
            Literal::String(_) => Type::Pointer(
                Box::new(Type::Primitive(PrimitiveType::Char)),
                PointerKind::Raw,
            ),
            Literal::Char(_) => Type::Primitive(PrimitiveType::Char),
            Literal::Bool(_) => Type::Primitive(PrimitiveType::Bool),
            Literal::Nullptr => Type::Pointer(Box::new(Type::Void), PointerKind::Raw),
        };

        TypeInfo { ty, is_lvalue: false }
    }

    fn infer_member_access(&self, object_type: &Type, member: InternedString, is_arrow: bool) -> TypeInfo {
        // Dereference if accessing through pointer
        let base_type = if is_arrow {
            match object_type {
                Type::Pointer(inner, _) => &**inner,
                _ => return TypeInfo { ty: Type::Error, is_lvalue: false },
            }
        } else {
            object_type
        };

        // Look up the member in the type
        match base_type {
            Type::Named(path) => {
                // Find the class/struct symbol and look up the member
                if let Some(class_symbol) = self.find_type_symbol(path) {
                    if let Some(member_symbol) = self.find_member_symbol(class_symbol, member) {
                        if let Some(member_type) = self.get_symbol_type(member_symbol) {
                            return TypeInfo { ty: member_type, is_lvalue: true };
                        }
                    }
                }
                TypeInfo { ty: Type::Error, is_lvalue: false }
            }
            _ => TypeInfo { ty: Type::Error, is_lvalue: false },
        }
    }

    fn infer_binary_op(&self, op: BinaryOp, left: &TypeInfo, right: &TypeInfo) -> TypeInfo {
        use BinaryOp::*;
        use PrimitiveType::*;

        match op {
            // Comparison operators return bool
            Eq | Ne | Lt | Le | Gt | Ge => {
                TypeInfo {
                    ty: Type::Primitive(Bool),
                    is_lvalue: false,
                }
            }

            // Logical operators return bool
            And | Or => {
                TypeInfo {
                    ty: Type::Primitive(Bool),
                    is_lvalue: false,
                }
            }

            // Arithmetic and bitwise operators preserve type
            Add | Sub | Mul | Div | Mod | BitAnd | BitOr | BitXor | Shl | Shr => {
                // Simplified - should do proper type promotion
                TypeInfo {
                    ty: left.ty.clone(),
                    is_lvalue: false,
                }
            }

            PtrToMember | PtrToMemberArrow => {
                // Complex type deduction for pointer-to-member
                TypeInfo {
                    ty: Type::Error,
                    is_lvalue: false,
                }
            }
        }
    }

    fn infer_unary_op(&self, op: UnaryOp, operand: &TypeInfo) -> TypeInfo {
        match op {
            UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitwiseNot => {
                TypeInfo {
                    ty: operand.ty.clone(),
                    is_lvalue: false,
                }
            }
            UnaryOp::Not => {
                TypeInfo {
                    ty: Type::Primitive(PrimitiveType::Bool),
                    is_lvalue: false,
                }
            }
            UnaryOp::Deref => {
                match &operand.ty {
                    Type::Pointer(inner, _) => {
                        TypeInfo {
                            ty: (**inner).clone(),
                            is_lvalue: true,
                        }
                    }
                    _ => TypeInfo { ty: Type::Error, is_lvalue: false },
                }
            }
            UnaryOp::AddressOf => {
                TypeInfo {
                    ty: Type::Pointer(Box::new(operand.ty.clone()), PointerKind::Raw),
                    is_lvalue: false,
                }
            }
        }
    }

    fn get_symbol_type(&self, symbol_id: SymbolId) -> Option<Type> {
        let table = self.symbol_table.read();
        let symbol = table.get_symbol(symbol_id)?;

        // Return type based on symbol kind
        match symbol.kind {
            SymbolKind::Variable | SymbolKind::Field | SymbolKind::UProperty => {
                // Would need to store type in symbol
                // For now, return error
                Some(Type::Error)
            }
            SymbolKind::Function | SymbolKind::Method | SymbolKind::UFunction => {
                // Would need to get function signature
                Some(Type::Error)
            }
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::UClass | SymbolKind::UStruct => {
                // Return named type
                Some(Type::Named(TypePath::simple(symbol.name)))
            }
            _ => Some(Type::Error),
        }
    }

    fn find_type_symbol(&self, path: &TypePath) -> Option<SymbolId> {
        let table = self.symbol_table.read();

        if path.segments.len() == 1 {
            let name = path.segments[0];
            // Linear search for the type (inefficient but works)
            for symbol_id in table.all_symbols() {
                if let Some(symbol) = table.get_symbol(symbol_id) {
                    if symbol.name == name {
                        match symbol.kind {
                            SymbolKind::Class | SymbolKind::Struct
                            | SymbolKind::UClass | SymbolKind::UStruct => {
                                return Some(symbol_id);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        None
    }

    fn find_member_symbol(&self, class_symbol: SymbolId, member_name: InternedString) -> Option<SymbolId> {
        let table = self.symbol_table.read();
        let children = table.children(class_symbol);

        for &child_id in &children {
            if let Some(child) = table.get_symbol(child_id) {
                if child.name == member_name {
                    return Some(child_id);
                }
            }
        }

        None
    }
}
