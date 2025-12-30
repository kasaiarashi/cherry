use cherry_link::{
    ParsedSolution, ParsedVcxproj, SlnProject, VcxprojFileType, VsProjectType,
    parse_csproj, parse_solution, parse_vcxproj,
};
use collections::{HashMap, HashSet};
use gpui::SharedString;
use std::path::{Path, PathBuf};
use ui::IconName;

/// View mode for the project panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectPanelViewMode {
    #[default]
    FileTree,
    SolutionView,
}

/// Unique identifier for a solution entry
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SolutionEntryId {
    Solution(PathBuf),
    Folder(String),       // GUID
    Project(String),      // GUID
    SourceFile(PathBuf),  // Full path
}

/// A visible entry in the solution view
#[derive(Debug, Clone)]
pub enum SolutionEntry {
    Solution {
        name: String,
        path: PathBuf,
    },
    Folder {
        name: String,
        guid: String,
        depth: usize,
    },
    Project {
        name: String,
        path: PathBuf,
        guid: String,
        project_type: VsProjectType,
        depth: usize,
    },
    SourceFile {
        name: String,
        path: PathBuf,
        file_type: VcxprojFileType,
        depth: usize,
    },
    Directory {
        name: String,
        path: PathBuf,
        depth: usize,
    },
}

impl SolutionEntry {
    pub fn id(&self) -> SolutionEntryId {
        match self {
            Self::Solution { path, .. } => SolutionEntryId::Solution(path.clone()),
            Self::Folder { guid, .. } => SolutionEntryId::Folder(guid.clone()),
            Self::Project { guid, .. } => SolutionEntryId::Project(guid.clone()),
            Self::SourceFile { path, .. } => SolutionEntryId::SourceFile(path.clone()),
            Self::Directory { path, .. } => SolutionEntryId::SourceFile(path.clone()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Solution { name, .. } => name,
            Self::Folder { name, .. } => name,
            Self::Project { name, .. } => name,
            Self::SourceFile { name, .. } => name,
            Self::Directory { name, .. } => name,
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Solution { .. } => 0,
            Self::Folder { depth, .. } => *depth,
            Self::Project { depth, .. } => *depth,
            Self::SourceFile { depth, .. } => *depth,
            Self::Directory { depth, .. } => *depth,
        }
    }

    pub fn is_expandable(&self) -> bool {
        !matches!(self, Self::SourceFile { .. })
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Solution { .. } => IconName::FileTree,
            Self::Folder { .. } => IconName::Folder,
            Self::Project { project_type, .. } => match project_type {
                VsProjectType::CppProject => IconName::FileCode,
                VsProjectType::CSharpProject => IconName::FileCode,
                _ => IconName::FileCode,
            },
            Self::SourceFile { file_type: _, name, .. } => {
                // Use file extension to determine icon
                let ext = Path::new(name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                match ext {
                    "cpp" | "c" | "cc" | "cxx" => IconName::FileCode,
                    "h" | "hpp" | "hxx" => IconName::FileCode,
                    "cs" => IconName::FileCode,
                    _ => IconName::File,
                }
            }
            Self::Directory { .. } => IconName::Folder,
        }
    }

    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Self::SourceFile { path, .. } => Some(path),
            _ => None,
        }
    }
}

/// Details for rendering a solution entry
#[derive(Debug, Clone)]
pub struct SolutionEntryDetails {
    pub id: SolutionEntryId,
    pub name: SharedString,
    pub icon: IconName,
    pub depth: usize,
    pub is_expanded: bool,
    pub is_selected: bool,
    pub is_expandable: bool,
}

/// State for solution view mode
#[derive(Default)]
pub struct SolutionViewState {
    pub solution_path: Option<PathBuf>,
    pub parsed_solution: Option<ParsedSolution>,
    pub parsed_projects: HashMap<PathBuf, ParsedVcxproj>,
    pub visible_entries: Vec<SolutionEntry>,
    pub expanded_ids: HashSet<SolutionEntryId>,
    pub selection: Option<SolutionEntryId>,
}

