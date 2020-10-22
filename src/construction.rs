use anyhow::anyhow;
use crate::{
    consts,
    error::ApiError,
    filters::{handle, with_options},
    libra::Libra,
    options::Options,
    types::{
        AccountIdentifier, Amount,
        ConstructionCombineRequest, ConstructionCombineResponse,
        ConstructionDeriveRequest, ConstructionDeriveResponse,
        ConstructionHashRequest,
        ConstructionMetadataRequest, ConstructionMetadataResponse,
        ConstructionParseRequest, ConstructionParseResponse,
        ConstructionPayloadsRequest, ConstructionPayloadsResponse,
        ConstructionPreprocessRequest, ConstructionPreprocessResponse,
        ConstructionSubmitRequest,
        TransactionIdentifier, TransactionIdentifierResponse,
        ConstructionMetadata,
        Currency,
        MetadataOptions, Operation, OperationIdentifier,
        SigningPayload, SignatureType, CurveType,
    },
};
use libra_crypto::{
    ed25519::Ed25519PublicKey,
    ed25519::Ed25519Signature,
    hash::{CryptoHash, CryptoHasher},
    ValidCryptoMaterialStringExt};
use libra_types::{
    account_config::constants::coins,
    chain_id::ChainId,
    transaction::{
        authenticator::{AuthenticationKey, Scheme},
        RawTransaction, RawTransactionHasher, SignedTransaction, Transaction, TransactionPayload,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use log::debug;
use std::{
    convert::TryInto,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use transaction_builder_generated::stdlib::{self, ScriptCall};
use warp::Filter;


pub fn routes(options: Options) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
    warp::post()
        .and(
            warp::path!("construction" / "derive")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(derive))
        )
        .or(
            warp::path!("construction" / "preprocess")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(preprocess))
        )
        .or(
            warp::path!("construction" / "metadata")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(metadata))
        )
        .or(
            warp::path!("construction" / "payloads")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(payloads))
        )
        .or(
            warp::path!("construction" / "parse")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(parse))
        )
        .or(
            warp::path!("construction" / "combine")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(combine))
        )
        .or(
            warp::path!("construction" / "hash")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(hash))
        )
        .or(
            warp::path!("construction" / "submit")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(submit))
        )
}

async fn derive(derive_request: ConstructionDeriveRequest, options: Options) -> Result<ConstructionDeriveResponse, ApiError> {
    debug!("/construction/derive");

    let network_identifier = derive_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let public_key = Ed25519PublicKey::from_encoded_string(&derive_request.public_key.hex_bytes)
        .map_err(|_| ApiError::deserialization_failed("Ed25519PublicKey"))?;
    let address = AuthenticationKey::ed25519(&public_key).derived_address().to_string().to_lowercase();

    let sub_account = None;
    let account_identifier = AccountIdentifier {
        address,
        sub_account,
    };

    let response = ConstructionDeriveResponse {
        account_identifier,
    };

    Ok(response)
}

async fn preprocess(preprocess_request: ConstructionPreprocessRequest, options: Options) -> Result<ConstructionPreprocessResponse, ApiError> {
    debug!("/construction/preprocess");

    let network_identifier = preprocess_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let transfer = extract_transfer_from_operations(&preprocess_request.operations)
        .map_err(|e| ApiError::BadTransferOperations(format!("{}", e)))?;

    let response = ConstructionPreprocessResponse {
        options: MetadataOptions {
            sender_address: (&transfer.sender).into(),
        }
    };

    Ok(response)
}

// In order to construct a transaction, we need the chain id and the account sequence number.
async fn metadata(metadata_request: ConstructionMetadataRequest, options: Options) -> Result<ConstructionMetadataResponse, ApiError> {
    debug!("/construction/metadata");

    let network_identifier = metadata_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let address = metadata_request.options.sender_address;
    
    let libra = Libra::new(&options.libra_endpoint);
    let (account, metadata) = libra.get_account_with_metadata(&address).await?;

    if account.is_none() {
        return Err(ApiError::AccountNotFound);
    }

    let chain_id = metadata.chain_id;
    let sequence_number = account.unwrap().sequence_number;

    let metadata = ConstructionMetadata {
        chain_id,
        sequence_number,
    };
    let response = ConstructionMetadataResponse {
        metadata,
    };

    Ok(response)
}

