use bus_mapping::{
    circuit_input_builder::{BuilderClient, ExecState},
    evm::OpcodeId,
};
use integration_tests::{get_client, log_init, TX_ID};
use zkevm_circuits::evm_circuit::{
    test::{run_test_circuit_complete_fixed_table, run_test_circuit_incomplete_fixed_table},
    witness::block_convert,
};

#[tokio::main]
async fn main() {
    log_init();
    log::info!("test evm circuit, tx: {}", *TX_ID);
    let cli = get_client();
    let cli = BuilderClient::new(cli).await.unwrap();
    let builder = cli.gen_inputs_tx(&*TX_ID).await.unwrap();

    if builder.block.txs.is_empty() {
        log::info!("skip empty block");
        return;
    }

    let block = block_convert(&builder.block, &builder.code_db);
    let need_bitwise_lookup = builder.block.txs.iter().any(|tx| {
        tx.steps().iter().any(|step| {
            matches!(
                step.exec_state,
                ExecState::Op(OpcodeId::ADD)
                    | ExecState::Op(OpcodeId::OR)
                    | ExecState::Op(OpcodeId::XOR)
            )
        })
    });
    if need_bitwise_lookup {
        run_test_circuit_complete_fixed_table(block).expect("evm_circuit verification failed");
    } else {
        run_test_circuit_incomplete_fixed_table(block).expect("evm_circuit verification failed");
    }
    log::info!("prove done");
}
