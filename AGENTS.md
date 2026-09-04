# Game Save Utility - AI Agent Engineering Guide

This document is the primary architecture and engineering reference for AI agents and human contributors working on **Game Save Utility**.

---

## 1. Project Overview

- **Project Goal**: A lightweight, reliable Windows desktop application for managing, backing up, and restoring local save data for single-player games.
- **Tech Stack**:
  - **Language**: Rust (edition 2021)
  - **GUI Framework**: `eframe` / `egui` (0.27)
  - **Target OS**: Windows (`x86_64-pc-windows-msvc` / `x86_64-pc-windows-gnu`)
  - **Distribution**: Standalone portable single executable (`.exe`), with embedded application icons, native Windows subsystem (`windows_subsystem = "windows"` in release mode to eliminate console pop-ups), and zero runtime dependencies (no Electron, .NET Runtime, Python, or Node required).
- **Runtime Data Directory**:
  - `%LOCALAPPDATA%\GameSaveUtility` containing:
    - `config.json` (and `config.json.bak` for atomic resilience)
    - `logs/` (timestamped operational logs)
    - `backups/` (default root for managed game saves)
  - Seamless migration logic automatically detects and migrates legacy `GameSaveBackupTool` data directories and location pointers.

---

## 2. Core Architecture & Module Responsibilities

The codebase in `src/` is partitioned into clear functional modules:

```
src/
├── main.rs          # Application entry point & native window initialization
├── app.rs           # Core state machine (GameSaveApp) and runtime event loop
├── backup.rs        # Backup creation, restore orchestration, staging, and cleanup
├── snapshot.rs      # Content-addressable storage (CAS), deduplication, and fingerprinting
├── archive.rs       # ZIP format compression and extraction engine
├── config.rs        # Configuration persistence, migration, and backup recovery
├── models.rs        # Domain types, application state models, and localized error definitions
├── presets.rs       # Centralized database of game save paths and Steam App ID mappings
├── steam.rs         # Steam library detection, manifest parsing, and Cloud cache conflict check
├── scheduler.rs     # Timer calculations for periodic backups and change detection
├── cloud.rs         # Cloud save modification time comparison and desync warnings
├── tray.rs          # Windows system tray integration via native winapi/shellapi
├── fs_utils.rs      # Safe filesystem utilities (atomic helpers, path expansions, validations)
├── logger.rs        # Daily rotating logger under app data logs/
├── i18n.rs          # Bilingual localization (Simplified Chinese and English)
├── help.rs          # Standalone documentation and help modal dialog
└── ui/
    ├── mod.rs       # Master layout composition, top toolbar, status bar, and split panels
    ├── game_list.rs # Left sidebar: game list management, manual add, and Steam scan triggers
    ├── backup_list.rs # Right panel: reverse-chronological backup history, actions, and selection
    └── dialogs.rs   # Modal dialogs (game edit, Steam scan table, confirmations, settings)
```

### Module Responsibilities Detail

