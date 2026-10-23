use rust_magi::{
    config::config::{DEFAULT_MODEL_NAME, MagiConfiguration},
};

#[test]
fn configuration_create() {
    use rust_magi::agent::factory::MagiCoreID;
    // test self init MagiConfiguration
    let test_agent1_model_name: String = "agent1_model_name".into();
    let test_agent2_model_name: String = "agent2_model_name".into();
    let test_agent3_model_name: String = "agent3_model_name".into();
    let my_config = MagiConfiguration::new(
        test_agent1_model_name.clone(),
        test_agent2_model_name.clone(),
        test_agent3_model_name.clone(),
    );
    assert_eq!(
        my_config.get_model_name(&MagiCoreID::Balthasar),
        test_agent1_model_name
    );
    assert_eq!(
        my_config.get_model_name(&MagiCoreID::Casper),
        test_agent2_model_name
    );
    assert_eq!(
        my_config.get_model_name(&MagiCoreID::Melchior),
        test_agent3_model_name
    );

    println!("config test all pass!");
}
