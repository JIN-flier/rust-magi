use crate::agent::factory::MagiCoreID;
use std::path::Path;

pub const DEFAULT_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiConfiguration {
    balthasar_model_name: String,
    casper_model_name: String,
    melchior_model_name: String,
}

impl MagiConfiguration {
    pub fn new(
        balthasar_model_name: String,
        casper_model_name: String,
        melchior_model_name: String,
    ) -> Self {
        MagiConfiguration {
            balthasar_model_name,
            casper_model_name,
            melchior_model_name,
        }
    }

    pub fn get_config_from_file() -> Self {
        if let Ok(Path_ptr) = search_config_file() {
            todo!()
        } else {
            create_default_config_file();
        };
        todo!()
    }

    pub fn get_model_name(&self, id: &MagiCoreID) -> String {
        match id {
            MagiCoreID::Balthasar => self.balthasar_model_name.clone(),
            MagiCoreID::Casper => self.casper_model_name.clone(),
            MagiCoreID::Melchior => self.melchior_model_name.clone(),
            _ => DEFAULT_MODEL_NAME.into(),
        }
    }
}

fn search_config_file() -> anyhow::Result<Box<Path>> {
    todo!()
}

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
        let my_config = MagiConfiguration::new(
            test_balthasar_model_name.clone(),
            test_casper_model_name.clone(),
            test_melchior_model_name.clone(),
        );
        assert_eq!(
            my_config,
            MagiConfiguration {
                balthasar_model_name: test_balthasar_model_name.clone(),
                casper_model_name: test_casper_model_name.clone(),
                melchior_model_name: test_melchior_model_name.clone(),
            }
        );
    }
}
