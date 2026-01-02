// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Convert AST to symbol table entries

use crate::ast::*;
use crate::index::symbol::{Symbol, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner};
use parking_lot::RwLock;
use std::sync::Arc;

/// Builds symbol table from AST
pub struct AstSymbolBuilder {
    symbol_table: Arc<RwLock<SymbolTable>>,
    interner: Arc<RwLock<Interner>>,
}

impl AstSymbolBuilder {
    pub fn new(symbol_table: Arc<RwLock<SymbolTable>>, interner: Arc<RwLock<Interner>>) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Build symbols from a translation unit
    pub fn build_from_ast(&mut self, ast: &TranslationUnit) {
        let file_id = ast.file_id;
        for decl in &ast.declarations {
            self.process_declaration(decl, None, file_id);
        }
    }

    fn process_declaration(&mut self, decl: &Declaration, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        match decl {
            Declaration::Namespace(ns) => self.process_namespace(ns, parent, file_id),
            Declaration::Class(cls) => self.process_class(cls, parent, file_id, SymbolKind::Class),
            Declaration::Struct(st) => self.process_class(st, parent, file_id, SymbolKind::Struct),
            Declaration::Enum(en) => self.process_enum(en, parent, file_id, SymbolKind::Enum),
            Declaration::Function(func) => self.process_function(func, parent, file_id),
            Declaration::Variable(var) => self.process_variable(var, parent, file_id),
            Declaration::UClass(ucls) => self.process_uclass(ucls, parent, file_id),
            Declaration::UStruct(ust) => self.process_ustruct(ust, parent, file_id),
            Declaration::UEnum(uen) => self.process_uenum(uen, parent, file_id),
            _ => {}, // Other declarations not yet supported
        }
    }

    fn process_namespace(&mut self, ns: &NamespaceDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        let name = match ns.name {
            Some(n) => n,
            None => self.interner.write().intern("<anonymous>"),
        };

        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::Namespace, name, ns.span, file_id);
        symbol.parent = parent;
        let symbol_id = table.add_symbol(symbol);
        drop(table);

