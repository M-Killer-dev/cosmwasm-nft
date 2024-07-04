pub use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw721_base::state::EmptyOptionalCw721Contract;
pub use cw721_base::Cw721Contract;

use cw721_base::entry::{execute as _execute, query as _query};

pub mod msg;
pub mod query;
pub mod state;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-non-transferable";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Cw721NonTransferableContract<'a> = EmptyOptionalCw721Contract<'a>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::query::admin;
    use crate::state::{Config, CONFIG};
    use cosmwasm_std::{
        entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    };
    use cw721_base::error::Cw721ContractError;
    use cw721_base::msg::{Cw721ExecuteMsg, Cw721InstantiateMsg};
    use cw721_base::traits::Cw721Execute;
    use cw721_base::{EmptyOptionalCollectionExtensionMsg, EmptyOptionalNftExtensionMsg};

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg<EmptyOptionalCollectionExtensionMsg>,
    ) -> Result<Response, Cw721ContractError> {
        let admin_addr: Option<Addr> = msg
            .admin
            .as_deref()
            .map(|s| deps.api.addr_validate(s))
            .transpose()?;

        let config = Config { admin: admin_addr };

        CONFIG.save(deps.storage, &config)?;

        let cw721_base_instantiate_msg = Cw721InstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            collection_info_extension: msg.collection_info_extension,
            minter: msg.minter,
            creator: msg.creator,
            withdraw_address: msg.withdraw_address,
        };

        Cw721NonTransferableContract::default().instantiate_with_version(
            deps.branch(),
            &env,
            &info,
            cw721_base_instantiate_msg,
            "contract_name",
            "contract_version",
        )?;

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::default()
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<
            EmptyOptionalNftExtensionMsg,
            EmptyOptionalCollectionExtensionMsg,
            Empty,
        >,
    ) -> Result<Response, Cw721ContractError> {
        let config = CONFIG.load(deps.storage)?;
        match config.admin {
            Some(admin) => {
                if admin == info.sender {
                    _execute(deps, env, info, msg)
                } else {
                    Err(Cw721ContractError::Ownership(
                        cw721_base::OwnershipError::NotOwner,
                    ))
                }
            }
            None => match msg {
                Cw721ExecuteMsg::Mint {
                    token_id,
                    owner,
                    token_uri,
                    extension,
                } => Cw721NonTransferableContract::default()
                    .mint(deps, &env, &info, token_id, owner, token_uri, extension),
                _ => Err(Cw721ContractError::Ownership(
                    cw721_base::OwnershipError::NotOwner,
                )),
            },
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, Cw721ContractError> {
        match msg {
            QueryMsg::Admin {} => Ok(to_json_binary(&admin(deps)?)?),
            _ => _query(deps, env, msg.into()),
        }
    }
}
