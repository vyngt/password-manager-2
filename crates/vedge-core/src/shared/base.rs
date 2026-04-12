use crate::errors::AppResult;

pub trait BaseUseCase<T, O> {
    fn execute(&self, input: T) -> impl std::future::Future<Output = AppResult<O>> + Send;
}
