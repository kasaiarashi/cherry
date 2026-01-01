// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 macro recognition and parsing
//!
//! Handles UE-specific macros like:
//! - UCLASS, USTRUCT, UENUM, UINTERFACE
//! - UPROPERTY, UFUNCTION
//! - GENERATED_BODY, GENERATED_UCLASS_BODY

use crate::util::Span;

/// Kind of UE macro
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UEMacroKind {
    UClass,
    UStruct,
    UEnum,
    UInterface,
    UProperty,
    UFunction,
    UDelegate,
    GeneratedBody,
    GeneratedUClassBody,
}

impl UEMacroKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "UCLASS" => Some(Self::UClass),
            "USTRUCT" => Some(Self::UStruct),
            "UENUM" => Some(Self::UEnum),
            "UINTERFACE" => Some(Self::UInterface),
            "UPROPERTY" => Some(Self::UProperty),
            "UFUNCTION" => Some(Self::UFunction),
            "UDELEGATE" => Some(Self::UDelegate),
            "GENERATED_BODY" => Some(Self::GeneratedBody),
            "GENERATED_UCLASS_BODY" => Some(Self::GeneratedUClassBody),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UClass => "UCLASS",
            Self::UStruct => "USTRUCT",
            Self::UEnum => "UENUM",
            Self::UInterface => "UINTERFACE",
            Self::UProperty => "UPROPERTY",
            Self::UFunction => "UFUNCTION",
            Self::UDelegate => "UDELEGATE",
            Self::GeneratedBody => "GENERATED_BODY",
            Self::GeneratedUClassBody => "GENERATED_UCLASS_BODY",
        }
    }
}

/// A UE macro invocation
#[derive(Debug, Clone)]
pub struct UEMacro {
    pub kind: UEMacroKind,
    pub span: Span,
    pub specifiers: Vec<String>,
}

impl UEMacro {
    pub fn new(kind: UEMacroKind, span: Span) -> Self {
        Self {
            kind,
            span,
            specifiers: Vec::new(),
        }
    }

    pub fn with_specifiers(mut self, specifiers: Vec<String>) -> Self {
        self.specifiers = specifiers;
        self
    }
}

/// Parse UE macro specifiers from a string
/// Example: "BlueprintType, Blueprintable, meta=(DisplayName=\"My Class\")"
pub fn parse_specifiers(input: &str) -> Vec<String> {
    // Simple comma-separated parsing
    // TODO: Handle nested parentheses and meta tags properly in later phases
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::FileId;

    #[test]
    fn test_macro_kind_from_str() {
        assert_eq!(UEMacroKind::from_str("UCLASS"), Some(UEMacroKind::UClass));
        assert_eq!(
            UEMacroKind::from_str("UPROPERTY"),
            Some(UEMacroKind::UProperty)
        );
        assert_eq!(UEMacroKind::from_str("INVALID"), None);
    }

    #[test]
    fn test_macro_kind_as_str() {
        assert_eq!(UEMacroKind::UClass.as_str(), "UCLASS");
        assert_eq!(UEMacroKind::UProperty.as_str(), "UPROPERTY");
    }

    #[test]
    fn test_parse_specifiers() {
        let spec = "BlueprintType, Blueprintable";
        let parsed = parse_specifiers(spec);

        assert_eq!(parsed.len(), 2);
        assert!(parsed.contains(&"BlueprintType".to_string()));
        assert!(parsed.contains(&"Blueprintable".to_string()));
    }

    #[test]
    fn test_macro_creation() {
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let macro_inst = UEMacro::new(UEMacroKind::UClass, span)
            .with_specifiers(vec!["BlueprintType".to_string()]);

        assert_eq!(macro_inst.kind, UEMacroKind::UClass);
        assert_eq!(macro_inst.specifiers.len(), 1);
    }
}
