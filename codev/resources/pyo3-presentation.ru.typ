#set page(paper: "presentation-16-9", margin: 2cm)
#set text(font: "DejaVu Sans", size: 24pt)
#set heading(numbering: none)

#let slide(title, body) = {
  pagebreak(weak: true)
  heading(level: 1, title)
  v(1em)
  body
}

#let box-node(content, fill: rgb("#e3f2fd")) = {
  rect(
    fill: fill,
    radius: 8pt,
    inset: 12pt,
    stroke: rgb("#1976d2"),
    content
  )
}

#let arrow = text(size: 28pt)[ → ]

// Титульный слайд
#align(center + horizon)[
  #text(size: 48pt, weight: "bold")[PyO3]

  #v(0.5em)
  #text(size: 32pt)[Интеграция Rust и Python]

  #v(2em)
  #text(size: 20pt, fill: gray)[проект telegram_reader]
]

#slide[Что такое PyO3?][
  *PyO3* — привязки Rust к интерпретатору Python

  #v(1em)

  #grid(
    columns: (1fr, auto, 1fr),
    align: center + horizon,
    box-node(fill: rgb("#fff3e0"))[
      #text(weight: "bold")[Python]

      Высокоуровневый

      Богатая экосистема
    ],
    text(size: 36pt)[ ⟷ ],
    box-node(fill: rgb("#e8f5e9"))[
      #text(weight: "bold")[Rust]

      Производительность

      Безопасность памяти
    ]
  )

  #v(1em)

  ```rust
  #[pyfunction]
  fn sum_as_string(a: i64, b: i64) -> String {
      (a + b).to_string()
  }
  ```
]

#slide[Архитектура интеграции][
  #align(center)[
    #stack(
      dir: ttb,
      spacing: 1em,
      box-node(fill: rgb("#fff3e0"))[
        #text(weight: "bold")[Python приложение]

        mcp_telegram_server.py, bot_analytics.py
      ],
      text(size: 24pt)[▼ import telegram_reader],
      box-node(fill: rgb("#e3f2fd"))[
        #text(weight: "bold")[PyO3 привязки]

        src/python.rs — типы и функции
      ],
      text(size: 24pt)[▼ вызовы Rust],
      box-node(fill: rgb("#e8f5e9"))[
        #text(weight: "bold")[Rust библиотека]

        config, session, chat, integrations
      ],
    )
  ]
]

#slide[Зачем PyO3?][
  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      *Производительность*
      #v(0.5em)
      #align(center)[
        #stack(
          dir: ltr,
          spacing: 0.5em,
          rect(fill: rgb("#ffcdd2"), width: 20pt, height: 100pt),
          rect(fill: rgb("#c8e6c9"), width: 20pt, height: 10pt),
        )
      ]
      #align(center)[
        #text(size: 16pt)[Python vs Rust (10-100x)]
      ]
    ],
    [
      *Безопасность*
      - Система типов Rust
      - Никаких segfault
      - Гарантии памяти
      - Нет data races
    ]
  )

  #v(1em)

  *Применение:*
  - CPU-интенсивные алгоритмы
  - Regex и парсинг текста
  - Агрегация данных
]

#slide[Структура проекта][
  ```
  telegram_reader/
  ├── Cargo.toml          # Rust + PyO3 конфиг
  ├── pyproject.toml      # Maturin сборка
  ├── src/
  │   ├── lib.rs          # Корень библиотеки
  │   ├── python.rs       # PyO3 привязки ←
  │   ├── config.rs       # Конфигурация
  │   └── session.rs      # Telegram сессия
  └── python/
      └── telegram_reader/
          └── __init__.py # Python пакет
  ```
]

