# prompt：系统提示词拼接

[返回 src 导航](../README.md)

本模块提供人格、意图识别、投票和会议提示词；不发送模型请求、不处理运行时消息，也不维护对话历史。

## create_prompt 子模块

公共导入路径：`rust_magi::prompt::create_prompt::{create_sys_prompt, create_vote_prompt, create_meeting_prompt}`。

```text
pub fn create_sys_prompt(id: &MagiCoreID, msg: &str) -> String
```

根据身份选择人格前缀，原样拼接 `msg`，最后追加两个换行符，返回新字符串。函数不做去空白、转义、模板变量替换或输入校验。

```rust
use rust_magi::{
    agent::factory::MagiCoreID,
    prompt::create_prompt::create_sys_prompt,
};

fn main() {
    let prompt = create_sys_prompt(&MagiCoreID::Casper, "评审文章");
    assert!(prompt.starts_with("你是 MAGI 的 CASPER-3"));
    assert!(prompt.ends_with("\n\n评审文章\n\n"));
}
```

`MagiCore::new` 使用这个返回值作为 Rig Agent 的 preamble。当前工厂传给三位成员的 `msg` 均为 `AGENT1_PREABLE`；CLI 则通过 `new_with_preamble` 传入 `MAGI_GENERAL_PREABLE`。`create_sys_prompt` 只支持三位成员身份，传入 `FastCheap` 会触发占位 `todo!()`。

`create_vote_prompt(&Intention::Vote { option_a, option_b }, query)` 将原始问题及两个选项放入投票提示词，选项 A 映射 `true`，选项 B 映射 `false`。传入会议意图会触发 `todo!()`；调用方应先匹配意图。`create_meeting_prompt(query, rounds, candidate)` 加入问题、可选候选结论及此前公开讨论记录。`intent_recognise::INTENT_PROMPT` 供 `IntentWorkflow` 区分投票与会议。

## personality 私有子模块

[personality.rs](personality.rs) 不对库调用方开放；其中的常量由 `create_prompt` 使用。

| 常量 | 人格侧面与判断重点 |
| --- | --- |
| `BALTHASAR_PERSONALITY` | 母亲：照护、责任、安全与长期影响，尊重被保护者的选择 |
| `CASPER_PERSONALITY` | 女人／个体：个人情感、自尊、依恋与自主意愿，区分价值偏好和事实 |
| `MELCHIOR_PERSONALITY` | 科学家：证据、逻辑、可行性与可验证性，主动检验和修正假设 |

原作以赤木直子的科学家、母亲、女人三个侧面构成 MAGI，见 TV 第 13、21 话；《The End of Evangelion》中 Casper 否决自毁命令，体现了这些侧面可能产生冲突。[设定参考](https://evangelion.fandom.com/wiki/Naoko_Akagi)。具体判断准则是本项目为协作任务编写的演绎，不是官方人格全文。Casper 的「女人」指直子个人的侧面，不概括女性群体。

每份提示词包含身份、判断重点、交流协作方式与任务约束，并保留两个结尾换行符。三者都应独立判断、依据新信息修正观点、遵循任务的输出 schema；人格不固定赞成或反对票，也不改变会议共识规则。提示词不要求模型复演剧情或将用户代入剧情人物。

## 修改与验证

修改人格前缀应在 `personality.rs` 完成；修改拼接规则应在 `create_prompt.rs` 完成；修改业务任务措辞应在 [workflow](../workflow/README.md) 完成。不要通过对外公开私有模板来绕过统一拼接接口。

`tests/prompts.rs` 直接覆盖人格身份选择、选项映射和会议历史拼接；工作流测试再验证 Mock Agent 接收到相应内容。运行 `cargo test --all-targets` 检查整体兼容性。
