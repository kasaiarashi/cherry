// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! AST visitor traits for tree traversal

use super::nodes::*;

/// Visitor trait for traversing the AST
pub trait AstVisitor {
    fn visit_translation_unit(&mut self, unit: &TranslationUnit) {
        walk_translation_unit(self, unit);
    }

    fn visit_declaration(&mut self, decl: &Declaration) {
        walk_declaration(self, decl);
    }

    fn visit_namespace(&mut self, ns: &NamespaceDecl) {
        walk_namespace(self, ns);
    }

    fn visit_class(&mut self, class: &ClassDecl) {
        walk_class(self, class);
    }

    fn visit_enum(&mut self, enum_decl: &EnumDecl) {
        walk_enum(self, enum_decl);
    }

    fn visit_function(&mut self, func: &FunctionDecl) {
        walk_function(self, func);
    }

    fn visit_variable(&mut self, var: &VariableDecl) {
        walk_variable(self, var);
    }

    fn visit_type(&mut self, _ty: &Type) {}
    fn visit_expr(&mut self, _expr: &Expr) {}
}

/// Walk functions for default traversal

pub fn walk_translation_unit<V: AstVisitor + ?Sized>(visitor: &mut V, unit: &TranslationUnit) {
    for decl in &unit.declarations {
        visitor.visit_declaration(decl);
    }
}

pub fn walk_declaration<V: AstVisitor + ?Sized>(visitor: &mut V, decl: &Declaration) {
    match decl {
        Declaration::Namespace(ns) => visitor.visit_namespace(ns),
        Declaration::Class(class) | Declaration::Struct(class) => visitor.visit_class(class),
        Declaration::Enum(enum_decl) => visitor.visit_enum(enum_decl),
        Declaration::Function(func) => visitor.visit_function(func),
        Declaration::Variable(var) => visitor.visit_variable(var),
        Declaration::TypeAlias(_) => {}
        Declaration::Template(_) => {}
        Declaration::UsingDeclaration(_) => {}
        Declaration::UsingDirective(_) => {}
        Declaration::UClass(uclass) => visitor.visit_class(&uclass.class_decl),
        Declaration::UStruct(ustruct) => visitor.visit_class(&ustruct.struct_decl),
        Declaration::UEnum(uenum) => visitor.visit_enum(&uenum.enum_decl),
    }
}

pub fn walk_namespace<V: AstVisitor + ?Sized>(visitor: &mut V, ns: &NamespaceDecl) {
    for decl in &ns.declarations {
        visitor.visit_declaration(decl);
    }
}

pub fn walk_class<V: AstVisitor + ?Sized>(visitor: &mut V, class: &ClassDecl) {
    for base in &class.bases {
        visitor.visit_type(&Type::Named(base.type_path.clone()));
    }

    for member in &class.members {
        match member {
            ClassMember::Field(field) => visitor.visit_type(&field.ty),
            ClassMember::Method(method) => visitor.visit_function(method),
            ClassMember::Constructor(_) => {}
            ClassMember::Destructor(_) => {}
            ClassMember::TypeAlias(_) => {}
            ClassMember::NestedClass(nested) => visitor.visit_class(nested),
            ClassMember::NestedEnum(nested) => visitor.visit_enum(nested),
            ClassMember::UProperty(uprop) => visitor.visit_type(&uprop.field.ty),
            ClassMember::UFunction(ufunc) => visitor.visit_function(&ufunc.function),
        }
    }
}

pub fn walk_enum<V: AstVisitor + ?Sized>(visitor: &mut V, enum_decl: &EnumDecl) {
    if let Some(underlying) = &enum_decl.underlying_type {
        visitor.visit_type(underlying);
    }
}

pub fn walk_function<V: AstVisitor + ?Sized>(visitor: &mut V, func: &FunctionDecl) {
    visitor.visit_type(&func.return_type);
    for param in &func.parameters {
        visitor.visit_type(&param.ty);
    }
}

pub fn walk_variable<V: AstVisitor + ?Sized>(visitor: &mut V, var: &VariableDecl) {
    visitor.visit_type(&var.ty);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{FileId, Span};

    struct TestVisitor {
        class_count: usize,
        function_count: usize,
    }

    impl AstVisitor for TestVisitor {
        fn visit_class(&mut self, class: &ClassDecl) {
            self.class_count += 1;
            walk_class(self, class);
        }

        fn visit_function(&mut self, func: &FunctionDecl) {
            self.function_count += 1;
            walk_function(self, func);
        }
    }

    #[test]
    fn test_visitor_basic() {
        let mut visitor = TestVisitor {
            class_count: 0,
            function_count: 0,
        };

        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        // Create a simple translation unit with one class and one function
        let unit = TranslationUnit {
            file_id,
            declarations: vec![
                Declaration::Class(ClassDecl {
                    name: lasso::Spur::default(),
                    span,
                    access: AccessSpecifier::Public,
                    bases: vec![],
                    members: vec![],
                    is_struct: false,
                    template_params: None,
                }),
                Declaration::Function(FunctionDecl {
                    name: lasso::Spur::default(),
                    span,
                    return_type: Type::Void,
                    parameters: vec![],
                    is_const: false,
                    is_static: false,
                    is_virtual: false,
                    is_override: false,
                    is_final: false,
                    is_inline: false,
                    body: None,
                }),
            ],
            errors: vec![],
        };

        visitor.visit_translation_unit(&unit);

        assert_eq!(visitor.class_count, 1);
        assert_eq!(visitor.function_count, 1);
    }
}
