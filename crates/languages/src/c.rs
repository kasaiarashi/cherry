// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.
//
// C/C++ Language Support
//
// Note: This is a stub implementation. The actual code intelligence
// will be provided by the cherry-sight crate.

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::{App, AsyncApp};
pub use language::*;
use lsp::{InitializeParams, LanguageServerBinary, LanguageServerName};
use std::path::PathBuf;
use std::sync::Arc;

pub struct CLspAdapter;

impl CLspAdapter {
    // Placeholder - will be replaced by cherry-sight
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("cherry-sight");
}

// Cherry-sight is built into the Cherry IDE - find it in the IDE's bin directory
impl LspInstaller for CLspAdapter {
    type BinaryVersion = ();

    async fn fetch_latest_server_version(
        &self,
        _: &dyn LspAdapterDelegate,
        _: bool,
        _: &mut AsyncApp,
    ) -> Result<Self::BinaryVersion> {
        // cherry-sight is built-in with the IDE
        Ok(())
    }

    async fn fetch_server_binary(
        &self,
        _: Self::BinaryVersion,
        _: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        // Find cherry-sight-lsp in the IDE's installation directory
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .ok_or_else(|| anyhow!("Failed to get executable directory"))?;

        let binary_name = if cfg!(windows) {
            "cherry-sight-lsp.exe"
        } else {
            "cherry-sight-lsp"
        };

        let binary_path = exe_dir.join(binary_name);

        Ok(LanguageServerBinary {
            path: binary_path,
            arguments: vec![],
            env: None,
        })
    }

