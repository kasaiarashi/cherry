// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 class boilerplate generation

/// UE5 boilerplate code generator
pub struct UE5Boilerplate;

impl UE5Boilerplate {
    pub fn new() -> Self {
        Self
    }

    /// Generate complete UCLASS boilerplate
    pub fn generate_uclass(&self, class_name: &str, parent_class: &str) -> String {
        format!(
            r#"#pragma once

#include "CoreMinimal.h"
#include "{}.h"
#include "{}.generated.h"

UCLASS(Blueprintable, BlueprintType)
class {}_API {} : public {}
{{
    GENERATED_BODY()

public:
    {}();

protected:
    virtual void BeginPlay() override;

public:
    virtual void Tick(float DeltaTime) override;
}};
"#,
            parent_class,
            class_name,
            class_name.to_uppercase(),
            class_name,
            parent_class,
            class_name
        )
    }

    /// Generate UPROPERTY with common specifiers
    pub fn generate_uproperty(&self, property_type: &str, property_name: &str) -> String {
        format!(
            "    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = \"{}\")\n    {} {};\n",
            class_name_from_property(property_name),
            property_type,
            property_name
        )
    }

    /// Generate UFUNCTION
    pub fn generate_ufunction(&self, function_name: &str) -> String {
        format!(
            "    UFUNCTION(BlueprintCallable, Category = \"Gameplay\")\n    void {}();\n",
            function_name
        )
    }
}

fn class_name_from_property(property: &str) -> String {
    // Simple heuristic: capitalize first letter
    let mut chars = property.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

impl Default for UE5Boilerplate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uclass() {
        let generator = UE5Boilerplate::new();
        let code = generator.generate_uclass("MyActor", "AActor");

        assert!(code.contains("UCLASS"));
        assert!(code.contains("GENERATED_BODY"));
        assert!(code.contains("MyActor"));
        assert!(code.contains("AActor"));
    }

    #[test]
    fn test_generate_uproperty() {
        let generator = UE5Boilerplate::new();
        let code = generator.generate_uproperty("float", "Health");

        assert!(code.contains("UPROPERTY"));
        assert!(code.contains("float Health"));
    }
}
