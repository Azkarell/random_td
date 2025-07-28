
pub enum AlgorithmStepResult<T> {
    Success(T),
    Failed,
    NotExecuted,
}

pub trait AlgorithmStep {
    type Input;
    type Output;

    fn execute(&mut self, input: &Self::Input);
    fn get_result(&self) -> AlgorithmStepResult<&Self::Output>;
}
