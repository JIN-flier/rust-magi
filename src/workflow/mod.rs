//! 业务流程编排层：三人会议和二元投票。

/// 三人讨论，直到共同认可候选结论或达到轮数上限。
pub mod meeting;
/// 三份结构化选择和多数决。
pub mod vote;
pub mod result;