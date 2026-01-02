// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Asset reference tracking system

use crate::index::symbol::SymbolId;
use crate::util::{FileId, Span};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Kind of Unreal Engine asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    /// Blueprint class (.uasset)
    Blueprint,
    /// Material (.uasset)
    Material,
    /// Texture (.uasset)
    Texture,
    /// Static mesh (.uasset)
    StaticMesh,
    /// Skeletal mesh (.uasset)
    SkeletalMesh,
    /// Sound (.uasset)
    Sound,
    /// Particle system (.uasset)
    ParticleSystem,
    /// Animation (.uasset)
    Animation,
    /// Widget Blueprint (.uasset)
    Widget,
    /// Data table (.uasset)
    DataTable,
    /// Config file (.ini)
    Config,
    /// Other asset type
    Other,
}

/// Asset reference from C++ code
#[derive(Debug, Clone)]
pub struct AssetReference {
    /// Asset path (e.g., "/Game/Blueprints/MyBlueprint")
    pub asset_path: PathBuf,
    /// Asset kind
    pub kind: AssetKind,
    /// Location in C++ source where referenced
    pub span: Span,
    /// Source file containing the reference
    pub file_id: FileId,
    /// Symbol that references this asset (if applicable)
    pub symbol_id: Option<SymbolId>,
    /// Reference type (hard ref, soft ref, class default)
    pub is_hard_reference: bool,
}

/// Tracks asset references from C++ code
pub struct AssetTracker {
    /// Asset path -> References from C++
    asset_to_references: HashMap<PathBuf, Vec<AssetReference>>,
    /// File ID -> Assets referenced in that file
    file_to_assets: HashMap<FileId, Vec<PathBuf>>,
    /// Symbol ID -> Assets referenced by that symbol
    symbol_to_assets: HashMap<SymbolId, Vec<PathBuf>>,
    /// All known asset paths
    known_assets: HashSet<PathBuf>,
}

impl AssetTracker {
    pub fn new() -> Self {
        Self {
            asset_to_references: HashMap::new(),
            file_to_assets: HashMap::new(),
            symbol_to_assets: HashMap::new(),
            known_assets: HashSet::new(),
        }
    }

    /// Add an asset reference
    pub fn add_reference(&mut self, reference: AssetReference) {
        let asset_path = reference.asset_path.clone();
        let file_id = reference.file_id;

        // Mark asset as known
        self.known_assets.insert(asset_path.clone());

        // Add to asset -> references map
        self.asset_to_references
            .entry(asset_path.clone())
            .or_default()
            .push(reference.clone());

        // Add to file -> assets map
        self.file_to_assets
            .entry(file_id)
            .or_default()
            .push(asset_path.clone());

        // Add to symbol -> assets map
        if let Some(symbol_id) = reference.symbol_id {
            self.symbol_to_assets
                .entry(symbol_id)
                .or_default()
                .push(asset_path);
        }
    }

    /// Get all references to an asset
    pub fn get_asset_references(&self, asset_path: &PathBuf) -> Option<&Vec<AssetReference>> {
        self.asset_to_references.get(asset_path)
    }

    /// Get all assets referenced in a file
    pub fn get_assets_in_file(&self, file_id: FileId) -> Option<&Vec<PathBuf>> {
        self.file_to_assets.get(&file_id)
    }

    /// Get all assets referenced by a symbol
    pub fn get_assets_for_symbol(&self, symbol_id: SymbolId) -> Option<&Vec<PathBuf>> {
        self.symbol_to_assets.get(&symbol_id)
    }

    /// Check if an asset is referenced
    pub fn is_asset_referenced(&self, asset_path: &PathBuf) -> bool {
        self.asset_to_references.contains_key(asset_path)
    }

    /// Find unreferenced assets (assets that exist but have no references)
    pub fn find_unreferenced_assets(&self, all_assets: &HashSet<PathBuf>) -> Vec<PathBuf> {
        all_assets
            .iter()
            .filter(|path| !self.is_asset_referenced(path))
            .cloned()
            .collect()
    }

    /// Find missing assets (referenced but don't exist)
    pub fn find_missing_assets(&self, existing_assets: &HashSet<PathBuf>) -> Vec<PathBuf> {
        self.known_assets
            .iter()
            .filter(|path| !existing_assets.contains(*path))
            .cloned()
            .collect()
    }

    /// Get hard references (will cause loading)
    pub fn get_hard_references(&self, asset_path: &PathBuf) -> Vec<&AssetReference> {
        if let Some(refs) = self.asset_to_references.get(asset_path) {
            refs.iter().filter(|r| r.is_hard_reference).collect()
        } else {
            Vec::new()
        }
    }

    /// Get soft references (won't cause loading)
    pub fn get_soft_references(&self, asset_path: &PathBuf) -> Vec<&AssetReference> {
        if let Some(refs) = self.asset_to_references.get(asset_path) {
            refs.iter().filter(|r| !r.is_hard_reference).collect()
        } else {
            Vec::new()
        }
    }

    /// Get all assets of a specific kind
    pub fn get_assets_by_kind(&self, kind: AssetKind) -> Vec<PathBuf> {
        self.asset_to_references
            .iter()
            .filter(|(_, refs)| refs.iter().any(|r| r.kind == kind))
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Get asset dependency chain (transitive dependencies)
    pub fn get_dependency_chain(&self, asset_path: &PathBuf) -> Vec<PathBuf> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        self.collect_dependencies(asset_path, &mut visited, &mut chain);
        chain
    }