        // Process nested declarations
        for decl in &ns.declarations {
            self.process_declaration(decl, Some(symbol_id), file_id);
        }
    }

    fn process_class(&mut self, cls: &ClassDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId, kind: SymbolKind) {
        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, kind, cls.name, cls.span, file_id);
        symbol.parent = parent;
        symbol.doc_comment = cls.doc_comment.clone();

        // Set visibility from access specifier
        symbol.visibility = cls.access.into();

        let symbol_id = table.add_symbol(symbol);
        drop(table);

        // Process members
        for member in &cls.members {
            self.process_class_member(member, Some(symbol_id), file_id);
        }
    }

    fn process_enum(&mut self, en: &EnumDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId, kind: SymbolKind) {
        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, kind, en.name, en.span, file_id);
        symbol.parent = parent;
        let symbol_id = table.add_symbol(symbol);
        drop(table);

        // Process enum variants
        for variant in &en.variants {
            let mut table = self.symbol_table.write();
            let id = table.next_id();
            let mut symbol = Symbol::new(id, SymbolKind::EnumVariant, variant.name, variant.span, file_id);
            symbol.parent = Some(symbol_id);
            table.add_symbol(symbol);
        }
    }

    fn process_function(&mut self, func: &FunctionDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        let kind = if parent.is_some() {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };

        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, kind, func.name, func.span, file_id);
        symbol.parent = parent;

        // Transfer documentation
        symbol.doc_comment = func.doc_comment.clone();

        // Transfer type information - create a Function type with return type and parameters
        let param_types: Vec<Type> = func.parameters.iter().map(|p| p.ty.clone()).collect();
        symbol.symbol_type = Some(Arc::new(Type::Function {
            return_type: Box::new(func.return_type.clone()),
            params: param_types,
        }));

        // Transfer flags
        symbol.flags.is_const = func.is_const;
        symbol.flags.is_static = func.is_static;
        symbol.flags.is_virtual = func.is_virtual;
        symbol.flags.is_override = func.is_override;
        symbol.flags.is_final = func.is_final;
        symbol.flags.is_inline = func.is_inline;

        let function_id = table.add_symbol(symbol);
        drop(table);

        // Create parameter symbols as children of the function
        for param in &func.parameters {
            if let Some(param_name) = param.name {
                let mut table = self.symbol_table.write();
                let param_id = table.next_id();
                let mut param_symbol = Symbol::new(param_id, SymbolKind::Parameter, param_name, param.span, file_id);
                param_symbol.parent = Some(function_id);
                param_symbol.symbol_type = Some(Arc::new(param.ty.clone()));
                let param_symbol_id = table.add_symbol(param_symbol);

                // Add parameter to function's children list
                if let Some(func_symbol) = table.get_symbol_mut(function_id) {
                    func_symbol.children.push(param_symbol_id);
                }
            }
        }
    }

    fn process_variable(&mut self, var: &VariableDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::Variable, var.name, var.span, file_id);
        symbol.parent = parent;
        table.add_symbol(symbol);
    }

    fn process_class_member(&mut self, member: &ClassMember, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        match member {
            ClassMember::Field(field) => {
                let mut table = self.symbol_table.write();
                let id = table.next_id();
                let mut symbol = Symbol::new(id, SymbolKind::Field, field.name, field.span, file_id);
                symbol.parent = parent;
                // Store field type information
                symbol.symbol_type = Some(Arc::new(field.ty.clone()));
                // Store flags
                symbol.flags.is_static = field.is_static;
                table.add_symbol(symbol);
            }
            ClassMember::Method(method) => {
                self.process_function(method, parent, file_id);
            }
            ClassMember::NestedClass(cls) => {
                self.process_class(cls, parent, file_id, SymbolKind::Class);
            }
            ClassMember::NestedEnum(en) => {
                self.process_enum(en, parent, file_id, SymbolKind::Enum);
            }
            ClassMember::UProperty(uprop) => {
                // Process as a field but with UProperty kind and macro info in doc_comment
                let mut table = self.symbol_table.write();
                let id = table.next_id();
                let mut symbol = Symbol::new(id, SymbolKind::UProperty, uprop.field.name, uprop.field.span, file_id);
                symbol.parent = parent;
                // Store field type information
                symbol.symbol_type = Some(Arc::new(uprop.field.ty.clone()));
                // Store UPROPERTY macro as doc comment
                let specifiers_str = self.format_uproperty_specifiers(&uprop.specifiers);
                let macro_text = format!("UPROPERTY({})", specifiers_str);
                symbol.doc_comment = Some(macro_text);
                // Store flags
                symbol.flags.is_static = uprop.field.is_static;
                table.add_symbol(symbol);
            }
            ClassMember::UFunction(ufunc) => {
                // Process as a full function with UFUNCTION macro info
                let kind = SymbolKind::UFunction;
                let func = &ufunc.function;

                let mut table = self.symbol_table.write();
                let id = table.next_id();
                let mut symbol = Symbol::new(id, kind, func.name, func.span, file_id);
                symbol.parent = parent;

                // Store UFUNCTION macro as doc comment
                let specifiers_str = self.format_ufunction_specifiers(&ufunc.specifiers);
                let macro_text = format!("UFUNCTION({})", specifiers_str);
                symbol.doc_comment = Some(if let Some(ref doc) = func.doc_comment {
                    format!("{}\n{}", macro_text, doc)
                } else {
                    macro_text
                });

                // Transfer type information - create a Function type with return type and parameters
                let param_types: Vec<Type> = func.parameters.iter().map(|p| p.ty.clone()).collect();
                symbol.symbol_type = Some(Arc::new(Type::Function {
                    return_type: Box::new(func.return_type.clone()),
                    params: param_types,
                }));

                // Transfer flags
                symbol.flags.is_const = func.is_const;
                symbol.flags.is_static = func.is_static;
                symbol.flags.is_virtual = func.is_virtual;
                symbol.flags.is_override = func.is_override;
                symbol.flags.is_final = func.is_final;
                symbol.flags.is_inline = func.is_inline;

                let function_id = table.add_symbol(symbol);
                drop(table);

                // Create parameter symbols as children of the function
                for param in &func.parameters {
                    if let Some(param_name) = param.name {
                        let mut table = self.symbol_table.write();
                        let param_id = table.next_id();
                        let mut param_symbol = Symbol::new(param_id, SymbolKind::Parameter, param_name, param.span, file_id);
                        param_symbol.parent = Some(function_id);
                        param_symbol.symbol_type = Some(Arc::new(param.ty.clone()));
                        let param_symbol_id = table.add_symbol(param_symbol);

                        // Add parameter to function's children list
                        if let Some(func_symbol) = table.get_symbol_mut(function_id) {
                            func_symbol.children.push(param_symbol_id);
                        }
                    }
                }
            }
            _ => {} // Other members not yet supported
        }
    }

    fn process_uclass(&mut self, ucls: &UClassDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::UClass, ucls.class_decl.name, ucls.class_decl.span, file_id);
        symbol.parent = parent;
        let symbol_id = table.add_symbol(symbol);
        drop(table);

        // Process members
        for member in &ucls.class_decl.members {
            self.process_class_member(member, Some(symbol_id), file_id);
        }
    }

    /// Format UPropertySpecifiers into a string representation
    fn format_uproperty_specifiers(&self, specs: &crate::ast::UPropertySpecifiers) -> String {
        let mut parts: Vec<String> = Vec::new();

        if specs.edit_anywhere {
            parts.push("EditAnywhere".to_string());
        }
        if specs.edit_default_only {
            parts.push("EditDefaultsOnly".to_string());
        }
        if specs.edit_instance_only {
            parts.push("EditInstanceOnly".to_string());
        }
        if specs.visible_anywhere {
            parts.push("VisibleAnywhere".to_string());
        }
        if specs.blueprint_read_write {
            parts.push("BlueprintReadWrite".to_string());
        }
        if specs.blueprint_read_only {
            parts.push("BlueprintReadOnly".to_string());
        }
        if specs.replicated {
            parts.push("Replicated".to_string());
        }

        if let Some(ref category) = specs.category {
            parts.push(format!("Category=\"{}\"", category));
        }

        parts.join(", ")
    }

    /// Format UFunctionSpecifiers into a string representation
    fn format_ufunction_specifiers(&self, specs: &crate::ast::UFunctionSpecifiers) -> String {
        let mut parts: Vec<String> = Vec::new();

        if specs.blueprint_callable {
            parts.push("BlueprintCallable".to_string());
        }
        if specs.blueprint_pure {
            parts.push("BlueprintPure".to_string());
        }
        if specs.blueprint_implementable_event {
            parts.push("BlueprintImplementableEvent".to_string());
        }
        if specs.blueprint_native_event {
            parts.push("BlueprintNativeEvent".to_string());
        }
        if specs.exec {
            parts.push("Exec".to_string());
        }
        if specs.server {
            parts.push("Server".to_string());
        }
        if specs.client {
            parts.push("Client".to_string());
        }
        if specs.reliable {
            parts.push("Reliable".to_string());
        }
        if specs.unreliable {
            parts.push("Unreliable".to_string());
        }

        if let Some(ref category) = specs.category {
            parts.push(format!("Category=\"{}\"", category));
        }

        parts.join(", ")
    }

    fn process_ustruct(&mut self, ust: &UStructDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        let mut table = self.symbol_table.write();
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::UStruct, ust.struct_decl.name, ust.struct_decl.span, file_id);
        symbol.parent = parent;
        let symbol_id = table.add_symbol(symbol);
        drop(table);

        // Process members
        for member in &ust.struct_decl.members {
            self.process_class_member(member, Some(symbol_id), file_id);
        }
    }

    fn process_uenum(&mut self, uen: &UEnumDecl, parent: Option<crate::index::symbol::SymbolId>, file_id: FileId) {
        self.process_enum(&uen.enum_decl, parent, file_id, SymbolKind::UEnum);
    }
}