- **`main.rs`**: Configures native window options via `eframe::NativeOptions`, loads the embedded 256x256 icon (`assets/app-icon-256.png`), restores persisted window geometry (size, min constraints, maximized state), and kicks off the eframe loop.
- **`app.rs`**: Encapsulates `GameSaveApp`. Manages active game selections, background timers for scheduled auto-backup and save alteration checks, global status messaging (`StatusMessage`), keyboard shortcuts (`Ctrl+S` for quick backup, `Ctrl+R` for restore), and window close/tray behavior.
- **`backup.rs`**: Orchestrates backup operations across storage modes (`IncrementalFolder`, `ZipArchive`, `FolderCopy`). Handles temporary directory staging, `metadata.json` generation, atomic directory commits, pre-restore automatic safety backups, two-phase restore staging with rollback, and retention cleanup based on `max_backups`.
- **`snapshot.rs`**: Content-addressable storage (CAS) engine. Employs SHA-256 to hash individual files within save directories, deduplicates chunks into `.objects/`, generates `manifest.json`, and provides the two-tier change detection system (`QuickDirectoryFingerprint` and `calculate_directory_snapshot_hash`).
- **`archive.rs`**: Handles ZIP archive compression and decompression when a game profile selects ZIP backup mode.
- **`config.rs`**: Handles loading, saving, and migrating `AppConfig`. Creates atomic `.bak` snapshots before writing `config.json`, detects corruption, and safely migrates legacy directories from `%LOCALAPPDATA%\GameSaveBackupTool` to `%LOCALAPPDATA%\GameSaveUtility`.
- **`models.rs`**: Central domain definitions including `AppConfig`, `GameConfig`, `BackupEntry`, `BackupMetadata`, `Language`, `BackupStorageMode`, and `AppError`. Contains `user_message_for_language` to ensure 100% localized, user-safe error text.
- **`presets.rs`**: Centralized repository of known game save paths and Steam App ID resolutions. Acts as the single source of truth for both manual preset selection and Steam library matching to avoid heuristic duplication.
- **`steam.rs`**: Scans Steam installations by locating `libraryfolders.vdf`, parsing game manifests (`appmanifest_<id>.acf`), discovering local Steam Cloud caches under `userdata/<steam_id>/<app_id>`, and comparing modification timestamps against local saves.
- **`scheduler.rs`**: Pure calculation helpers for auto-backup intervals (supporting minutes and hours) and scheduled background verification triggers.
- **`cloud.rs`**: Evaluates timestamps between local game save paths and Steam Cloud sync caches to alert users of potential save desynchronization or overwrite risks.
- **`tray.rs`**: Windows platform tray integration. Creates system tray icon, handles left-click double-click window restore, right-click context menu (Show / Exit), and notifications.
- **`fs_utils.rs`**: Filesystem safety abstractions: recursive directory copy, safe recursive removal, environment variable expansion (`%LOCALAPPDATA%`, `%APPDATA%`, `%USERPROFILE%`), path sanitization, and recursion prevention check (`is_same_or_child_path`).
- **`logger.rs`**: Lightweight, thread-safe logger writing timestamped daily log files into `%LOCALAPPDATA%\GameSaveUtility\logs\`.
- **`i18n.rs`**: Localization registry for UI labels and messages with strict type-checked keys (`Text` enum) across `Language::ZhCn` and `Language::EnUs`.
- **`help.rs`**: In-app documentation browser rendered as an independent detachable modal dialog with category tree navigation and search capabilities.
- **`ui/`**: Egui presentation layer divided into clean layout components (`ui/mod.rs`), game list sidebar (`ui/game_list.rs`), backup history table (`ui/backup_list.rs`), and interactive dialogs (`ui/dialogs.rs`).

---

## 3. Key Safety Invariants & Engineering Rules

Any modification to the codebase must strictly adhere to the following safety invariants:

### 3.1. Backup Staging & Atomic Commits
- **Never write directly to final destination**: Backups must be staged in a temporary directory (`.tmp_{dir_name}_{uuid}`) under the game's backup root.
- **Validate before commit**: Only after all files or CAS objects are stored, and `metadata.json` has been successfully serialized and verified, the directory is atomically renamed to its final target path (`unique_child_path`).
- **Clean on failure**: If any stage of backup creation fails, the temporary staging directory must be purged immediately, leaving no incomplete or corrupted backup nodes.
- **Recursion protection**: Before backup begins, `is_same_or_child_path` must check that the backup destination is not within the game's save folder, preventing unbounded recursive copying.

### 3.2. Restore Staging & Rollback Guarantee
- **Pre-restore safety backup**: Before overwriting or deleting any existing game save files during restore, an automatic pre-restore backup (`BackupLabelKind::PreRestore`) must be completed.
- **Staged materialization**: Backed-up files (whether from CAS object store, ZIP archive, or raw files) are materialized into a temporary staging folder (`.gst_restore_stage_{uuid}`) located in the parent directory of the target save folder.
- **Two-phase directory swap**:
  1. The existing active save directory is renamed to `.gst_restore_old_{uuid}`.
  2. The staging directory `.gst_restore_stage_{uuid}` is renamed to the target save path.
  3. If step 2 fails, the operation immediately attempts to restore `.gst_restore_old_{uuid}` back to the target save path.
  4. Only after step 2 succeeds is `.gst_restore_old_{uuid}` deleted.
- **Informative failure state**: In the unlikely event of a swap failure, the user is informed of the exact path to the pre-restore backup.

### 3.3. Fingerprint-First Change Checks
- **Avoid heavy I/O in UI loop**: Calculating SHA-256 hashes over multi-gigabyte save folders in the main UI thread causes frame drops and sluggishness.
- **Two-tier verification**:
  - **Tier 1 (Fast Fingerprint)**: `snapshot::quick_directory_fingerprint` rapidly inspects file count, aggregate byte size, and the latest modification time (`mtime`).
  - **Tier 2 (Full Hash)**: Only if the quick fingerprint deviates from `last_save_fingerprints` does the background tick invoke the full `calculate_directory_snapshot_hash`.
- Unchanged save states immediately skip hashing and avoid generating redundant auto-backups.

### 3.4. Deduplication & Centralized Rule Ownership
- **Content-addressable storage (CAS)**: Incremental backups store unique blobs in `.objects/{hash}`. Duplicate files across multiple backups share the same disk blob, with backup nodes containing only `manifest.json` pointing to the hashes.
- **Single source of truth for presets**: Preset game save definitions and Steam App ID heuristics are owned exclusively by `presets.rs`. `steam.rs` calls `presets::find_steam_known_paths` and must not maintain duplicate path resolution logic.

### 3.5. Error Localization & No Leakage
- **No untranslated internal strings**: Any error surfaced to the user must be routed through `AppError::user_message_for_language(language)`.
- **Strict bilingual parity**: Both `Language::ZhCn` and `Language::EnUs` must be fully supported. Context strings in `localize_error_context` and messages in `localize_error_message` must have explicit translations for both languages without falling back to raw technical English error strings on the Chinese UI.

---

## 4. Release Naming & Asset Conventions

- **Version Authority**: `Cargo.toml` (`[package] version = "x.y.z"`).
- **PE Metadata**: `build.rs` parses `Cargo.toml` at compile time and embeds version and copyright info into the Windows binary resources.
- **Git Tag Naming**: Semantic version prefixed with `v` (e.g., `v0.1.2`).
- **Standardized Asset Names**:
  - Binary executable: `GameSaveUtility-v{version}-windows-x64.exe`
  - Compressed bundle: `GameSaveUtility-v{version}-windows-x64.zip`
  - Cryptographic checksums: `GameSaveUtility-v{version}-windows-x64.sha256`
- **Release Automation**:
  - `scripts/build-release.ps1`: Builds with `--release`, standardizes filenames, creates the zip archive, and calculates SHA-256 hashes.
  - `.github/workflows/release.yml`: Triggers on push to `v*` tags, runs formatting/test/clippy verification, executes `build-release.ps1`, and generates a draft GitHub Release with assets attached.

---

## 5. Common Development Commands

Run these standard commands from the repository root:

- **Check Code Formatting**:
  ```bash
  cargo fmt --check
  ```
- **Apply Formatting**:
  ```bash
  cargo fmt
  ```
- **Run Unit & Integration Tests**:
  ```bash
  cargo test --all-targets
  ```
- **Run Clippy Linter**:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- **Run Development Build**:
  ```bash
  cargo run
  ```
- **Build Optimized Release Binary**:
  ```bash
  cargo build --release
  ```
- **Package Release Assets (PowerShell / Windows)**:
  ```powershell
  .\scripts\build-release.ps1
  ```

---

## Project Initialization

<!-- light-project:pointer:start -->
Read `docs/agents/light-project.md` before Light Project workflows. Its managed block is the stable project contract; preserve manual notes outside it.
<!-- light-project:pointer:end -->