async fn payloads(payloads_request: ConstructionPayloadsRequest, options: Options) -> Result<ConstructionPayloadsResponse, ApiError> {
    debug!("/construction/payloads");

    let network_identifier = payloads_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let ConstructionMetadata { chain_id, sequence_number } = payloads_request.metadata;

    // The only payload we allow to construct is a single p2p payment.

    // TODO: allow other currencies
    let transfer = extract_transfer_from_operations(&payloads_request.operations)
        .map_err(|e| ApiError::BadTransferOperations(format!("{}", e)))?;

    let sender = transfer.sender.clone();
    let max_gas_amount = 10_000;
    let gas_unit_price = 0;
    let gas_currency_code = transfer.currency.clone();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let expiration_timestamp_secs = (now + Duration::from_secs(10))
        .as_secs();

    let currency = TypeTag::Struct(StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("Coin1".to_string()).unwrap(),
        name: Identifier::new("Coin1".to_string()).unwrap(),
        type_params: vec![],
    });
    let payee = transfer.receiver.clone();
    let script = stdlib::encode_peer_to_peer_with_metadata_script(
        currency,
        payee,
        transfer.amount,
        vec![],
        vec![],
    );
    
    let raw_transaction = RawTransaction::new_script(
        sender,
        sequence_number,
        script,
        max_gas_amount,
        gas_unit_price,
        gas_currency_code,
        expiration_timestamp_secs,
        ChainId::new(chain_id),
    );

    let raw_bytes = lcs::to_bytes(&raw_transaction)?;
    let unsigned_transaction = hex::encode(raw_bytes);

    let mut bytes = RawTransactionHasher::seed().to_vec();
    lcs::serialize_into(&mut bytes, &raw_transaction)?;

    let payloads = vec![
        SigningPayload {
            address: (&sender).into(),
            hex_bytes: hex::encode(&bytes),
            signature_type: Some(SignatureType::Ed25519),
        }
    ];

    let response = ConstructionPayloadsResponse {
        unsigned_transaction,
        payloads,
    };

    Ok(response)
}

