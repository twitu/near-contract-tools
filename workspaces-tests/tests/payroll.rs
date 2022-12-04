#![cfg(not(windows))]
use near_sdk::serde_json::json;
use workspaces::{Account, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/payroll_example.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup() -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize user accounts
    let mut accounts = Vec::new();
    for _ in 0..4 {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    let owner = &accounts[0];

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract
        .call("new")
        .args(owner.id().as_bytes().to_vec())
        .transact()
        .await
        .unwrap()
        .unwrap();

    Setup { contract, accounts }
}

#[tokio::test]
async fn success() {
    let Setup { contract, accounts } = setup().await;

    let contract_id = contract.id();
    let owner = &accounts[0];
    let manager = &accounts[1];
    let employee_1 = &accounts[2];
    let employee_2 = &accounts[3];

    // setup roles
    owner
        .call(contract_id, "payroll_add_manager")
        .args(manager.id().as_bytes().to_vec())
        .transact()
        .await
        .unwrap()
        .unwrap();

    // setup roles
    manager
        .call(contract_id, "payroll_add_employee")
        .args(employee_1.id().as_bytes().to_vec())
        .transact()
        .await
        .unwrap()
        .unwrap();

    manager
        .call(contract_id, "payroll_add_employee")
        .args(employee_2.id().as_bytes().to_vec())
        .transact()
        .await
        .unwrap()
        .unwrap();

    employee_1
        .call(contract_id, "payroll_log_time")
        .args_json(json! ({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    employee_1
        .call(contract_id, "payroll_log_time")
        .args_json(json! ({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    employee_2
        .call(contract_id, "payroll_log_time")
        .args_json(json! ({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let request_id = employee_1
        .call(contract_id, "payroll_request_pay")
        .args_json(json! ({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    employee_1
        .call(contract_id, "payroll_approve_pay")
        .args_json(json! ({
            "request_id": request_id,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}
