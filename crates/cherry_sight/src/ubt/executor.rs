// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UBT command execution

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};

/// Build result from UBT
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u128,
}

/// UBT command types
#[derive(Debug, Clone)]
pub enum UBTCommand {
    /// Build a target
    Build {
        target: String,
        platform: String,
        configuration: String,
    },
    /// Clean build artifacts
    Clean {
        target: String,
    },
    /// Generate project files
    GenerateProjectFiles,
    /// Get module dependencies
    GetModuleDependencies {
        module: String,
    },
}

/// Executes UnrealBuildTool commands
pub struct UBTExecutor {
    /// Path to UBT executable
    ubt_path: PathBuf,
    /// Project file path (.uproject)
    project_path: PathBuf,
    /// Engine root path
    #[allow(dead_code)]
    engine_path: PathBuf,
}

impl UBTExecutor {
    /// Create a new UBT executor
    pub fn new(engine_path: PathBuf, project_path: PathBuf) -> Self {
        let ubt_path = engine_path
            .join("Engine")
            .join("Binaries")
            .join("DotNET")
            .join("UnrealBuildTool")
            .join("UnrealBuildTool.exe");

        Self {
            ubt_path,
            project_path,
            engine_path,
        }
    }

    /// Execute a UBT command
    #[allow(clippy::disallowed_methods)] // UBT commands are expected to be long-running
    pub fn execute(&self, command: &UBTCommand) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        let mut cmd = Command::new(&self.ubt_path);
        self.configure_command(&mut cmd, command)?;

        let output = cmd
            .output()
            .with_context(|| format!("Failed to execute UBT command: {:?}", command))?;

        let duration_ms = start.elapsed().as_millis();

        Ok(BuildResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    /// Execute a UBT command asynchronously
    pub async fn execute_async(&self, command: &UBTCommand) -> Result<BuildResult> {
        // Would use tokio::process::Command for true async
        // For now, just wrap sync execution
        self.execute(command)
    }

    /// Configure command arguments based on UBT command type
    fn configure_command(&self, cmd: &mut Command, ubt_command: &UBTCommand) -> Result<()> {
        match ubt_command {
            UBTCommand::Build { target, platform, configuration } => {
                cmd.arg(target)
                    .arg(platform)
                    .arg(configuration)
                    .arg("-Project")
                    .arg(&self.project_path)
                    .arg("-WaitMutex")
                    .arg("-FromMsBuild");
            }
            UBTCommand::Clean { target } => {
                cmd.arg(target)
                    .arg("-Clean")
                    .arg("-Project")
                    .arg(&self.project_path);
            }
            UBTCommand::GenerateProjectFiles => {
                // Note: This is typically done via GenerateProjectFiles.bat
                cmd.arg("-ProjectFiles")
                    .arg("-Project")
                    .arg(&self.project_path)
                    .arg("-Game")
                    .arg("-Engine");
            }
            UBTCommand::GetModuleDependencies { module } => {
                cmd.arg("-Mode=QueryTargets")
                    .arg("-Module")
                    .arg(module)
                    .arg("-Project")
                    .arg(&self.project_path);
            }
        }

        Ok(())
    }

    /// Check if UBT is available
    pub fn is_available(&self) -> bool {
        self.ubt_path.exists()
    }

    /// Get UBT version
    #[allow(clippy::disallowed_methods)] // Quick version check
    pub fn get_version(&self) -> Result<String> {
        let output = Command::new(&self.ubt_path)
            .arg("-Version")
            .output()
            .context("Failed to get UBT version")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Parse build log file
    pub fn parse_build_log(&self, log_path: &Path) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(log_path)
            .with_context(|| format!("Failed to read build log: {:?}", log_path))?;

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubt_executor_creation() {
        let engine_path = PathBuf::from("C:/UnrealEngine/UE_5.3");
        let project_path = PathBuf::from("C:/Projects/MyProject/MyProject.uproject");

        let executor = UBTExecutor::new(engine_path, project_path);

        assert!(executor.ubt_path.to_string_lossy().contains("UnrealBuildTool.exe"));
    }

    #[test]
    fn test_build_command_creation() {
        let command = UBTCommand::Build {
            target: "MyProjectEditor".to_string(),
            platform: "Win64".to_string(),
            configuration: "Development".to_string(),
        };

        // Structure test
        match command {
            UBTCommand::Build { target, platform, configuration } => {
                assert_eq!(target, "MyProjectEditor");
                assert_eq!(platform, "Win64");
                assert_eq!(configuration, "Development");
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_clean_command_creation() {
        let command = UBTCommand::Clean {
            target: "MyProject".to_string(),
        };

        match command {
            UBTCommand::Clean { target } => {
                assert_eq!(target, "MyProject");
            }
            _ => panic!("Wrong command type"),
        }
    }
}