#slide[Поток данных][
  #align(center)[
    #grid(
      columns: (1fr, auto, 1fr, auto, 1fr),
      align: center + horizon,
      gutter: 0.3em,
      box-node(fill: rgb("#fff3e0"))[Python\nвызов],
      arrow,
      box-node(fill: rgb("#e3f2fd"))[PyO3\nконверсия],
      arrow,
      box-node(fill: rgb("#e8f5e9"))[Rust\nлогика],
    )

    #v(1em)

    #grid(
      columns: (1fr, auto, 1fr, auto, 1fr),
      align: center + horizon,
      gutter: 0.3em,
      box-node(fill: rgb("#fff3e0"))[Python\nобъект],
      text(size: 28pt)[ ← ],
      box-node(fill: rgb("#e3f2fd"))[PyO3\nконверсия],
      text(size: 28pt)[ ← ],
      box-node(fill: rgb("#e8f5e9"))[Rust\nрезультат],
    )
  ]

  #v(1em)

  #align(center)[
    #text(size: 18pt, fill: gray)[Автоматическая конверсия типов: String ↔ str, Vec ↔ list, HashMap ↔ dict]
  ]
]

#slide[Конфигурация Cargo.toml][
  ```toml
  [lib]
  name = "telegram_reader"
  crate-type = ["lib", "cdylib"]

  [dependencies]
  pyo3 = { version = "0.23", features = [
      "extension-module",
      "abi3-py311"
  ]}
  ```

  #v(1em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 1em,
    box-node(fill: rgb("#e3f2fd"))[
      `cdylib`

      Динамическая библиотека
    ],
    box-node(fill: rgb("#e8f5e9"))[
      `abi3`

      Один wheel для всех версий
    ]
  )
]

#slide[pyproject.toml (Maturin)][
  ```toml
  [build-system]
  requires = ["maturin>=1.0,<2.0"]
  build-backend = "maturin"

  [tool.maturin]
  features = ["pyo3/extension-module"]
  module-name = "telegram_reader"
  python-source = "python"
  ```

  #v(1em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 1em,
    box-node(fill: rgb("#fff3e0"))[
      `maturin develop`

      Разработка
    ],
    box-node(fill: rgb("#e8f5e9"))[
      `maturin build`

      Release wheel
    ]
  )
]

#slide[Определение Python-классов][
  ```rust
  #[pyclass]
  #[derive(Clone)]
  pub struct ChatTarget {
      #[pyo3(get)]
      pub name: String,
      #[pyo3(get)]
      pub kind: String,
      #[pyo3(get)]
      pub id: Option<i64>,
  }

  #[pymethods]
  impl ChatTarget {
      fn __repr__(&self) -> String {
          format!("ChatTarget(name='{}')", self.name)
      }
  }
  ```
]

#slide[Определение Python-функций][
  ```rust
  #[pyfunction]
  pub fn list_configured_chats(py: Python<'_>)
      -> PyResult<PyObject>
  {
      let config = Config::new();
      let list = PyList::empty(py);

      for (name, entity) in &config.chats {
          let target = ChatTarget::from((name, entity));
          list.append(target.into_pyobject(py)?)?;
      }

      Ok(list.into())
  }
  ```
]

#slide[Определение модуля][
  ```rust
  #[pymodule]
  pub fn telegram_reader(m: &Bound<'_, PyModule>)
      -> PyResult<()>
  {
      m.add_class::<ChatTarget>()?;
      m.add_class::<Message>()?;
      m.add_function(
          wrap_pyfunction!(list_configured_chats, m)?
      )?;
      m.add_function(
          wrap_pyfunction!(resolve_chat, m)?
      )?;
      Ok(())
  }
  ```
]

#slide[Использование в Python][
  ```python
  from telegram_reader import (
      list_configured_chats,
      resolve_chat,
      check_session,
      ChatTarget,
  )

  # Список всех настроенных чатов
  chats = list_configured_chats()

  # Разрешить чат по имени
  target = resolve_chat("my_channel")
  print(target.name, target.kind, target.id)

  # Проверка сессии
  if check_session():
      print("Готово!")
  ```
]

