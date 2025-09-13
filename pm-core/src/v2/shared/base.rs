pub trait BaseUseCase<T, O> {
    async fn execute(&self, input: T) -> anyhow::Result<O>;
}
