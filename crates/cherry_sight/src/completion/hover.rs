// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Hover information provider

use crate::ast::PrimitiveType;
use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner, Position};

/// Hover contents
#[derive(Debug, Clone)]
pub struct HoverContents {
    pub contents: Vec<String>,
}

/// Provides hover information
pub struct HoverProvider<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> HoverProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get hover information at a position
    pub fn get_hover(&self, _file_id: FileId, _position: Position) -> Option<HoverContents> {
        // Would need to:
        // 1. Find symbol at position
        // 2. Get type information
        // 3. Format documentation

        None
    }

    /// Create hover contents for a symbol
    pub fn create_hover_for_symbol(&self, symbol_id: SymbolId) -> Option<HoverContents> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;
        let name = self.interner.resolve(symbol.name);

        let mut contents = Vec::new();

        // Build signature based on symbol kind
        let signature = match symbol.kind {
            SymbolKind::Function | SymbolKind::Method | SymbolKind::UFunction => {
                self.format_function_signature(symbol, &name)
            }
            SymbolKind::Class | SymbolKind::UClass => {
                self.format_class_signature(symbol, &name)
            }
            SymbolKind::Struct | SymbolKind::UStruct => {
                self.format_struct_signature(symbol, &name)
            }
            SymbolKind::Variable | SymbolKind::Field | SymbolKind::UProperty => {
                self.format_variable_signature(symbol, &name)
            }
            _ => format!("{} {}", symbol_kind_to_str(symbol.kind), name),
        };

        // Wrap in code block for better formatting
        contents.push(format!("```cpp\n{}\n```", signature));

        // Add documentation if available
        if let Some(ref doc) = symbol.doc_comment {
            contents.push(String::new()); // Blank line
            contents.push(doc.clone());
        }

        Some(HoverContents { contents })
    }

    /// Format a function signature
    fn format_function_signature(&self, symbol: &crate::index::symbol::Symbol, name: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Add modifiers
        if symbol.flags.is_static {
            parts.push("static".to_string());
        }
        if symbol.flags.is_virtual {
            parts.push("virtual".to_string());
        }
        if symbol.flags.is_inline {
            parts.push("inline".to_string());
        }

        // Add return type and function name with parameters
        if let Some(ref func_type) = symbol.symbol_type {
            if let crate::ast::Type::Function { return_type, params } = func_type.as_ref() {
                // Format return type
                let ret_type_str = self.format_type(return_type);

                // Get parameter names from child symbols
                let param_symbols: Vec<_> = symbol.children.iter()
                    .filter_map(|&child_id| {
                        let child = self.symbol_table.get_symbol(child_id)?;
                        if child.kind == SymbolKind::Parameter {
                            Some(child)
                        } else {
                            None
                        }
                    })
                    .collect();

                // Format parameters with both type and name
                let params_str: Vec<String> = params.iter().enumerate()
                    .map(|(i, param_type)| {
                        let type_str = self.format_type(param_type);
                        // Try to get parameter name from child symbol
                        if let Some(param_sym) = param_symbols.get(i) {
                            let param_name = self.interner.resolve(param_sym.name);
                            format!("{} {}", type_str, param_name)
                        } else {
                            // No name available, just show type
                            type_str
                        }
                    })
                    .collect();

                let signature = format!("{}({})", name, params_str.join(", "));

                // Build complete signature
                let mut complete_parts = parts;
                complete_parts.push(ret_type_str);
                complete_parts.push(signature);

                // Add const qualifier
                if symbol.flags.is_const {
                    complete_parts.push("const".to_string());
                }

                // Add override/final
                if symbol.flags.is_override {
                    complete_parts.push("override".to_string());
                }
                if symbol.flags.is_final {
                    complete_parts.push("final".to_string());
                }

                return complete_parts.join(" ");
            }
        }

        // Fallback if no type information
        format!("{} {}(...)", symbol_kind_to_str(symbol.kind), name)
    }

    /// Format a class signature
    fn format_class_signature(&self, symbol: &crate::index::symbol::Symbol, name: &str) -> String {
        let mut signature = format!("class {}", name);

        // Add base classes if any
        if !symbol.bases.is_empty() {
            let bases: Vec<String> = symbol.bases.iter()
                .filter_map(|&base_id| {
                    self.symbol_table.get_symbol(base_id)
                        .map(|s| self.interner.resolve(s.name).to_string())
                })
                .collect();

            if !bases.is_empty() {
                signature.push_str(&format!(" : {}", bases.join(", ")));
            }
        }

        signature
    }

    /// Format a struct signature
    fn format_struct_signature(&self, symbol: &crate::index::symbol::Symbol, name: &str) -> String {
        let mut signature = format!("struct {}", name);

        // Add base classes if any
        if !symbol.bases.is_empty() {
            let bases: Vec<String> = symbol.bases.iter()
                .filter_map(|&base_id| {
                    self.symbol_table.get_symbol(base_id)
                        .map(|s| self.interner.resolve(s.name).to_string())
                })
                .collect();

            if !bases.is_empty() {
                signature.push_str(&format!(" : {}", bases.join(", ")));
            }
        }

        signature
    }

    /// Format a variable signature
    fn format_variable_signature(&self, symbol: &crate::index::symbol::Symbol, name: &str) -> String {
        let mut parts = Vec::new();

        if symbol.flags.is_static {
            parts.push("static".to_string());
        }
        if symbol.flags.is_const {
            parts.push("const".to_string());
        }

        // Add type if available
        if let Some(ref var_type) = symbol.symbol_type {
            parts.push(self.format_type(var_type));
        } else {
            parts.push("auto".to_string());
        }

        parts.push(name.to_string());

        parts.join(" ")
    }

    /// Format a type as a string
    fn format_type(&self, ty: &crate::ast::Type) -> String {
        use crate::ast::{Type, PointerKind, ReferenceKind, PrimitiveType};

        match ty {
            Type::Void => "void".to_string(),
            Type::Primitive(prim) => self.format_primitive_type(*prim),
            Type::Named(type_path) => {
                type_path.segments.iter()
                    .map(|&s| self.interner.resolve(s))
                    .collect::<Vec<_>>()
                    .join("::")
            }
            Type::Pointer(inner, kind) => {
                let inner_str = self.format_type(inner);
                match kind {
                    PointerKind::Raw => format!("{}*", inner_str),
                    PointerKind::Member => format!("{}::*", inner_str),
                }
            }
            Type::Reference(inner, kind) => {
                let inner_str = self.format_type(inner);
                match kind {
                    ReferenceKind::LValue => format!("{}&", inner_str),
                    ReferenceKind::RValue => format!("{}&&", inner_str),
                }
            }
            Type::Array(inner, size) => {
                let inner_str = self.format_type(inner);
                if let Some(sz) = size {
                    format!("{}[{}]", inner_str, sz)
                } else {
                    format!("{}[]", inner_str)
                }
            }
            Type::TemplateInstantiation { template, args } => {
                let template_name = template.segments.iter()
                    .map(|&s| self.interner.resolve(s))
                    .collect::<Vec<_>>()
                    .join("::");

                let args_str: Vec<String> = args.iter()
                    .map(|arg| match arg {
                        crate::ast::TemplateArg::Type(t) => self.format_type(t),
                        crate::ast::TemplateArg::Expr(_) => "...".to_string(),
                    })
                    .collect();

                format!("{}<{}>", template_name, args_str.join(", "))
            }
            Type::Function { return_type, params } => {
                let params_str: Vec<String> = params.iter()
                    .map(|p| self.format_type(p))
                    .collect();

                format!("{}({})", self.format_type(return_type), params_str.join(", "))
            }
            Type::Auto => "auto".to_string(),
            Type::Decltype(_) => "decltype(...)".to_string(),
            Type::Error => "<error>".to_string(),
        }
    }

    /// Format a primitive type
    fn format_primitive_type(&self, prim: PrimitiveType) -> String {
        match prim {
            PrimitiveType::Bool => "bool",
            PrimitiveType::Char => "char",
            PrimitiveType::SignedChar => "signed char",
            PrimitiveType::UnsignedChar => "unsigned char",
            PrimitiveType::WChar => "wchar_t",
            PrimitiveType::Char16 => "char16_t",
            PrimitiveType::Char32 => "char32_t",
            PrimitiveType::Short => "short",
            PrimitiveType::UnsignedShort => "unsigned short",
            PrimitiveType::Int => "int",
            PrimitiveType::UnsignedInt => "unsigned int",
            PrimitiveType::Long => "long",
            PrimitiveType::UnsignedLong => "unsigned long",
            PrimitiveType::LongLong => "long long",
            PrimitiveType::UnsignedLongLong => "unsigned long long",
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
            PrimitiveType::LongDouble => "long double",
        }.to_string()
    }
}

fn symbol_kind_to_str(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class | SymbolKind::UClass => "class",
        SymbolKind::Struct | SymbolKind::UStruct => "struct",
        SymbolKind::Enum | SymbolKind::UEnum => "enum",
        SymbolKind::Function | SymbolKind::UFunction => "function",
        SymbolKind::Method => "method",
        SymbolKind::Variable => "variable",
        SymbolKind::Field | SymbolKind::UProperty => "field",
        SymbolKind::Namespace => "namespace",
        _ => "symbol",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::Span;

    #[test]
    fn test_hover_for_symbol() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        symbol.doc_comment = Some("Test class documentation".to_string());
        table.add_symbol(symbol);

        let provider = HoverProvider::new(&table, &interner);
        let hover = provider.create_hover_for_symbol(id);

        assert!(hover.is_some());
        let contents = hover.unwrap();
        assert!(contents.contents.iter().any(|s| s.contains("class MyClass")));
        assert!(contents.contents.iter().any(|s| s.contains("Test class")));
    }
}
