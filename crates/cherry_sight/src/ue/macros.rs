// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 macro recognition and validation

use crate::util::Span;
use std::collections::HashMap;
use std::str::FromStr;

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

impl FromStr for UEMacroKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UCLASS" => Ok(Self::UClass),
            "USTRUCT" => Ok(Self::UStruct),
            "UENUM" => Ok(Self::UEnum),
            "UINTERFACE" => Ok(Self::UInterface),
            "UPROPERTY" => Ok(Self::UProperty),
            "UFUNCTION" => Ok(Self::UFunction),
            "UDELEGATE" => Ok(Self::UDelegate),
            "GENERATED_BODY" => Ok(Self::GeneratedBody),
            "GENERATED_UCLASS_BODY" => Ok(Self::GeneratedUClassBody),
            _ => Err(format!("Unknown UE macro: {}", s)),
        }
    }
}

impl UEMacroKind {
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
    pub specifiers: MacroSpecifiers,
}

/// Macro specifiers container
#[derive(Debug, Clone, Default)]
pub struct MacroSpecifiers {
    pub flags: Vec<String>,
    pub meta: HashMap<String, String>,
}

impl MacroSpecifiers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case(flag))
    }

    pub fn get_meta(&self, key: &str) -> Option<&String> {
        self.meta.get(key)
    }

    pub fn add_flag(&mut self, flag: String) {
        if !self.has_flag(&flag) {
            self.flags.push(flag);
        }
    }

    pub fn add_meta(&mut self, key: String, value: String) {
        self.meta.insert(key, value);
    }
}

/// UCLASS specifier validator
pub struct UClassValidator;

impl UClassValidator {
    pub const VALID_SPECIFIERS: &'static [&'static str] = &[
        "Abstract",
        "AdvancedClassDisplay",
        "AutoCollapseCategories",
        "AutoExpandCategories",
        "Blueprintable",
        "BlueprintType",
        "ClassGroup",
        "CollapseCategories",
        "Config",
        "Const",
        "ConversionRoot",
        "CustomConstructor",
        "DefaultToInstanced",
        "DependsOn",
        "Deprecated",
        "DontAutoCollapseCategories",
        "DontCollapseCategories",
        "EditInlineNew",
        "HideCategories",
        "HideDropdown",
        "HideFunctions",
        "Intrinsic",
        "MinimalAPI",
        "NoExport",
        "NonTransient",
        "NotBlueprintable",
        "NotPlaceable",
        "Placeable",
        "ShowCategories",
        "ShowFunctions",
        "Transient",
        "Within",
    ];

    pub fn validate(specifiers: &MacroSpecifiers) -> Vec<String> {
        let mut errors = Vec::new();

        for flag in &specifiers.flags {
            if !Self::VALID_SPECIFIERS.contains(&flag.as_str()) {
                errors.push(format!("Unknown UCLASS specifier: {}", flag));
            }
        }

        // Validate conflicting specifiers
        if specifiers.has_flag("Blueprintable") && specifiers.has_flag("NotBlueprintable") {
            errors.push("Cannot use both Blueprintable and NotBlueprintable".to_string());
        }

        if specifiers.has_flag("Placeable") && specifiers.has_flag("NotPlaceable") {
            errors.push("Cannot use both Placeable and NotPlaceable".to_string());
        }

        errors
    }
}

/// UPROPERTY specifier validator
pub struct UPropertyValidator;

