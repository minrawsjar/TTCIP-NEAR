use near_api::{AccountId, NearToken};
use near_sdk::serde_json::json;

#[derive(near_sdk::serde::Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct UserBalanceResult {
    balance: String,
}

async fn test_basics_on(contract_wasm: Vec<u8>) -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let sandbox_network =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let alice = create_subaccount(&sandbox, "alice.sandbox").await?;
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let txtc_token = create_subaccount(&sandbox, "txtc.sandbox").await?;
    let contract = create_subaccount(&sandbox, "contract.sandbox")
        .await?
        .as_contract();

    let signer = near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )?;

    // Deploy and init ttcip-pool (owner, min_deposit, txtc_token_id)
    let min_deposit = "1000000000000000000000000"; // 1 NEAR in yocto
    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(contract_wasm)
        .with_init_call(
            "init",
            json!({
                "owner": owner.account_id(),
                "min_deposit": min_deposit,
                "txtc_token_id": txtc_token.account_id()
            }),
        )?
        .with_signer(signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Register alice
    contract
        .call_function(
            "register_user",
            json!({
                "phone_number": "+1234567890",
                "near_account": alice.account_id()
            }),
        )
        .transaction()
        .with_signer(owner.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Alice deposits 2 NEAR
    contract
        .call_function("deposit", json!({"phone_number": "+1234567890"}))
        .transaction()
        .deposit(NearToken::from_near(2))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Check balance (balance is yoctoNEAR as string in JSON)
    let result: UserBalanceResult = contract
        .call_function("get_balance", json!({"phone_number": "+1234567890"}))
        .read_only()
        .fetch_from(&sandbox_network)
        .await?
        .data;
    assert_eq!(result.balance, "2000000000000000000000000");

    Ok(())
}

async fn create_subaccount(
    sandbox: &near_sandbox::Sandbox,
    name: &str,
) -> testresult::TestResult<near_api::Account> {
    let account_id: AccountId = name.parse().unwrap();
    sandbox
        .create_account(account_id.clone())
        .initial_balance(NearToken::from_near(10))
        .send()
        .await?;
    Ok(near_api::Account(account_id))
}

#[tokio::test]
async fn test_contract_is_operational() -> testresult::TestResult<()> {
    let contract_wasm_path = cargo_near_build::build_with_cli(Default::default())?;
    let contract_wasm = std::fs::read(contract_wasm_path)?;

    test_basics_on(contract_wasm).await
}

#[tokio::test]
async fn test_add_liquidity() -> testresult::TestResult<()> {
    let contract_wasm_path = cargo_near_build::build_with_cli(Default::default())?;
    let contract_wasm = std::fs::read(contract_wasm_path)?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let sandbox_network =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let txtc_token = create_subaccount(&sandbox, "txtc.sandbox").await?;
    let contract = create_subaccount(&sandbox, "contract.sandbox")
        .await?
        .as_contract();

    let signer = near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )?;

    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(contract_wasm)
        .with_init_call(
            "init",
            json!({
                "owner": owner.account_id(),
                "min_deposit": "1000000000000000000000000",
                "txtc_token_id": txtc_token.account_id()
            }),
        )?
        .with_signer(signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Add NEAR liquidity
    contract
        .call_function("add_liquidity_near", json!({}))
        .transaction()
        .deposit(NearToken::from_near(5))
        .with_signer(owner.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Verify reserves (only NEAR added so far)
    // Note: get_pool_reserves returns (U128, U128) -> (txtc, near)
    let result: (near_sdk::json_types::U128, near_sdk::json_types::U128) = contract
        .call_function("get_pool_reserves", json!({}))
        .read_only()
        .fetch_from(&sandbox_network)
        .await?
        .data;
    
    assert_eq!(result.1 .0, 5000000000000000000000000); // 5 NEAR

    Ok(())
}
