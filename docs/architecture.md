# 🏗️ Architecture — Visual Textbook

A complete, GitHub-rendered visual guide to **stden/pythorust_tg**, a Rust-first
Telegram automation toolkit. Every diagram is grounded in real modules
(`src/…`), binaries (`src/bin/…`), and the MySQL schema — not idealized.

> **How to read this doc.** It climbs from the outside in: system context →
> containers → core components → then runtime flows (sequences, pipelines,
> decision trees) → data model → layering. Skim the diagrams top-to-bottom for a
> mental model; dive into a section when you touch that subsystem.

**Legend (shared across diagrams)**

| Shape / style | Meaning |
|---|---|
| `[(cylinder)]` | datastore (MySQL · Qdrant · Neo4j · SQLite) |
| rectangle | Rust module / binary / process |
| rhombus `{ }` | decision point |
| dotted edge | optional / "exposes" / cross-cutting |
| subgraph box | a bounded layer, phase, or process group |

---

## 1. 🌐 C4 — System Context

*Purpose:* the 10,000-ft view — who uses the toolkit and which external systems
it depends on. *Placement:* `docs/architecture.md` (top) or `README.md`.

```mermaid
C4Context
    title System Context — Telegram Automation Toolkit
    Person(op, "Operator", "Runs CLI commands and bots")
    System(tk, "telegram_reader (Rust)", "MTProto automation, AI, moderation, export, RAG")
    System_Ext(tg, "Telegram", "MTProto (grammers) + Bot API (teloxide)")
    System_Ext(ai, "AI Providers", "OpenAI, Gemini, Claude, Ollama, Whisper, Yandex TTS")
    System_Ext(db, "Datastores", "MySQL, Qdrant, Neo4j, SQLite session")
    System_Ext(n8n, "n8n", "Workflow automation")
    System_Ext(lin, "Linear", "Issue tracking")
    Rel(op, tk, "Runs commands / bots")
    Rel(tk, tg, "Reads & sends messages")
    Rel(tk, ai, "Prompts / completions")
    Rel(tk, db, "Reads / writes")
    Rel(tk, n8n, "Monitors / backs up")
    Rel(tk, lin, "Syncs issues")
    UpdateRelStyle(op, tk, $offsetY="-30")
```

---

## 2. 📦 C4 — Container Diagram

*Purpose:* the deployable/runnable pieces inside the toolkit and how they split
responsibility. *Placement:* `docs/architecture.md`.

```mermaid
C4Container
    title Container Diagram — telegram_reader
    Person(op, "Operator", "CLI user / bot operator")

    System_Boundary(tk, "telegram_reader (Rust workspace)") {
        Container(cli, "CLI", "Rust · Clap · Tokio", "telegram_reader subcommands: read, digest, crm, moderate, react, hunt, analyze, n8n, linear")
        Container(lib, "Core Library", "Rust · src/lib.rs", "commands · integrations · analysis · lightrag · session · metrics")
        Container(bots, "Bots", "Rust · teloxide", "sales · credit_expert · task_assistant · devops_ai · community_game · ai_project_consultant")
        Container(tools, "Specialized Binaries", "Rust · 52 src/bin targets", "data · export · moderation · analysis · ops · linear · dev")
        Container(py, "Python (legacy)", "Python · PyO3", "chat_analysis · mcp_telegram_server.py")
        ContainerDb(sqlite, "Session store", "SQLite", "telegram_session.session (single, file-locked)")
    }

    System_Ext(tg, "Telegram", "MTProto + Bot API")
    System_Ext(ai, "AI Providers", "OpenAI · Gemini · Claude · Ollama")
    ContainerDb_Ext(mysql, "MySQL", "bot_users · bot_sessions · bot_messages · bot_experiments")
    ContainerDb_Ext(qdrant, "Qdrant", "vector store")
    ContainerDb_Ext(neo4j, "Neo4j", "knowledge graph")
    System_Ext(n8n, "n8n", "automation")

    Rel(op, cli, "Runs")
    Rel(op, bots, "Operates")
    Rel(cli, lib, "Uses")
    Rel(bots, lib, "Uses")
    Rel(tools, lib, "Uses")
    Rel(lib, tg, "grammers / teloxide")
    Rel(lib, ai, "completions / embeddings")
    Rel(bots, mysql, "dialog state")
    Rel(lib, qdrant, "vectors")
    Rel(lib, neo4j, "entities")
    Rel(cli, sqlite, "session")
    Rel(tools, n8n, "monitor / backup")
    Rel(py, lib, "PyO3 bindings")
```

