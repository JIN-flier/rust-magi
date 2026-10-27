# config：每个身份的模型名称

[返回 src 导航](../README.md)

实现位于 [config.rs](config.rs)，公共路径为 `rust_magi::config::config::{MagiConfiguration, DEFAULT_MODEL_NAME}`。这个模块目前只保存模型名称，不保存 API Key、服务地址、提示词或工作流超时。

## 接口

| 接口 | 功能与返回值 | 当前状态 |
| --- | --- | --- |
| `MagiConfiguration::new(balthasar_model_name: String, casper_model_name: String, melchior_model_name: String, cheap_model_name: String) -> Self` | 按四个身份保存模型名 | 已实现；不校验空值或服务端可用性 |
| `get_model_name(&self, id: &MagiCoreID) -> String` | 返回对应模型名的副本 | 已实现；无 I/O |
| `MagiConfiguration::get_config_from_file() -> Self` | 预留的文件配置加载入口 | 未实现，调用会 panic |
| `DEFAULT_MODEL_NAME: &str` | 预留回退名称 | 四个身份分支已穷尽，当前不会使用回退分支 |

构造函数参数顺序是 **Balthasar、Casper、Melchior、FastCheap**，不要根据 `MagiCoreID` 的声明顺序传参。最后一个模型用于意图识别。结构体字段是私有的，没有 setter；调整配置时可重新构造对象。

## 使用示例

```rust
use rust_magi::{
    agent::factory::MagiCoreID,
    config::config::MagiConfiguration,
};

fn main() {
    let config = MagiConfiguration::new(
        "writer-model".into(),
        "review-model".into(),
        "fact-model".into(),
        "intent-model".into(),
    );
    assert_eq!(config.get_model_name(&MagiCoreID::Casper), "review-model");
    assert_eq!(config.get_model_name(&MagiCoreID::FastCheap), "intent-model");
}
```

示例名称只展示映射关系；真实调用时由 [agent 工厂](../agent/README.md) 把名称交给服务端。当前 CLI 从 [constance.rs](../constance.rs) 取默认模型名；服务凭证与可选地址来自环境。

## 文件加载尚未实现

`get_config_from_file` 会先调用私有函数 `search_config_file()`，该函数直接执行 `todo!()`。`create_default_config_file()` 也只有 `todo!()`，因此目前不会查找、解析或写入配置文件，尚无约定的路径、文件格式或覆盖优先级。

后续实现文件配置前，需要明确这些规则并补充文件缺失、解析失败、空模型名及默认值行为的测试。不要将当前 `.env` 加载与本模块的配置文件加载混为一谈：前者在 `main` 中调用 dotenvy，后者仍不可用。

## 测试

`cargo test --test config` 验证模型名按身份映射；`cargo test --lib config::config::tests` 验证构造后的字段。当前没有文件加载测试，因为对应功能尚未实现。
