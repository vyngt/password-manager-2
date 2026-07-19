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
pub mod backup;
pub mod common;
pub mod emergency_kit;
pub mod entry;
pub mod envelope;
pub mod error;
pub mod export;
pub mod health;
pub mod import;
pub mod misc;
pub mod recovery;
pub mod rekey;
pub mod secret_update;
pub mod settings;
pub mod snapshot;
pub mod tag;
pub mod totp;

pub use audit::{ACTION_NAMES, AuditEventDto, AuditPageDto, AuditQueryDto};
pub use backup::{
    BackupPreviewDto, BackupReportDto, BackupStatusDto, DeleteVaultReportDto, OpenBackupReportDto,
    ReplaceReportDto, TargetStateDto, VaultDetailsDto,
};
pub use common::{
    CommonMetaDto, EntryTypeDto, Timestamp, b64_decode, b64_decode_fixed, b64_encode,
    ts_from_string, ts_to_string,
};
pub use emergency_kit::{EmergencyKitDto, SecretKeyUnlockOutcomeDto, UnlockWithSecretKeyInputDto};
pub use entry::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, DocumentPayloadDto, EnvVarsPayloadDto,
    FolderPayloadDto, HistoryEntryDto, IdentityPayloadDto, IndexEntryDto, LoginPayloadDto,
    NotePayloadDto, PayloadDto, SshKeyPayloadDto,
};
pub use envelope::ErrorEnvelope;
pub use error::IpcError;
pub use export::ExportReportDto;
pub use health::{
    AgeConfidenceDto, FindingDto, FindingKindDto, HealthReportDto, HealthScanInputDto,
    HealthSummaryDto, SecretFieldDto, SeverityDto, SkipReasonDto, SkippedDto,
};
pub use import::{ImportActionDto, ImportFailureDto, ImportPreviewRow, ImportReportDto};
pub use misc::{
    ChangePasswordInputDto, CreateVaultInputDto, CreateVaultOutputDto, EnvExportFormatDto,
    ExportedDocumentDto, FieldSelectorDto, MaintenanceReportDto, SecretKeyRotationOutputDto,
    UnlockResultDto, UnlockVaultInputDto,
};
pub use recovery::{RecoveryEnrollOutputDto, UnlockWithRecoveryKeyInputDto};
pub use rekey::{RekeyInputDto, RekeyResultDto};
pub use secret_update::{EnvVarUpdateDto, SecretListUpdateDto, SecretUpdateDto};
pub use settings::{
    AppSettingDto, CreateCustomThemeInputDto, ExtensionSessionDto, KnownDeviceDto,
    RegisteredVaultDto, RegisteredVaultStatusDto, ThemeDto, UpdateCustomThemeInputDto,
};
pub use snapshot::{RevertReportDto, SeamlessRevertDto, SnapshotDto, SnapshotReportDto};
pub use tag::{CreateTagDto, RenameTagDto, TagMetaDto};
pub use totp::{TotpAlgorithmDto, TotpCodeDto, TotpEnrolmentDto, TotpUpdateDto};
