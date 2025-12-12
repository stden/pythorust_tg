# Spec: Stories API (publishing and viewing stories)

**ID:** 0006-stories-api
**Status:** Draft
**Author:** Codex CLI
**Date:** 2025-02-24

---

## 1. Problem
- The toolkit has support for chats, reactions, and media, but no support for working with Telegram Stories (creation and viewing).
- Content managers publish stories manually using phones and only see native analytics.
- Analytics and story export for backups/AI processing is impossible through current CLI/bots.

## 2. Goals and Scope
**P0 (minimum):**
- Publishing a story from CLI (Rust) with photo/video + caption.
- Privacy settings (all, contacts, selected users/exclusions).
- Viewing list of stories from a user/channel, marking as read.
- Downloading media and story metadata to a local folder (`stories/<peer>/`).

**P1 (after P0):**
- Reposting a story to another chat/channel.
- Replying to a story with option to send to DM.
- Basic metrics (views, reactions) in JSON/Markdown.

**Out of scope:** highlights, editing (stickers/text layers), interactive elements (polls/quizzes), masks/AR.

## 3. User Scenarios
- **Content operator:** `cargo run -- stories publish --peer @channel --photo cover.jpg --caption "Launch"` publishes announcement to channel without phone.
- **Analyst:** `cargo run -- stories fetch --peer @channel --limit 10 --download` saves latest stories for reporting and forwards metadata to AI analysis.
- **Support/bot:** retrieves user story, marks as viewed, saves copy to archive if needed.

## 4. Solution Design
### Architectural Components
- **stories service** (`src/stories.rs`): wrapper over grammers `stories.*` methods (send, get, read, archive).
- **CLI layer** (`stories` subcommand in `src/main.rs` + `src/commands/stories.rs`): argument parsing, service calls, output.
- **storage**: local directory `stories/<peer>/` for media and `metadata.json` with id, expires_at, caption, views, reactions.
- **config re-use**: peer lookup from `config.yml`/alias or raw @username/id, as in other commands.

### CLI Interface (P0)
```
cargo run -- stories publish \
  --peer @channel_or_user \
  --photo path.jpg | --video clip.mp4 \
  --caption "Text" \
  [--ttl-hours 24] \
  [--privacy all|contacts|close-friends|include=id1,id2|exclude=id3,id4]

cargo run -- stories fetch \
  --peer @channel_or_user \
  [--limit 10] \
  [--download] \
  [--mark-read]
```
- `--photo/--video` are mutually exclusive; one media file per story for now.
- `--privacy` maps to `stories.sendStory` send options (allow/deny lists).
- `--download` saves file(s) to `stories/<peer>/<story_id>.<ext>` and metadata.
- `--mark-read` calls `stories.readStories` after fetching.

### Server Layer (Rust)
- Function `publish_story(client, peer, media: StoryMedia, options: StoryOptions) -> Result<StoryInfo>`.
- Function `fetch_stories(client, peer, limit, download: bool, mark_read: bool) -> Result<Vec<StoryInfo>>`.
- `StoryMedia`: `Photo { path } | Video { path, duration, spoiler? }`.
- `StoryOptions`: `ttl_hours`, `caption`, `privacy: PrivacyScope`, `silent: bool` (default false).
- `StoryInfo`: id, peer, posted_at, expires_at, caption, views (if available), reactions (if available), local_paths (optional).
- Rate limit/lock handling as in other commands (reuse `SessionLock`).

### Storage/Formats
- Metadata: `stories/<peer>/metadata.json` with latest `fetched_at`, array of `StoryInfo`.
- Media: file with original extension; name = story_id.
- Logs: `tracing` with key fields (`peer`, `story_id`, `privacy`).

### Errors and Edge Cases
- Peer error: explicit message, hint to check `config.yml`.
- Unsupported media: validate extensions (jpg/png/mp4) before API call.
- Large files: check size and early fail with advice (Telegram limit 2GB, but P0 can restrict to e.g. 200MB).
- TTL: default 24h if not specified.
- No permission to publish to channel: clearly report and exit.

## 5. Testing and Acceptance
- Integration smoke test: upload photo story to test private channel and read back (`--limit 1`, `--download`).
- Unit mocks: validation of privacy/ttl/CLI argument parsing.
- Manual verification: open Telegram client, ensure story is published and visible in `fetch`.
- Acceptance P0:
  - Publishing photo story with caption and privacy=contacts completes without errors.
  - `fetch` returns list of ids, expiration times, and saves file with `--download`.
  - `--mark-read` stops returning story as unread (if API provides status).

## 6. Open Questions
- Is support for text-only stories (without media) needed in P0?
- Is sending both photo+video simultaneously (Carousel) required, or strictly one media file?
- Is Python fallback (Telethon) needed if Rust is unavailable?
- What file size and video bitrate limits should we consider acceptable in CLI?

## 7. Risks
- Incomplete Stories support in current `grammers` version (need to check methods in 0.8).
- Telegram restrictions on publishing to public channels without privileges.
- Potential ban/flood limits with frequent story publishing/reading.
