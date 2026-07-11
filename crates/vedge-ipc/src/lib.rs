//! Wire-format contract shared by `vedge-tauri` (command producers) and
//! `vedge-app` (command consumers).
//!
//! This crate is deliberately small: just serde types + base64/timestamp
//! helpers + the `{kind, message}` error envelope. It depends on `serde`,
//! `serde_json`, `chrono`, `base64`, `thiserror` — nothing from
//! `vedge-core` or `tauri`. That keeps it WASM-compatible and avoids
//! forcing either side to pull heavy deps across the IPC boundary.
//!
//! Conversions between these DTOs and `vedge-core` domain types live in
//! `vedge-tauri/src/dto_convert/` — that's where the vedge-core
//! dependency legitimately sits.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
    )
)]

pub mod audit;
pub mod common;
pub mod emergency_kit;
pub mod entry;
pub mod envelope;
pub mod error;
pub mod misc;
pub mod settings;
pub mod tag;

pub use audit::{AuditEventDto, AuditPageDto, AuditQueryDto};
pub use common::{
    CommonMetaDto, EntryTypeDto, Timestamp, b64_decode, b64_decode_fixed, b64_encode,
    ts_from_string, ts_to_string,
};
pub use emergency_kit::{EmergencyKitDto, RecoveryOutcomeDto, UnlockWithRecoveryKeyInputDto};
pub use entry::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, DocumentPayloadDto, EnvVarDto, EnvVarsPayloadDto,
    FolderPayloadDto, HistoryEntryDto, IdentityPayloadDto, IndexEntryDto, LoginPayloadDto,
    NotePayloadDto, PayloadDto, SshKeyPayloadDto,
};
pub use envelope::ErrorEnvelope;
pub use error::IpcError;
pub use misc::{
    ChangePasswordInputDto, CreateVaultInputDto, CreateVaultOutputDto, ExportedDocumentDto,
    FieldSelectorDto, MaintenanceReportDto, UnlockVaultInputDto,
};
pub use settings::{
    AppSettingDto, CreateCustomThemeInputDto, ExtensionSessionDto, KnownDeviceDto, RecentVaultDto,
    RecentVaultStatusDto, ThemeDto, UpdateCustomThemeInputDto,
};
pub use tag::{CreateTagDto, RenameTagDto, TagMetaDto};
