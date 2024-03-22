mod error;
mod execute;
pub mod msg;
mod query;
pub mod state;

#[cfg(test)]
mod contract_tests;

use cosmwasm_std::Empty;
use cw721_base::DefaultOptionNftMetadataExtension;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-expiration";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type MinterResponse = cw721_base::msg::MinterResponse;

pub type NftInfo = cw721_base::state::NftInfo<DefaultOptionNftMetadataExtension>;

pub mod entry {
    use crate::{
        error::ContractError,
        msg::{InstantiateMsg, QueryMsg},
        state::Cw721ExpirationContract,
    };

    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw721_base::{
        msg::Cw721ExecuteMsg, DefaultOptionCollectionMetadataExtension,
        DefaultOptionCollectionMetadataExtensionMsg, DefaultOptionNftMetadataExtension,
        DefaultOptionNftMetadataExtensionMsg, RoyaltyInfo,
    };

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg<DefaultOptionCollectionMetadataExtensionMsg>,
    ) -> Result<Response, ContractError> {
        let contract = Cw721ExpirationContract::<
            DefaultOptionNftMetadataExtension,
            DefaultOptionNftMetadataExtensionMsg,
            DefaultOptionCollectionMetadataExtension,
            DefaultOptionCollectionMetadataExtensionMsg,
            Empty,
        >::default();
        contract.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<
            DefaultOptionNftMetadataExtensionMsg,
            DefaultOptionCollectionMetadataExtensionMsg,
        >,
    ) -> Result<Response, ContractError> {
        let contract = Cw721ExpirationContract::<
            DefaultOptionNftMetadataExtension,
            DefaultOptionNftMetadataExtensionMsg,
            DefaultOptionCollectionMetadataExtension,
            DefaultOptionCollectionMetadataExtensionMsg,
            Empty,
        >::default();
        contract.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(
        deps: Deps,
        env: Env,
        msg: QueryMsg<DefaultOptionNftMetadataExtension, DefaultOptionCollectionMetadataExtension>,
    ) -> Result<Binary, ContractError> {
        let contract = Cw721ExpirationContract::<
            DefaultOptionNftMetadataExtension,
            DefaultOptionNftMetadataExtensionMsg,
            DefaultOptionCollectionMetadataExtension,
            DefaultOptionCollectionMetadataExtensionMsg,
            Empty,
        >::default();
        contract.query(deps, env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
        // TODO: allow migration e.g. from cw721-base
        panic!("This contract does not support migrations")
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw2::ContractVersion;
    use cw721_base::{
        DefaultOptionCollectionMetadataExtension, DefaultOptionCollectionMetadataExtensionMsg,
        DefaultOptionNftMetadataExtensionMsg,
    };

    use crate::{error::ContractError, msg::InstantiateMsg, state::Cw721ExpirationContract};

    use super::*;

    #[test]
    fn proper_cw2_initialization() {
        let mut deps = mock_dependencies();

        // assert min expiration
        let error = entry::instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("mrt", &[]),
            InstantiateMsg {
                expiration_days: 0,
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                collection_metadata_extension: None,
                minter: Some("minter".into()),
                creator: Some("creator".into()),
                withdraw_address: None,
            },
        )
        .unwrap_err();
        assert_eq!(error, ContractError::MinExpiration {});

        // Make sure cw2 version info is properly initialized during instantiation.
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("mrt", &[]),
            InstantiateMsg {
                expiration_days: 1,
                name: "name".into(),
                symbol: "symbol".into(),
                collection_metadata_extension: None,
                minter: Some("minter".into()),
                creator: Some("creator".into()),
                withdraw_address: None,
            },
        )
        .unwrap();
        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(
            version,
            ContractVersion {
                contract: CONTRACT_NAME.into(),
                version: CONTRACT_VERSION.into(),
            },
        );

        assert_eq!(
            1,
            Cw721ExpirationContract::<
                DefaultOptionNftMetadataExtension,
                DefaultOptionNftMetadataExtensionMsg,
                DefaultOptionCollectionMetadataExtension,
                DefaultOptionCollectionMetadataExtensionMsg,
                Empty,
            >::default()
            .expiration_days
            .load(deps.as_ref().storage)
            .unwrap()
        );
    }
}
