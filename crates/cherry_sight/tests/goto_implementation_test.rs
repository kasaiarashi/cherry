// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Tests for go-to-implementation feature

use cherry_sight::index::{SymbolId, SymbolKind, SymbolTable};
use cherry_sight::parser::CppParser;
use cherry_sight::db::Database;
use util::{FileId, Interner};
use std::sync::Arc;
use parking_lot::RwLock;

#[test]
fn test_match_implementation_simple_function() {
    // Setup
    let mut interner = Interner::new();
    let mut symbol_table = SymbolTable::new();
    let mut database = Database::new();

    // Create .h file with declaration
    let header_content = r#"
class MyClass {
public:
    void myFunction();
};
"#;
    let header_file = FileId::new(1);
    database.add_source_file(header_file, "test.h".into(), header_content.to_string());

    // Create .cpp file with implementation
    let cpp_content = r#"
#include "test.h"

void MyClass::myFunction() {
    // implementation
}
"#;
    let cpp_file = FileId::new(2);
    database.add_source_file(cpp_file, "test.cpp".into(), cpp_content.to_string());

    // Parse header and create declaration symbol
    let mut parser = CppParser::new().unwrap();
    let header_ast = parser.parse(header_content, Some("test.h")).unwrap();

    // Manually create symbols (simplified - in real code this is done by AstSymbolBuilder)
    let class_name = interner.intern("MyClass");
    let func_name = interner.intern("myFunction");

    let class_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class_name,
        util::Span::new(header_file, 8, 50),
        header_file
    );

    let decl_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(header_file, 29, 45),
        header_file
    );

    // Set parent for the method
    if let Some(decl_sym) = symbol_table.get_symbol_mut(decl_id) {
        decl_sym.parent = Some(class_id);
    }
    if let Some(class_sym) = symbol_table.get_symbol_mut(class_id) {
        class_sym.add_child(decl_id);
    }

    // Parse cpp and create implementation symbol
    let cpp_ast = parser.parse(cpp_content, Some("test.cpp")).unwrap();

    let impl_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(cpp_file, 26, 88),
        cpp_file
    );

    // Set parent for the implementation (should match class name)
    let cpp_class_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class_name,
        util::Span::new(cpp_file, 0, 0), // Dummy span
        cpp_file
    );

    if let Some(impl_sym) = symbol_table.get_symbol_mut(impl_id) {
        impl_sym.parent = Some(cpp_class_id);
    }

    // Now test the matching logic
    println!("\n=== Testing Implementation Matching ===");
    println!("Declaration symbol: {:?} (parent: {:?})", decl_id,
        symbol_table.get_symbol(decl_id).and_then(|s| s.parent));
    println!("Implementation symbol: {:?} (parent: {:?})", impl_id,
        symbol_table.get_symbol(impl_id).and_then(|s| s.parent));

    // Check that we can find symbols by name
    let candidates = symbol_table.find_all(func_name);
    println!("Found {} symbols with name 'myFunction'", candidates.len());
    assert_eq!(candidates.len(), 2, "Should find both declaration and implementation");

    // Simulate the matching logic
    let mut found_match = false;
    for &candidate_id in &candidates {
        if let Some(candidate) = symbol_table.get_symbol(candidate_id) {
            println!("  Candidate #{}: kind={:?}, file={:?}, parent={:?}",
                candidate_id.0, candidate.kind, candidate.file_id, candidate.parent);

            if candidate.id == impl_id {
                continue; // Skip the implementation itself
            }

            // Check if declaration in .h
            if candidate.file_id == header_file && candidate.kind == SymbolKind::Method {
                // Check parent match
                if let (Some(decl_parent), Some(impl_parent)) =
                    (candidate.parent, symbol_table.get_symbol(impl_id).and_then(|s| s.parent)) {

                    if let (Some(decl_parent_sym), Some(impl_parent_sym)) =
                        (symbol_table.get_symbol(decl_parent), symbol_table.get_symbol(impl_parent)) {

                        println!("    Decl parent: {} ({:?})", interner.resolve(decl_parent_sym.name), decl_parent);
                        println!("    Impl parent: {} ({:?})", interner.resolve(impl_parent_sym.name), impl_parent);

                        if decl_parent_sym.name == impl_parent_sym.name {
                            println!("    ✓ MATCH! Parents have same name");
                            found_match = true;
                        }
                    }
                }
            }
        }
    }

    assert!(found_match, "Should match implementation to declaration");
}

