use crate::error::ApiError;
use libra_json_rpc_client::{
    views::{
        AccountView, MetadataView, TransactionView, VMStatusView,
    },
    AccountAddress, JsonRpcAsyncClient, JsonRpcAsyncClientError, JsonRpcBatch, JsonRpcResponse,
    SignedTransaction,
};
use std::str::FromStr;
use std::fmt::Display;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum LibraError {
    #[error("json-rpc error: {0}")]
    JsonRpcResponse(JsonRpcAsyncClientError),
    #[error("request failed: {0}")]
    RequestFailed(#[from] anyhow::Error),
    #[error("unexpected response (expected {expected:?}, found {found:?})")]
    UnexpectedResponse {
        expected: String,
        found: String,
    },
}

impl LibraError {
    pub fn unexpected_response<D1,D2>(expected: D1, found: D2) -> LibraError
    where
        D1: Display,
        D2: Display,
    {
        LibraError::UnexpectedResponse {
            expected: expected.to_string(),
            found: found.to_string(),
        }
    }
}

impl std::convert::From<LibraError> for warp::reject::Rejection {
    fn from(libra_error: LibraError) -> Self {
        let api_error: ApiError = libra_error.into();
        warp::reject::custom(api_error)
    }
}

impl std::convert::From<JsonRpcAsyncClientError> for LibraError {
    fn from(json_async_error: JsonRpcAsyncClientError) -> Self {
        LibraError::JsonRpcResponse(json_async_error)
    }
}

pub struct Libra {
    client: JsonRpcAsyncClient,
}

impl Libra {
    pub fn new(endpoint: &Url) -> Libra {
        Libra {
            client: JsonRpcAsyncClient::new(endpoint.clone()),
        }
    }

    pub async fn get_metadata(&self, version: Option<u64>) -> Result<MetadataView, LibraError> {
        let mut batch = JsonRpcBatch::new();
        batch.add_get_metadata_request(version);

        let mut result = self.client.execute(batch).await?;

        if result.len() != 1 {
            return Err(LibraError::unexpected_response("1 result", format!("{} results", result.len())));
        }


        let result = result.remove(0)?;
        match result {
            JsonRpcResponse::MetadataViewResponse(metadata) => Ok(metadata),
            _ => Err(LibraError::unexpected_response("MetadataViewResponse", "other")),
        }
    }

    pub async fn get_transactions(&self, start_version: u64, limit: u64, include_events: bool) -> Result<Vec<TransactionView>, LibraError> {
        let mut batch = JsonRpcBatch::new();
        batch.add_get_transactions_request(start_version, limit, include_events);

        let mut result = self.client.execute(batch).await?;

        if result.len() != 1 {
            return Err(LibraError::unexpected_response("1 result", format!("{} results", result.len())));
        }


        let result = result.remove(0)?;
        match result {
            JsonRpcResponse::TransactionsResponse(views) => Ok(views),
            _ => Err(LibraError::unexpected_response("TransactionsResponse", "other")),
        }
    }

    pub async fn get_network_status(&self) -> Result<u64, LibraError> {
        let mut batch = JsonRpcBatch::new();
        batch.add_get_network_status_request();

        let mut result = self.client.execute(batch).await?;

        if result.len() != 1 {
            return Err(LibraError::unexpected_response("1 result", format!("{} results", result.len())));
        }

        let result = result.remove(0)?;
        match result {
            JsonRpcResponse::NetworkStatusResponse(peer_count) => {
                peer_count.as_u64()
                    .ok_or_else(|| LibraError::unexpected_response("u64", "non-u64 number"))
            },
            _ => Err(LibraError::unexpected_response("NetworkStatusResponse", "other")),
        }
    }

    pub async fn get_account_with_metadata(&self, address: &str) -> Result<(Option<AccountView>, MetadataView), LibraError> {
        let mut batch = JsonRpcBatch::new();
        let account_address = AccountAddress::from_str(address)?;
        batch.add_get_account_request(account_address);
        batch.add_get_metadata_request(None);

        let mut result = self.client.execute(batch).await?;

        if result.len() != 2 {
            return Err(LibraError::unexpected_response("2 results", format!("{} results", result.len())));
        }

        let account_result = result.remove(0)?;
        let metadata_result = result.remove(0)?;

        if let (JsonRpcResponse::AccountResponse(account), JsonRpcResponse::MetadataViewResponse(metadata)) = (account_result, metadata_result) {
            Ok((account, metadata))
        } else {
            Err(LibraError::unexpected_response("(AccountResponse, MetadataViewResponse)", "other"))
        }
    }

    pub async fn submit(&self, transaction: &SignedTransaction) -> Result<(), LibraError> {
        let mut batch = JsonRpcBatch::new();
        batch.add_submit_request(transaction.clone()).expect("shouldn't fail to serialize a constructed type");

        let mut result = self.client.execute(batch).await?;

        if result.len() != 1 {
            return Err(LibraError::unexpected_response("1 result", format!("{} results", result.len())));
        }

        let result = result.remove(0)?;
        if matches!(result, JsonRpcResponse::SubmissionResponse) {
            Ok(())
        } else {
            Err(LibraError::unexpected_response("SubmissionResponse", "other"))
        }
    }
}

pub fn vmstatus_to_str(vm_status: &VMStatusView) -> &'static str {
    match vm_status {
        VMStatusView::Executed => "executed",
        VMStatusView::OutOfGas => "out-of-gas",
        VMStatusView::MoveAbort { .. }=> "move-abort",
        VMStatusView::ExecutionFailure { .. } => "execution-failure",
        VMStatusView::MiscellaneousError => "miscellaneous-error",
    }
}

pub fn vmstatus_all_strs() -> Vec<&'static str> {
    vec![
        "executed",
        "out-of-gas",
        "move-abort",
        "execution-failure",
        "verification-error",
        "deserializaton-error",
        "publishing-failure",
    ]
}
