use alloy_sol_types::{sol, SolCall, SolInterface};
use alloy_sol_types::private::FixedBytes;
use hex_literal::hex;
use ethabi::{Token, decode, ParamType};
use ethabi::Uint;
use std::error::Error;
use std::convert::TryInto;

const CALLTYPE_SINGLE: [u8; 1] = [0x00];
const CALLTYPE_BATCH: [u8; 1] = [0x01];
const EXECTYPE_DEFAULT: [u8; 1] = [0x00];
const EXECTYPE_TRY: [u8; 1] = [0x01];
const CALLTYPE_DELEGATE: [u8; 1] = [0xFF];
const MODE_DEFAULT: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
const UNUSED: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
const MODE_PAYLOAD: [u8; 22] = [0x00; 22]; // 22 bytes of 0x00

#[derive(Debug)]
enum CallType {
    Single,
    Batch,
    Unknown,
}

sol! {
    #[derive(Debug, PartialEq)]
    // type ExecutionMode is bytes32;
    interface IERC7579Account {
        function execute(bytes32 mode, bytes calldata executionCalldata) external;
        // function executeFromExecutor(bytes32 mode, bytes calldata executionCalldata) external;
    }
}

sol! {
    #[derive(Debug, PartialEq)]
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

sol! {
    #[derive(Debug, PartialEq)]
    interface IERC721 {
        function transferFrom(address from, address to, uint256 tokenId) external returns (bool);
    }
}

#[derive(Copy, Clone)]
enum TxType {
    ERC20,
    ERC721,
}

fn limit_erc20_value(call_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if call_data[..4] != IERC20::transferCall::SELECTOR {
        panic!("Invalid function selector")
    }
    let decoded = IERC20::IERC20Calls::abi_decode(&call_data, false).unwrap();

    match decoded {
        IERC20::IERC20Calls::transfer(IERC20::transferCall { to, amount }) => {
            println!("ERC20 transfer to {:?} amount {:?}", to, amount);
        }
    };

    Ok(())
}

fn limit_erc721_value(call_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if call_data[..4] != IERC721::transferFromCall::SELECTOR {
        panic!("Invalid function selector")
    }
    let decoded = IERC721::IERC721Calls::abi_decode(&call_data, false).unwrap();

    match decoded {
        IERC721::IERC721Calls::transferFrom(IERC721::transferFromCall { from, to, tokenId }) => {
            println!(
                "ERC721 transfer from {:?} to {:?} tokenId {:?}",
                from, to, tokenId
            );
        }
    }

    Ok(())
}

fn decode_mode(mode: &[u8]) -> Result<CallType, Box<dyn std::error::Error>> {
    if mode.len() != 32 {
        return Err(Box::from("Invalid mode length"));
    }

    let calltype = &mode[0..1];
    let exectype = &mode[1..2];
    let mode_default = &mode[2..6];
    let unused = &mode[6..10];
    let mode_payload = &mode[10..32];

    if calltype == &CALLTYPE_SINGLE && exectype == &EXECTYPE_DEFAULT &&
        mode_default == MODE_DEFAULT && unused == UNUSED &&
        mode_payload == MODE_PAYLOAD {
        println!("Matched CALLTYPE_SINGLE with default execution and mode payload");
        return Ok(CallType::Single);
    } else if calltype == &CALLTYPE_BATCH && exectype == &EXECTYPE_DEFAULT &&
        mode_default == MODE_DEFAULT && unused == UNUSED &&
        mode_payload == MODE_PAYLOAD {
        println!("Matched CALLTYPE_BATCH with default execution and mode payload");
        return Ok(CallType::Batch);
    } else {
        println!("No match found");
        return Ok(CallType::Unknown);
    }
}

fn decode_single_call_data(execution_calldata: &[u8]) -> Result<(String, Uint, Vec<u8>), Box<dyn std::error::Error>> {
    if execution_calldata.len() < 52 {
        return Err(Box::from("Invalid single call data length"));
    }

    let address_slice = &execution_calldata[0..20];
    let value_slice = &execution_calldata[20..52];
    let function_call_data = &execution_calldata[52..];

    let address = format!("0x{}", hex::encode(address_slice));

    // Convert the value slice to U256
    let value = Uint::from_big_endian(value_slice);

    let function_call_data = function_call_data.to_vec();

    Ok((address, value, function_call_data))
}


fn decode_batch_call_data(execution_calldata: &[u8]) -> Result<Vec<(String, Uint, Vec<u8>)>, Box<dyn std::error::Error>> {
    let decoded = decode(
        &[ParamType::Array(Box::new(ParamType::Tuple(vec![
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Bytes,
        ])))],
        execution_calldata,
    )?;

    if let Token::Array(executions) = &decoded[0] {
        let mut results = Vec::new();
        for execution in executions {
            if let Token::Tuple(items) = execution {
                if let (Token::Address(address), Token::Uint(value), Token::Bytes(call_data)) = (&items[0], &items[1], &items[2]) {
                    results.push((format!("{:?}", address), *value, call_data.clone()));
                }
            }
        }
        Ok(results)
    } else {
        Err(Box::from("Invalid batch call data format"))
    }
}


fn check_calldata(
    call_data: &[u8],
    call_types: &[TxType],
) -> Result<(), Box<dyn std::error::Error>> {
    let call_data_decode =
        IERC7579Account::IERC7579AccountCalls::abi_decode(&call_data, false).unwrap();

    // Match against the expected variants
    match call_data_decode {
        IERC7579Account::IERC7579AccountCalls::execute(IERC7579Account::executeCall {
            mode,
            executionCalldata,
        }) => {
            // println!("Nexus execute call mode {:?} callData {:?}", mode, executionCalldata);
            // Convert FixedBytes<32> to &[u8]
            let call_type = decode_mode(mode.as_slice())?;

            // Decode executionCalldata based on the mode
            match call_type {
                CallType::Single => {
                    // println!("Decoding single call data: {:?}", executionCalldata);
                    let decoded = decode(
                        &[ParamType::Address, ParamType::Uint(256), ParamType::Bytes],
                        &executionCalldata,
                    );
                    // println!("Decoded single call data: {:?}", decoded);

                    let (target_address, value, function_call_data) = decode_single_call_data(&executionCalldata)?;
                    println!("Single call data: target_address: {}, value: {}, function_call_data: {}", target_address, value, hex::encode(&function_call_data));

                    // Execute logic based on the call type
                    for &typ in call_types {
                        match typ {
                            TxType::ERC20 => limit_erc20_value(&function_call_data)?,
                            TxType::ERC721 => limit_erc721_value(&function_call_data)?,
                        }
                    }
                },
                CallType::Batch => {
                    let executions = decode_batch_call_data(&executionCalldata)?;
                    for (i, (target_address, value, call_data)) in executions.iter().enumerate() {
                        let encoded_call_data = hex::encode(call_data);
                        println!("Batch call data: target_address: {}, value: {}, call_data: {}", target_address, value, encoded_call_data);
                
                        // Log the function selector for debugging
                        let function_selector = &call_data[..4];
                        // println!("Function selector: {:?}", hex::encode(function_selector));
                
                        // Execute logic based on the call type
                        if let Some(&typ) = call_types.get(i) {
                            match typ {
                                TxType::ERC20 => {
                                    limit_erc20_value(call_data)?;
                                },
                                TxType::ERC721 => {
                                    limit_erc721_value(call_data)?;
                                },
                            }
                        }
                    }
                }
                
                CallType::Unknown => {
                    println!("Unknown call type");
                }
            }
        }
    }
    Ok(())
}


fn main() {
    let erc20_call_data = hex!("e9ae5c530000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000380b306bf915c4d645ff596e518faf3f9669b970160000000000000000000000000000000000000000000000000000000000000000273ea3e30000000000000000");

    match check_calldata(&erc20_call_data, &[TxType::ERC20]) {
        Ok(_) => println!("✅ erc20\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }

    let nft_call_data = hex!("e9ae5c530000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000380b306bf915c4d645ff596e518faf3f9669b970160000000000000000000000000000000000000000000000000000000000000000273ea3e30000000000000000");

    match check_calldata(&nft_call_data, &[TxType::ERC721]) {
        Ok(_) => println!("✅ erc721\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }

    let batch_call_data = hex!("e9ae5c53010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000240000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000012000000000000000000000000056623d18e54cbbcae340ec449e3c5d1dc0bf60cd000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044095ea7b300000000000000000000000015d34aaf54267db7d7c367839aaf71a00a2c6a650000000000000000000000000000000000000000000000000de0b6b3a76400000000000000000000000000000000000000000000000000000000000000000000000000000000000056623d18e54cbbcae340ec449e3c5d1dc0bf60cd000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044095ea7b30000000000000000000000009965507d1a55bcc2695c58ba16fb37d819b0a4dc0000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000");

    match check_calldata(&batch_call_data, &[TxType::ERC721, TxType::ERC20]) {
        Ok(_) => println!("✅ batch\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }
}