// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol definitions and metadata

use crate::ast::{AccessSpecifier, Type};
use crate::util::{FileId, InternedString, Span};
use std::sync::Arc;

/// Unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// The kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Namespace,
    Class,
    Struct,
    Enum,
    EnumVariant,
    Function,
    Method,
    Constructor,
    Destructor,
    Variable,
    Field,
    Parameter,
    TypeAlias,
    Template,
    TemplateParameter,
    Macro,
    // UE5-specific
    UClass,
    UStruct,
    UEnum,
    UProperty,
    UFunction,
}

impl SymbolKind {
    pub fn is_type(&self) -> bool {
        matches!(
            self,
            Self::Class
                | Self::Struct
                | Self::Enum
                | Self::TypeAlias
                | Self::UClass
                | Self::UStruct
                | Self::UEnum
        )
    }

    pub fn is_callable(&self) -> bool {
        matches!(
            self,
            Self::Function
                | Self::Method
                | Self::Constructor
                | Self::Destructor
                | Self::UFunction
        )
    }

    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Self::Namespace | Self::Class | Self::Struct | Self::UClass | Self::UStruct
        )
    }
}

/// Symbol visibility/access level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

impl From<AccessSpecifier> for Visibility {
    fn from(access: AccessSpecifier) -> Self {
        match access {
            AccessSpecifier::Public => Self::Public,
            AccessSpecifier::Protected => Self::Protected,
            AccessSpecifier::Private => Self::Private,
        }
    }
}

/// Symbol modifiers/flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SymbolFlags {
    pub is_const: bool,
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_inline: bool,
    pub is_abstract: bool,
    pub is_template: bool,
    pub is_exported: bool,
}

/// A symbol in the code
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Unique identifier for this symbol
    pub id: SymbolId,

    /// Kind of symbol
    pub kind: SymbolKind,

    /// Symbol name
    pub name: InternedString,

    /// Fully qualified name (e.g., "MyNamespace::MyClass::MyMethod")
    pub qualified_name: Option<InternedString>,

    /// Source location
    pub span: Span,

    /// Declaration file
    pub file_id: FileId,

    /// Parent symbol (for nested symbols)
    pub parent: Option<SymbolId>,

    /// Child symbols (for containers like classes, namespaces)
    pub children: Vec<SymbolId>,

    /// Visibility/access level
    pub visibility: Visibility,

    /// Symbol modifiers
    pub flags: SymbolFlags,

    /// Type information (for variables, functions, etc.)
    pub symbol_type: Option<Arc<Type>>,

    /// Base classes (for class/struct symbols)
    pub bases: Vec<SymbolId>,

    /// Derived classes (for class/struct symbols)
    pub derived: Vec<SymbolId>,

    /// Overridden symbol (for virtual methods)
    pub overrides: Option<SymbolId>,

    /// Symbols that override this one
    pub overridden_by: Vec<SymbolId>,

    /// Documentation comment
    pub doc_comment: Option<String>,

    /// Implementation location (for functions with separate declaration/definition)
    /// Points to the function body in .cpp file if different from declaration
    pub implementation_span: Option<Span>,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(
        id: SymbolId,
        kind: SymbolKind,
        name: InternedString,
        span: Span,
        file_id: FileId,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            qualified_name: None,
            span,
            file_id,
            parent: None,
            children: Vec::new(),
            visibility: Visibility::Public,
            flags: SymbolFlags::default(),
            symbol_type: None,
            bases: Vec::new(),
            derived: Vec::new(),
            overrides: None,
            overridden_by: Vec::new(),
            doc_comment: None,
            implementation_span: None,
        }
    }

    /// Add a child symbol
    pub fn add_child(&mut self, child_id: SymbolId) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    /// Add a base class
    pub fn add_base(&mut self, base_id: SymbolId) {
        if !self.bases.contains(&base_id) {
            self.bases.push(base_id);
        }
    }

    /// Add a derived class
    pub fn add_derived(&mut self, derived_id: SymbolId) {
        if !self.derived.contains(&derived_id) {
            self.derived.push(derived_id);
        }
    }

    /// Mark this symbol as overriding another
    pub fn set_overrides(&mut self, overridden_id: SymbolId) {
        self.overrides = Some(overridden_id);
    }

    /// Add a symbol that overrides this one
    pub fn add_overridden_by(&mut self, overriding_id: SymbolId) {
        if !self.overridden_by.contains(&overriding_id) {
            self.overridden_by.push(overriding_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Interner;

    #[test]
    fn test_symbol_creation() {
        let mut interner = Interner::new();
        let name = interner.intern("MyClass");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let symbol = Symbol::new(SymbolId::new(0), SymbolKind::Class, name, span, file_id);

        assert_eq!(symbol.id, SymbolId::new(0));
        assert_eq!(symbol.kind, SymbolKind::Class);
        assert_eq!(symbol.name, name);
        assert_eq!(symbol.span, span);
    }

    #[test]
    fn test_symbol_kind_predicates() {
        assert!(SymbolKind::Class.is_type());
        assert!(SymbolKind::Function.is_callable());
        assert!(SymbolKind::Namespace.is_container());

        assert!(!SymbolKind::Variable.is_type());
        assert!(!SymbolKind::Variable.is_callable());
    }

    #[test]
    fn test_symbol_hierarchy() {
        let mut interner = Interner::new();
        let parent_name = interner.intern("Parent");
        let child_name = interner.intern("Child");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let mut parent = Symbol::new(
            SymbolId::new(0),
            SymbolKind::Class,
            parent_name,
            span,
            file_id,
        );

        let child_id = SymbolId::new(1);
        parent.add_child(child_id);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0], child_id);
    }

    #[test]
    fn test_inheritance_tracking() {
        let mut interner = Interner::new();
        let base_name = interner.intern("Base");
        let derived_name = interner.intern("Derived");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let mut base = Symbol::new(
            SymbolId::new(0),
            SymbolKind::Class,
            base_name,
            span,
            file_id,
        );

        let mut derived = Symbol::new(
            SymbolId::new(1),
            SymbolKind::Class,
            derived_name,
            span,
            file_id,
        );

        derived.add_base(base.id);
        base.add_derived(derived.id);

        assert_eq!(derived.bases.len(), 1);
        assert_eq!(derived.bases[0], base.id);
        assert_eq!(base.derived.len(), 1);
        assert_eq!(base.derived[0], derived.id);
    }
}
