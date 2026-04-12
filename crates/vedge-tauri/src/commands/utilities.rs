use vedge_core::business::usecases::utilities::{
    GeneratePasswordInput, GeneratePasswordUseCase, ReadFromFileInput, ReadFromFileUseCase,
    WriteToFileInput, WriteToFileUseCase,
};
use vedge_core::shared::base::BaseUseCase;
use tokio::sync::Mutex;

use crate::interface::ApiError;
use crate::store::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn generate_password(
    request: GeneratePasswordInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, ApiError> {
    let state = state.lock().await;

    let usecase = GeneratePasswordUseCase::new(state.registry.utilities_service.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn read_from_file(
    request: ReadFromFileInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, ApiError> {
    let state = state.lock().await;

    let usecase = ReadFromFileUseCase::new(state.registry.utilities_service.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn write_to_file(
    request: WriteToFileInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), ApiError> {
    let state = state.lock().await;

    let usecase = WriteToFileUseCase::new(state.registry.utilities_service.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}
