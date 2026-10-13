# Spec: TUI Shell for Kubernetes (Rust k9s)

**ID:** 0005-rust-k9s  
**Status:** Draft  
**Date:** 2025-11-24  
**Author:** Project Team

---

## 1. Problem
- `kubectl` is verbose and flag-heavy.
- Incidents require a fast cluster overview: pods, logs, events, restarts.
- Need a cross-platform TUI shell without heavy deps (like k9s) but written in Rust.
- Security: strict kubeconfig context control, minimal RBAC ops, no token leaks in logs.

## 2. Goals
- Provide a TUI shell for key kubectl operations: view resources, logs, describe, exec, port-forward.
- Work on top of existing kubeconfig (multi-context/namespace support).
- Responsive UI (low input/render latency) and stable on large clusters (1000+ pods).
- Built-in safety: block dangerous ops by default, explicit confirmation on delete/exec, mask secrets in UI/logs.

## 3. User scenarios
- **On-call overview:** open k9s-rs, pick context, browse namespace, find pod by name/label, open live logs, filter errors.
- **Pod diagnostics:** describe pod, check events, open container logs, port-forward to service/pod.
- **Admin:** switch namespace, recreate pod (delete + notice), watch rollout status for Deployment/StatefulSet/DaemonSet.
- **Triage:** quickly find CrashLoopBackOff pods, view recent events, compare restarts, dump log sample to file.

## 4. Scope
- **Included:** Pods, Deployments, StatefulSets, DaemonSets, Jobs/CronJobs, Services, Ingresses, Nodes, Events, Secrets/ConfigMaps (metadata only; Secret contents hidden).
- **Operations:** list/watch, describe, logs (tail/follow, choose container), exec (interactive shell with warning), port-forward, delete (with confirmation), scale (Deployment/StatefulSet) with RBAC check.
- **Navigation:** Vim-like/arrow keys, fuzzy search, contextual panes (resources, details, events, logs).
- **Settings:** kubeconfig override, prefer-context/namespace, hotkeys, themes.
- **Observability:** minimal telemetry off by default; structured logs to stderr (JSON).
- **Not in MVP:** manifest editing, apply/patch, resource creation, Helm/ArgoCD console.
- **Priorities (MVP → P1):**

| Feature | Details | Priority |
|---------|---------|----------|
| List/Watch resources | Tables for Pods/Deployments/Services/Nodes, quick filters | P0 |
| Logs | Tail/follow, container selection, toggle timestamp, grep/regex, export to file | P0 |
| Describe/Events | Key fields + recent events per resource | P0 |
| Context/Namespace switch | List from kubeconfig, quick hotkeys | P0 |
| Exec | Interactive shell with warning, container selection | P0 |
| Port-forward | Port hints, active tunnel management | P0 |
| Delete/Scale | Confirmation, RBAC check, safe termination | P0 |
| Themes/Keybinds | Light/dark, custom bindings, save config | P1 |
| Profiles | Saved filters/namespace/context | P1 |
| Quick-dump | Export selected logs/events to file from UI | P1 |

## 5. Functional requirements
- Multiple contexts and namespaces with quick switching (hotkeys).
- Multi-container pod support (choose container for logs/exec).
- Filtering/search: by name, namespace, status, labels; fuzzy search; CrashLoopBackOff/Failed/NotReady filters.
- Logs: tail N, follow, timestamp toggle, grep/regex filter, export to file.
- Exec: interactive session, explicit warning/confirmation, default shell config (/bin/sh, /bin/bash).
- Port-forward: port hints from Service/Pod, show active tunnels, safe teardown.
- Describe and Events: show key fields (conditions, restarts, node, image), recent events.
- RBAC-aware: if operation forbidden, show clear message without crashing.
- Themes: light/dark, customizable colors; readable for color blindness.

## 6. Non-functional requirements
- Performance: startup < 500 ms on local kubeconfig, table refresh ≤ 200 ms on changes; smooth scroll.
- Scalability: works on clusters with 1000+ pods per namespace; efficient watch (kube-runtime/stream).
- Cross-platform: Linux, macOS, Windows (terminal/input support without ncurses dependency).
- Reliability: UI must not crash on API errors; display and recover from network issues.
- Security: do not log tokens/secrets; hide Secret contents (metadata/name/type only); exec/delete require confirmation; config should avoid storing secrets.

## 7. Architecture (high level)
- **Core:** async runtime (tokio) + kube-rs client (watch/list/exec/log/port-forward).
- **UI:** ratatui/crossterm (or tui-rs) with components: ResourceTable, DetailPane, LogsPane, StatusBar, CommandPalette.
- **State management:** central store (context, namespace, filters, selection), subscriptions to watch streams.
- **Cache:** in-memory resource cache with incremental updates (watch), degrade to list on failures.
- **Operations:** services for logs, exec, port-forward with timeout/cancellation control.
- **Config:** toml/yaml in XDG/OS-specific path (~/.config/k9s-rs/config.{toml,yml}); load kubeconfig from $KUBECONFIG or defaults.
- **Security:** redaction layer for secrets, block writing sensitive data to logs/files.
- **Components:** ConfigLoader → KubeClientFactory → WatchService (per kind) → Store (Arc<RwLock<>>) → UI Renderer → ActionQueue (operations) → Feedback into Store.
- **Data flow:**
```
CLI args/env
   ↓
Load config (config.yml + kubeconfig)
   ↓
KubeClientFactory (context/namespace)
   ↓
WatchService (per resource kind)
   ↓
Store (state + cache)
   ↓
UI Renderer (tui)
   ↓
User actions → ActionQueue → operations → Store
```

## 8. Interfaces
- **CLI:** `k9s-rs [--context ctx] [--namespace ns] [--kubeconfig path] [--log-level info]`
- **Keybindings:**
  - Navigation: ↑ ↓ j k (tables), Tab (switch pane), / (search)
  - Actions: d (describe), l (logs), e (events), x (exec), p (port-forward), DEL (delete), s (scale), g (switch context), n (namespace)
  - Logs: f (filter), t (toggle timestamp), E (export)
  - Help: ?
- **Config file:**
```toml
[ui]
theme = "dark"
keybinds = { logs_follow = "F", delete = "Ctrl+D" }

[kube]
prefer_context = "prod-cluster"
prefer_namespace = "default"
```

## 9. Telemetry and logging
- Structured JSON logs to stderr (level, msg, target, context/namespace).
- Optional metrics endpoint (prometheus) for resource counts, API latency, errors.
- No PII or kube tokens in logs; redact secrets.

## 10. Testing
- Unit tests for parsers (resource → view models), filters, config loader.
- Integration tests with fake kube API (kube-test/MockService) for list/watch/logs/exec errors.
- Snapshot tests for UI components (ratatui) if feasible.
- Performance test on large mocked namespace (1000 pods) to ensure smooth navigation.

## 11. Risks
- Large clusters causing high CPU/memory → pagination + throttled watch updates.
- RBAC errors leading to blank UI → friendly errors per operation.
- Platform-specific terminal quirks (Windows) → test crossterm compatibility.
- Port-forward leaks if not cleaned up → track and auto-close on exit.

## 12. MVP acceptance
- List/watch Pods/Deployments/Services/Nodes works on real cluster via kubeconfig.
- Logs tail/follow with container selection and search.
- Describe/events view for pods/resources.
- Context/namespace switching works.
- Exec with confirmation; port-forward with teardown.
- No token leaks in logs; app handles API errors without crashing.