async fn parse(parse_request: ConstructionParseRequest, options: Options) -> Result<ConstructionParseResponse, ApiError> {
    debug!("/construction/parse");

    let network_identifier = parse_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let (raw_transaction, account_identifier_signers) = if parse_request.signed {
        let signed_bytes = hex::decode(parse_request.transaction)?;
        let checked_transaction = lcs::from_bytes::<SignedTransaction>(&signed_bytes)
            .map_err(|_| ApiError::deserialization_failed("SignedTransaction"))?
            .check_signature()
            .map_err(|_| ApiError::BadSignature)?;

        if matches!(checked_transaction.authenticator().scheme(), Scheme::MultiEd25519) {
            return Err(ApiError::BadSignatureType);
        }

        let raw_transaction = checked_transaction.into_raw_transaction();
        let signers = vec![
            AccountIdentifier {
                address: (&raw_transaction.sender()).into(),
                sub_account: None,
            },
        ];
        (raw_transaction, signers)
    } else {
        let raw_bytes = hex::decode(parse_request.transaction)?;
        let raw_transaction: RawTransaction = lcs::from_bytes(&raw_bytes)
            .map_err(|_| ApiError::deserialization_failed("RawTransaction"))?;
        (raw_transaction, vec![])
    };

    // verify that script is a peer to peer payment
    let (currency, payee, amount) = if let TransactionPayload::Script(script) = raw_transaction.clone().into_payload() {
        if let Some(ScriptCall::PeerToPeerWithMetadata { currency, payee, amount, .. }) = ScriptCall::decode(&script) {
            (currency, payee, amount)
        } else {
            return Err(ApiError::BadTransactionScript);
        }
    } else {
        return Err(ApiError::BadTransactionPayload);
    };

    // TODO: switch to coin_for_name()
    if currency != coins::coin1_tmp_tag() {
        return Err(ApiError::BadCoin);
    }

    let operations = vec![
        Operation {
            operation_identifier: OperationIdentifier {
                index: 0,
                network_index: None,
            },
            related_operations: None,
            type_: "sentpayment".to_string(),
            status: "".to_string(),
            account: Some(AccountIdentifier {
                address: (&raw_transaction.sender()).into(),
                sub_account: None,
            }),
            amount: Some(Amount {
                value: format!("-{}", amount),
                currency: Currency {
                    symbol: "Coin1".to_string(),
                    decimals: 6,
                },
            }),
        },
        Operation {
            operation_identifier: OperationIdentifier {
                index: 1,
                network_index: None,
            },
            related_operations: Some(vec![
                OperationIdentifier {
                    index: 0,
                    network_index: None,
                },
            ]),
            type_: "receivedpayment".to_string(),
            status: "".to_string(),
            account: Some(AccountIdentifier {
                address: (&payee).into(),
                sub_account: None,
            }),
            amount: Some(Amount {
                value: format!("{}", amount),
                currency: Currency {
                    symbol: "Coin1".to_string(),
                    decimals: 6,
                },
            }),
        },
    ];

    let response = ConstructionParseResponse {
        operations,
        account_identifier_signers,
    };

    Ok(response)
}

async fn combine(combine_request: ConstructionCombineRequest, options: Options) -> Result<ConstructionCombineResponse, ApiError> {
    debug!("/construction/combine");

    let network_identifier = combine_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let raw_bytes = hex::decode(combine_request.unsigned_transaction)?;
    let raw_transaction: RawTransaction = lcs::from_bytes(&raw_bytes)
        .map_err(|_| ApiError::deserialization_failed("RawTransaction"))?;

    if combine_request.signatures.len() != 1 {
        return Err(ApiError::BadSignatureCount);
    }

    let signature = &combine_request.signatures[0];

    if signature.signature_type != SignatureType::Ed25519 || signature.public_key.curve_type != CurveType::Edwards25519 {
        return Err(ApiError::BadSignatureType);
    }

    let public_key: Ed25519PublicKey = hex::decode(&signature.public_key.hex_bytes)?
        .as_slice()
        .try_into()
        .map_err(|_| ApiError::deserialization_failed("Ed25519PublicKey"))?;
    let signature: Ed25519Signature = hex::decode(&signature.hex_bytes)?
        .as_slice()
        .try_into()
        .map_err(|_| ApiError::deserialization_failed("Ed25519Signature"))?;

    let signed_transaction = SignedTransaction::new(raw_transaction, public_key, signature);
    // TODO: verify sig
    let signed_bytes = lcs::to_bytes(&signed_transaction)?;
    let signed_transaction = hex::encode(&signed_bytes);

    let response = ConstructionCombineResponse {
        signed_transaction,
    };

    Ok(response)
}

async fn hash(hash_request: ConstructionHashRequest, options: Options) -> Result<TransactionIdentifierResponse, ApiError> {
    debug!("/construction/hash");

    let network_identifier = hash_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let signed_bytes = hex::decode(&hash_request.signed_transaction)?;
    let signed_transaction: SignedTransaction = lcs::from_bytes(&signed_bytes)
        .map_err(|_| ApiError::deserialization_failed("SignedTransaction"))?;
    let hash = Transaction::UserTransaction(signed_transaction).hash().to_hex();

    let transaction_identifier = TransactionIdentifier {
        hash,
    };

    let response = TransactionIdentifierResponse {
        transaction_identifier,
    };

    Ok(response)
}

