use alloy_sol_types::{sol, SolCall, SolInterface};
use hex_literal::hex;

sol! {
    #[derive(Debug, PartialEq)]
    interface ISmartAccount {
        function execute_ncC(address dest, uint256 value, bytes calldata func) external;
        function executeBatch_y6U(address[] dest, uint256[] value, bytes[] calldata func) external;
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

fn limit_erc20_value(call_data: &[u8]) {
    if call_data[..4] != IERC20::transferCall::SELECTOR {
        panic!("Invalid function selector")
    }
    let decoded = IERC20::IERC20Calls::abi_decode(&call_data, false).unwrap();

    match decoded {
        IERC20::IERC20Calls::transfer(IERC20::transferCall { to, amount }) => {
            println!("ERC20 transfer to {:?} amount {:?}", to, amount);
        }
    }
}

fn limit_erc721_value(call_data: &[u8]) {
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
}

fn check_calldata(
    call_data: &[u8],
    call_type: &[TxType],
) -> Result<(), Box<dyn std::error::Error>> {
    let call_data_decode =
        ISmartAccount::ISmartAccountCalls::abi_decode(&call_data, false).unwrap();

    let batch_trx_order = [TxType::ERC20, TxType::ERC721];

    // Match against the expected variants
    match call_data_decode {
        ISmartAccount::ISmartAccountCalls::execute_ncC(ISmartAccount::execute_ncCCall {
            dest,
            value,
            func,
        }) => {
            println!("Signle call: execute_ncC to {:?} value {:?}", dest, value);
            for &typ in call_type {
                match typ {
                    TxType::ERC20 => limit_erc20_value(&func),
                    TxType::ERC721 => limit_erc721_value(&func),
                }
            }
        }

        ISmartAccount::ISmartAccountCalls::executeBatch_y6U(
            ISmartAccount::executeBatch_y6UCall { dest, value, func },
        ) => {
            println!(
                "Batch call: executeBatch_y6U to {:?} value {:?}",
                dest, value
            );
            for (i, &typ) in call_type.iter().enumerate() {
                match typ {
                    TxType::ERC20 => limit_erc20_value(&func[i]),
                    TxType::ERC721 => limit_erc721_value(&func[i]),
                }
            }
        }
    }

    Ok(())
}

fn main() {
    let erc20_call_data = hex!("0000189a0000000000000000000000003d74673d28ad26ada59b023a78cfdbee520ef695000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb00000000000000000000000015d34aaf54267db7d7c367839aaf71a00a2c6a650000000000000000000000000000000000000000000000000a749d4eedc7800000000000000000000000000000000000000000000000000000000000");

    match check_calldata(&erc20_call_data, &[TxType::ERC20]) {
        Ok(_) => println!("✅ erc20\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }

    let nft_call_data = hex!("0000189a0000000000000000000000001758f42af7026fbbb559dc60ece0de3ef81f665e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000006423b872dd00000000000000000000000092662d3236431dceabafe44a59eaa35cb7869a7c0000000000000000000000004eafee57d2df2351baacd721f6da51ec57f51f17000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    match check_calldata(&nft_call_data, &[TxType::ERC721]) {
        Ok(_) => println!("✅ erc721\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }

    let batch_call_data = hex!("00004680000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000020000000000000000000000001758f42af7026fbbb559dc60ece0de3ef81f665e000000000000000000000000478a74f40055dcc345ea4fe625a6873be147bc850000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000006423b872dd00000000000000000000000092662d3236431dceabafe44a59eaa35cb7869a7c0000000000000000000000004eafee57d2df2351baacd721f6da51ec57f51f170000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044a9059cbb00000000000000000000000092662d3236431dceabafe44a59eaa35cb7869a7c000000000000000000000000000000000000000000000000016345785d8a000000000000000000000000000000000000000000000000000000000000");

    match check_calldata(&batch_call_data, &[TxType::ERC721, TxType::ERC20]) {
        Ok(_) => println!("✅ batch\n"),
        Err(e) => println!("Failed to check call data: {:?}", e),
    }
}