#slide[Обработка ошибок][
  #grid(
    columns: (1fr, 1fr),
    gutter: 1em,
    [
      *Rust:*
      ```rust
      #[pyfunction]
      fn resolve_chat(chat: &str)
          -> PyResult<ChatTarget>
      {
          if chat.is_empty() {
              return Err(
                  PyRuntimeError::new_err(
                      "Требуется имя"
                  )
              );
          }
          Ok(target)
      }
      ```
    ],
    [
      *Python:*
      ```python
      try:
          target = resolve_chat("")
      except RuntimeError as e:
          print(f"Ошибка: {e}")
      ```

      #v(1em)

      Автоматическое
      преобразование
      в исключения
    ]
  )
]

#slide[Преобразование типов][
  #table(
    columns: (1fr, 1fr, 2fr),
    align: (left, left, left),
    table.header([*Rust*], [*Python*], [*Описание*]),
    [`String`], [`str`], [Текстовые данные],
    [`i64`, `i32`], [`int`], [Целые числа],
    [`f64`], [`float`], [Числа с плавающей точкой],
    [`bool`], [`bool`], [Логические значения],
    [`Vec<T>`], [`list`], [Списки],
    [`HashMap<K,V>`], [`dict`], [Словари],
    [`Option<T>`], [`T | None`], [Опциональные значения],
    [`PyResult<T>`], [exception], [Ошибки → исключения],
  )
]

#slide[План миграции — Фаза 1][
  #text(weight: "bold", size: 20pt)[Быстрые победы (1-2 дня)]

  #v(0.5em)

  #table(
    columns: (2fr, 2fr, 1fr),
    align: (left, left, center),
    table.header([*Функция*], [*Описание*], [*Ускорение*]),
    [`parse_reactions()`], [Парсинг emoji реакций], [5-10x],
    [`sanitize_filename()`], [Очистка имён файлов], [5-10x],
    [`contains_phone()`], [Поиск телефона regex], [15-30x],
    [`detect_conversion()`], [Детекция конверсии], [10-20x],
    [`parse_iso_datetime()`], [Парсинг ISO дат], [20-50x],
  )
]

#slide[План миграции — Фаза 2][
  #text(weight: "bold", size: 20pt)[Высокое влияние (3-5 дней)]

  #v(0.5em)

  #table(
    columns: (2fr, 2fr, 1fr),
    align: (left, left, center),
    table.header([*Модуль*], [*Описание*], [*Ускорение*]),
    [`bot_analytics`], [Агрегация сессий], [10-20x],
    [`compute_retention`], [Расчёт D1/D7], [5-10x],
    [`format_messages_for_llm`], [Форматирование], [8-15x],
    [`generate_report`], [Markdown отчёты], [5-8x],
  )
]

#slide[Ожидаемые результаты][
  #align(center)[
    #grid(
      columns: (1fr, 1fr),
      gutter: 2em,
      [
        *До (Python)*
        #v(0.5em)
        #rect(fill: rgb("#ffcdd2"), width: 100%, height: 30pt, inset: 8pt)[
          Экспорт 5000 сообщений: ~10 сек
        ]
        #rect(fill: rgb("#ffcdd2"), width: 100%, height: 30pt, inset: 8pt)[
          Аналитика 50k: ~30 сек
        ]
        #rect(fill: rgb("#ffcdd2"), width: 100%, height: 30pt, inset: 8pt)[
          A/B анализ 10k: ~5 сек
        ]
      ],
      [
        *После (Rust)*
        #v(0.5em)
        #rect(fill: rgb("#c8e6c9"), width: 100%, height: 30pt, inset: 8pt)[
          Экспорт 5000 сообщений: ~1 сек
        ]
        #rect(fill: rgb("#c8e6c9"), width: 100%, height: 30pt, inset: 8pt)[
          Аналитика 50k: ~2 сек
        ]
        #rect(fill: rgb("#c8e6c9"), width: 100%, height: 30pt, inset: 8pt)[
          A/B анализ 10k: ~0.2 сек
        ]
      ]
    )
  ]
]

