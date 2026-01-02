// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Name resolution - resolve identifiers to their declarations

use crate::ast::*;
use crate::index::{SymbolId, SymbolTable};
use crate::semantic::scope::{ScopeId, ScopeKind, ScopeManager};
use crate::util::{FileId, Interner, InternedString, Span};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Name resolution result
#[derive(Debug, Clone)]
pub struct NameResolution {
    /// Map from expression/identifier spans to the symbol they refer to
    pub references: HashMap<Span, SymbolId>,

    /// Scope hierarchy
    pub scopes: ScopeManager,

    /// Map from spans to their containing scope
    pub span_to_scope: HashMap<Span, ScopeId>,
}

/// Name resolver
pub struct NameResolver {
    symbol_table: Arc<RwLock<SymbolTable>>,
    interner: Arc<RwLock<Interner>>,
    scopes: ScopeManager,
    current_scope: Option<ScopeId>,
    file_id: FileId,

    /// Track identifier references
    references: HashMap<Span, SymbolId>,
    span_to_scope: HashMap<Span, ScopeId>,
}

impl NameResolver {
    pub fn new(
        symbol_table: Arc<RwLock<SymbolTable>>,
        interner: Arc<RwLock<Interner>>,
        file_id: FileId,
    ) -> Self {
        let mut scopes = ScopeManager::new();
        let global_scope = scopes.create_scope(ScopeKind::Global, None, file_id);

        Self {
            symbol_table,
            interner,
            scopes,
            current_scope: Some(global_scope),
            file_id,
            references: HashMap::new(),
            span_to_scope: HashMap::new(),
        }
    }

    /// Resolve names in a translation unit
    pub fn resolve(&mut self, ast: &TranslationUnit) -> NameResolution {
        for decl in &ast.declarations {
            self.resolve_declaration(decl);
        }

        NameResolution {
            references: self.references.clone(),
            scopes: self.scopes.clone(),
            span_to_scope: self.span_to_scope.clone(),
        }
    }

