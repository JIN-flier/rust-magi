//! 三个身份各自的模型名称，以及尚未实现的配置文件入口。

use crate::agent::factory::MagiCoreID;
use std::path::Path;

/// 预留的回退模型名；现有三个枚举分支都会返回各自配置，不会走回退分支。
pub const DEFAULT_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";

/// 保存每个身份的模型名，不包含凭证、客户端、工作流参数或持久化状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiConfiguration {
    balthasar_model_name: String,
    casper_model_name: String,
    melchior_model_name: String,
    cheap_model_name: String,
}

impl MagiConfiguration {
    /// 按 Balthasar、Casper、Melchior 的顺序保存三个模型名。
    ///
    /// 不校验模型是否存在，也不替换空字符串；实际可用性由模型服务决定。
    pub fn new(
        balthasar_model_name: String,
        casper_model_name: String,
        melchior_model_name: String,
        cheap_model_name: String,
    ) -> Self {
        MagiConfiguration {
            balthasar_model_name,
            casper_model_name,
            melchior_model_name,
            cheap_model_name,
        }
    }

    /// 预留的文件加载入口，目前未实现路径搜索、文件解析或默认文件写入。
    ///
    /// # Panics
    ///
    /// 当前会在调用内部 `search_config_file()` 时触发 `todo!()`。
    /// 在实现之前，请使用 [`Self::new`] 显式构造配置。
    pub fn get_config_from_file() -> Self {
        if let Ok(Path_ptr) = search_config_file() {
            todo!()
        } else {
            create_default_config_file();
        };
        todo!()
    }

    /// 返回指定身份的模型名副本，不读取环境或配置文件。
    pub fn get_model_name(&self, id: &MagiCoreID) -> String {
        match id {
            MagiCoreID::Balthasar => self.balthasar_model_name.clone(),
            MagiCoreID::Casper => self.casper_model_name.clone(),
            MagiCoreID::Melchior => self.melchior_model_name.clone(),
            MagiCoreID::FastCheap => self.cheap_model_name.clone(),
            _ => DEFAULT_MODEL_NAME.into(),
        }
    }
}

/// 配置文件定位占位函数：尚无约定的路径或搜索顺序，调用即 panic。
fn search_config_file() -> anyhow::Result<Box<Path>> {
    todo!()
}

/// 默认配置文件创建占位函数：尚无文件格式或写入实现，调用即 panic。
fn create_default_config_file() -> anyhow::Result<Box<Path>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_para_init() {
        let test_balthasar_model_name: String = "balthasar_model_name".into();
        let test_casper_model_name: String = "casper_model_name".into();
        let test_melchior_model_name: String = "melchior_model_name".into();
        let test_cheap_model_name: String = "cheap_model_name".into();
        let my_config = MagiConfiguration::new(
            test_balthasar_model_name.clone(),
            test_casper_model_name.clone(),
            test_melchior_model_name.clone(),
            test_cheap_model_name.clone(),
        );
        assert_eq!(
            my_config,
            MagiConfiguration {
                balthasar_model_name: test_balthasar_model_name.clone(),
                casper_model_name: test_casper_model_name.clone(),
                melchior_model_name: test_melchior_model_name.clone(),
                cheap_model_name: test_cheap_model_name.clone(),
            }
        );
    }
}
