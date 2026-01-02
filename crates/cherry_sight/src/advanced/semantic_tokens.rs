// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Semantic token highlighting

use crate::index::symbol::SymbolKind;
use crate::index::SymbolTable;
use crate::util::{FileId, Interner, Span};

/// Semantic token type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    Namespace,
    Class,
    Enum,
    Interface,
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Function,
    Method,
    Macro,
    Keyword,
    Comment,
    String,
    Number,
    Operator,
    Type,
}

/// Semantic token modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenModifier {
    Declaration,
    Definition,
    Readonly,
    Static,
    Deprecated,
    Abstract,
    Async,
    Modification,
    Documentation,
    DefaultLibrary,
}

/// Semantic token with type and modifiers
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub span: Span,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
}

/// Provides semantic tokens for highlighting
pub struct SemanticTokensProvider<'a> {
    #[allow(dead_code)]
    symbol_table: &'a SymbolTable,
    #[allow(dead_code)]
    interner: &'a Interner,
}

impl<'a> SemanticTokensProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get all semantic tokens for a file
    pub fn get_semantic_tokens(&self, _file_id: FileId) -> Vec<SemanticToken> {
        // Would need:
        // 1. File symbol iteration (not yet implemented in SymbolTable)
        // 2. Full AST traversal to get all tokens (not just symbols)
        // For now, return empty - full implementation needs infrastructure

        Vec::new()
    }

    /// Map symbol kind to semantic token type
    #[allow(dead_code)]
    fn symbol_kind_to_token_type(&self, kind: SymbolKind) -> SemanticTokenType {
        match kind {
            SymbolKind::Namespace => SemanticTokenType::Namespace,
            SymbolKind::Class | SymbolKind::UClass => SemanticTokenType::Class,
            SymbolKind::Struct | SymbolKind::UStruct => SemanticTokenType::Struct,
            SymbolKind::Enum | SymbolKind::UEnum => SemanticTokenType::Enum,
            SymbolKind::Function | SymbolKind::UFunction => SemanticTokenType::Function,
            SymbolKind::Method => SemanticTokenType::Method,
            SymbolKind::Variable => SemanticTokenType::Variable,
            SymbolKind::Field | SymbolKind::UProperty => SemanticTokenType::Property,
            SymbolKind::Parameter => SemanticTokenType::Parameter,
            SymbolKind::TypeAlias => SemanticTokenType::Type,
            SymbolKind::Macro => SemanticTokenType::Macro,
            _ => SemanticTokenType::Variable,
        }
    }

    /// Get modifiers for a symbol
    #[allow(dead_code)]
    fn get_modifiers_for_symbol(&self, symbol_id: crate::index::symbol::SymbolId) -> Vec<SemanticTokenModifier> {
        let mut modifiers = Vec::new();

        if let Some(_symbol) = self.symbol_table.get_symbol(symbol_id) {
            // Add declaration modifier for symbols
            modifiers.push(SemanticTokenModifier::Declaration);

            // Add definition modifier if symbol has a body
            // Would need AST to check if symbol has definition vs just declaration

            // Check for static modifier
            // Would need to parse storage class specifiers

            // Check for readonly/const
            // Would need type information
        }

        modifiers
    }

    /// Get tokens for a specific range
    pub fn get_tokens_in_range(&self, file_id: FileId, start: u32, end: u32) -> Vec<SemanticToken> {
        self.get_semantic_tokens(file_id)
            .into_iter()
            .filter(|token| {
                token.span.start >= start && token.span.end <= end
            })
            .collect()
    }

    /// Convert to LSP semantic tokens format (delta encoding)
    pub fn encode_tokens(&self, tokens: &[SemanticToken]) -> Vec<(u32, u32, u32, u32, u32)> {
        // LSP semantic tokens use delta encoding:
        // (line_delta, char_delta, length, token_type, modifiers_bitset)

        let mut encoded = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;

        for token in tokens {
            // Would need line index to convert offset to line/column
            // For now, use placeholder values
            let line = 0u32;
            let char = token.span.start;
            let length = token.span.end - token.span.start;
            let token_type_idx = self.token_type_to_index(token.token_type);
            let modifiers_bitset = self.modifiers_to_bitset(&token.modifiers);

            let line_delta = line.saturating_sub(prev_line);
            let char_delta = if line == prev_line {
                char.saturating_sub(prev_char)
            } else {
                char
            };

            encoded.push((line_delta, char_delta, length, token_type_idx, modifiers_bitset));

            prev_line = line;
            prev_char = if line_delta > 0 { 0 } else { char };
        }

        encoded
    }

    /// Convert token type to index
    fn token_type_to_index(&self, token_type: SemanticTokenType) -> u32 {
        match token_type {
            SemanticTokenType::Namespace => 0,
            SemanticTokenType::Class => 1,
            SemanticTokenType::Enum => 2,
            SemanticTokenType::Interface => 3,
            SemanticTokenType::Struct => 4,
            SemanticTokenType::TypeParameter => 5,
            SemanticTokenType::Parameter => 6,
            SemanticTokenType::Variable => 7,
            SemanticTokenType::Property => 8,
            SemanticTokenType::EnumMember => 9,
            SemanticTokenType::Function => 10,
            SemanticTokenType::Method => 11,
            SemanticTokenType::Macro => 12,
            SemanticTokenType::Keyword => 13,
            SemanticTokenType::Comment => 14,
            SemanticTokenType::String => 15,
            SemanticTokenType::Number => 16,
            SemanticTokenType::Operator => 17,
            SemanticTokenType::Type => 18,
        }
    }

    /// Convert modifiers to bitset
    fn modifiers_to_bitset(&self, modifiers: &[SemanticTokenModifier]) -> u32 {
        let mut bitset = 0u32;

        for modifier in modifiers {
            let bit = match modifier {
                SemanticTokenModifier::Declaration => 0,
                SemanticTokenModifier::Definition => 1,
                SemanticTokenModifier::Readonly => 2,
                SemanticTokenModifier::Static => 3,
                SemanticTokenModifier::Deprecated => 4,
                SemanticTokenModifier::Abstract => 5,
                SemanticTokenModifier::Async => 6,
                SemanticTokenModifier::Modification => 7,
                SemanticTokenModifier::Documentation => 8,
                SemanticTokenModifier::DefaultLibrary => 9,
            };
            bitset |= 1 << bit;
        }

        bitset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_token_creation() {
        let file_id = FileId::new(1);
        let token = SemanticToken {
            span: Span::new(file_id, 0, 10),
            token_type: SemanticTokenType::Class,
            modifiers: vec![SemanticTokenModifier::Declaration],
        };

        assert_eq!(token.token_type, SemanticTokenType::Class);
        assert_eq!(token.modifiers.len(), 1);
    }

    #[test]
    fn test_semantic_tokens_provider() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let provider = SemanticTokensProvider::new(&table, &interner);

        let tokens = provider.get_semantic_tokens(FileId::new(1));
        assert_eq!(tokens.len(), 0); // No symbols
    }

    #[test]
    fn test_symbol_kind_mapping() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let provider = SemanticTokensProvider::new(&table, &interner);

        assert_eq!(
            provider.symbol_kind_to_token_type(SymbolKind::Class),
            SemanticTokenType::Class
        );
        assert_eq!(
            provider.symbol_kind_to_token_type(SymbolKind::Function),
            SemanticTokenType::Function
        );
    }

    #[test]
    fn test_token_type_indices() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let provider = SemanticTokensProvider::new(&table, &interner);

        assert_eq!(provider.token_type_to_index(SemanticTokenType::Namespace), 0);
        assert_eq!(provider.token_type_to_index(SemanticTokenType::Class), 1);
        assert_eq!(provider.token_type_to_index(SemanticTokenType::Function), 10);
    }
}
