// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Blueprint signature validation

use super::uasset_parser::FunctionMetadata;
use std::collections::HashMap;

/// Signature validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(Vec<String>),
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    pub fn errors(&self) -> Vec<String> {
        match self {
            ValidationResult::Valid => Vec::new(),
            ValidationResult::Invalid(errors) => errors.clone(),
        }
    }
}

/// C++ function signature
#[derive(Debug, Clone)]
pub struct CppSignature {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<CppParameter>,
    pub is_const: bool,
}

/// C++ parameter
#[derive(Debug, Clone)]
pub struct CppParameter {
    pub name: String,
    pub param_type: String,
    pub is_reference: bool,
    pub is_const: bool,
}

/// Blueprint signature validator
pub struct SignatureValidator {
    type_mappings: HashMap<String, String>,
}

impl SignatureValidator {
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // UE5 type mappings
        type_mappings.insert("bool".to_string(), "bool".to_string());
        type_mappings.insert("int32".to_string(), "int".to_string());
        type_mappings.insert("float".to_string(), "float".to_string());
        type_mappings.insert("FString".to_string(), "string".to_string());
        type_mappings.insert("FName".to_string(), "name".to_string());
        type_mappings.insert("FVector".to_string(), "vector".to_string());
        type_mappings.insert("FRotator".to_string(), "rotator".to_string());
        type_mappings.insert("UObject*".to_string(), "object".to_string());
        type_mappings.insert("AActor*".to_string(), "actor".to_string());

        Self { type_mappings }
    }

    /// Validate Blueprint function against C++ signature
    pub fn validate_function(
        &self,
        cpp_sig: &CppSignature,
        bp_func: &FunctionMetadata,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        // Check function name
        if cpp_sig.name != bp_func.name {
            errors.push(format!(
                "Function name mismatch: C++ '{}' vs Blueprint '{}'",
                cpp_sig.name, bp_func.name
            ));
        }

        // Check return type
        if !self.types_compatible(&cpp_sig.return_type, &bp_func.return_type) {
            errors.push(format!(
                "Return type mismatch: C++ '{}' vs Blueprint '{}'",
                cpp_sig.return_type, bp_func.return_type
            ));
        }

        // Check parameter count
        if cpp_sig.parameters.len() != bp_func.parameters.len() {
            errors.push(format!(
                "Parameter count mismatch: C++ {} vs Blueprint {}",
                cpp_sig.parameters.len(),
                bp_func.parameters.len()
            ));
        } else {
            // Check each parameter
            for (i, (cpp_param, bp_param)) in cpp_sig
                .parameters
                .iter()
                .zip(bp_func.parameters.iter())
                .enumerate()
            {
                if !self.types_compatible(&cpp_param.param_type, &bp_param.param_type) {
                    errors.push(format!(
                        "Parameter {} type mismatch: C++ '{}' vs Blueprint '{}'",
                        i, cpp_param.param_type, bp_param.param_type
                    ));
                }

                // Check reference/const qualifiers
                if cpp_param.is_reference != bp_param.is_reference {
                    errors.push(format!(
                        "Parameter {} reference qualifier mismatch",
                        i
                    ));
                }
            }
        }

        if errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(errors)
        }
    }

    /// Check if types are compatible between C++ and Blueprint
    fn types_compatible(&self, cpp_type: &str, bp_type: &str) -> bool {
        // Normalize types
        let cpp_normalized = self.normalize_cpp_type(cpp_type);
        let bp_normalized = bp_type.to_lowercase();

        // Check direct match
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

    /// Normalize C++ type for comparison
    fn normalize_cpp_type(&self, cpp_type: &str) -> String {
        cpp_type
            .trim()
            .replace("const ", "")
            .replace(['&', '*'], "")
            .trim()
            .to_lowercase()
    }

    /// Validate all Blueprint functions against C++ signatures
    pub fn validate_all(
        &self,
        cpp_signatures: &[CppSignature],
        bp_functions: &[FunctionMetadata],
    ) -> HashMap<String, ValidationResult> {
        let mut results = HashMap::new();

        // Create lookup map for C++ signatures
        let cpp_map: HashMap<_, _> = cpp_signatures
            .iter()
            .map(|sig| (sig.name.clone(), sig))
            .collect();

        for bp_func in bp_functions {
            if let Some(cpp_sig) = cpp_map.get(&bp_func.name) {
                let result = self.validate_function(cpp_sig, bp_func);
                results.insert(bp_func.name.clone(), result);
            } else {
                results.insert(
                    bp_func.name.clone(),
                    ValidationResult::Invalid(vec![format!(
                        "No matching C++ function found for '{}'",
                        bp_func.name
                    )]),
                );
            }
        }

        results
    }

    /// Check if a Blueprint function can be called from C++
    pub fn is_cpp_callable(&self, bp_func: &FunctionMetadata) -> bool {
        bp_func.flags.is_blueprint_callable
    }

    /// Check if a C++ function can be overridden in Blueprint
    pub fn is_blueprint_implementable(&self, bp_func: &FunctionMetadata) -> bool {
        bp_func.flags.is_blueprint_implementable
    }
}