---

## 3. 🧩 Component — Core Library

*Purpose:* how `src/lib.rs` decomposes; `commands/` orchestrates, everything
else is a capability it calls. *Placement:* `docs/architecture.md`.

```mermaid
flowchart TB
    main["main.rs<br/>Clap CLI · --metrics-addr"]

    subgraph lib["telegram_reader lib (src/lib.rs)"]
        config["config.rs<br/>config.yml · chats · senders"]
        session["session.rs<br/>SessionLock (fs2) + grammers Client"]
        commands["commands/*<br/>read · digest · crm · moderate · react ·<br/>like · hunt · analyze · n8n · linear · search"]
        integrations["integrations/*<br/>openai · gemini · claude · ollama ·<br/>whisper · yandex_tts"]
        analysis["analysis/*<br/>embeddings · vector_db (Qdrant) ·<br/>graph_db (Neo4j)"]
        lightrag["lightrag/*<br/>chunker · entity_extractor ·<br/>graph · retriever"]
        prompts["prompts.rs<br/>prompts/*.md loader"]
        reactions["reactions.rs"]
        outp["export.rs · chat.rs · linear.rs"]
        metrics["metrics.rs<br/>Prometheus"]
        error["error.rs<br/>thiserror"]
        python["python.rs<br/>PyO3 / MCP"]
    end

    main --> commands
    commands --> session
    commands --> integrations
    commands --> analysis
    commands --> prompts
    commands --> reactions
    commands --> outp
    commands --> config
    analysis --> lightrag
    session --> config
    main -. serves .-> metrics
    commands -. Result/Error .-> error
    python -. embeds .-> lib
```

---

## 4. 🔐 Sequence — Session Initialization & Locking

*Purpose:* the **exclusive single-session** invariant — only one process may use
the Telegram session at a time, enforced by a file lock. *Placement:*
`docs/session-and-locking.md` or this doc.

```mermaid
sequenceDiagram
    actor U as Operator
    participant P1 as Process A
    participant L as SessionLock (fs2)
    participant FS as telegram_reader.lock
    participant C as Client (grammers)
    participant S as telegram_session.session
    participant TG as Telegram (MTProto)
    participant P2 as Process B

    Note over U,S: One-time: telegram_reader init-session
    U->>P1: init-session
    P1->>C: connect + sign in (phone/code)
    C->>TG: auth
    TG-->>C: authorized
    C->>S: persist SqliteSession

    Note over P1,P2: Every later command
    U->>P1: telegram_reader read <chat>
    P1->>L: acquire()
    L->>FS: flock (exclusive)
    FS-->>L: held
    P1->>C: get_client() + SqliteSession
    C->>TG: reuse session
    par Contending process
        P2->>L: acquire()
        L->>FS: flock (exclusive)
        FS-->>P2: BLOCKED (lock busy)
    end
    P1-->>L: drop ⇒ release
    FS-->>P2: now granted
```

---

## 5. 📤 Flowchart — Chat Export / Read Pipeline

*Purpose:* the `read` / `export` command end-to-end, including hygiene deletions.
Mirrors `src/commands/read.rs` + `src/export.rs`. *Placement:*
`docs/commands/read-export.md`.

