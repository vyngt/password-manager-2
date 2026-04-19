//! App settings + themes. Mirrors
//! `vedge-tauri/src/commands/settings.rs`.

use serde::Serialize;
use serde_json::Value;

use vedge_ipc::{AppSettingDto, CreateCustomThemeInputDto, ThemeDto, UpdateCustomThemeInputDto};

use crate::api::call::{call, call_noargs, call_void};
use crate::api::error::ApiError;

// ---- app_settings ------------------------------------------------------------

pub async fn get_app_setting(key: &str) -> Result<Option<AppSettingDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        key: &'a str,
    }
    call("get_app_setting", &Args { key }).await
}

pub async fn set_app_setting(key: &str, value: &Value) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        key: &'a str,
        value: &'a Value,
    }
    call_void("set_app_setting", &Args { key, value }).await
}

pub async fn delete_app_setting(key: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        key: &'a str,
    }
    call_void("delete_app_setting", &Args { key }).await
}

pub async fn list_app_settings(prefix: &str) -> Result<Vec<AppSettingDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        prefix: &'a str,
    }
    call("list_app_settings", &Args { prefix }).await
}

// ---- themes ------------------------------------------------------------------

pub async fn list_themes() -> Result<Vec<ThemeDto>, ApiError> {
    call_noargs("list_themes").await
}

pub async fn get_theme(id: &str) -> Result<ThemeDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call("get_theme", &Args { id }).await
}

pub async fn get_active_theme() -> Result<ThemeDto, ApiError> {
    call_noargs("get_active_theme").await
}

pub async fn set_active_theme(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("set_active_theme", &Args { id }).await
}

pub async fn create_custom_theme(input: &CreateCustomThemeInputDto) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a CreateCustomThemeInputDto,
    }
    call("create_custom_theme", &Args { input }).await
}

pub async fn update_custom_theme(input: &UpdateCustomThemeInputDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UpdateCustomThemeInputDto,
    }
    call_void("update_custom_theme", &Args { input }).await
}

pub async fn duplicate_theme(source_id: &str, new_name: Option<&str>) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        source_id: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        new_name: Option<&'a str>,
    }
    call(
        "duplicate_theme",
        &Args {
            source_id,
            new_name,
        },
    )
    .await
}

pub async fn delete_custom_theme(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("delete_custom_theme", &Args { id }).await
}
