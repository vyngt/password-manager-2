use vedge_core::business::usecases::theme::{
    CreateColorSchemeInput, CreateColorSchemeUseCase, DeleteColorSchemeInput,
    DeleteColorSchemeUseCase, GetColorSchemeInput, GetColorSchemeUseCase, GetCurrentThemeUseCase,
    ListColorSchemesInput, ListColorSchemesUseCase, UpdateColorSchemeInput,
    UpdateColorSchemeUseCase, UpdateThemeInput, UpdateThemeUseCase,
};
use vedge_core::shared::base::BaseUseCase;
use tokio::sync::Mutex;

use crate::interface::ApiError;
use crate::store::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_current_theme(
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::Theme, ApiError> {
    let state = state.lock().await;

    let usecase = GetCurrentThemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(()).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn create_color_scheme(
    request: CreateColorSchemeInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::ColorScheme, ApiError> {
    let state = state.lock().await;

    let usecase = CreateColorSchemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn get_color_scheme(
    request: GetColorSchemeInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::ColorScheme, ApiError> {
    let state = state.lock().await;

    let usecase = GetColorSchemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn update_color_scheme(
    request: UpdateColorSchemeInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::ColorScheme, ApiError> {
    let state = state.lock().await;

    let usecase = UpdateColorSchemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn delete_color_scheme(
    request: DeleteColorSchemeInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::ColorScheme, ApiError> {
    let state = state.lock().await;

    let usecase = DeleteColorSchemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn list_color_schemes(
    request: ListColorSchemesInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<
    vedge_core::shared::pager::PaginationOutput<
        vedge_core::business::domain::entities::theme::ColorScheme,
    >,
    ApiError,
> {
    let state = state.lock().await;

    let usecase = ListColorSchemesUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn update_theme(
    request: UpdateThemeInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<vedge_core::business::domain::entities::theme::Theme, ApiError> {
    let state = state.lock().await;

    let usecase = UpdateThemeUseCase::new(state.registry.theme_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}