    async fn cached_server_binary(
        &self,
        _: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        // Try to find cherry-sight-lsp in the IDE's directory
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))?;

        let binary_name = if cfg!(windows) {
            "cherry-sight-lsp.exe"
        } else {
            "cherry-sight-lsp"
        };

        let binary_path = exe_dir.join(binary_name);

        if binary_path.exists() {
            Some(LanguageServerBinary {
                path: binary_path,
                arguments: vec![],
                env: None,
            })
        } else {
            None
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for CLspAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }

    async fn label_for_completion(
        &self,
        completion: &lsp::CompletionItem,
        language: &Arc<Language>,
    ) -> Option<CodeLabel> {
        let label_detail = match &completion.label_details {
            Some(label_detail) => match &label_detail.detail {
                Some(detail) => detail.trim(),
                None => "",
            },
            None => "",
        };

        let mut label = completion
            .label
            .strip_prefix('•')
            .unwrap_or(&completion.label)
            .trim()
            .to_owned();

        if !label_detail.is_empty() {
            let should_add_space = match completion.kind {
                Some(lsp::CompletionItemKind::FUNCTION | lsp::CompletionItemKind::METHOD) => false,
                _ => true,
            };

            if should_add_space && !label.ends_with(' ') && !label_detail.starts_with(' ') {
                label.push(' ');
            }
            label.push_str(label_detail);
        }

        match completion.kind {
            Some(lsp::CompletionItemKind::FIELD) if completion.detail.is_some() => {
                let detail = completion.detail.as_ref().unwrap();
                let text = format!("{} {}", detail, label);
                let source = Rope::from(format!("struct S {{ {} }}", text).as_str());
                let runs = language.highlight_text(&source, 11..11 + text.len());
                let filter_range = completion
                    .filter_text
                    .as_deref()
                    .and_then(|filter_text| {
                        text.find(filter_text)
                            .map(|start| start..start + filter_text.len())
                    })
                    .unwrap_or(detail.len() + 1..text.len());
                return Some(CodeLabel::new(text, filter_range, runs));
            }
            Some(lsp::CompletionItemKind::CONSTANT | lsp::CompletionItemKind::VARIABLE)
                if completion.detail.is_some() =>
            {
                let detail = completion.detail.as_ref().unwrap();
                let text = format!("{} {}", detail, label);
                let runs = language.highlight_text(&Rope::from(text.as_str()), 0..text.len());
                let filter_range = completion
                    .filter_text
                    .as_deref()
                    .and_then(|filter_text| {
                        text.find(filter_text)
                            .map(|start| start..start + filter_text.len())
                    })
                    .unwrap_or(detail.len() + 1..text.len());
                return Some(CodeLabel::new(text, filter_range, runs));
            }
            Some(lsp::CompletionItemKind::FUNCTION | lsp::CompletionItemKind::METHOD)
                if completion.detail.is_some() =>
            {
                let detail = completion.detail.as_ref().unwrap();
                let text = format!("{} {}", detail, label);
                let runs = language.highlight_text(&Rope::from(text.as_str()), 0..text.len());
                let filter_range = completion
                    .filter_text
                    .as_deref()
                    .and_then(|filter_text| {
                        text.find(filter_text)
                            .map(|start| start..start + filter_text.len())
                    })
                    .unwrap_or_else(|| {
                        let filter_start = detail.len() + 1;
                        let filter_end = text
                            .rfind('(')
                            .filter(|end| *end > filter_start)
                            .unwrap_or(text.len());
                        filter_start..filter_end
                    });

                return Some(CodeLabel::new(text, filter_range, runs));
            }
            Some(kind) => {
                let highlight_name = match kind {
                    lsp::CompletionItemKind::STRUCT
                    | lsp::CompletionItemKind::INTERFACE
                    | lsp::CompletionItemKind::CLASS
                    | lsp::CompletionItemKind::ENUM => Some("type"),
                    lsp::CompletionItemKind::ENUM_MEMBER => Some("variant"),
                    lsp::CompletionItemKind::KEYWORD => Some("keyword"),
                    lsp::CompletionItemKind::VALUE | lsp::CompletionItemKind::CONSTANT => {
                        Some("constant")
                    }
                    _ => None,
                };
                if let Some(highlight_id) = language
                    .grammar()
                    .and_then(|g| g.highlight_id_for_name(highlight_name?))
                {
                    let mut label = CodeLabel::plain(label, completion.filter_text.as_deref());
                    label.runs.push((
                        0..label.text.rfind('(').unwrap_or(label.text.len()),
                        highlight_id,
                    ));
                    return Some(label);
                }
            }
            _ => {}
        }
        Some(CodeLabel::plain(label, completion.filter_text.as_deref()))
    }

    async fn label_for_symbol(
        &self,
        name: &str,
        kind: lsp::SymbolKind,
        language: &Arc<Language>,
    ) -> Option<CodeLabel> {
        let (text, filter_range, display_range) = match kind {
            lsp::SymbolKind::METHOD | lsp::SymbolKind::FUNCTION => {
                let text = format!("void {} () {{}}", name);
                let filter_range = 0..name.len();
                let display_range = 5..5 + name.len();
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::STRUCT => {
                let text = format!("struct {} {{}}", name);
                let filter_range = 7..7 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::ENUM => {
                let text = format!("enum {} {{}}", name);
                let filter_range = 5..5 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::INTERFACE | lsp::SymbolKind::CLASS => {
                let text = format!("class {} {{}}", name);
                let filter_range = 6..6 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::CONSTANT => {
                let text = format!("const int {} = 0;", name);
                let filter_range = 10..10 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::MODULE => {
                let text = format!("namespace {} {{}}", name);
                let filter_range = 10..10 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            lsp::SymbolKind::TYPE_PARAMETER => {
                let text = format!("typename {} {{}};", name);
                let filter_range = 9..9 + name.len();
                let display_range = 0..filter_range.end;
                (text, filter_range, display_range)
            }
            _ => return None,
        };

        Some(CodeLabel::new(
            text[display_range.clone()].to_string(),
            filter_range,
            language.highlight_text(&text.as_str().into(), display_range),
        ))
    }

    fn prepare_initialize_params(
        &self,
        original: InitializeParams,
        _: &App,
    ) -> Result<InitializeParams> {
        Ok(original)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, BorrowAppContext, TestAppContext};
    use language::{AutoindentMode, Buffer};
    use settings::SettingsStore;
    use std::num::NonZeroU32;
    use unindent::Unindent;

    #[gpui::test]
    async fn test_c_autoindent_basic(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let test_settings = SettingsStore::test(cx);
            cx.set_global(test_settings);
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |s| {
                    s.project.all_languages.defaults.tab_size = NonZeroU32::new(2);
                });
            });
        });
        let language = crate::language("c", tree_sitter_c::LANGUAGE.into());

        cx.new(|cx| {
            let mut buffer = Buffer::local("", cx).with_language(language, cx);

            buffer.edit([(0..0, "int main() {}")], None, cx);

            let ix = buffer.len() - 1;
            buffer.edit([(ix..ix, "\n\n")], Some(AutoindentMode::EachLine), cx);
            assert_eq!(
                buffer.text(),
                "int main() {\n  \n}",
                "content inside braces should be indented"
            );

            buffer
        });
    }

    #[gpui::test]
    async fn test_c_autoindent_if_else(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let test_settings = SettingsStore::test(cx);
            cx.set_global(test_settings);
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |s| {
                    s.project.all_languages.defaults.tab_size = NonZeroU32::new(2);
                });
            });
        });
        let language = crate::language("c", tree_sitter_c::LANGUAGE.into());

        cx.new(|cx| {
            let mut buffer = Buffer::local("", cx).with_language(language, cx);

            buffer.edit(
                [(
                    0..0,
                    r#"
                    int main() {
                    if (a)
                    b;
                    }
                    "#
                    .unindent(),
                )],
                Some(AutoindentMode::EachLine),
                cx,
            );
            assert_eq!(
                buffer.text(),
                r#"
                int main() {
                  if (a)
                    b;
                }
                "#
                .unindent(),
                "body of if-statement without braces should be indented"
            );

            buffer
        });
    }
}