```mermaid
flowchart TB
    start(["telegram_reader read &lt;chat&gt; [--limit N] [--delete-unengaged]"])
    lock["Acquire SessionLock + connect client"]
    resolve["Resolve chat via config.yml"]
    fetch["Fetch messages (newest → oldest)"]
    more{"collected &lt; limit<br/>and more available?"}
    next["Take next message"]
    decide["Per-message decision tree<br/>(see diagram 10)"]
    writer["ExportWriter:<br/>append Markdown + create media dir"]
    summary["Print counts:<br/>exported / deleted"]

    start --> lock --> resolve --> fetch --> more
    more -- yes --> next --> decide --> writer --> more
    more -- no --> summary
```

---

## 6. 🤖 Sequence — Auto-Responder / AI Reply

*Purpose:* the `auto-answer` flow — detect an incoming message, load a prompt,
call the LLM, reply. Mirrors `src/commands/autoanswer.rs`. *Placement:*
`docs/commands/auto-answer.md`.

```mermaid
sequenceDiagram
    autonumber
    participant Loop as auto-answer loop
    participant C as Client (grammers)
    participant TG as Telegram
    participant PR as prompts/*.md
    participant AI as OpenAI (OPENAI_MODEL)

    Loop->>C: iter_dialogs / iter_messages
    C->>TG: poll updates
    TG-->>C: new inbound user message
    alt message addressed to us
        Loop->>PR: load system prompt
        PR-->>Loop: system text
        Loop->>AI: chat completion (system + user)
        AI-->>Loop: assistant reply
        Loop->>C: msg.reply(reply)
        C->>TG: send
    else ignore
        Loop-->>Loop: skip
    end
```

---

## 7. 💬 Component — Bot Architecture

*Purpose:* how a teloxide bot wires the Bot API, the shared library, MySQL
persistence, and A/B prompt experiments. Mirrors `src/bin/bots/sales_bot.rs`.
*Placement:* `docs/bots/architecture.md`.

```mermaid
flowchart TB
    tgapi[("Telegram Bot API")]

    subgraph bot["sales_bot (teloxide)"]
        disp["Dispatcher / update handler"]
        ab["A/B engine<br/>prompt_variants() →<br/>get_or_assign_variant()"]
        gen["Reply generation"]
    end

    subgraph shared["shared: telegram_reader lib"]
        oai["integrations/openai"]
        prompts["prompts.rs"]
    end

    subgraph db["BotDb (mysql_async)"]
        users[("bot_users")]
        sessions[("bot_sessions")]
        messages[("bot_messages")]
        experiments[("bot_experiments")]
    end

    tgapi --> disp
    disp --> ab
    ab --> experiments
    disp --> gen
    gen --> prompts
    gen --> oai
    gen --> tgapi
    disp --> users
    disp --> sessions
    disp --> messages
```

---

## 8. 🔎 Flowchart — LightRAG Indexing & Retrieval

*Purpose:* turning chat history into a hybrid (vector + graph) knowledge base and
answering against it. Mirrors `src/lightrag/*` + `src/analysis/*`. *Placement:*
`docs/lightrag/pipeline.md`.

```mermaid
flowchart LR
    subgraph index["Indexing"]
        src[("Chat export / MySQL history")]
        chunk["lightrag/chunker"]
        embed["analysis/embeddings"]
        ents["lightrag/entity_extractor"]
        qdrant[("Qdrant<br/>analysis/vector_db")]
        neo4j[("Neo4j<br/>analysis/graph_db")]
    end
    subgraph query["Retrieval"]
        q["User question"]
        retr["lightrag/retriever<br/>(vector + graph fusion)"]
        ctx["Assembled context"]
        llm["LLM (integrations/*)"]
        ans["Answer"]
    end

    src --> chunk
    chunk --> embed --> qdrant
    chunk --> ents --> neo4j
    q --> retr
    qdrant --> retr
    neo4j --> retr
    retr --> ctx --> llm --> ans
```

---

## 9. 🧭 Flowchart — SPIDER-SOLO Development Protocol

