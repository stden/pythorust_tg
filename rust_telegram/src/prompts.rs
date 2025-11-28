//! Загрузчик системных промптов из файлов.
//!
//! Промпты хранятся в каталоге `prompts/` в корне проекта.

use std::path::PathBuf;

use crate::{Error, Result};

/// Доступные промпты.
#[derive(Debug, Clone, Copy)]
pub enum Prompt {
    /// Продающий агент (SPIN, AIDA).
    SalesAgent,
    /// Калькулятор - только числа.
    Calculator,
    /// Дружелюбный AI-ассистент.
    FriendlyAI,
    /// Модератор чата.
    Moderator,
    /// Создатель дайджестов.
    Digest,
    /// Парсер CRM данных.
    CrmParser,
}

impl Prompt {
    /// Имя файла промпта (Markdown).
    pub fn filename(&self) -> &'static str {
        match self {
            Prompt::SalesAgent => "sales_agent.md",
            Prompt::Calculator => "calculator.md",
            Prompt::FriendlyAI => "friendly_ai.md",
            Prompt::Moderator => "moderator.md",
            Prompt::Digest => "digest.md",
            Prompt::CrmParser => "crm_parser.md",
        }
    }

    /// Загрузить промпт из файла.
    pub fn load(&self) -> Result<String> {
        load_prompt(self.filename())
    }
}

/// Загрузить промпт по имени файла.
pub fn load_prompt(filename: &str) -> Result<String> {
    load_prompt_with_base_dir(prompts_dir().as_path(), filename)
}

/// Загрузить промпт по имени файла из указанного каталога.
pub fn load_prompt_with_base_dir(base_dir: &std::path::Path, filename: &str) -> Result<String> {
    let path = base_dir.join(filename);
    std::fs::read_to_string(&path)
        .map_err(|e| Error::InvalidArgument(format!("Не удалось загрузить промпт {}: {}", filename, e)))
}

/// Путь к каталогу промптов.
pub fn prompts_dir() -> PathBuf {
    // Ищем prompts/ относительно текущей директории или родительской
    let candidates = [
        PathBuf::from("prompts"),
        PathBuf::from("../prompts"),
        PathBuf::from("../../prompts"),
    ];

    for path in candidates {
        if path.exists() {
            return path;
        }
    }

    // Fallback
    PathBuf::from("prompts")
}

/// Список всех доступных промптов.
pub fn list_prompts() -> Vec<Prompt> {
    vec![
        Prompt::SalesAgent,
        Prompt::Calculator,
        Prompt::FriendlyAI,
        Prompt::Moderator,
        Prompt::Digest,
        Prompt::CrmParser,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_prompt_filenames() {
        assert_eq!(Prompt::SalesAgent.filename(), "sales_agent.md");
        assert_eq!(Prompt::Moderator.filename(), "moderator.md");
    }

    #[test]
    fn test_list_prompts() {
        let prompts = list_prompts();
        assert_eq!(prompts.len(), 6);
    }

    #[test]
    fn test_load_prompt_success() {
        let dir = tempdir().unwrap();
        let prompts_path = dir.path();

        let prompt_content = "This is a test prompt.";
        fs::write(prompts_path.join("test_prompt.md"), prompt_content).unwrap();

        let loaded_content = load_prompt_with_base_dir(prompts_path, "test_prompt.md").unwrap();
        assert_eq!(loaded_content, prompt_content);
    }
}
