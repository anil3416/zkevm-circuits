use crate::circuit_input_builder::{CircuitInputStateRef, ExecStep};
use crate::evm::Opcode;
use crate::Error;
use core::borrow::Borrow;
use eth_types::evm_types::Memory;
use eth_types::{GethExecStep, ToAddress};

#[derive(Debug, Copy, Clone)]
pub(crate) struct Return;

impl Opcode for Return {
    fn gen_associated_ops(
        &self,
        state: &mut CircuitInputStateRef,
        geth_steps: &[GethExecStep],
    ) -> Result<Vec<ExecStep>, Error> {
        let exec_step = state.new_step(&geth_steps[0])?;
        state.handle_return(&geth_steps[0])?;
        Ok(vec![exec_step])
    }

    fn reconstruct_memory(
        &self,
        state: &mut CircuitInputStateRef,
        geth_steps: &[GethExecStep],
    ) -> Result<Memory, Error> {
        let current_call = state.call()?.clone();

        let geth_step = &geth_steps[0];
        let offset = geth_step.stack.nth_last(0)?.as_usize();
        let length = geth_step.stack.nth_last(1)?.as_usize();

        // we need to keep the memory until handle return complete
        let memory = geth_steps[0].memory.borrow().clone();

        // skip reconstruction for root-level return/revert
        if !current_call.is_root {
            let caller = state.caller()?.clone();
            if !current_call.is_create() {
                // handle normal return/revert
                // copy return data
                // update to the caller memory
                let caller_ctx = state.caller_ctx_mut()?;
                println!("call {:?} ", current_call);
                println!("caller_call {:?}", caller);
                println!("in return  reconstruct_memory");
                println!("before caller_ctx.memory {:?}", caller_ctx.memory);
                println!(
                    "current_call.return_data_offset {} length {} ",
                    current_call.return_data_offset, length
                );
                let return_offset = current_call.return_data_offset as usize;
                caller_ctx
                    .memory
                    .extend_at_least(return_offset + current_call.return_data_length as usize);
                let copy_len = std::cmp::min(current_call.return_data_length as usize, length);

                println!("current_call.return_data_offset {} current_call.return_data_length {} copy_len {}", current_call.return_data_offset, current_call.return_data_length, copy_len);
                caller_ctx.memory.0[return_offset..return_offset + copy_len]
                    .copy_from_slice(&memory.0[offset..offset + copy_len]);

                caller_ctx.return_data.resize(length as usize, 0);
                caller_ctx
                    .return_data
                    .copy_from_slice(&memory.0[offset..offset + length]);
                println!(
                    "after rebuild caller_ctx.memory {:?} caller_ctx.return_data {:?}",
                    caller_ctx.memory, caller_ctx.return_data
                );
                caller_ctx.last_call = Some(current_call);
            } else {
                // dealing with contract creation
                assert!(offset + length <= memory.0.len());
                let code = memory.0[offset..offset + length].to_vec();
                let contract_addr = geth_steps[1].stack.nth_last(0)?.to_address();
                state.code_db.insert(Some(contract_addr), code);
            }
            let caller_ctx = state.caller_ctx()?;
            Ok(caller_ctx.memory.borrow().clone())
        } else {
            Ok(memory)
        }
    }
}

// TODO: circuit implement

#[cfg(test)]
mod return_tests {
    use crate::mock::BlockData;
    use eth_types::geth_types::GethData;
    use eth_types::{bytecode, word};
    use mock::test_ctx::helpers::{account_0_code_account_1_no_code, tx_from_1_to_0};
    use mock::TestContext;

    #[test]
    fn test_ok() {
        // // deployed contract
        // PUSH1 0x20
        // PUSH1 0
        // PUSH1 0
        // CALLDATACOPY
        // PUSH1 0x20
        // PUSH1 0
        // RETURN
        //
        // bytecode: 0x6020600060003760206000F3
        //
        // // constructor
        // PUSH12 0x6020600060003760206000F3
        // PUSH1 0
        // MSTORE
        // PUSH1 0xC
        // PUSH1 0x14
        // RETURN
        //
        // bytecode: 0x6B6020600060003760206000F3600052600C6014F3
        let code = bytecode! {
            PUSH21(word!("6B6020600060003760206000F3600052600C6014F3"))
            PUSH1(0)
            MSTORE

            PUSH1 (0x15)
            PUSH1 (0xB)
            PUSH1 (0)
            CREATE

            PUSH1 (0x20)
            PUSH1 (0x20)
            PUSH1 (0x20)
            PUSH1 (0)
            PUSH1 (0)
            DUP6
            PUSH2 (0xFFFF)
            CALL
            STOP
        };
        // Get the execution steps from the external tracer
        let block: GethData = TestContext::<2, 1>::new(
            None,
            account_0_code_account_1_no_code(code),
            tx_from_1_to_0,
            |block, _tx| block.number(0xcafeu64),
        )
        .unwrap()
        .into();

        let mut builder = BlockData::new_from_geth_data(block.clone()).new_circuit_input_builder();
        builder
            .handle_block(&block.eth_block, &block.geth_traces)
            .unwrap();
    }

    #[test]
    fn test_revert() {
        // // deployed contract
        // PUSH1 0x20
        // PUSH1 0
        // PUSH1 0
        // CALLDATACOPY
        // PUSH1 0x20
        // PUSH1 0
        // REVERT
        //
        // bytecode: 0x6020600060003760206000FD
        //
        // // constructor
        // PUSH12 0x6020600060003760206000FD
        // PUSH1 0
        // MSTORE
        // PUSH1 0xC
        // PUSH1 0x14
        // RETURN
        //
        // bytecode: 0x6B6020600060003760206000FD600052600C6014F3
        let code = bytecode! {
            PUSH21(word!("6B6020600060003760206000FD600052600C6014F3"))
            PUSH1(0)
            MSTORE

            PUSH1 (0x15)
            PUSH1 (0xB)
            PUSH1 (0)
            CREATE

            PUSH1 (0x20)
            PUSH1 (0x20)
            PUSH1 (0x20)
            PUSH1 (0)
            PUSH1 (0)
            DUP6
            PUSH2 (0xFFFF)
            CALL
            STOP
        };
        // Get the execution steps from the external tracer
        let block: GethData = TestContext::<2, 1>::new(
            None,
            account_0_code_account_1_no_code(code),
            tx_from_1_to_0,
            |block, _tx| block.number(0xcafeu64),
        )
        .unwrap()
        .into();

        let mut builder = BlockData::new_from_geth_data(block.clone()).new_circuit_input_builder();
        builder
            .handle_block(&block.eth_block, &block.geth_traces)
            .unwrap();
    }
}
