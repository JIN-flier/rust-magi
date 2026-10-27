
pub trait WorkflowResult {
    /// 显示结果
    fn show(&self) -> String;
}