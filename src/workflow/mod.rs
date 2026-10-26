//! 业务流程编排层：文章评审、三人会议和二元投票。

/// 文章评审流程及其结构化结果。
pub mod article_review;
/// 三人讨论，直到共同认可候选结论或达到轮数上限。
pub mod meeting;
/// 三份结构化选择和多数决。
pub mod vote;
