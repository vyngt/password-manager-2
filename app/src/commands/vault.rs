use backend::business::usecases::vault::{
    ChangeKeyVaultInput, ChangeKeyVaultUseCase, CreateVaultItemInput, CreateVaultItemUseCase,
    DeleteVaultItemInput, DeleteVaultItemUseCase, ExportVaultItemsInput, ExportVaultItemsUseCase,
    GetVaultItemInput, GetVaultItemUseCase, ImportVaultItemsInput, ImportVaultItemsUseCase,
    ListVaultItemInput, ListVaultItemUseCase, UnlockVaultInput, UnlockVaultUseCase,
    UpdateVaultItemInput, UpdateVaultItemUseCase,
};
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

#[tauri::command]
pub async fn change_key_vault(
    request: ChangeKeyVaultInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, ApiError> {
    let state = state.lock().await;

    let usecase = ChangeKeyVaultUseCase::new(state.registry.vault_service.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn create_vault_item(
    request: CreateVaultItemInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<backend::business::domain::entities::vault_item::VaultItem, ApiError> {
    let state = state.lock().await;

    let usecase = CreateVaultItemUseCase::new(state.registry.vault_item_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn get_vault_item(
    request: GetVaultItemInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<backend::business::domain::entities::vault_item::VaultItem, ApiError> {
    let state = state.lock().await;

    let usecase = GetVaultItemUseCase::new(state.registry.vault_item_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn update_vault_item(
    request: UpdateVaultItemInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<backend::business::domain::entities::vault_item::VaultItem, ApiError> {
    let state = state.lock().await;

    let usecase = UpdateVaultItemUseCase::new(state.registry.vault_item_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn delete_vault_item(
    request: DeleteVaultItemInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<backend::business::domain::entities::vault_item::VaultItem, ApiError> {
    let state = state.lock().await;

    let usecase = DeleteVaultItemUseCase::new(state.registry.vault_item_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn list_vault_items(
    request: ListVaultItemInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<
    backend::shared::pager::PaginationOutput<
        backend::business::domain::entities::vault_item::VaultItem,
    >,
    ApiError,
> {
    let state = state.lock().await;

    let usecase = ListVaultItemUseCase::new(state.registry.vault_item_repository.clone());
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn export_vault_items(
    request: ExportVaultItemsInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), ApiError> {
    let state = state.lock().await;

    let usecase = ExportVaultItemsUseCase::new(
        state.registry.vault_item_repository.clone(),
        state.registry.utilities_service.clone(),
    );
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}

#[tauri::command]
pub async fn import_vault_items(
    request: ImportVaultItemsInput,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), ApiError> {
    let state = state.lock().await;

    let usecase = ImportVaultItemsUseCase::new(
        state.registry.vault_item_repository.clone(),
        state.registry.utilities_service.clone(),
    );
    let result = usecase.execute(request).await;

    match result {
        Ok(result) => Ok(result),
        Err(err) => Err(ApiError::from(err)),
    }
}
