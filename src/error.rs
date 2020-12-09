use crate::{
    diem,
    types::{self, ErrorDetails},
};
use thiserror::Error;
use warp::{http::StatusCode, reply::Reply};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("bad block request")]
    BadBlockRequest,
    #[error("diem error: {0:?}")]
    DiemError(#[from] diem::DiemError),
    #[error("bad network")]
    BadNetwork,
    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("serialization failed: {0:?}")]
    SerializationFailed(#[from] lcs::Error),
    #[error("bad transfer operations")]
    BadTransferOperations(String),
    #[error("account not found")]
    AccountNotFound,
    #[error("system time error: {0:?}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("hex decoding failed: {0:?}")]
    HexDecodingFailed(#[from] hex::FromHexError),
    #[error("bad signature")]
    BadSignature,
    #[error("bad signature type")]
    BadSignatureType,
    #[error("bad transaction script")]
    BadTransactionScript,
    #[error("bad transaction payload")]
    BadTransactionPayload,
    #[error("bad coin")]
    BadCoin,
    #[error("bad siganture count")]
    BadSignatureCount,
    #[error("historic balances unsupported")]
    HistoricBalancesUnsupported,
}

impl ApiError {
    pub fn code(&self) -> u64 {
        match self {
            ApiError::BadBlockRequest => 20,
            ApiError::DiemError(_) => 30,
            ApiError::BadNetwork => 40,
            ApiError::DeserializationFailed(_) => 50,
            ApiError::SerializationFailed(_) => 60,
            ApiError::BadTransferOperations(_) => 70,
            ApiError::AccountNotFound => 80,
            ApiError::SystemTimeError(_) => 90,
            ApiError::HexDecodingFailed(_) => 100,
            ApiError::BadSignature => 110,
            ApiError::BadSignatureType => 120,
            ApiError::BadTransactionScript => 130,
            ApiError::BadTransactionPayload => 140,
            ApiError::BadCoin => 150,
            ApiError::BadSignatureCount => 160,
            ApiError::HistoricBalancesUnsupported => 170,
        }
    }

    pub fn retriable(&self) -> bool {
        match self {
            ApiError::BadBlockRequest => false,
            ApiError::DiemError(_) => true,
            ApiError::BadNetwork => false,
            ApiError::DeserializationFailed(_) => false,
            ApiError::SerializationFailed(_) => false,
            ApiError::BadTransferOperations(_) => false,
            ApiError::AccountNotFound => true,
            ApiError::SystemTimeError(_) => true,
            ApiError::HexDecodingFailed(_) => false,
            ApiError::BadSignature => false,
            ApiError::BadSignatureType => false,
            ApiError::BadTransactionScript => false,
            ApiError::BadTransactionPayload => false,
            ApiError::BadCoin => false,
            ApiError::BadSignatureCount => false,
            ApiError::HistoricBalancesUnsupported => false,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadBlockRequest => StatusCode::BAD_REQUEST,
            ApiError::DiemError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadNetwork => StatusCode::BAD_REQUEST,
            ApiError::DeserializationFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::SerializationFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::BadTransferOperations(_) => StatusCode::BAD_REQUEST,
            ApiError::AccountNotFound => StatusCode::NOT_FOUND,
            ApiError::SystemTimeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::HexDecodingFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::BadSignature => StatusCode::BAD_REQUEST,
            ApiError::BadSignatureType => StatusCode::BAD_REQUEST,
            ApiError::BadTransactionScript => StatusCode::BAD_REQUEST,
            ApiError::BadTransactionPayload => StatusCode::BAD_REQUEST,
            ApiError::BadCoin => StatusCode::BAD_REQUEST,
            ApiError::BadSignatureCount => StatusCode::BAD_REQUEST,
            ApiError::HistoricBalancesUnsupported => StatusCode::BAD_REQUEST,
        }
    }

    pub fn message(&self) -> String {
        let full = format!("{}", self);
        let parts: Vec<_> = full.split(":").collect();
        parts[0].to_string()
    }

    pub(crate) fn details(&self) -> ErrorDetails {
        let error = format!("{}", self);
        ErrorDetails { error }
    }

    pub fn deserialization_failed(type_: &str) -> ApiError {
        ApiError::DeserializationFailed(type_.to_string())
    }

    pub(crate) fn all_errors() -> Vec<types::Error> {
        vec![
            types::Error {
                message: "bad block request".to_string(),
                code: 20,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "diem error".to_string(),
                code: 30,
                retriable: true,
                details: None,
            },
            types::Error {
                message: "bad network".to_string(),
                code: 40,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "deserialization failed".to_string(),
                code: 50,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "serialization failed".to_string(),
                code: 60,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad transfer operations".to_string(),
                code: 70,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "account not found".to_string(),
                code: 80,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "system time error".to_string(),
                code: 90,
                retriable: true,
                details: None,
            },
            types::Error {
                message: "hex decoding failed".to_string(),
                code: 100,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad signature".to_string(),
                code: 110,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad signature type".to_string(),
                code: 120,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad transaction script".to_string(),
                code: 130,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad transaction payload".to_string(),
                code: 140,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad coin".to_string(),
                code: 150,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "bad signature count".to_string(),
                code: 160,
                retriable: false,
                details: None,
            },
            types::Error {
                message: "historic balances unsupported".to_string(),
                code: 170,
                retriable: false,
                details: None,
            },
        ]
    }

    pub fn into_error(self) -> types::Error {
        types::Error {
            message: self.message(),
            code: self.code(),
            retriable: self.retriable(),
            details: Some(self.details()),
        }
    }
}

impl warp::reject::Reject for ApiError {}

impl std::convert::From<ApiError> for warp::reject::Rejection {
    fn from(api_error: ApiError) -> Self {
        warp::reject::custom(api_error)
    }
}

impl Reply for ApiError {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self.into_error()).into_response()
    }
}
