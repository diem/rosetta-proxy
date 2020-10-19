use crate::{
    consts,
    error::ApiError,
    filters::{handle, with_options},
    libra::{self, Libra},
    options::Options,
    types::{AccountIdentifier, Amount, Block, BlockIdentifier, BlockRequest, BlockResponse, Currency, Operation, OperationIdentifier, Transaction, TransactionIdentifier},
};
use libra_json_rpc_client::views::{AmountView, EventDataView, TransactionDataView};
use log::debug;
use warp::Filter;

pub fn routes(options: Options) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
    warp::post()
        .and(
            warp::path!("block")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(block))
        )
}

async fn block(block_request: BlockRequest, options: Options) -> Result<BlockResponse, ApiError> {
    debug!("/block");

    let network_identifier = block_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let libra = Libra::new(&options.libra_endpoint);

    let block_version = block_request
        .block_identifier
        .index
        .ok_or_else(|| ApiError::BadBlockRequest)?;

    let metadata = libra.get_metadata(Some(block_version)).await?;
    let tx = if block_version == 0 {
        // For the genesis block, we populate parent_block_identifier with the
        // same genesis block. Refer to
        // https://www.rosetta-api.org/docs/common_mistakes.html#malformed-genesis-block
        let one_tx = libra.get_transactions(block_version, 1, true).await?;
        vec![one_tx[0].clone(), one_tx[0].clone()]
    } else {
        libra.get_transactions(block_version - 1, 2, true).await?
    };

    let block_identifier = BlockIdentifier {
        index: tx[1].version,
        hash: tx[1].hash.clone(),
    };

    let parent_block_identifier = BlockIdentifier {
        index: tx[0].version,
        hash: tx[0].hash.clone(),
    };

    // block timestamp is in usecs, and Rosetta wants millis
    let timestamp = metadata.timestamp / 1000;

    let status = libra::vmstatus_to_str(&tx[1].vm_status);

    let mut operations = tx[1]
        .events
        .iter()
        .filter(|event| {
            // NOTE: mint, preburn, burn, and cancelburn emit extra sent/recv
            // payment events, which are used instead, so we filter thse
            // out. We also filter out some other events we don't care about.
            match &event.data {
                // events that are represented better with sent/recv payment
                EventDataView::Mint { .. } |
                EventDataView::ReceivedMint { .. } |
                EventDataView::Preburn { .. } |
                EventDataView::Burn { .. } |
                EventDataView::CancelBurn { .. } => false,
                // events that we don't care about
                EventDataView::ComplianceKeyRotation { .. } |
                EventDataView::BaseUrlRotation { .. } |
                EventDataView::Unknown { .. } => false,
                _ => true,
            }
        })
        .enumerate()
        .map(|(index, event)| {
            let index = index as u64;
            let operation_identifier = OperationIdentifier {
                index,
                network_index: None,
            };

            #[derive(Debug)]
            enum AmountKind<'a> {
                Credit(&'a AmountView),
                Debit(&'a AmountView),
            }
            use AmountKind::*;

            let (type_, amount, account) = match &event.data {
                // NOTE: mint, preburn, burn, and cancelburn are filtered out above.
                EventDataView::Mint { .. } => unreachable!(),
                EventDataView::ReceivedMint { .. } => unreachable!(),
                EventDataView::Preburn { .. } => unreachable!(),
                EventDataView::Burn { .. } => unreachable!(),
                EventDataView::CancelBurn { .. } => unreachable!(),
                EventDataView::ComplianceKeyRotation { .. } => unreachable!(),
                EventDataView::BaseUrlRotation { .. } => unreachable!(),
                EventDataView::Unknown { .. } => unreachable!(),

                EventDataView::ToLBRExchangeRateUpdate { .. } =>
                    ("to_lbr_exchange_rate_update", None, None),
                EventDataView::AdminTransaction { .. } =>
                    ("upgrade", None, None),
                EventDataView::NewEpoch { .. } =>
                    ("newepoch", None, None),
                EventDataView::NewBlock { .. } =>
                    ("newblock", None, None),
                EventDataView::CreateAccount { .. } =>
                    ("createaccount", None, None),

                EventDataView::ReceivedPayment { amount, receiver, .. } =>
                    ("receivedpayment", Some(Credit(amount)), Some(receiver)),
                EventDataView::SentPayment { amount, sender, .. } =>
                    ("sentpayment", Some(Debit(amount)), Some(sender)),
            };

            let type_ = type_.to_string();
            let status = status.to_string();
            let account = account.map(|account| {
                AccountIdentifier {
                    address: account.0.clone(),
                    sub_account: None,
                }
            });
            let amount = amount.map(|amount| {
                let (currency, value) = match amount {
                    Credit(amount) => (amount.currency.clone(), format!("{}", amount.amount)),
                    Debit(amount) => (amount.currency.clone(), format!("-{}", amount.amount)),
                };
                Amount {
                    value,
                    currency: Currency {
                        symbol: currency,
                        decimals: 6, // TODO: use get_currencies instead of hardcoding
                    },
                }
            });

            Operation {
                operation_identifier,
                related_operations: None,
                type_,
                status,
                account,
                amount,
            }
        })
        .collect::<Vec<_>>();

    // Handle transcation fees

    // There are no events for transaction fees, since gas is used regardless
    // of transaction status. We append the sent_fee operation to represent fee
    // payment. Fee receipt is not represented, since the fees do not live in a "balance"
    // under the association account, so are not visible to Rosetta.
    if let TransactionDataView::UserTransaction { sender, gas_unit_price, gas_currency, .. } = &tx[1].transaction {
        if *gas_unit_price > 0 {
            let value = gas_unit_price * tx[1].gas_used;

            let currency = Currency {
                symbol: gas_currency.clone(),
                decimals: 6, // TODO: use get_currencies instead of hardcoding
            };

            let status = "executed".to_string(); // NOTE: tx fees are always charged
            
            let sent_fee_op = Operation {
                operation_identifier: OperationIdentifier {
                    index: tx[1].events.len() as u64,
                    network_index: None,
                },
                related_operations: None,
                type_: "sentfee".to_string(),
                status: status.clone(),
                account: Some(AccountIdentifier {
                    address: sender.clone(),
                    sub_account: None,
                }),
                amount: Some(Amount {
                    value: format!("-{}", value),
                    currency: currency.clone(),
                }),
            };

            operations.push(sent_fee_op);
        }
    }

    let transactions = vec![
        Transaction {
            transaction_identifier: TransactionIdentifier {
                hash: tx[1].hash.clone(),
            },
            operations,
        }
    ];
    
    let block = Block {
        block_identifier,
        parent_block_identifier,
        timestamp,
        transactions,
    };
    
    let response = BlockResponse {
        block,
    };

    Ok(response)
}
