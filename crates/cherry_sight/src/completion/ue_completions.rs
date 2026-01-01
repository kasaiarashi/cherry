// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5-specific completions

use crate::completion::provider::{CompletionItem, CompletionKind};

/// Provides UE5-specific completions
pub struct UE5CompletionProvider;

impl UE5CompletionProvider {
    pub fn new() -> Self {
        Self
    }

    /// Get UCLASS specifier completions
    pub fn get_uclass_specifiers(&self) -> Vec<CompletionItem> {
        let specifiers = [
            ("Blueprintable", "Makes this class blueprintable"),
            ("BlueprintType", "Makes this class usable as a blueprint type"),
            ("Abstract", "Marks class as abstract"),
            ("NotPlaceable", "Prevents placing in level"),
            ("Placeable", "Allows placing in level"),
            ("Config", "Allows configuration from ini files"),
            ("MinimalAPI", "Exports minimal API for DLL"),
        ];

        specifiers
            .iter()
            .map(|(name, doc)| CompletionItem {
                label: name.to_string(),
                kind: CompletionKind::Property,
                detail: Some("UCLASS specifier".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: None,
                sort_text: Some(format!("a{}", name)),
                filter_text: None,
            })
            .collect()
    }

    /// Get UPROPERTY specifier completions
    pub fn get_uproperty_specifiers(&self) -> Vec<CompletionItem> {
        let specifiers = [
            ("EditAnywhere", "Editable in editor and instances"),
            ("EditDefaultsOnly", "Only editable in blueprint defaults"),
            ("EditInstanceOnly", "Only editable in level instances"),
            ("VisibleAnywhere", "Visible in editor"),
            ("BlueprintReadWrite", "Readable and writable in blueprints"),
            ("BlueprintReadOnly", "Read-only in blueprints"),
            ("Category", "Category for editor grouping"),
            ("Replicated", "Replicated over network"),
        ];

        specifiers
            .iter()
            .map(|(name, doc)| CompletionItem {
                label: name.to_string(),
                kind: CompletionKind::Property,
                detail: Some("UPROPERTY specifier".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: None,
                sort_text: Some(format!("a{}", name)),
                filter_text: None,
            })
            .collect()
    }

    /// Get UFUNCTION specifier completions
    pub fn get_ufunction_specifiers(&self) -> Vec<CompletionItem> {
        let specifiers = [
            ("BlueprintCallable", "Callable from blueprints"),
            ("BlueprintPure", "Pure function (no side effects)"),
            ("BlueprintImplementableEvent", "Implemented in blueprint"),
            ("BlueprintNativeEvent", "Has native and blueprint implementations"),
            ("Server", "Execute on server"),
            ("Client", "Execute on client"),
            ("Reliable", "Guaranteed delivery"),
            ("Unreliable", "Best effort delivery"),
        ];

        specifiers
            .iter()
            .map(|(name, doc)| CompletionItem {
                label: name.to_string(),
                kind: CompletionKind::Property,
                detail: Some("UFUNCTION specifier".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: None,
                sort_text: Some(format!("a{}", name)),
                filter_text: None,
            })
            .collect()
    }
}

impl Default for UE5CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uclass_specifiers() {
        let provider = UE5CompletionProvider::new();
        let completions = provider.get_uclass_specifiers();

        assert!(completions.iter().any(|c| c.label == "Blueprintable"));
        assert!(completions.iter().any(|c| c.label == "Abstract"));
    }

    #[test]
    fn test_uproperty_specifiers() {
        let provider = UE5CompletionProvider::new();
        let completions = provider.get_uproperty_specifiers();

        assert!(completions.iter().any(|c| c.label == "EditAnywhere"));
        assert!(completions.iter().any(|c| c.label == "BlueprintReadWrite"));
    }

    #[test]
    fn test_ufunction_specifiers() {
        let provider = UE5CompletionProvider::new();
        let completions = provider.get_ufunction_specifiers();

        assert!(completions.iter().any(|c| c.label == "BlueprintCallable"));
        assert!(completions.iter().any(|c| c.label == "Server"));
    }
}
