// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Blueprint property validation

use super::uasset_parser::PropertyMetadata;
use std::collections::HashMap;

/// Property validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyValidationError {
    TypeMismatch { expected: String, found: String },
    InvalidFlags { reason: String },
    MissingInCpp { property_name: String },
    MissingInBlueprint { property_name: String },
    AccessModifierConflict { reason: String },
}

impl PropertyValidationError {
    pub fn message(&self) -> String {
        match self {
            PropertyValidationError::TypeMismatch { expected, found } => {
                format!("Type mismatch: expected '{}', found '{}'", expected, found)
            }
            PropertyValidationError::InvalidFlags { reason } => {
                format!("Invalid property flags: {}", reason)
            }
            PropertyValidationError::MissingInCpp { property_name } => {
                format!("Property '{}' exists in Blueprint but not in C++", property_name)
            }
            PropertyValidationError::MissingInBlueprint { property_name } => {
                format!("Property '{}' exists in C++ but not in Blueprint", property_name)
            }
            PropertyValidationError::AccessModifierConflict { reason } => {
                format!("Access modifier conflict: {}", reason)
            }
        }
    }
}

/// C++ property definition
#[derive(Debug, Clone)]
pub struct CppProperty {
    pub name: String,
    pub property_type: String,
    pub is_editable: bool,
    pub is_blueprint_read_only: bool,
    pub is_blueprint_read_write: bool,
    pub is_replicated: bool,
}

/// Property validator
pub struct PropertyValidator {
    type_mappings: HashMap<String, String>,
}

impl PropertyValidator {
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // UE5 property type mappings
        type_mappings.insert("bool".to_string(), "bool".to_string());
        type_mappings.insert("int32".to_string(), "int".to_string());
        type_mappings.insert("float".to_string(), "float".to_string());
        type_mappings.insert("double".to_string(), "double".to_string());
        type_mappings.insert("FString".to_string(), "string".to_string());
        type_mappings.insert("FName".to_string(), "name".to_string());
        type_mappings.insert("FVector".to_string(), "vector".to_string());
        type_mappings.insert("FRotator".to_string(), "rotator".to_string());
        type_mappings.insert("FTransform".to_string(), "transform".to_string());
        type_mappings.insert("UObject*".to_string(), "object".to_string());
        type_mappings.insert("AActor*".to_string(), "actor".to_string());
        type_mappings.insert("TArray".to_string(), "array".to_string());
        type_mappings.insert("TMap".to_string(), "map".to_string());

        Self { type_mappings }
    }

    /// Validate Blueprint property against C++ property
    pub fn validate_property(
        &self,
        cpp_prop: &CppProperty,
        bp_prop: &PropertyMetadata,
    ) -> Result<(), PropertyValidationError> {
        // Check type compatibility
        if !self.types_compatible(&cpp_prop.property_type, &bp_prop.property_type) {
            return Err(PropertyValidationError::TypeMismatch {
                expected: cpp_prop.property_type.clone(),
                found: bp_prop.property_type.clone(),
            });
        }

        // Check flags consistency
        if cpp_prop.is_blueprint_read_only && bp_prop.flags.is_blueprint_read_write {
            return Err(PropertyValidationError::InvalidFlags {
                reason: format!(
                    "Property '{}' is read-only in C++ but read-write in Blueprint",
                    cpp_prop.name
                ),
            });
        }

        // Check EditAnywhere consistency
        if cpp_prop.is_editable != bp_prop.flags.is_editable {
            return Err(PropertyValidationError::AccessModifierConflict {
                reason: format!(
                    "Property '{}' editable status mismatch",
                    cpp_prop.name
                ),
            });
        }

        Ok(())
    }

    /// Check if types are compatible
    fn types_compatible(&self, cpp_type: &str, bp_type: &str) -> bool {
        let cpp_normalized = self.normalize_type(cpp_type);
        let bp_normalized = self.normalize_type(bp_type);

        if cpp_normalized == bp_normalized {
            return true;
        }

        // Check type mapping
        if let Some(mapped) = self.type_mappings.get(cpp_type) {
            if mapped == &bp_normalized {
                return true;
            }
        }

        false
    }

    /// Normalize type string
    fn normalize_type(&self, type_str: &str) -> String {
        type_str
            .trim()
            .replace("const ", "")
            .replace(['&', '*'], "")
            .trim()
            .to_lowercase()
    }

    /// Validate all properties
    pub fn validate_all(
        &self,
        cpp_properties: &[CppProperty],
        bp_properties: &[PropertyMetadata],
    ) -> Vec<PropertyValidationError> {
        let mut errors = Vec::new();

        // Create lookup maps
        let cpp_map: HashMap<_, _> = cpp_properties
            .iter()
            .map(|p| (p.name.clone(), p))
            .collect();

        let bp_map: HashMap<_, _> = bp_properties
            .iter()
            .map(|p| (p.name.clone(), p))
            .collect();

        // Check properties in Blueprint
        for bp_prop in bp_properties {
            if let Some(cpp_prop) = cpp_map.get(&bp_prop.name) {
                if let Err(err) = self.validate_property(cpp_prop, bp_prop) {
                    errors.push(err);
                }
            } else {
                errors.push(PropertyValidationError::MissingInCpp {
                    property_name: bp_prop.name.clone(),
                });
            }
        }

        // Check for properties in C++ not in Blueprint
        for cpp_prop in cpp_properties {
            if !bp_map.contains_key(&cpp_prop.name) {
                // Only report if property is supposed to be Blueprint-visible
                if cpp_prop.is_editable
                    || cpp_prop.is_blueprint_read_only
                    || cpp_prop.is_blueprint_read_write
                {
                    errors.push(PropertyValidationError::MissingInBlueprint {
                        property_name: cpp_prop.name.clone(),
                    });
                }
            }
        }

        errors
    }

    /// Check if property is Blueprint-accessible
    pub fn is_blueprint_accessible(&self, prop: &PropertyMetadata) -> bool {
        prop.flags.is_blueprint_read_only || prop.flags.is_blueprint_read_write
    }

    /// Check if property should be replicated
    pub fn should_replicate(&self, cpp_prop: &CppProperty) -> bool {
        cpp_prop.is_replicated
    }
}

