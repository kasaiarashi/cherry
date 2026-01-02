// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Blueprint cross-referencing system

use crate::index::symbol::SymbolId;
use crate::util::{FileId, Span};
use std::collections::HashMap;
use std::path::PathBuf;

/// Kind of Blueprint reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlueprintRefKind {
    /// BlueprintCallable function
    Callable,
    /// BlueprintPure function
    Pure,
    /// BlueprintImplementableEvent
    ImplementableEvent,
    /// BlueprintNativeEvent
    NativeEvent,
    /// BlueprintReadOnly property
    ReadOnlyProperty,
    /// BlueprintReadWrite property
    ReadWriteProperty,
    /// Blueprint class inheritance
    Inheritance,
    /// Blueprint component reference
    Component,
}

/// Blueprint reference information
#[derive(Debug, Clone)]
pub struct BlueprintReference {
    /// Symbol being referenced from Blueprint
    pub symbol_id: SymbolId,
    /// Kind of Blueprint reference
    pub kind: BlueprintRefKind,
    /// Location in C++ source
    pub span: Span,
    /// C++ source file
    pub file_id: FileId,
    /// Blueprint asset path (if known)
    pub blueprint_path: Option<PathBuf>,
    /// Blueprint asset name
    pub blueprint_name: Option<String>,
}

/// Tracks Blueprint-to-C++ cross-references
pub struct BlueprintRefTracker {
    /// Symbol ID -> Blueprint references
    symbol_to_blueprints: HashMap<SymbolId, Vec<BlueprintReference>>,
    /// Blueprint path -> Referenced symbols
    blueprint_to_symbols: HashMap<PathBuf, Vec<SymbolId>>,
    /// Implementable events that need Blueprint implementation
    implementable_events: Vec<BlueprintReference>,
    /// Native events with C++ base implementation
    native_events: Vec<BlueprintReference>,
}

impl BlueprintRefTracker {
    pub fn new() -> Self {
        Self {
            symbol_to_blueprints: HashMap::new(),
            blueprint_to_symbols: HashMap::new(),
            implementable_events: Vec::new(),
            native_events: Vec::new(),
        }
    }

    /// Add a Blueprint reference to a C++ symbol
    pub fn add_reference(&mut self, reference: BlueprintReference) {
        let symbol_id = reference.symbol_id;

        // Track implementable/native events separately
        match reference.kind {
            BlueprintRefKind::ImplementableEvent => {
                self.implementable_events.push(reference.clone());
            }
            BlueprintRefKind::NativeEvent => {
                self.native_events.push(reference.clone());
            }
            _ => {}
        }

        // Add to symbol -> blueprints map
        self.symbol_to_blueprints
            .entry(symbol_id)
            .or_default()
            .push(reference.clone());

        // Add to blueprint -> symbols map
        if let Some(bp_path) = &reference.blueprint_path {
            self.blueprint_to_symbols
                .entry(bp_path.clone())
                .or_default()
                .push(symbol_id);
        }
    }

    /// Get all Blueprint references for a C++ symbol
    pub fn get_blueprint_references(&self, symbol_id: SymbolId) -> Option<&Vec<BlueprintReference>> {
        self.symbol_to_blueprints.get(&symbol_id)
    }

    /// Get all C++ symbols referenced by a Blueprint
    pub fn get_symbols_in_blueprint(&self, blueprint_path: &PathBuf) -> Option<&Vec<SymbolId>> {
        self.blueprint_to_symbols.get(blueprint_path)
    }

    /// Get all implementable events (need Blueprint implementation)
    pub fn get_implementable_events(&self) -> &Vec<BlueprintReference> {
        &self.implementable_events
    }

    /// Get all native events (have C++ implementation)
    pub fn get_native_events(&self) -> &Vec<BlueprintReference> {
        &self.native_events
    }

    /// Find orphaned Blueprint references (Blueprint exists but C++ symbol removed)
    pub fn find_orphaned_references(&self, valid_symbols: &std::collections::HashSet<SymbolId>) -> Vec<BlueprintReference> {
        let mut orphaned = Vec::new();

        for (symbol_id, refs) in &self.symbol_to_blueprints {
            if !valid_symbols.contains(symbol_id) {
                orphaned.extend(refs.iter().cloned());
            }
        }

        orphaned
    }