    fn collect_dependencies(
        &self,
        asset_path: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        chain: &mut Vec<PathBuf>,
    ) {
        if visited.contains(asset_path) {
            return;
        }
        visited.insert(asset_path.clone());

        if let Some(refs) = self.asset_to_references.get(asset_path) {
            for reference in refs {
                // In a real implementation, would parse the asset to find its dependencies
                // For now, just track the reference
                chain.push(reference.asset_path.clone());
            }
        }
    }

    /// Clear all tracked references
    pub fn clear(&mut self) {
        self.asset_to_references.clear();
        self.file_to_assets.clear();
        self.symbol_to_assets.clear();
        self.known_assets.clear();
    }

    /// Get statistics about asset references
    pub fn get_statistics(&self) -> AssetStatistics {
        let total_assets = self.known_assets.len();
        let total_references = self.asset_to_references.values()
            .map(|refs| refs.len())
            .sum();

        let hard_refs = self.asset_to_references.values()
            .flatten()
            .filter(|r| r.is_hard_reference)
            .count();

        let soft_refs = total_references - hard_refs;

        AssetStatistics {
            total_assets,
            total_references,
            hard_references: hard_refs,
            soft_references: soft_refs,
        }
    }
}

/// Asset reference statistics
#[derive(Debug, Clone)]
pub struct AssetStatistics {
    pub total_assets: usize,
    pub total_references: usize,
    pub hard_references: usize,
    pub soft_references: usize,
}

impl Default for AssetTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{FileId, Span};

    #[test]
    fn test_add_asset_reference() {
        let mut tracker = AssetTracker::new();

        let file_id = FileId::new(1);
        let asset_path = PathBuf::from("/Game/Blueprints/MyBlueprint");

        let reference = AssetReference {
            asset_path: asset_path.clone(),
            kind: AssetKind::Blueprint,
            span: Span::new(file_id, 0, 10),
            file_id,
            symbol_id: None,
            is_hard_reference: true,
        };

        tracker.add_reference(reference);

        assert!(tracker.is_asset_referenced(&asset_path));
        let refs = tracker.get_asset_references(&asset_path);
        assert!(refs.is_some());
        assert_eq!(refs.unwrap().len(), 1);
    }

    #[test]
    fn test_hard_soft_references() {
        let mut tracker = AssetTracker::new();

        let file_id = FileId::new(1);
        let asset_path = PathBuf::from("/Game/Materials/MyMaterial");

        tracker.add_reference(AssetReference {
            asset_path: asset_path.clone(),
            kind: AssetKind::Material,
            span: Span::new(file_id, 0, 10),
            file_id,
            symbol_id: None,
            is_hard_reference: true,
        });

        tracker.add_reference(AssetReference {
            asset_path: asset_path.clone(),
            kind: AssetKind::Material,
            span: Span::new(file_id, 20, 30),
            file_id,
            symbol_id: None,
            is_hard_reference: false,
        });

        let hard_refs = tracker.get_hard_references(&asset_path);
        let soft_refs = tracker.get_soft_references(&asset_path);

        assert_eq!(hard_refs.len(), 1);
        assert_eq!(soft_refs.len(), 1);
    }

    #[test]
    fn test_find_unreferenced_assets() {
        let mut tracker = AssetTracker::new();

        let file_id = FileId::new(1);
        let asset1 = PathBuf::from("/Game/Asset1");
        let asset2 = PathBuf::from("/Game/Asset2");
        let asset3 = PathBuf::from("/Game/Asset3");

        tracker.add_reference(AssetReference {
            asset_path: asset1.clone(),
            kind: AssetKind::Blueprint,
            span: Span::new(file_id, 0, 10),
            file_id,
            symbol_id: None,
            is_hard_reference: true,
        });

        let mut all_assets = HashSet::new();
        all_assets.insert(asset1);
        all_assets.insert(asset2.clone());
        all_assets.insert(asset3.clone());

        let unreferenced = tracker.find_unreferenced_assets(&all_assets);
        assert_eq!(unreferenced.len(), 2);
        assert!(unreferenced.contains(&asset2));
        assert!(unreferenced.contains(&asset3));
    }

    #[test]
    fn test_assets_by_kind() {
        let mut tracker = AssetTracker::new();

        let file_id = FileId::new(1);

        tracker.add_reference(AssetReference {
            asset_path: PathBuf::from("/Game/BP1"),
            kind: AssetKind::Blueprint,
            span: Span::new(file_id, 0, 10),
            file_id,
            symbol_id: None,
            is_hard_reference: true,
        });

        tracker.add_reference(AssetReference {
            asset_path: PathBuf::from("/Game/Mat1"),
            kind: AssetKind::Material,
            span: Span::new(file_id, 10, 20),
            file_id,
            symbol_id: None,
            is_hard_reference: true,
        });

        let blueprints = tracker.get_assets_by_kind(AssetKind::Blueprint);
        assert_eq!(blueprints.len(), 1);

        let materials = tracker.get_assets_by_kind(AssetKind::Material);
        assert_eq!(materials.len(), 1);
    }
}
