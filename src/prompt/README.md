# prompt：系统提示词拼接

[返回 src 导航](../README.md)

本模块仅做字符串组合，不发送模型请求、不处理运行时消息，也不维护对话历史。

## create_prompt 子模块

公共导入路径：`rust_magi::prompt::create_prompt::create_sys_prompt`。

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
    assert_eq!(prompt, "personality:helpful\n\n评审文章\n\n");
}
```

`MagiCore::new` 使用这个返回值作为 Rig Agent 的 preamble。当前工厂传给三个身份的 `msg` 均为 `AGENT1_PREABLE`；工作流里的写作、评审和核查任务则是后续请求的 prompt，不是此函数拼接的系统提示词。

## personality 私有子模块

[personality.rs](personality.rs) 不对库调用方开放；其中的常量由 `create_prompt` 使用。

| 常量 | 当前字面值 |
| --- | --- |
| `BALTHASAR_PERSONALITY` | `personality:kind\n\n` |
| `CASPER_PERSONALITY` | `personality:helpful\n\n` |
| `MELCHIOR_PERSONALITY` | `personality:harmful\n\n` |

这些是现有占位文本，尚未落实根目录 README 中的母亲、个体、科学家人格。尤其是 Melchior 的 `harmful` 字面值不能解释为已经实现事实核查人格；本次文档补全只记录现状。

## 修改与验证

修改人格前缀应在 `personality.rs` 完成；修改拼接规则应在 `create_prompt.rs` 完成；修改业务任务措辞应在 [workflow](../workflow/README.md) 完成。不要通过对外公开私有模板来绕过统一拼接接口。

当前没有专门的 prompt 测试文件，现有工作流测试注入 Mock Agent，也不经过 `MagiCore::new`。后续修改前缀、换行或角色选择时，应补充直接覆盖 `create_sys_prompt` 的测试；运行 `cargo test` 检查整体兼容性。