impl SolutionViewState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a solution file
    pub fn load_solution(&mut self, path: &Path) -> anyhow::Result<()> {
        let solution = parse_solution(path)?;
        self.solution_path = Some(path.to_path_buf());
        self.parsed_solution = Some(solution);
        self.expanded_ids.clear();
        self.parsed_projects.clear();

        // Auto-expand solution root
        self.expanded_ids
            .insert(SolutionEntryId::Solution(path.to_path_buf()));

        Ok(())
    }

    /// Check if an entry is expanded
    pub fn is_expanded(&self, id: &SolutionEntryId) -> bool {
        self.expanded_ids.contains(id)
    }

    /// Toggle expansion state of an entry
    pub fn toggle_expanded(&mut self, id: &SolutionEntryId) {
        if self.expanded_ids.contains(id) {
            self.expanded_ids.remove(id);
        } else {
            self.expanded_ids.insert(id.clone());
        }
    }

    /// Expand an entry
    pub fn expand(&mut self, id: &SolutionEntryId) {
        self.expanded_ids.insert(id.clone());
    }

    /// Collapse an entry
    pub fn collapse(&mut self, id: &SolutionEntryId) {
        self.expanded_ids.remove(id);
    }

    /// Load a project file if not already loaded
    pub fn ensure_project_loaded(&mut self, project: &SlnProject, solution_dir: &Path) {
        let project_path = solution_dir.join(&project.relative_path);

        if self.parsed_projects.contains_key(&project_path) {
            return;
        }

        let result = match project.project_type {
            VsProjectType::CppProject => parse_vcxproj(&project_path),
            VsProjectType::CSharpProject => parse_csproj(&project_path),
            _ => return,
        };

        if let Ok(parsed) = result {
            self.parsed_projects.insert(project_path, parsed);
        }
    }

    /// Update visible entries based on current expansion state
    pub fn update_visible_entries(&mut self) {
        let mut entries = Vec::new();

        let Some(solution) = self.parsed_solution.clone() else {
            self.visible_entries = entries;
            return;
        };

        let solution_path = self.solution_path.clone().unwrap_or_default();
        let solution_dir = solution_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        // Add solution root
        entries.push(SolutionEntry::Solution {
            name: solution.name.clone(),
            path: solution_path.clone(),
        });

        let solution_expanded = self.is_expanded(&SolutionEntryId::Solution(solution_path.clone()));

        if solution_expanded {
            // Add root-level projects/folders
            self.add_children_entries(
                &mut entries,
                &solution,
                None, // No parent = root level
                1,    // depth
                &solution_dir,
            );
        }

        self.visible_entries = entries;
    }

    fn add_children_entries(
        &mut self,
        entries: &mut Vec<SolutionEntry>,
        solution: &ParsedSolution,
        parent_guid: Option<&str>,
        depth: usize,
        solution_dir: &Path,
    ) {
        // Get children: either root items (parent_guid=None) or children of parent
        let children: Vec<_> = if let Some(parent) = parent_guid {
            solution.children_of(parent).collect()
        } else {
            solution.root_projects().collect()
        };

        // Sort: folders first, then projects, alphabetically within each group
        let mut sorted_children = children;
        sorted_children.sort_by(|a, b| {
            let a_is_folder = a.project_type.is_folder();
            let b_is_folder = b.project_type.is_folder();
            match (a_is_folder, b_is_folder) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for project in sorted_children {
            if project.project_type.is_folder() {
                // Solution folder
                let folder_id = SolutionEntryId::Folder(project.guid.clone());
                let is_expanded = self.is_expanded(&folder_id);

                entries.push(SolutionEntry::Folder {
                    name: project.name.clone(),
                    guid: project.guid.clone(),
                    depth,
                });

                if is_expanded {
                    self.add_children_entries(
                        entries,
                        solution,
                        Some(&project.guid),
                        depth + 1,
                        solution_dir,
                    );
                }
            } else if project.project_type.is_project() {
                // Actual project (.vcxproj or .csproj)
                let project_id = SolutionEntryId::Project(project.guid.clone());
                let is_expanded = self.is_expanded(&project_id);

                entries.push(SolutionEntry::Project {
                    name: project.name.clone(),
                    path: solution_dir.join(&project.relative_path),
                    guid: project.guid.clone(),
                    project_type: project.project_type,
                    depth,
                });

                if is_expanded {
                    // Load project file if needed
                    self.ensure_project_loaded(project, solution_dir);

                    let project_path = solution_dir.join(&project.relative_path);
                    if let Some(parsed_project) = self.parsed_projects.get(&project_path) {
                        self.add_project_files(entries, parsed_project, depth + 1, &project_path);
                    }
                }
            }
        }
    }

    fn add_project_files(
        &self,
        entries: &mut Vec<SolutionEntry>,
        project: &ParsedVcxproj,
        base_depth: usize,
        project_path: &Path,
    ) {
        let project_dir = project_path.parent().unwrap_or(Path::new("."));
        let tree = project.directory_tree();

        // Add files from the directory tree
        self.add_directory_node_entries(entries, &tree, base_depth, project_dir);
    }

    fn add_directory_node_entries(
        &self,
        entries: &mut Vec<SolutionEntry>,
        node: &cherry_link::DirectoryNode,
        depth: usize,
        project_dir: &Path,
    ) {
        // Sort children: directories first, then files
        let mut sorted_children = node.children.clone();
        sorted_children.sort_by(|a, b| a.name.cmp(&b.name));

        let mut sorted_files = node.files.clone();
        sorted_files.sort_by(|a, b| {
            let a_name = a.file_name().unwrap_or("");
            let b_name = b.file_name().unwrap_or("");
            a_name.cmp(b_name)
        });

        // Add directories first
        for child in &sorted_children {
            let dir_path = project_dir.join(&child.path);
            let dir_id = SolutionEntryId::SourceFile(dir_path.clone());
            let is_expanded = self.is_expanded(&dir_id);

            entries.push(SolutionEntry::Directory {
                name: child.name.clone(),
                path: dir_path,
                depth,
            });

            if is_expanded {
                self.add_directory_node_entries(entries, child, depth + 1, project_dir);
            }
        }

        // Add files
        for file in &sorted_files {
            let file_path = project_dir.join(&file.relative_path);
            let file_name = file.file_name().unwrap_or("").to_string();

            entries.push(SolutionEntry::SourceFile {
                name: file_name,
                path: file_path,
                file_type: file.file_type,
                depth,
            });
        }
    }

    /// Get details for rendering an entry
    pub fn details_for_entry(&self, entry: &SolutionEntry) -> SolutionEntryDetails {
        let id = entry.id();
        let is_expanded = self.is_expanded(&id);
        let is_selected = self.selection.as_ref() == Some(&id);

        SolutionEntryDetails {
            id,
            name: entry.name().to_string().into(),
            icon: entry.icon(),
            depth: entry.depth(),
            is_expanded,
            is_selected,
            is_expandable: entry.is_expandable(),
        }
    }

    /// Get build configurations from the loaded solution
    pub fn build_configurations(&self) -> Vec<String> {
        self.parsed_solution
            .as_ref()
            .map(|s| s.unique_configurations())
            .unwrap_or_default()
    }

    /// Get platforms from the loaded solution
    pub fn platforms(&self) -> Vec<String> {
        self.parsed_solution
            .as_ref()
            .map(|s| s.unique_platforms())
            .unwrap_or_default()
    }

    /// Get the detected engine path
    pub fn engine_path(&self) -> Option<PathBuf> {
        self.parsed_solution.as_ref().and_then(|s| s.detect_engine_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solution_entry_id_eq() {
        let id1 = SolutionEntryId::Folder("abc".to_string());
        let id2 = SolutionEntryId::Folder("abc".to_string());
        let id3 = SolutionEntryId::Folder("def".to_string());

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_solution_view_state_expansion() {
        let mut state = SolutionViewState::new();
        let id = SolutionEntryId::Folder("test".to_string());

        assert!(!state.is_expanded(&id));

        state.expand(&id);
        assert!(state.is_expanded(&id));

        state.collapse(&id);
        assert!(!state.is_expanded(&id));

        state.toggle_expanded(&id);
        assert!(state.is_expanded(&id));

        state.toggle_expanded(&id);
        assert!(!state.is_expanded(&id));
    }
}
