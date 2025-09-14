pub trait BaseUseCase<T, O> {
    fn execute(&self, input: T) -> impl std::future::Future<Output = anyhow::Result<O>> + Send;
}