impl Default for PropertyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::uasset_parser::PropertyFlags;

    #[test]
    fn test_property_validation_error_message() {
        let err = PropertyValidationError::TypeMismatch {
            expected: "int32".to_string(),
            found: "float".to_string(),
        };
        assert!(err.message().contains("Type mismatch"));
    }

    #[test]
    fn test_property_validator_creation() {
        let validator = PropertyValidator::new();
        assert!(!validator.type_mappings.is_empty());
    }

    #[test]
    fn test_validate_matching_property() {
        let validator = PropertyValidator::new();

        let cpp_prop = CppProperty {
            name: "Health".to_string(),
            property_type: "float".to_string(),
            is_editable: true,
            is_blueprint_read_only: false,
            is_blueprint_read_write: true,
            is_replicated: false,
        };

        let bp_prop = PropertyMetadata {
            name: "Health".to_string(),
            property_type: "float".to_string(),
            flags: PropertyFlags {
                is_editable: true,
                is_blueprint_read_only: false,
                is_blueprint_read_write: true,
                is_replicated: false,
            },
        };

        assert!(validator.validate_property(&cpp_prop, &bp_prop).is_ok());
    }

    #[test]
    fn test_validate_type_mismatch() {
        let validator = PropertyValidator::new();

        let cpp_prop = CppProperty {
            name: "Count".to_string(),
            property_type: "int32".to_string(),
            is_editable: true,
            is_blueprint_read_only: false,
            is_blueprint_read_write: true,
            is_replicated: false,
        };

        let bp_prop = PropertyMetadata {
            name: "Count".to_string(),
            property_type: "float".to_string(),
            flags: PropertyFlags::default(),
        };

        let result = validator.validate_property(&cpp_prop, &bp_prop);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PropertyValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_validate_read_only_conflict() {
        let validator = PropertyValidator::new();

        let cpp_prop = CppProperty {
            name: "MaxHealth".to_string(),
            property_type: "float".to_string(),
            is_editable: true,
            is_blueprint_read_only: true,
            is_blueprint_read_write: false,
            is_replicated: false,
        };

        let bp_prop = PropertyMetadata {
            name: "MaxHealth".to_string(),
            property_type: "float".to_string(),
            flags: PropertyFlags {
                is_editable: true,
                is_blueprint_read_only: false,
                is_blueprint_read_write: true,
                is_replicated: false,
            },
        };

        let result = validator.validate_property(&cpp_prop, &bp_prop);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_blueprint_accessible() {
        let validator = PropertyValidator::new();

        let prop = PropertyMetadata {
            name: "Score".to_string(),
            property_type: "int".to_string(),
            flags: PropertyFlags {
                is_blueprint_read_only: true,
                ..Default::default()
            },
        };

        assert!(validator.is_blueprint_accessible(&prop));
    }

    #[test]
    fn test_should_replicate() {
        let validator = PropertyValidator::new();

        let prop = CppProperty {
            name: "NetworkHealth".to_string(),
            property_type: "float".to_string(),
            is_editable: true,
            is_blueprint_read_only: false,
            is_blueprint_read_write: true,
            is_replicated: true,
        };

        assert!(validator.should_replicate(&prop));
    }

    #[test]
    fn test_validate_all() {
        let validator = PropertyValidator::new();

        let cpp_props = vec![CppProperty {
            name: "Health".to_string(),
            property_type: "float".to_string(),
            is_editable: true,
            is_blueprint_read_only: false,
            is_blueprint_read_write: true,
            is_replicated: false,
        }];

        let bp_props = vec![PropertyMetadata {
            name: "Health".to_string(),
            property_type: "float".to_string(),
            flags: PropertyFlags {
                is_editable: true,
                is_blueprint_read_write: true,
                ..Default::default()
            },
        }];

        let errors = validator.validate_all(&cpp_props, &bp_props);
        assert_eq!(errors.len(), 0);
    }
}