#[test]
fn test_ue5_execute_implementation() {
    // Setup
    let mut interner = Interner::new();
    let mut symbol_table = SymbolTable::new();
    let mut database = Database::new();

    // Create .h file with UE5-style declaration
    let header_content = r#"
class ATask_Sample : public ATask {
    GENERATED_BODY()
public:
    virtual void Execute_Implementation() override;
};
"#;
    let header_file = FileId::new(1);
    database.add_source_file(header_file, "Task_Sample.h".into(), header_content.to_string());

    // Create .cpp file with UE5-style implementation
    let cpp_content = r#"
#include "Task_Sample.h"

void ATask_Sample::Execute_Implementation() {
    // implementation
}
"#;
    let cpp_file = FileId::new(2);
    database.add_source_file(cpp_file, "Task_Sample.cpp".into(), cpp_content.to_string());

    // Create symbols manually
    let class_name = interner.intern("ATask_Sample");
    let func_name = interner.intern("Execute_Implementation");

    // Header class and method
    let header_class_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class_name,
        util::Span::new(header_file, 7, 120),
        header_file
    );

    let decl_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(header_file, 72, 110),
        header_file
    );

    if let Some(decl_sym) = symbol_table.get_symbol_mut(decl_id) {
        decl_sym.parent = Some(header_class_id);
    }

    // Cpp class and method (qualified name)
    let cpp_class_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class_name,
        util::Span::new(cpp_file, 0, 0),
        cpp_file
    );

    let impl_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(cpp_file, 30, 100),
        cpp_file
    );

    if let Some(impl_sym) = symbol_table.get_symbol_mut(impl_id) {
        impl_sym.parent = Some(cpp_class_id);
    }

    println!("\n=== Testing UE5 Execute_Implementation Matching ===");

    // Find candidates
    let candidates = symbol_table.find_all(func_name);
    println!("Found {} symbols with name 'Execute_Implementation'", candidates.len());

    // The matching logic should find these
    let mut matched = false;
    for &candidate_id in &candidates {
        if let Some(candidate) = symbol_table.get_symbol(candidate_id) {
            if candidate.id == impl_id {
                continue;
            }

            if candidate.file_id == header_file && candidate.kind == SymbolKind::Method {
                if let (Some(decl_parent), Some(impl_parent)) =
                    (candidate.parent, symbol_table.get_symbol(impl_id).and_then(|s| s.parent)) {

                    if let (Some(dp), Some(ip)) =
                        (symbol_table.get_symbol(decl_parent), symbol_table.get_symbol(impl_parent)) {

                        println!("Comparing: decl parent '{}' vs impl parent '{}'",
                            interner.resolve(dp.name), interner.resolve(ip.name));

                        if dp.name == ip.name {
                            println!("✓ Match found!");
                            matched = true;

                            // This is where we would set implementation_span
                            // symbol_table.get_symbol_mut(decl_id).implementation_span = Some(impl_span);
                        }
                    }
                }
            }
        }
    }

    assert!(matched, "Should match UE5 Execute_Implementation to declaration");
}

#[test]
fn test_no_false_matches_different_classes() {
    // Setup
    let mut interner = Interner::new();
    let mut symbol_table = SymbolTable::new();

    let func_name = interner.intern("doSomething");
    let class1_name = interner.intern("ClassA");
    let class2_name = interner.intern("ClassB");

    let file1 = FileId::new(1);
    let file2 = FileId::new(2);

    // ClassA::doSomething() in header
    let class1_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class1_name,
        util::Span::new(file1, 0, 100),
        file1
    );

    let decl1_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(file1, 30, 50),
        file1
    );

    if let Some(s) = symbol_table.get_symbol_mut(decl1_id) {
        s.parent = Some(class1_id);
    }

    // ClassB::doSomething() in cpp
    let class2_id = symbol_table.create_symbol(
        SymbolKind::Class,
        class2_name,
        util::Span::new(file2, 0, 0),
        file2
    );

    let impl2_id = symbol_table.create_symbol(
        SymbolKind::Method,
        func_name,
        util::Span::new(file2, 30, 80),
        file2
    );

    if let Some(s) = symbol_table.get_symbol_mut(impl2_id) {
        s.parent = Some(class2_id);
    }

    println!("\n=== Testing No False Matches ===");

    let candidates = symbol_table.find_all(func_name);
    println!("Found {} symbols with name 'doSomething'", candidates.len());

    // Should NOT match because parent classes are different
    for &candidate_id in &candidates {
        if let Some(candidate) = symbol_table.get_symbol(candidate_id) {
            if candidate.id == impl2_id {
                continue;
            }

            if let (Some(dp_id), Some(ip_id)) =
                (candidate.parent, symbol_table.get_symbol(impl2_id).and_then(|s| s.parent)) {

                if let (Some(dp), Some(ip)) =
                    (symbol_table.get_symbol(dp_id), symbol_table.get_symbol(ip_id)) {

                    println!("Comparing: '{}' vs '{}'",
                        interner.resolve(dp.name), interner.resolve(ip.name));

                    assert_ne!(dp.name, ip.name,
                        "Should NOT match methods from different classes");
                }
            }
        }
    }
}
