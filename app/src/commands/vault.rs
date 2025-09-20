use backend::business::usecases::{UnlockVaultInput, UnlockVaultUseCase};
use backend::shared::base::BaseUseCase;
use tokio::sync::Mutex;

use crate::interface::ApiError;
use crate::store::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn unlock_vault(
    request: UnlockVaultInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, ApiError> {
    let state = state.lock().await;

    let usecase = UnlockVaultUseCase::new(state.registry.vault_service.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}