*Purpose:* the single-agent methodology used in `codev/` — each feature flows
through phases and produces exactly three linked documents (spec, plan, review).
Mirrors `codev/protocols/spider-solo/protocol.md`. *Placement:*
`docs/process/spider-solo.md`.

```mermaid
flowchart TB
    subgraph phases["Phases (self-review at every checkpoint — no multi-agent)"]
        direction LR
        S["S — Specify<br/>explore problem & options"]
        P["P — Plan<br/>structured decomposition"]
        I["I — Implement<br/>build increment"]
        D["D — Defend<br/>tests / TDD"]
        E["E — Evaluate<br/>self-review vs spec"]
        R["R — Review<br/>refine / revise"]
        S --> P --> I --> D --> E --> R
        E -. gaps found .-> I
        R -. next increment .-> I
    end

    subgraph docs["Artifacts (shared id, e.g. 0007-*)"]
        spec["codev/specs/NNNN-*.md"]
        plan["codev/plans/NNNN-*.md"]
        review["codev/reviews/NNNN-*.md"]
    end

    S --> spec
    P --> plan
    R --> review
    spec -. informs .-> plan -. informs .-> review
```

---

## 10. 🌳 Flowchart — Message Processing Decision Tree

*Purpose:* the per-message rules applied during read/moderation — hygiene
deletions, engagement filtering, media download. Mirrors `read.rs` +
the "download media for popular posts" feature. *Placement:*
`docs/commands/message-rules.md`.

```mermaid
flowchart TB
    msg(["Incoming message"])
    zoom{"Contains Zoom<br/>invite link?"}
    delz["🗑️ delete_messages()"]
    uneng{"--delete-unengaged<br/>enabled?"}
    react{"reactions == 0<br/>(unengaged)?"}
    delu["🗑️ delete (low engagement)"]
    keep["Keep → export to Markdown"]
    popular{"reactions ≥<br/>media threshold?"}
    media["⬇️ download media"]
    done(["Next message"])

    msg --> zoom
    zoom -- yes --> delz --> done
    zoom -- no --> uneng
    uneng -- yes --> react
    react -- yes --> delu --> done
    react -- no --> keep
    uneng -- no --> keep
    keep --> popular
    popular -- yes --> media --> done
    popular -- no --> done
```

---

## 11. 🗄️ ER Diagram — MySQL Bot / Analytics Schema

*Purpose:* the relational model behind teloxide bots and A/B experiments.
Columns are exact (`src/bin/bots/sales_bot.rs`). Relationships are
application-enforced via indexed keys (not hard FOREIGN KEYs). *Placement:*
`docs/data/schema.md`.

```mermaid
erDiagram
    bot_users ||--o{ bot_sessions : has
    bot_users ||--o{ bot_messages : exchanges
    bot_users ||--o{ bot_experiments : assigned
    bot_sessions ||--o{ bot_experiments : scopes
    bot_users {
        bigint id PK "Telegram user id"
        varchar username
        varchar first_name
        varchar last_name
        varchar language_code
        tinyint is_premium
        tinyint is_bot
    }
    bot_sessions {
        bigint id PK
        bigint user_id FK
        varchar bot_name
        varchar state
        tinyint is_active
        timestamp session_start
        timestamp session_end
    }
    bot_messages {
        bigint id PK
        bigint telegram_message_id
        bigint user_id FK
        varchar bot_name
        varchar direction
        text message_text
        bigint reply_to_message_id
        timestamp created_at
    }
    bot_experiments {
        bigint id PK
        varchar bot_name
        varchar experiment_name
        bigint session_id FK
        bigint user_id FK
        varchar variant
        tinyint conversion
        int conversion_value
        timestamp assigned_at
        timestamp closed_at
    }
```

---

## 12. 🧱 Architecture Layers / Module Dependency

*Purpose:* the dependency direction — higher layers depend only downward.
Useful for reasoning about change blast-radius. *Placement:* `docs/architecture.md`.