impl UPropertyValidator {
    pub const VALID_SPECIFIERS: &'static [&'static str] = &[
        "AdvancedDisplay",
        "AssetRegistrySearchable",
        "BlueprintAssignable",
        "BlueprintCallable",
        "BlueprintGetter",
        "BlueprintReadOnly",
        "BlueprintReadWrite",
        "BlueprintSetter",
        "Category",
        "Config",
        "DuplicateTransient",
        "EditAnywhere",
        "EditDefaultsOnly",
        "EditFixedSize",
        "EditInline",
        "EditInstanceOnly",
        "Export",
        "GlobalConfig",
        "Instanced",
        "Interp",
        "Localized",
        "Native",
        "NoClear",
        "NoExport",
        "NonPIEDuplicateTransient",
        "NonTransactional",
        "NotReplicated",
        "Replicated",
        "ReplicatedUsing",
        "RepRetry",
        "SaveGame",
        "SerializeText",
        "SimpleDisplay",
        "SkipSerialization",
        "TextExportTransient",
        "Transient",
        "VisibleAnywhere",
        "VisibleDefaultsOnly",
        "VisibleInstanceOnly",
    ];

    pub fn validate(specifiers: &MacroSpecifiers) -> Vec<String> {
        let mut errors = Vec::new();

        for flag in &specifiers.flags {
            if !Self::VALID_SPECIFIERS.contains(&flag.as_str()) {
                errors.push(format!("Unknown UPROPERTY specifier: {}", flag));
            }
        }

        // Validate edit specifier conflicts
        let edit_count = [
            "EditAnywhere",
            "EditDefaultsOnly",
            "EditInstanceOnly",
            "VisibleAnywhere",
            "VisibleDefaultsOnly",
            "VisibleInstanceOnly",
        ]
        .iter()
        .filter(|s| specifiers.has_flag(s))
        .count();

        if edit_count > 1 {
            errors.push("Can only use one edit/visible specifier".to_string());
        }

        // Validate blueprint specifier conflicts
        if specifiers.has_flag("BlueprintReadOnly") && specifiers.has_flag("BlueprintReadWrite") {
            errors.push("Cannot use both BlueprintReadOnly and BlueprintReadWrite".to_string());
        }

        errors
    }
}

/// UFUNCTION specifier validator
pub struct UFunctionValidator;

impl UFunctionValidator {
    pub const VALID_SPECIFIERS: &'static [&'static str] = &[
        "BlueprintAuthorityOnly",
        "BlueprintCallable",
        "BlueprintCosmetic",
        "BlueprintGetter",
        "BlueprintImplementableEvent",
        "BlueprintNativeEvent",
        "BlueprintPure",
        "BlueprintSetter",
        "CallInEditor",
        "Category",
        "Client",
        "CustomThunk",
        "Exec",
        "FieldNotify",
        "Meta",
        "NetMulticast",
        "Reliable",
        "SealedEvent",
        "Server",
        "ServiceRequest",
        "ServiceResponse",
        "Unreliable",
        "WithValidation",
    ];

    pub fn validate(specifiers: &MacroSpecifiers) -> Vec<String> {
        let mut errors = Vec::new();

        for flag in &specifiers.flags {
            if !Self::VALID_SPECIFIERS.contains(&flag.as_str()) {
                errors.push(format!("Unknown UFUNCTION specifier: {}", flag));
            }
        }

        // Validate network specifier rules
        if specifiers.has_flag("Server")
            || specifiers.has_flag("Client")
            || specifiers.has_flag("NetMulticast")
        {
            let has_reliable = specifiers.has_flag("Reliable");
            let has_unreliable = specifiers.has_flag("Unreliable");

            if !has_reliable && !has_unreliable {
                errors.push("Network functions must specify Reliable or Unreliable".to_string());
            }

            if has_reliable && has_unreliable {
                errors.push("Cannot use both Reliable and Unreliable".to_string());
            }
        }

        // BlueprintPure functions cannot modify state
        // BlueprintPure implies BlueprintCallable, so that combination is allowed

        errors
    }
}

/// Parse UE macro specifiers from a string
pub fn parse_specifiers(input: &str) -> MacroSpecifiers {
    let mut specifiers = MacroSpecifiers::new();

    // Split by commas, but be careful with nested parentheses in meta tags
    let mut depth = 0;
    let mut current = String::new();

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                process_specifier(&mut specifiers, current.trim());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Process last specifier
    if !current.trim().is_empty() {
        process_specifier(&mut specifiers, current.trim());
    }

    specifiers
}

fn process_specifier(specifiers: &mut MacroSpecifiers, spec: &str) {
    if let Some(eq_pos) = spec.find('=') {
        // Meta tag: key=value or key=(value)
        let key = spec[..eq_pos].trim().to_string();
        let value = spec[eq_pos + 1..].trim();

        // Remove surrounding parentheses if present
        let value = if value.starts_with('(') && value.ends_with(')') {
            &value[1..value.len() - 1]
        } else {
            value
        };

        // Remove quotes if present
        let value = value.trim_matches('"').to_string();

        specifiers.add_meta(key, value);
    } else {
        // Simple flag
        specifiers.add_flag(spec.to_string());
    }
}

