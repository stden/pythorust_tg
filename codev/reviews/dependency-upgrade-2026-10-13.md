# Dependency upgrade review — 2026-10-13

All twelve current Dependabot branches were merged before the dependency audit.
Version data was queried directly from crates.io, PyPI and npm. All direct
dependencies use their latest stable releases; transitive dependencies were
updated to the newest versions permitted by their upstream constraints.

| Registry | Library | Before | After |
|---|---|---|---|
| cargo | grammers-client | 0.8 | 0.10.0 |
| cargo | grammers-session | 0.8 | 0.10.0 |
| cargo | grammers-tl-types | 0.8 | 0.10.0 |
| cargo | grammers-mtsender | 0.8 | 0.10.0 |
| cargo | tokio | 1.52 | 1.53.2 |
| cargo | clap | 4.6 | 4.6.7 |
| cargo | serde | 1.0 | 1.0.229 |
| cargo | serde_json | 1.0 | 1.0.154 |
| cargo | serde_yaml | 0.9 | 0.9.34 |
| cargo | anyhow | 1.0 | 1.0.104 |
| cargo | thiserror | 2.0 | 2.0.21 |
| cargo | tracing | 0.1 | 0.1.44 |
| cargo | tracing-subscriber | 0.3 | 0.3.23 |
| cargo | chrono | 0.4 | 0.4.45 |
| cargo | prometheus | 0.14 | 0.14.0 |
| cargo | http-body-util | 0.1 | 0.1.5 |
| cargo | hyper-util | 0.1 | 0.1.21 |
| cargo | bytes | 1.12 | 1.12.1 |
| cargo | fs2 | 0.4 | 0.4.3 |
| cargo | walkdir | 2.5 | 2.5.0 |
| cargo | reqwest | 0.12 | 0.13.5 |
| cargo | async-openai | 0.32 | 0.42.2 |
| cargo | dotenvy | 0.15 | 0.15.7 |
| cargo | regex | 1.12 | 1.13.1 |
| cargo | base64 | 0.22 | 0.23.1 |
| cargo | hyper | 1 | 1.12.0 |
| cargo | qdrant-client | 1.18 | 1.19.0 |
| cargo | neo4rs | 0.8 | 0.8.0 |
| cargo | uuid | 1.23 | 1.28.0 |
| cargo | futures | 0.3 | 0.3.34 |
| cargo | once_cell | 1.21 | 1.21.4 |
| cargo | csv | 1.4 | 1.4.0 |
| cargo | schemars | 1.2 | 1.2.3 |
| cargo | mysql_async | 0.36 | 0.37.1 |
| cargo | teloxide | 0.17 | 0.17.0 |
| cargo | rand | 0.8 | 0.10.3 |
| cargo | kube | 3.1 | 4.2.0 |
| cargo | k8s-openapi | 0.27 | 0.28.0 |
| cargo | pyo3 | 0.24 | 0.29.3 |
| cargo | tokio-test | 0.4 | 0.4.6 |
| cargo | httpmock | 0.8 | 0.8.3 |
| cargo | tempfile | 3.27 | 3.27.0 |
| cargo | criterion | 0.5 | 0.8.2 |
| pypi | telethon | telethon>=1.44.0 | telethon>=1.45.0 |
| pypi | openai | openai>=2.44.0 | openai>=3.29.0 |
| pypi | mcp | mcp>=1.28.1,<2.0.0 | mcp>=2.3.0 |
| pypi | requests | requests>=2.34.2 | requests>=2.34.2 |
| pypi | pytest | pytest>=9.1.1 | pytest>=9.1.1 |
| pypi | pytest-cov | pytest-cov>=7.1.0 | pytest-cov>=7.1.0 |
| pypi | pytest-asyncio | pytest-asyncio>=1.4.0 | pytest-asyncio>=1.4.0 |
| pypi | python-dotenv | python-dotenv>=1.2.2 | python-dotenv>=1.2.4 |
| pypi | behave | behave>=1.3.3 | behave>=1.3.3 |
| pypi | kurigram | kurigram>=2.2.23 | kurigram>=2.2.26 |
| pypi | PyYAML | PyYAML>=6.0.3 | PyYAML>=6.0.3 |
| pypi | httpx | httpx>=0.28.1 | httpx>=0.28.1 |
| pypi | respx | respx>=0.23.1 | respx>=0.23.1 |
| pypi | ruff | ruff>=0.15.20 | ruff>=0.17.0 |
| pypi | pymysql | pymysql>=1.2.0 | pymysql>=1.2.3 |
| pypi | aiohttp | aiohttp>=3.14.1 | aiohttp>=3.14.5 |
| pypi | boto3 | boto3>=1.43.37 | boto3>=1.43.112 |
| pypi | mysql-connector-python | mysql-connector-python>=9.7.0 | mysql-connector-python>=26.7.0 |
| pypi | cryptography | cryptography>=49.0.0 | cryptography>=50.0.2 |
| pypi | maturin | maturin>=1.14.1,<2.0 | maturin>=1.15.0,<2.0 |
| npm | @playwright/test | ^1.56.1 | ^1.64.0 |

## Compatibility work

- KISS: retained the existing layout and changed API call sites directly.
- DRY: one async peer-reference conversion replaces repeated conversion/error mapping.
- grammers imports and session/update initialization use the 0.10 async API.
- PyO3 uses explicit Clone extraction and Py<PyAny>; rand and Criterion use current APIs.
- reqwest enables its separate form serialization feature.
- MCP uses MCPServer/Context from the MCP 2 SDK.
- The core2 vendor patch was removed because grammers 0.10 no longer depends on it.
- Cargo.lock and uv.lock are tracked, and CI uses locked/frozen resolution.
- Ruff rule selection and Python/config discovery are explicit, preserving the previous CI policy.
- The project selects the installed stable Rust toolchain (1.99.0 during verification).

## Upstream constraint

grammers-crypto 0.10.0 requires the glass_pumpkin 2 prerelease. rc1 changes
safe_prime::check to return Result and changes BigUint versions, breaking the
published grammers source. Cargo.lock deliberately retains 2.0.0-rc0.
After a future unrestricted cargo update, restore this version with:
cargo update -p glass_pumpkin --precise 2.0.0-rc0.

## Verification

Final command outcomes are recorded in AGENTS.md, section 8.