    /// Check if a function is callable from Blueprint
    pub fn is_blueprint_callable(&self, symbol_id: SymbolId) -> bool {
        if let Some(refs) = self.symbol_to_blueprints.get(&symbol_id) {
            refs.iter().any(|r| matches!(
                r.kind,
                BlueprintRefKind::Callable | BlueprintRefKind::Pure
            ))
        } else {
            false
        }
    }

    /// Check if a property is Blueprint-accessible
    pub fn is_blueprint_accessible(&self, symbol_id: SymbolId) -> bool {
        if let Some(refs) = self.symbol_to_blueprints.get(&symbol_id) {
            refs.iter().any(|r| matches!(
                r.kind,
                BlueprintRefKind::ReadOnlyProperty | BlueprintRefKind::ReadWriteProperty
            ))
        } else {
            false
        }
    }

    /// Get Blueprint references by kind
    pub fn get_references_by_kind(&self, kind: BlueprintRefKind) -> Vec<&BlueprintReference> {
        self.symbol_to_blueprints
            .values()
            .flatten()
            .filter(|r| r.kind == kind)
            .collect()
    }

    /// Clear all tracked references
    pub fn clear(&mut self) {
        self.symbol_to_blueprints.clear();
        self.blueprint_to_symbols.clear();
        self.implementable_events.clear();
        self.native_events.clear();
    }
}

impl Default for BlueprintRefTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::SymbolId;
    use crate::util::{FileId, Span};

    #[test]
    fn test_add_blueprint_reference() {
        let mut tracker = BlueprintRefTracker::new();

        let file_id = FileId::new(1);
        let symbol_id = SymbolId::new(100);

        let reference = BlueprintReference {
            symbol_id,
            kind: BlueprintRefKind::Callable,
            span: Span::new(file_id, 0, 10),
            file_id,
            blueprint_path: Some(PathBuf::from("/Game/Blueprints/MyBlueprint")),
            blueprint_name: Some("MyBlueprint".to_string()),
        };

        tracker.add_reference(reference);

        let refs = tracker.get_blueprint_references(symbol_id);
        assert!(refs.is_some());
        assert_eq!(refs.unwrap().len(), 1);
    }

    #[test]
    fn test_implementable_events() {
        let mut tracker = BlueprintRefTracker::new();

        let file_id = FileId::new(1);
        let symbol_id = SymbolId::new(100);

        let event = BlueprintReference {
            symbol_id,
            kind: BlueprintRefKind::ImplementableEvent,
            span: Span::new(file_id, 0, 10),
            file_id,
            blueprint_path: None,
            blueprint_name: Some("OnCustomEvent".to_string()),
        };

        tracker.add_reference(event);

        let events = tracker.get_implementable_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, BlueprintRefKind::ImplementableEvent);
    }

    #[test]
    fn test_blueprint_callable_check() {
        let mut tracker = BlueprintRefTracker::new();

        let file_id = FileId::new(1);
        let symbol_id = SymbolId::new(100);

        let reference = BlueprintReference {
            symbol_id,
            kind: BlueprintRefKind::Callable,
            span: Span::new(file_id, 0, 10),
            file_id,
            blueprint_path: None,
            blueprint_name: None,
        };

        tracker.add_reference(reference);

        assert!(tracker.is_blueprint_callable(symbol_id));
        assert!(!tracker.is_blueprint_callable(SymbolId::new(999)));
    }

    #[test]
    fn test_orphaned_references() {
        let mut tracker = BlueprintRefTracker::new();

        let file_id = FileId::new(1);
        let symbol_id_1 = SymbolId::new(100);
        let symbol_id_2 = SymbolId::new(200);

        tracker.add_reference(BlueprintReference {
            symbol_id: symbol_id_1,
            kind: BlueprintRefKind::Callable,
            span: Span::new(file_id, 0, 10),
            file_id,
            blueprint_path: None,
            blueprint_name: None,
        });

        tracker.add_reference(BlueprintReference {
            symbol_id: symbol_id_2,
            kind: BlueprintRefKind::Callable,
            span: Span::new(file_id, 10, 20),
            file_id,
            blueprint_path: None,
            blueprint_name: None,
        });

        let mut valid_symbols = std::collections::HashSet::new();
        valid_symbols.insert(symbol_id_1);

        let orphaned = tracker.find_orphaned_references(&valid_symbols);
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0].symbol_id, symbol_id_2);
    }
}
