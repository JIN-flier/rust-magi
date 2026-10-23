use super::personality;
use crate::agent::factory::MagiCoreID;

pub fn create_sys_prompt(id: &MagiCoreID, msg: &str) -> String {
    let personality = match id {
        MagiCoreID::Balthasar => personality::BALTHASAR_PERSONALITY.to_string(),
        MagiCoreID::Casper => personality::CASPER_PERSONALITY.to_string(),
        MagiCoreID::Melchior => personality::MELCHIOR_PERSONALITY.to_string(),
        _ => personality::DEFAULT_PERSONALITY.to_string(),
    };
    personality + msg + "\n\n"
}
