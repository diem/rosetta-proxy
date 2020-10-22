use crate::{
    consts,
    error::ApiError,
    filters::{EmptyRequest, handle, with_empty_request, with_options},
    libra::{self, Libra},
    options::Options,
    types::{
        Allow, BlockIdentifier, NetworkIdentifier, NetworkListResponse,
        NetworkOptionsResponse, NetworkRequest, NetworkStatusResponse,
        OperationStatus, Peer, Version,
    },
};
use log::debug;
use warp::Filter;

pub fn routes(options: Options) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
    warp::post()
        .and(
            warp::path!("network" / "list")
                .and(with_empty_request())
                .and(with_options(options.clone()))
                .and_then(handle(network_list))
        )
        .or(
            warp::path!("network" / "options")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(network_options))
        )
        .or(
            warp::path!("network" / "status")
                .and(warp::body::json())
                .and(with_options(options.clone()))
                .and_then(handle(network_status))
        )
}


async fn network_list(_empty: EmptyRequest, options: Options) -> Result<NetworkListResponse, ApiError> {
    debug!("/network/list");
    let response = NetworkListResponse {
        network_identifiers: vec![
            NetworkIdentifier {
                blockchain: consts::BLOCKCHAIN.to_string(),
                network: options.network.clone(),
                sub_network_identifier: None,
            }
        ],
    };
    
    Ok(response)
}

async fn network_options(network_request: NetworkRequest, options: Options) -> Result<NetworkOptionsResponse, ApiError> {
    debug!("/network/options");
    if network_request.network_identifier.blockchain != consts::BLOCKCHAIN || network_request.network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let version = Version {
        rosetta_version: consts::ROSETTA_VERSION.to_string(),
        node_version: consts::NODE_VERSION.to_string(),
        middleware_version: consts::MIDDLEWARE_VERSION.to_string(),
    };

    let mut operation_statuses = Vec::new();
    for op in libra::vmstatus_all_strs() {
        operation_statuses.push(OperationStatus {
            status: op.to_string(),
            successful: op == "executed",
        });
    }

    let operation_types = vec![
        "burn".to_string(),
        "cancelburn".to_string(),
        "mint".to_string(),
        "to_lbr_exchange_rate_update".to_string(),
        "preburn".to_string(),
        "receivedpayment".to_string(),
        "sentpayment".to_string(),
        "upgrade".to_string(),
        "newepoch".to_string(),
        "newblock".to_string(),
        "createaccount".to_string(),
        "unknown".to_string(),
        "sentfee".to_string(), // NOTE: not from libra events, since tx fees aren't events
        "receivedfee".to_string(), // NOTE: not from libra events, since tx fees aren't events
    ];

    let errors = ApiError::all_errors();

    let allow = Allow {
        operation_statuses,
        operation_types,
        errors,
        historical_balance_lookup: false,
        timestamp_start_index: Some(3), // FIXME: hardcoded based on current testnet
        call_methods: vec![],
        balance_exemptions: vec![],
    };

    let response = NetworkOptionsResponse {
        version,
        allow,
    };

    Ok(response)
}

async fn network_status(network_request: NetworkRequest, options: Options) -> Result<NetworkStatusResponse, ApiError> {
    debug!("/network/status");
    if network_request.network_identifier.blockchain != consts::BLOCKCHAIN || network_request.network_identifier.network != options.network {
        return Err(ApiError::BadNetwork);
    }

    let libra = Libra::new(&options.libra_endpoint);
    let metadata = libra.get_metadata(None).await?;

    let genesis_tx = libra.get_transactions(0, 1, false).await?;
    let latest_tx = libra.get_transactions(metadata.version, 1, false).await?;
    let num_peers = libra.get_network_status().await?;

    let genesis_block_identifier = BlockIdentifier {
        index: genesis_tx[0].version,
        hash: genesis_tx[0].hash.clone(),
    };

    // note: libra timestamps are in microseconds, so we convert to milliseconds
    let current_block_timestamp = metadata.timestamp / 1000;

    let current_block_identifier = BlockIdentifier {
        index: latest_tx[0].version,
        hash: latest_tx[0].hash.clone(),
    };

    let peers: Vec<Peer> = (0..num_peers)
        .map(|i| Peer {
            peer_id: format!("peer{}", i),
        })
        .collect();

    let response = NetworkStatusResponse {
        current_block_identifier,
        current_block_timestamp,
        genesis_block_identifier,
        peers,
    };
    
    Ok(response)
}