#slide[Архитектура модуля][
  ```
  telegram_reader (PyO3)
  ├── ChatTarget, Message, SendResult  ✓
  ├── list_configured_chats()          ✓
  ├── resolve_chat()                   ✓
  ├── check_session()                  ✓
  │
  ├── text_processing/     ← Фаза 1
  │   ├── parse_reactions()
  │   ├── sanitize_filename()
  │   ├── contains_phone()
  │   └── detect_conversion()
  │
  └── analytics/           ← Фаза 2
      ├── SessionMetrics
      └── compute_retention()
  ```
]

#slide[Тестирование Rust-кода][
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_chat_target_from_channel() {
          let entity = ChatEntity::Channel(12345);
          let target = ChatTarget::from(("test", &entity));

          assert_eq!(target.name, "test");
          assert_eq!(target.kind, "channel");
          assert_eq!(target.id, Some(12345));
      }
  }
  ```

  #align(center)[
    `cargo test python::` — запуск тестов PyO3 модуля
  ]
]

#slide[Лучшие практики][
  #grid(
    columns: (1fr, 1fr),
    gutter: 1.5em,
    [
      #box-node(fill: rgb("#e8f5e9"))[
        *1. Тонкие привязки*

        Логика в чистом Rust
      ]

      #v(0.5em)

      #box-node(fill: rgb("#e8f5e9"))[
        *2. `#[pyo3(get)]`*

        Автоматические геттеры
      ]

      #v(0.5em)

      #box-node(fill: rgb("#e8f5e9"))[
        *3. `__repr__`*

        Удобная отладка
      ]
    ],
    [
      #box-node(fill: rgb("#e3f2fd"))[
        *4. `PyResult`*

        Правильные ошибки
      ]

      #v(0.5em)

      #box-node(fill: rgb("#e3f2fd"))[
        *5. `abi3`*

        Один wheel для всех
      ]

      #v(0.5em)

      #box-node(fill: rgb("#e3f2fd"))[
        *6. Тесты в Rust*

        Быстрая обратная связь
      ]
    ]
  )
]

#slide[Советы по производительности][
  #box-node(fill: rgb("#fff3e0"))[
    *Избегайте частых конвертаций Rust ↔ Python*

    Группируйте операции, передавайте батчи
  ]

  #v(1em)

  *Освобождение GIL для долгих вычислений:*

  ```rust
  #[pyfunction]
  fn heavy_computation(py: Python<'_>, data: Vec<i64>)
      -> PyResult<i64>
  {
      py.allow_threads(|| {
          // GIL освобождён — можно параллелить
          data.iter().sum()
      })
  }
  ```
]

#slide[Ресурсы][
  #v(1em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      *Документация*
      - PyO3: https://pyo3.rs
      - Maturin: https://maturin.rs
    ],
    [
      *Проект*
      - `src/python.rs`
      - `python/telegram_reader/`
      - `codev/pyo3_migration_plan.ru.md`
    ]
  )

  #v(2em)

  #align(center)[
    #box-node(fill: rgb("#e8f5e9"))[
      ```bash
      cargo test python:: && maturin develop
      ```
    ]
  ]
]

// Финальный слайд
#pagebreak()
#align(center + horizon)[
  #text(size: 48pt, weight: "bold")[Вопросы?]

  #v(2em)

  #grid(
    columns: (1fr, 1fr, 1fr),
    gutter: 1em,
    box-node(fill: rgb("#e8f5e9"))[
      Сборка

      `maturin build`
    ],
    box-node(fill: rgb("#e3f2fd"))[
      Тесты

      `cargo test`
    ],
    box-node(fill: rgb("#fff3e0"))[
      Импорт

      `import telegram_reader`
    ]
  )
]
