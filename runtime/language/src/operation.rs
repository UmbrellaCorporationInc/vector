//! Plugin operation for resolving language quality-gate prompts.

use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::{path::IoPath, text::read_file_text};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Input for the `QualityGate` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct QualityGateInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The ordered set of language identifiers to resolve.
    pub languages: Vec<String>,
}

/// Output for the `QualityGate` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct QualityGateOutput {
    /// The concatenated quality-gate prompt body for the requested languages.
    pub prompt: String,
}

#[derive(Debug, Deserialize)]
struct LanguageRuleEntry {
    #[serde(rename = "quality-gate")]
    quality_gate: Option<String>,
}

type LanguageRules = HashMap<String, LanguageRuleEntry>;

#[derive(Debug, Error)]
enum QualityGateError {
    #[error("languages input must not be empty")]
    EmptyLanguages,
    #[error("duplicate language '{0}' is not allowed")]
    DuplicateLanguage(String),
    #[error("failed to read .vector/language-rules.yaml: {0}")]
    ConfigRead(String),
    #[error("failed to parse .vector/language-rules.yaml: {0}")]
    ConfigParse(String),
    #[error("language '{0}' is missing a quality-gate mapping")]
    MissingQualityGate(String),
    #[error("quality-gate prompt '{0}' did not resolve to any governed prompts document")]
    PromptNotFound(String),
    #[error("quality-gate prompt '{0}' resolved to multiple governed prompts documents")]
    PromptAmbiguous(String),
    #[error("failed to read prompt document '{0}': {1}")]
    PromptRead(String, String),
    #[error("prompt document '{0}' is missing YAML frontmatter")]
    MissingFrontmatter(String),
}

async fn quality_gate(
    input: QualityGateInput,
    output: &mut impl PluginSender<QualityGateOutput>,
) -> RuntimeResult<()> {
    let prompt = resolve_quality_gate_prompt(&input).await.map_err(|error| {
        RuntimeError::operation(format!("language quality gate failed: {error}"))
    })?;

    output.send(QualityGateOutput { prompt }).await?;
    Ok(())
}

async fn resolve_quality_gate_prompt(input: &QualityGateInput) -> Result<String, QualityGateError> {
    let normalized_languages = normalize_languages(&input.languages)?;
    let config = load_language_rules(&input.root_dir).await?;
    let mut prompt_bodies = Vec::with_capacity(normalized_languages.len());

    for language in &normalized_languages {
        let Some(entry) = config.get(language) else {
            continue;
        };
        let prompt_ref = entry
            .quality_gate
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| QualityGateError::MissingQualityGate(language.clone()))?;
        let prompt_path = resolve_prompt_path(&input.root_dir, prompt_ref)?;
        let prompt_text = read_file_text(&IoPath::new(&prompt_path)).await.map_err(|error| {
            QualityGateError::PromptRead(prompt_path.display().to_string(), error.to_string())
        })?;
        let prompt_body = strip_frontmatter(&prompt_text).ok_or_else(|| {
            QualityGateError::MissingFrontmatter(prompt_path.display().to_string())
        })?;
        prompt_bodies.push(prompt_body.to_string());
    }

    Ok(prompt_bodies.join("\n\n"))
}

fn normalize_languages(languages: &[String]) -> Result<Vec<String>, QualityGateError> {
    if languages.is_empty() {
        return Err(QualityGateError::EmptyLanguages);
    }

    let mut normalized_languages = Vec::with_capacity(languages.len());
    let mut seen = std::collections::HashSet::with_capacity(languages.len());
    for language in languages {
        let normalized = language.to_lowercase();
        if !seen.insert(normalized.clone()) {
            return Err(QualityGateError::DuplicateLanguage(normalized));
        }
        normalized_languages.push(normalized);
    }

    Ok(normalized_languages)
}

async fn load_language_rules(root_dir: &IoPath) -> Result<LanguageRules, QualityGateError> {
    let path = root_dir.join(".vector").join("language-rules.yaml");
    let text = read_file_text(&path)
        .await
        .map_err(|error| QualityGateError::ConfigRead(error.to_string()))?;
    validate_language_rules_field_names(&text)?;
    serde_yaml::from_str::<LanguageRules>(&text)
        .map_err(|error| QualityGateError::ConfigParse(error.to_string()))
}

fn validate_language_rules_field_names(text: &str) -> Result<(), QualityGateError> {
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(text)
        .map_err(|error| QualityGateError::ConfigParse(error.to_string()))?;
    let serde_yaml::Value::Mapping(root) = yaml else {
        return Ok(());
    };

    for (language_key, entry_value) in root {
        let Some(language) = language_key.as_str() else {
            continue;
        };
        validate_language_rule_value(language, &entry_value)?;
    }

    Ok(())
}

fn validate_language_rule_value(
    current_path: &str,
    value: &serde_yaml::Value,
) -> Result<(), QualityGateError> {
    let serde_yaml::Value::Mapping(entry) = value else {
        return Ok(());
    };

    for (field_key, child_value) in entry {
        let Some(field_name) = field_key.as_str() else {
            continue;
        };
        let field_path = format!("{current_path}.{field_name}");
        if !is_kebab_case_identifier(field_name) {
            return Err(QualityGateError::ConfigParse(format!(
                "Invalid .vector YAML field name '{field_name}' at '{field_path}'; schema fields must be kebab-case"
            )));
        }
        validate_language_rule_value(&field_path, child_value)?;
    }

    Ok(())
}

fn is_kebab_case_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }

    let mut previous_was_hyphen = false;
    for ch in chars {
        if ch == '-' {
            if previous_was_hyphen {
                return false;
            }
            previous_was_hyphen = true;
            continue;
        }
        previous_was_hyphen = false;
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() {
            return false;
        }
    }

    !name.ends_with('-')
}

fn resolve_prompt_path(root_dir: &IoPath, prompt_ref: &str) -> Result<PathBuf, QualityGateError> {
    let prompts_dir = root_dir.as_path().join("doc").join("prompts");
    let mut matches = Vec::new();

    for entry in WalkDir::new(&prompts_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            continue;
        }

        let stem = path.file_stem().and_then(|value| value.to_str());
        if stem == Some(prompt_ref) {
            matches.push(path.to_path_buf());
        }
    }

    match matches.len() {
        0 => Err(QualityGateError::PromptNotFound(prompt_ref.to_string())),
        1 => Ok(matches.remove(0)),
        _ => Err(QualityGateError::PromptAmbiguous(prompt_ref.to_string())),
    }
}

fn strip_frontmatter(text: &str) -> Option<&str> {
    if text == "---" {
        return Some("");
    }

    let rest = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"))?;

    let end_marker = "\n---\n";
    if let Some(index) = rest.find(end_marker) {
        return Some(&rest[index + end_marker.len()..]);
    }

    let end_marker = "\r\n---\r\n";
    if let Some(index) = rest.find(end_marker) {
        return Some(&rest[index + end_marker.len()..]);
    }

    None
}

declare_plugin_operations! {
    QualityGateOp => quality_gate(QualityGateInput, QualityGateOutput)
}

impl QualityGateInput {
    /// Construct a `QualityGateInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: IoPath, languages: Vec<String>) -> Self {
        Self { root_dir, languages }
    }
}

impl QualityGateOp {
    /// Construct a new `QualityGateOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for QualityGateOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "operation_test.rs"]
mod tests;
