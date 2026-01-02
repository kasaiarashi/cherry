// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! .uasset metadata parser for Blueprint integration

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// .uasset file metadata
#[derive(Debug, Clone)]
pub struct UAssetMetadata {
    pub path: PathBuf,
    pub package_name: String,
    pub asset_class: AssetClass,
    pub dependencies: Vec<String>,
    pub exports: Vec<ExportEntry>,
    pub imports: Vec<ImportEntry>,
}

/// Asset class type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetClass {
    Blueprint,
    BlueprintGeneratedClass,
    WidgetBlueprint,
    AnimBlueprint,
    Other,
}

impl AssetClass {
    pub fn parse(s: &str) -> Self {
        match s {
            "Blueprint" => AssetClass::Blueprint,
            "BlueprintGeneratedClass" => AssetClass::BlueprintGeneratedClass,
            "WidgetBlueprint" => AssetClass::WidgetBlueprint,
            "AnimBlueprint" => AssetClass::AnimBlueprint,
            _ => AssetClass::Other,
        }
    }

    pub fn is_blueprint(&self) -> bool {
        matches!(
            self,
            AssetClass::Blueprint
                | AssetClass::BlueprintGeneratedClass
                | AssetClass::WidgetBlueprint
                | AssetClass::AnimBlueprint
        )
    }
}

/// Export entry in .uasset
#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub object_name: String,
    pub class_name: String,
    pub properties: HashMap<String, PropertyValue>,
}

/// Import entry in .uasset
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub package_name: String,
    pub object_name: String,
    pub class_name: String,
}

/// Property value in Blueprint
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Name(String),
    Object(String),
    Array(Vec<PropertyValue>),
    Struct(HashMap<String, PropertyValue>),
}

/// .uasset parser
pub struct UAssetParser {
    // Parser state
}

impl UAssetParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse .uasset file
    /// Note: Full binary parsing is complex - this is a simplified structure
    pub fn parse(&self, path: &Path) -> Result<UAssetMetadata, String> {
        // In full implementation, would parse binary .uasset format
        // For now, return basic metadata structure

        if !path.exists() {
            return Err(format!("File does not exist: {}", path.display()));
        }

        let package_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Check file extension
        if path.extension().and_then(|s| s.to_str()) != Some("uasset") {
            return Err("Not a .uasset file".to_string());
        }

        Ok(UAssetMetadata {
            path: path.to_path_buf(),
            package_name,
            asset_class: AssetClass::Blueprint, // Would parse from file
            dependencies: Vec::new(),           // Would parse from file
            exports: Vec::new(),                // Would parse from file
            imports: Vec::new(),                // Would parse from file
        })
    }

    /// Check if file is a Blueprint asset
    pub fn is_blueprint_asset(&self, path: &Path) -> bool {
        if let Ok(metadata) = self.parse(path) {
            metadata.asset_class.is_blueprint()
        } else {
            false
        }
    }

    /// Get Blueprint parent class
    pub fn get_parent_class(&self, _metadata: &UAssetMetadata) -> Option<String> {
        // Would parse parent class from .uasset data
        None
    }

    /// Get Blueprint functions
    pub fn get_functions(&self, _metadata: &UAssetMetadata) -> Vec<FunctionMetadata> {
        // Would parse function metadata from .uasset
        Vec::new()
    }

    /// Get Blueprint properties
    pub fn get_properties(&self, _metadata: &UAssetMetadata) -> Vec<PropertyMetadata> {
        // Would parse property metadata from .uasset
        Vec::new()
    }
}

impl Default for UAssetParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Function metadata from Blueprint
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<ParameterMetadata>,
    pub flags: FunctionFlags,
}

/// Parameter metadata
#[derive(Debug, Clone)]
pub struct ParameterMetadata {
    pub name: String,
    pub param_type: String,
    pub is_reference: bool,
    pub is_const: bool,
}

/// Function flags
#[derive(Debug, Clone, Default)]
pub struct FunctionFlags {
    pub is_event: bool,
    pub is_pure: bool,
    pub is_const: bool,
    pub is_blueprint_callable: bool,
    pub is_blueprint_implementable: bool,
}

/// Property metadata from Blueprint
#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    pub name: String,
    pub property_type: String,
    pub flags: PropertyFlags,
}

/// Property flags
#[derive(Debug, Clone, Default)]
pub struct PropertyFlags {
    pub is_editable: bool,
    pub is_blueprint_read_only: bool,
    pub is_blueprint_read_write: bool,
    pub is_replicated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_class_parse() {
        assert_eq!(
            AssetClass::parse("Blueprint"),
            AssetClass::Blueprint
        );
        assert_eq!(
            AssetClass::parse("WidgetBlueprint"),
            AssetClass::WidgetBlueprint
        );
        assert_eq!(AssetClass::parse("Unknown"), AssetClass::Other);
    }

    #[test]
    fn test_asset_class_is_blueprint() {
        assert!(AssetClass::Blueprint.is_blueprint());
        assert!(AssetClass::WidgetBlueprint.is_blueprint());
        assert!(!AssetClass::Other.is_blueprint());
    }

    #[test]
    fn test_uasset_parser_creation() {
        let parser = UAssetParser::new();
        assert!(std::ptr::addr_of!(parser) as usize != 0);
    }

    #[test]
    fn test_property_value_types() {
        let bool_val = PropertyValue::Bool(true);
        let int_val = PropertyValue::Int(42);
        let float_val = PropertyValue::Float(3.14);
        let string_val = PropertyValue::String("test".to_string());

        assert!(matches!(bool_val, PropertyValue::Bool(true)));
        assert!(matches!(int_val, PropertyValue::Int(42)));
        assert!(matches!(float_val, PropertyValue::Float(_)));
        assert!(matches!(string_val, PropertyValue::String(_)));
    }

    #[test]
    fn test_function_metadata() {
        let func = FunctionMetadata {
            name: "OnBeginPlay".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            flags: FunctionFlags {
                is_event: true,
                is_blueprint_implementable: true,
                ..Default::default()
            },
        };

        assert_eq!(func.name, "OnBeginPlay");
        assert!(func.flags.is_event);
    }

    #[test]
    fn test_property_metadata() {
        let prop = PropertyMetadata {
            name: "Health".to_string(),
            property_type: "float".to_string(),
            flags: PropertyFlags {
                is_editable: true,
                is_blueprint_read_write: true,
                ..Default::default()
            },
        };

        assert_eq!(prop.name, "Health");
        assert!(prop.flags.is_editable);
    }
}