impl Default for SignatureValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::uasset_parser::FunctionFlags;

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::Valid;
        assert!(valid.is_valid());
        assert_eq!(valid.errors().len(), 0);

        let invalid = ValidationResult::Invalid(vec!["Error 1".to_string()]);
        assert!(!invalid.is_valid());
        assert_eq!(invalid.errors().len(), 1);
    }

    #[test]
    fn test_signature_validator_creation() {
        let validator = SignatureValidator::new();
        assert!(!validator.type_mappings.is_empty());
    }

    #[test]
    fn test_type_compatibility() {
        let validator = SignatureValidator::new();

        assert!(validator.types_compatible("bool", "bool"));
        assert!(validator.types_compatible("int32", "int"));
        assert!(validator.types_compatible("FString", "string"));
        assert!(!validator.types_compatible("int32", "float"));
    }

    #[test]
    fn test_validate_matching_function() {
        let validator = SignatureValidator::new();

        let cpp_sig = CppSignature {
            name: "OnBeginPlay".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            is_const: false,
        };

        let bp_func = FunctionMetadata {
            name: "OnBeginPlay".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            flags: FunctionFlags::default(),
        };

        let result = validator.validate_function(&cpp_sig, &bp_func);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_mismatched_function() {
        let validator = SignatureValidator::new();

        let cpp_sig = CppSignature {
            name: "Foo".to_string(),
            return_type: "int32".to_string(),
            parameters: vec![],
            is_const: false,
        };

        let bp_func = FunctionMetadata {
            name: "Foo".to_string(),
            return_type: "float".to_string(),
            parameters: vec![],
            flags: FunctionFlags::default(),
        };

        let result = validator.validate_function(&cpp_sig, &bp_func);
        assert!(!result.is_valid());
        assert!(result.errors().len() > 0);
    }

    #[test]
    fn test_validate_parameter_count_mismatch() {
        let validator = SignatureValidator::new();

        let cpp_sig = CppSignature {
            name: "Foo".to_string(),
            return_type: "void".to_string(),
            parameters: vec![CppParameter {
                name: "x".to_string(),
                param_type: "int32".to_string(),
                is_reference: false,
                is_const: false,
            }],
            is_const: false,
        };

        let bp_func = FunctionMetadata {
            name: "Foo".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            flags: FunctionFlags::default(),
        };

        let result = validator.validate_function(&cpp_sig, &bp_func);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_is_blueprint_callable() {
        let validator = SignatureValidator::new();

        let func = FunctionMetadata {
            name: "GetHealth".to_string(),
            return_type: "float".to_string(),
            parameters: vec![],
            flags: FunctionFlags {
                is_blueprint_callable: true,
                ..Default::default()
            },
        };

        assert!(validator.is_cpp_callable(&func));
    }

    #[test]
    fn test_is_blueprint_implementable() {
        let validator = SignatureValidator::new();

        let func = FunctionMetadata {
            name: "OnTakeDamage".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            flags: FunctionFlags {
                is_blueprint_implementable: true,
                ..Default::default()
            },
        };

        assert!(validator.is_blueprint_implementable(&func));
    }
}