async fn submit(submit_request: ConstructionSubmitRequest, options: Options) -> Result<TransactionIdentifierResponse, ApiError> {
    debug!("/construction/submit");

    let network_identifier = submit_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN || network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let signed_bytes = hex::decode(&submit_request.signed_transaction)?;
    let signed_transaction: SignedTransaction = lcs::from_bytes(&signed_bytes)
        .map_err(|_| ApiError::deserialization_failed("SignedTransaction"))?;

    let libra = Libra::new(&options.libra_endpoint);
    libra.submit(&signed_transaction).await?;

    let hash = Transaction::UserTransaction(signed_transaction).hash().to_hex();

    let transaction_identifier = TransactionIdentifier {
        hash,
    };

    let response = TransactionIdentifierResponse {
        transaction_identifier,
    };

    Ok(response)
}

#[derive(Clone, Copy, Debug)]
enum Value {
    Credit(u64),
    Debit(u64),
}

impl Value {
    fn reconciles(&self, value: Value) -> bool {
        match (*self, value) {
            (Value::Credit(c), Value::Debit(d)) => c == d,
            (Value::Debit(d), Value::Credit(c)) => d == c,
            _ => false,
        }
    }

    fn amount(self) -> u64 {
        match self {
            Value::Credit(v) => v,
            Value::Debit(v) => v,
        }
    }
}

impl FromStr for Value {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(anyhow!("empty input"));
        }
        
        let (negative, number) = match s.strip_prefix("-") {
            None => (false, s),
            Some(num) => (true, num),
        };

        let v = number.parse::<u64>()?;

        match negative {
            true => Ok(Value::Debit(v)),
            false => Ok(Value::Credit(v)),
        }
    }
}

struct Transfer {
    sender: AccountAddress,
    receiver: AccountAddress,
    amount: u64,
    currency: String,
}

fn extract_transfer_from_operations(operations: &[Operation]) -> Result<Transfer, anyhow::Error> {
    if operations.len() != 2 {
        return Err(anyhow!("wrong number of ops"));
    }

    let is_p2p = match (operations[0].type_.as_ref(), operations[1].type_.as_ref()) {
        ("sentpayment", "receivedpayment") => true,
        ("receivedpayment", "sentpayment") => true,
        _ => false,
    };

    if !is_p2p {
        return Err(anyhow!("operations don't represent a transfer"));
    }

    if operations[0].account.is_none() || operations[0].amount.is_none() || operations[1].account.is_none() || operations[1].amount.is_none() {
        return Err(anyhow!("accounts/amounts missing"));
    }

    let (send_account, send_amount, recv_account, recv_amount) = if operations[0].type_ == "sentpayment" {
        (
            operations[0].account.as_ref().unwrap(),
            operations[0].amount.as_ref().unwrap(),
            operations[1].account.as_ref().unwrap(),
            operations[1].amount.as_ref().unwrap(),
        )
    } else {
        (
            operations[1].account.as_ref().unwrap(),
            operations[1].amount.as_ref().unwrap(),
            operations[0].account.as_ref().unwrap(),
            operations[0].amount.as_ref().unwrap(),
        )
    };

    if send_amount.currency != recv_amount.currency {
        return Err(anyhow!("mismatched currencies in ops"));
    }

    let send_value = send_amount.value.parse::<Value>()?;
    let recv_value = recv_amount.value.parse::<Value>()?;

    if let Value::Credit(_) = send_value {
        return Err(anyhow!("can't send negative amounts"));
    }

    if !send_value.reconciles(recv_value) {
        return Err(anyhow!("send and recv amounts don't net out"));
    }

    let sender = send_account.address.parse::<AccountAddress>()?;
    let receiver = recv_account.address.parse::<AccountAddress>()?;
    let amount = send_value.amount();
    let currency = send_amount.currency.symbol.clone();
 
    Ok(Transfer {
        sender,
        receiver,
        amount,
        currency,
    })
}