impl UEMacro {
    pub fn new(kind: UEMacroKind, span: Span) -> Self {
        Self {
            kind,
            span,
            specifiers: MacroSpecifiers::new(),
        }
    }

    pub fn with_specifiers(mut self, specifiers: MacroSpecifiers) -> Self {
        self.specifiers = specifiers;
        self
    }

    /// Validate macro specifiers based on macro kind
    pub fn validate(&self) -> Vec<String> {
        match self.kind {
            UEMacroKind::UClass => UClassValidator::validate(&self.specifiers),
            UEMacroKind::UProperty => UPropertyValidator::validate(&self.specifiers),
            UEMacroKind::UFunction => UFunctionValidator::validate(&self.specifiers),
            _ => Vec::new(), // Other macros don't have complex validation yet
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::FileId;

    #[test]
    fn test_macro_kind_from_str() {
        assert_eq!(UEMacroKind::from_str("UCLASS"), Ok(UEMacroKind::UClass));
        assert_eq!(
            UEMacroKind::from_str("UPROPERTY"),
            Ok(UEMacroKind::UProperty)
        );
        assert!(UEMacroKind::from_str("INVALID").is_err());
    }

    #[test]
    fn test_parse_simple_specifiers() {
        let spec = "BlueprintType, Blueprintable";
        let parsed = parse_specifiers(spec);

        assert!(parsed.has_flag("BlueprintType"));
        assert!(parsed.has_flag("Blueprintable"));
        assert_eq!(parsed.flags.len(), 2);
    }

    #[test]
    fn test_parse_meta_specifiers() {
        let spec = r#"Category="MyCategory", DisplayName="My Class""#;
        let parsed = parse_specifiers(spec);

        assert_eq!(parsed.get_meta("Category"), Some(&"MyCategory".to_string()));
        assert_eq!(parsed.get_meta("DisplayName"), Some(&"My Class".to_string()));
    }

    #[test]
    fn test_parse_mixed_specifiers() {
        let spec = r#"BlueprintType, Category="Gameplay", Blueprintable"#;
        let parsed = parse_specifiers(spec);

        assert!(parsed.has_flag("BlueprintType"));
        assert!(parsed.has_flag("Blueprintable"));
        assert_eq!(parsed.get_meta("Category"), Some(&"Gameplay".to_string()));
    }

    #[test]
    fn test_uclass_validation() {
        let mut specifiers = MacroSpecifiers::new();
        specifiers.add_flag("Blueprintable".to_string());
        specifiers.add_flag("BlueprintType".to_string());

        let errors = UClassValidator::validate(&specifiers);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_uclass_validation_conflict() {
        let mut specifiers = MacroSpecifiers::new();
        specifiers.add_flag("Blueprintable".to_string());
        specifiers.add_flag("NotBlueprintable".to_string());

        let errors = UClassValidator::validate(&specifiers);
        assert!(errors.len() > 0);
        assert!(errors[0].contains("Blueprintable"));
    }

    #[test]
    fn test_uproperty_validation() {
        let mut specifiers = MacroSpecifiers::new();
        specifiers.add_flag("EditAnywhere".to_string());
        specifiers.add_flag("BlueprintReadWrite".to_string());

        let errors = UPropertyValidator::validate(&specifiers);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_ufunction_network_validation() {
        let mut specifiers = MacroSpecifiers::new();
        specifiers.add_flag("Server".to_string());
        // Missing Reliable/Unreliable

        let errors = UFunctionValidator::validate(&specifiers);
        assert!(errors.len() > 0);
        assert!(errors[0].contains("Reliable"));
    }

    #[test]
    fn test_macro_creation() {
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let mut macro_specifiers = MacroSpecifiers::new();
        macro_specifiers.add_flag("BlueprintType".to_string());

        let macro_inst = UEMacro::new(UEMacroKind::UClass, span)
            .with_specifiers(macro_specifiers);

        assert_eq!(macro_inst.kind, UEMacroKind::UClass);
        assert!(macro_inst.specifiers.has_flag("BlueprintType"));

        let errors = macro_inst.validate();
        assert_eq!(errors.len(), 0);
    }
}
