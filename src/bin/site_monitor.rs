//! Утилита мониторинга сайтов
//!
//! Использование:
//!   site_monitor check <url>              - проверить один URL
//!   site_monitor check-all --config sites.yml  - проверить все сайты из конфига
//!   site_monitor watch --config sites.yml     - непрерывный мониторинг
//!
//! Примеры:
//!   cargo run --bin site_monitor check https://google.com
//!   cargo run --bin site_monitor check-all --config monitor.yml
//!   cargo run --bin site_monitor watch --config monitor.yml --interval 60

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use telegram_reader::commands::monitor::{
    check_sites, check_url, detect_changes, format_result, format_telegram_alert, load_config,
    save_history, CheckResult, MonitorConfig, SiteConfig,
};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "site_monitor")]
#[command(about = "Утилита мониторинга сайтов", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Проверить один URL
    Check {
        /// URL для проверки
        url: String,

        /// Таймаут в секундах
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },

    /// Проверить все сайты из конфигурации
    CheckAll {
        /// Путь к файлу конфигурации (YAML)
        #[arg(short, long)]
        config: PathBuf,

        /// Сохранить результаты в файл
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Непрерывный мониторинг сайтов
    Watch {
        /// Путь к файлу конфигурации (YAML)
        #[arg(short, long)]
        config: PathBuf,

        /// Интервал проверки в секундах
        #[arg(short, long, default_value = "60")]
        interval: u64,

        /// Файл для сохранения истории
        #[arg(long)]
        history: Option<PathBuf>,
    },

    /// Создать пример конфигурации
    InitConfig {
        /// Путь для сохранения
        #[arg(short, long, default_value = "monitor.yml")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логирования
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("site_monitor=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Check { url, timeout } => {
            info!("Проверяю {}...", url);
            let result = check_url(&url, timeout).await;
            println!("{}", format_result(&result));

            if let Some(error) = &result.error {
                error!("Ошибка: {}", error);
            }

            if !result.is_healthy {
                std::process::exit(1);
            }
        }

        Commands::CheckAll { config, output } => {
            info!("Загружаю конфигурацию из {:?}...", config);
            let monitor_config = load_config(&config).await?;

            info!("Проверяю {} сайтов...", monitor_config.sites.len());
            let results = check_sites(&monitor_config.sites).await;

            // Выводим результаты
            println!("\n📊 Результаты проверки:\n");
            for result in &results {
                println!("{}", format_result(result));
            }

            // Статистика
            let healthy = results.iter().filter(|r| r.is_healthy).count();
            let total = results.len();
            println!("\n✅ Доступно: {}/{}", healthy, total);

            // Сохраняем результаты если указан output
            if let Some(output_path) = output {
                save_history(&results, &output_path).await?;
                info!("Результаты сохранены в {:?}", output_path);
            }

            // Показываем alert если есть проблемы
            if let Some(alert) = format_telegram_alert(&results) {
                println!("\n{}", alert);
            }

            if healthy < total {
                std::process::exit(1);
            }
        }

        Commands::Watch {
            config,
            interval: check_interval,
            history,
        } => {
            info!("Запуск непрерывного мониторинга...");
            let monitor_config = load_config(&config).await?;

            let mut previous_results: HashMap<String, CheckResult> = HashMap::new();
            let mut interval_timer = interval(Duration::from_secs(check_interval));

            loop {
                interval_timer.tick().await;

                info!("Проверка {} сайтов...", monitor_config.sites.len());
                let results = check_sites(&monitor_config.sites).await;

                // Выводим результаты
                for result in &results {
                    println!("{}", format_result(result));
                }

                // Определяем изменения
                let changes = detect_changes(&results, &previous_results);
                if !changes.is_empty() {
                    println!("\n📢 Изменения:");
                    for change in &changes {
                        println!("  {}", change);
                    }
                }

                // Обновляем предыдущие результаты
                for result in &results {
                    previous_results.insert(result.url.clone(), result.clone());
                }

                // Сохраняем историю
                if let Some(ref history_path) = history {
                    let all_results: Vec<_> = previous_results.values().cloned().collect();
                    if let Err(e) = save_history(&all_results, history_path).await {
                        warn!("Не удалось сохранить историю: {}", e);
                    }
                }

                // Статистика
                let healthy = results.iter().filter(|r| r.is_healthy).count();
                let total = results.len();
                info!(
                    "Проверка завершена: {}/{} доступно. Следующая через {} сек.",
                    healthy, total, check_interval
                );

                println!("---");
            }
        }

        Commands::InitConfig { output } => {
            let example_config = MonitorConfig {
                sites: vec![
                    SiteConfig {
                        url: "https://google.com".to_string(),
                        name: Some("Google".to_string()),
                        timeout_secs: Some(10),
                        expected_status: Some(200),
                        check_content: Some(false),
                        content_selector: None,
                    },
                    SiteConfig {
                        url: "https://github.com".to_string(),
                        name: Some("GitHub".to_string()),
                        timeout_secs: Some(15),
                        expected_status: Some(200),
                        check_content: Some(false),
                        content_selector: None,
                    },
                ],
                check_interval_secs: Some(60),
                notify_telegram_chat: None,
            };

            let yaml = serde_yaml::to_string(&example_config)?;
            tokio::fs::write(&output, yaml).await?;
            println!("✅ Пример конфигурации сохранён в {:?}", output);
        }
    }

    Ok(())
}