```mermaid
flowchart TB
    subgraph L1["Presentation"]
        cli2["CLI (main.rs)"]
        bots2["Bots (src/bin/bots)"]
        tools2["Tools (src/bin/<category>)"]
    end
    subgraph L2["Application — commands/ & bot handlers"]
        cmd2["read · digest · crm · moderate · react · hunt · analyze · n8n · linear"]
    end
    subgraph L3["Domain capabilities"]
        an["analysis · lightrag"]
        rx["reactions"]
        pr["prompts"]
        ex["export"]
    end
    subgraph L4["Integration / drivers"]
        ai2["AI clients (integrations/*)"]
        tgc["Telegram client (grammers/teloxide)"]
        sql["MySQL (mysql_async)"]
        vdb["Qdrant / Neo4j"]
    end
    subgraph L5["Infrastructure / cross-cutting"]
        sess["session (lock)"]
        cfg["config"]
        met["metrics"]
        err["error"]
    end

    L1 --> L2 --> L3 --> L4 --> L5
    L2 -. uses .-> L5
    L3 -. uses .-> L4
```

---

## 📎 Usage Instructions

**Embedding.** GitHub renders ```` ```mermaid ```` blocks natively — no build
step. Link the diagrams from `README.md` and per-subsystem docs; each section
above lists a suggested *Placement* file if you want to split this textbook into
focused pages.

**Editing.** Paste any block into the [Mermaid Live Editor](https://mermaid.live)
to tweak and re-export. Keep node IDs short and put prose in quoted labels.

**Exporting static images (for slides / PDF / printed textbook):**
```bash
npx -y @mermaid-js/mermaid-cli -i docs/architecture.md -o build/architecture.md
# or per-diagram:  mmdc -i diagram.mmd -o diagram.svg -t neutral
```

**MkDocs.** Add the Material theme + `mkdocs-mermaid2-plugin`, then drop these
files under `docs/`:
```yaml
# mkdocs.yml
theme: { name: material }
markdown_extensions:
  - pymdownx.superfences:
      custom_fences:
        - { name: mermaid, class: mermaid, format: !!python/name:pymdownx.superfences.fence_code_format }
```

**Typst / PDF textbook.** Render each diagram to SVG with `mmdc` and `#image()`
them into a Typst document; pair each figure with the *Purpose* line as a caption.

**Suggested screenshots to add later:** a real `read` run console, a Grafana
panel fed by `metrics.rs`, and a Qdrant/Neo4j browser view of an indexed chat.

---

## 🎁 Bonus — PlantUML for the deployment view

Mermaid C4 covers context/containers well; for a richer **deployment** view
(nodes, artifacts, ports) PlantUML's C4 library is stronger. Drop this in a
`.puml` and render with the PlantUML server or CLI:

```plantuml
@startuml deployment
!include https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Deployment.puml

Deployment_Node(host, "Linux host", "systemd") {
  Deployment_Node(rt, "Rust runtime", "Tokio") {
    Container(cli, "telegram_reader CLI", "Rust")
    Container(bots, "Bots", "teloxide")
    Container(mon, "n8n_monitor / n8n_backup", "Rust · systemd unit")
  }
  ContainerDb(sqlite, "telegram_session.session", "SQLite")
}
Deployment_Node(data, "Data services") {
  ContainerDb(mysql, "MySQL", "bot_* tables")
  ContainerDb(qdrant, "Qdrant", "vectors")
  ContainerDb(neo4j, "Neo4j", "graph")
}
Deployment_Node(cloud, "External") {
  System_Ext(tg, "Telegram", "MTProto/Bot API")
  System_Ext(ai, "AI Providers", "OpenAI/…")
}

Rel(cli, sqlite, "file-locked session")
Rel(bots, mysql, "mysql_async")
Rel(cli, qdrant, "vectors")
Rel(cli, neo4j, "entities")
Rel(cli, tg, "grammers")
Rel(bots, tg, "teloxide")
Rel(cli, ai, "HTTPS")
@enduml
```
