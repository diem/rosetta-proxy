use crate::{
    consts,
    diem::Diem,
    error::ApiError,
    filters::{handle, with_options},
    options::Options,
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, BlockIdentifier, Currency},
};
use log::debug;
use warp::Filter;

pub fn routes(
    options: Options,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post().and(
        warp::path!("account" / "balance")
            .and(warp::body::json())
            .and(with_options(options.clone()))
            .and_then(handle(account_balance)),
    )
}

async fn account_balance(
    account_balance_request: AccountBalanceRequest,
    options: Options,
) -> Result<AccountBalanceResponse, ApiError> {
    debug!("/account/balance");

    let network_identifier = account_balance_request.network_identifier;
    if network_identifier.blockchain != consts::BLOCKCHAIN
        || network_identifier.network != options.network
    {
        return Err(ApiError::BadNetwork);
    }

    // NOTE: we don't support lookups of account balance at specific blocks
    if account_balance_request.block_identifier.is_some() {
        return Err(ApiError::HistoricBalancesUnsupported);
    }

    let diem = Diem::new(&options.diem_endpoint);

    let address = account_balance_request.account_identifier.address;

    let (account, metadata) = diem.get_account_with_metadata(&address).await?;

    if account.is_none() {
        return Err(ApiError::AccountNotFound);
    }

    let account = account.unwrap();

    let tx = diem.get_transactions(metadata.version, 1, false).await?;

    let block_identifier = BlockIdentifier {
        index: tx[0].version,
        hash: tx[0].hash.clone().to_string(),
    };

    let balances = account
        .balances
        .iter()
        .map(|amount| {
            let value = format!("{}", amount.amount);
            let currency = Currency {
                symbol: amount.currency.clone(),
                decimals: 6, // TODO: use api to fetch this instead of hardcoding
            };

            Amount { value, currency }
        })
        .collect::<Vec<_>>();

    let response = AccountBalanceResponse {
        block_identifier,
        balances,
    };

    Ok(response)
}
