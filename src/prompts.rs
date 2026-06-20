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
    let path = prompts_dir().join(filename);
    std::fs::read_to_string(&path).map_err(|e| {
        Error::InvalidArgument(format!("Не удалось загрузить промпт {}: {}", filename, e))
    })
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
    fn test_all_prompt_filenames_are_md() {
        for prompt in list_prompts() {
            assert!(
                prompt.filename().ends_with(".md"),
                "Prompt {:?} should have .md extension",
                prompt
            );
        }
    }

    #[test]
    fn test_calculator_filename() {
        assert_eq!(Prompt::Calculator.filename(), "calculator.md");
    }

    #[test]
    fn test_friendly_ai_filename() {
        assert_eq!(Prompt::FriendlyAI.filename(), "friendly_ai.md");
    }

    #[test]
    fn test_digest_filename() {
        assert_eq!(Prompt::Digest.filename(), "digest.md");
    }

    #[test]
    fn test_crm_parser_filename() {
        assert_eq!(Prompt::CrmParser.filename(), "crm_parser.md");
    }

    #[test]
    fn test_prompts_dir_returns_path() {
        let dir = prompts_dir();
        // Should return some path, even if fallback
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_load_prompt_nonexistent_file() {
        let result = load_prompt("nonexistent_file_12345.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_clone() {
        let prompt = Prompt::SalesAgent;
        let cloned = prompt;
        assert_eq!(prompt.filename(), cloned.filename());
    }

    #[test]
    fn test_prompt_debug() {
        let prompt = Prompt::Calculator;
        let debug_str = format!("{:?}", prompt);
        assert!(debug_str.contains("Calculator"));
    }

    #[test]
    fn test_list_prompts_contains_all_variants() {
        let prompts = list_prompts();
        let filenames: Vec<&str> = prompts.iter().map(|p| p.filename()).collect();

        assert!(filenames.contains(&"sales_agent.md"));
        assert!(filenames.contains(&"calculator.md"));
        assert!(filenames.contains(&"friendly_ai.md"));
        assert!(filenames.contains(&"moderator.md"));
        assert!(filenames.contains(&"digest.md"));
        assert!(filenames.contains(&"crm_parser.md"));
    }
}