    fn resolve_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Namespace(ns) => self.resolve_namespace(ns),
            Declaration::Class(cls) => self.resolve_class(cls),
            Declaration::Struct(st) => self.resolve_class(st),
            Declaration::Enum(en) => self.resolve_enum(en),
            Declaration::Function(func) => self.resolve_function(func),
            Declaration::Variable(var) => self.resolve_variable(var),
            Declaration::UClass(ucls) => self.resolve_class(&ucls.class_decl),
            Declaration::UStruct(ust) => self.resolve_class(&ust.struct_decl),
            Declaration::UEnum(uen) => self.resolve_enum(&uen.enum_decl),
            _ => {}
        }
    }

    fn resolve_namespace(&mut self, ns: &NamespaceDecl) {
        // Create scope for namespace
        let scope_id = self.scopes.create_scope(
            ScopeKind::Namespace,
            self.current_scope,
            self.file_id,
        );

        let prev_scope = self.current_scope;
        self.current_scope = Some(scope_id);

        // Resolve nested declarations
        for decl in &ns.declarations {
            self.resolve_declaration(decl);
        }

        self.current_scope = prev_scope;
    }

    fn resolve_class(&mut self, cls: &ClassDecl) {
        // Find the symbol for this class
        if let Some(symbol_id) = self.find_symbol_by_name_and_span(cls.name, cls.span) {
            // Create scope for class
            let scope_id = self.scopes.create_scope(
                ScopeKind::Class,
                self.current_scope,
                self.file_id,
            );

            // Link scope to symbol
            if let Some(scope) = self.scopes.get_scope_mut(scope_id) {
                scope.symbol = Some(symbol_id);
            }

            let prev_scope = self.current_scope;
            self.current_scope = Some(scope_id);

            // Resolve base classes
            for base in &cls.bases {
                self.resolve_type_path(&base.type_path, cls.span);
            }

            // Resolve members
            for member in &cls.members {
                self.resolve_class_member(member);
            }

            self.current_scope = prev_scope;
        }
    }

    fn resolve_enum(&mut self, en: &EnumDecl) {
        // Enum variants are already in symbol table, just need to resolve initializers
        for variant in &en.variants {
            if let Some(ref init) = variant.value {
                self.resolve_expr(init);
            }
        }
    }

    fn resolve_function(&mut self, func: &FunctionDecl) {
        if let Some(symbol_id) = self.find_symbol_by_name_and_span(func.name, func.span) {
            // Create scope for function
            let scope_id = self.scopes.create_scope(
                ScopeKind::Function,
                self.current_scope,
                self.file_id,
            );

            if let Some(scope) = self.scopes.get_scope_mut(scope_id) {
                scope.symbol = Some(symbol_id);
            }

            let prev_scope = self.current_scope;
            self.current_scope = Some(scope_id);

            // Add parameters to scope
            for param in &func.parameters {
                if let Some(name) = param.name {
                    // Parameters create local bindings
                    if let Some(current_scope) = self.current_scope {
                        // Create a pseudo-symbol for the parameter
                        // In a full implementation, we'd have parameter symbols
                        self.scopes.add_binding(current_scope, name, symbol_id);
                    }
                }
                self.resolve_type(&param.ty);
            }

            // Resolve return type
            self.resolve_type(&func.return_type);

            // Resolve body
            if let Some(ref body) = func.body {
                for stmt in &body.statements {
                    self.resolve_stmt(stmt);
                }
            }

            self.current_scope = prev_scope;
        }
    }

    fn resolve_variable(&mut self, var: &VariableDecl) {
        self.resolve_type(&var.ty);
        if let Some(ref init) = var.initializer {
            self.resolve_expr(init);
        }
    }

    fn resolve_class_member(&mut self, member: &ClassMember) {
        match member {
            ClassMember::Field(field) => {
                self.resolve_type(&field.ty);
                if let Some(ref init) = field.initializer {
                    self.resolve_expr(init);
                }
            }
            ClassMember::Method(method) => self.resolve_function(method),
            ClassMember::NestedClass(cls) => self.resolve_class(cls),
            ClassMember::NestedEnum(en) => self.resolve_enum(en),
            ClassMember::UProperty(uprop) => {
                self.resolve_type(&uprop.field.ty);
                if let Some(ref init) = uprop.field.initializer {
                    self.resolve_expr(init);
                }
            }
            ClassMember::UFunction(ufunc) => self.resolve_function(&ufunc.function),
            _ => {}
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        self.span_to_scope.insert(stmt.span, self.current_scope.unwrap());

        match &stmt.kind {
            StmtKind::VarDecl { name, ty, init } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty);
                }
                if let Some(init) = init {
                    self.resolve_expr(init);
                }
                // Add to current scope
                if let Some(current_scope) = self.current_scope {
                    // In a full implementation, create a symbol for this variable
                    // For now, we'll skip this
                }
            }
            StmtKind::Expr(expr) => self.resolve_expr(expr),
            StmtKind::If { condition, then_block, else_block } => {
                self.resolve_expr(condition);
                for stmt in then_block {
                    self.resolve_stmt(stmt);
                }
                if let Some(else_block) = else_block {
                    for stmt in else_block {
                        self.resolve_stmt(stmt);
                    }
                }
            }
            StmtKind::While { condition, body } | StmtKind::DoWhile { body, condition } => {
                self.resolve_expr(condition);
                for stmt in body {
                    self.resolve_stmt(stmt);
                }
            }
            StmtKind::For { init, condition, increment, body } => {
                if let Some(init) = init {
                    self.resolve_stmt(init);
                }
                if let Some(condition) = condition {
                    self.resolve_expr(condition);
                }
                if let Some(increment) = increment {
                    self.resolve_expr(increment);
                }
                for stmt in body {
                    self.resolve_stmt(stmt);
                }
            }
            StmtKind::RangeFor { var, ty, range, body } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty);
                }
                self.resolve_expr(range);
                for stmt in body {
                    self.resolve_stmt(stmt);
                }
            }
            StmtKind::Switch { condition, cases } => {
                self.resolve_expr(condition);
                for case in cases {
                    if let Some(pattern) = &case.pattern {
                        self.resolve_expr(pattern);
                    }
                    for stmt in &case.stmts {
                        self.resolve_stmt(stmt);
                    }
                }
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.resolve_expr(expr);
                }
            }
            StmtKind::Try { body, catch_clauses } => {
                for stmt in body {
                    self.resolve_stmt(stmt);
                }
                for clause in catch_clauses {
                    if let Some(ty) = &clause.exception_type {
                        self.resolve_type(ty);
                    }
                    for stmt in &clause.body {
                        self.resolve_stmt(stmt);
                    }
                }
            }
            StmtKind::Throw(expr) => {
                if let Some(expr) = expr {
                    self.resolve_expr(expr);
                }
            }
            StmtKind::Block(stmts) => {
                let scope_id = self.scopes.create_scope(
                    ScopeKind::Block,
                    self.current_scope,
                    self.file_id,
                );
                let prev_scope = self.current_scope;
                self.current_scope = Some(scope_id);

                for stmt in stmts {
                    self.resolve_stmt(stmt);
                }

                self.current_scope = prev_scope;
            }
            _ => {}
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        self.span_to_scope.insert(expr.span, self.current_scope.unwrap());

        match &expr.kind {
            ExprKind::Identifier(name) => {
                // Look up this identifier in the current scope
                if let Some(current_scope) = self.current_scope {
                    if let Some(symbol) = self.scopes.lookup(current_scope, *name) {
                        self.references.insert(expr.span, symbol);
                    }
                }
            }
            ExprKind::QualifiedName(path) => {
                self.resolve_type_path(path, expr.span);
            }
            ExprKind::Call { callee, args } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::MemberAccess { object, member, .. } => {
                self.resolve_expr(object);
                // Member will be resolved by type inference
            }
            ExprKind::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Unary { operand, .. } => {
                self.resolve_expr(operand);
            }
            ExprKind::PostIncrement(e) | ExprKind::PostDecrement(e)
            | ExprKind::PreIncrement(e) | ExprKind::PreDecrement(e) => {
                self.resolve_expr(e);
            }
            ExprKind::Assign { left, right } | ExprKind::CompoundAssign { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Ternary { condition, then_expr, else_expr } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_expr);
                self.resolve_expr(else_expr);
            }
            ExprKind::Cast { ty, expr } => {
                self.resolve_type(ty);
                self.resolve_expr(expr);
            }
            ExprKind::SizeOf(ty) | ExprKind::AlignOf(ty) | ExprKind::TypeId(ty) => {
                self.resolve_type(ty);
            }
            ExprKind::SizeOfExpr(e) => {
                self.resolve_expr(e);
            }
            ExprKind::Index { array, index } => {
                self.resolve_expr(array);
                self.resolve_expr(index);
            }
            ExprKind::Lambda { params, return_type, body, .. } => {
                let scope_id = self.scopes.create_scope(
                    ScopeKind::Function,
                    self.current_scope,
                    self.file_id,
                );
                let prev_scope = self.current_scope;
                self.current_scope = Some(scope_id);

                for param in params {
                    self.resolve_type(&param.ty);
                }
                if let Some(ret_ty) = return_type {
                    self.resolve_type(ret_ty);
                }
                for stmt in body {
                    self.resolve_stmt(stmt);
                }

                self.current_scope = prev_scope;
            }
            ExprKind::NewExpr { ty, args, .. } => {
                self.resolve_type(ty);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::DeleteExpr { expr, .. } => {
                self.resolve_expr(expr);
            }
            ExprKind::InitializerList(exprs) => {
                for e in exprs {
                    self.resolve_expr(e);
                }
            }
            ExprKind::Paren(e) => {
                self.resolve_expr(e);
            }
            ExprKind::Comma(exprs) => {
                for e in exprs {
                    self.resolve_expr(e);
                }
            }
            _ => {}
        }
    }

    fn resolve_type(&mut self, ty: &Type) {
        match ty {
            Type::Named(path) => {
                self.resolve_type_path(path, Span::new(self.file_id, 0, 0));
            }
            Type::Pointer(inner, _) | Type::Reference(inner, _) | Type::Array(inner, _) => {
                self.resolve_type(inner);
            }
            Type::TemplateInstantiation { template, args } => {
                self.resolve_type_path(template, Span::new(self.file_id, 0, 0));
                for arg in args {
                    match arg {
                        TemplateArg::Type(ty) => self.resolve_type(ty),
                        TemplateArg::Expr(expr) => self.resolve_expr(expr),
                    }
                }
            }
            Type::Function { return_type, params } => {
                self.resolve_type(return_type);
                for param_ty in params {
                    self.resolve_type(param_ty);
                }
            }
            Type::Decltype(expr) => {
                self.resolve_expr(expr);
            }
            _ => {}
        }
    }

    fn resolve_type_path(&mut self, path: &TypePath, span: Span) {
        // Try to resolve the path to a symbol
        if path.segments.len() == 1 {
            // Simple name lookup
            if let Some(current_scope) = self.current_scope {
                if let Some(symbol) = self.scopes.lookup(current_scope, path.segments[0]) {
                    self.references.insert(span, symbol);
                }
            }
        } else {
            // Qualified name lookup - would need more sophisticated resolution
            // For now, try to find the last segment
            if let Some(current_scope) = self.current_scope {
                if let Some(last_name) = path.segments.last() {
                    if let Some(symbol) = self.scopes.lookup(current_scope, *last_name) {
                        self.references.insert(span, symbol);
                    }
                }
            }
        }
    }

    /// Helper to find a symbol by name and span in the symbol table
    fn find_symbol_by_name_and_span(&self, name: InternedString, span: Span) -> Option<SymbolId> {
        let table = self.symbol_table.read();
        let symbols = table.symbols_in_file(self.file_id);

        for &symbol_id in &symbols {
            if let Some(symbol) = table.get_symbol(symbol_id) {
                if symbol.name == name && symbol.span == span {
                    return Some(symbol_id);
                }
            }
        }

        None
    }
}
