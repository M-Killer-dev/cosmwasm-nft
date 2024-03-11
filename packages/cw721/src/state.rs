use std::marker::PhantomData;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, CustomMsg, Decimal, Empty, StdResult, Storage, Timestamp};
use cw_ownable::{OwnershipStore, OWNERSHIP_KEY};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::Expiration;
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::error::Cw721ContractError;
use crate::execute::Update;
use crate::msg::CollectionInfoExtensionMsg;

/// Creator owns this contract and can update collection info!
/// !!! Important note here: !!!
/// - creator is stored using using cw-ownable's OWNERSHIP singleton, so it is not stored here
/// - in release v0.18.0 it was used for minter (which is confusing), but now it is used for creator
pub const CREATOR: OwnershipStore = OwnershipStore::new(OWNERSHIP_KEY);
/// - minter is stored in the contract storage using cw_ownable::OwnershipStore (same as for OWNERSHIP but with different key)
pub const MINTER: OwnershipStore = OwnershipStore::new("collection_minter");

/// Default CollectionInfoExtension with RoyaltyInfo
pub type DefaultOptionCollectionInfoExtension = Option<CollectionInfoExtension<RoyaltyInfo>>;
pub type DefaultOptionMetadataExtension = Option<Metadata>;

// explicit type for better distinction.
pub type EmptyMsg = Empty;

// ----------------------
// NOTE: below are max restrictions for default CollectionInfoExtension
// This may be quite restrictive and may be increased in the future.
// Custom contracts may also provide a custom CollectionInfoExtension.

/// Maximum length of the description field in the collection info.
pub const MAX_DESCRIPTION_LENGTH: u32 = 512;
/// Max increase/decrease of of royalty share percentage
pub const MAX_SHARE_DELTA_PCT: u64 = 2;
/// Max royalty share percentage
pub const MAX_ROYALTY_SHARE_PCT: u64 = 10;
// ----------------------

pub struct Cw721Config<
    'a,
    // Metadata defined in NftInfo (used for mint).
    TMetadataExtension,
    // Defines for `CosmosMsg::Custom<T>` in response. Barely used, so `Empty` can be used.
    TCustomResponseMessage,
    // Message passed for updating metadata.
    TMetadataExtensionMsg,
    // Extension defined in CollectionInfo.
    TCollectionInfoExtension,
    // Message passed for updating collection info extension.
    TCollectionInfoExtensionMsg,
> where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
    TCollectionInfoExtension: Serialize + DeserializeOwned + Clone,
    TCollectionInfoExtensionMsg: Serialize + DeserializeOwned + Clone,
{
    /// Note: replaces deprecated/legacy key "nft_info"!
    pub collection_info: Item<'a, CollectionInfo<TCollectionInfoExtension>>,
    pub token_count: Item<'a, u64>,
    /// Stored as (granter, operator) giving operator full control over granter's account.
    /// NOTE: granter is the owner, so operator has only control for NFTs owned by granter!
    pub operators: Map<'a, (&'a Addr, &'a Addr), Expiration>,
    pub nft_info:
        IndexedMap<'a, &'a str, NftInfo<TMetadataExtension>, TokenIndexes<'a, TMetadataExtension>>,
    pub withdraw_address: Item<'a, String>,

    pub(crate) _custom_response: PhantomData<TCustomResponseMessage>,
    pub(crate) _custom_metadata_extension_msg: PhantomData<TMetadataExtensionMsg>,
    pub(crate) _custom_collection_info_extension_msg: PhantomData<TCollectionInfoExtensionMsg>,
}

impl<
        TMetadataExtension,
        TCustomResponseMessage,
        TMetadataExtensionMsg,
        TCollectionInfoExtension,
        TCollectionInfoExtensionMsg,
    > Default
    for Cw721Config<
        'static,
        TMetadataExtension,
        TCustomResponseMessage,
        TMetadataExtensionMsg,
        TCollectionInfoExtension,
        TCollectionInfoExtensionMsg,
    >
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
    TCollectionInfoExtension: Serialize + DeserializeOwned + Clone,
    TCollectionInfoExtensionMsg: Serialize + DeserializeOwned + Clone,
{
    fn default() -> Self {
        Self::new(
            "collection_info", // Note: replaces deprecated/legacy key "nft_info"
            "num_tokens",
            "operators",
            "tokens",
            "tokens__owner",
            "withdraw_address",
        )
    }
}

impl<
        'a,
        TMetadataExtension,
        TCustomResponseMessage,
        TMetadataExtensionMsg,
        TCollectionInfoExtension,
        TCollectionInfoExtensionMsg,
    >
    Cw721Config<
        'a,
        TMetadataExtension,
        TCustomResponseMessage,
        TMetadataExtensionMsg,
        TCollectionInfoExtension,
        TCollectionInfoExtensionMsg,
    >
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
    TCollectionInfoExtension: Serialize + DeserializeOwned + Clone,
    TCollectionInfoExtensionMsg: Serialize + DeserializeOwned + Clone,
{
    fn new(
        collection_info_key: &'a str,
        token_count_key: &'a str,
        operator_key: &'a str,
        nft_info_key: &'a str,
        nft_info_owner_key: &'a str,
        withdraw_address_key: &'a str,
    ) -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, nft_info_key, nft_info_owner_key),
        };
        Self {
            collection_info: Item::new(collection_info_key),
            token_count: Item::new(token_count_key),
            operators: Map::new(operator_key),
            nft_info: IndexedMap::new(nft_info_key, indexes),
            withdraw_address: Item::new(withdraw_address_key),
            _custom_response: PhantomData,
            _custom_metadata_extension_msg: PhantomData,
            _custom_collection_info_extension_msg: PhantomData,
        }
    }

    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }
}

pub fn token_owner_idx<TMetadataExtension>(_pk: &[u8], d: &NftInfo<TMetadataExtension>) -> Addr {
    d.owner.clone()
}

#[cw_serde]
pub struct NftInfo<TMetadataExtension> {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: TMetadataExtension,
}

#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

pub struct TokenIndexes<'a, TMetadataExtension>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    pub owner: MultiIndex<'a, Addr, NftInfo<TMetadataExtension>, String>,
}

impl<'a, TMetadataExtension> IndexList<NftInfo<TMetadataExtension>>
    for TokenIndexes<'a, TMetadataExtension>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<NftInfo<TMetadataExtension>>> + '_> {
        let v: Vec<&dyn Index<NftInfo<TMetadataExtension>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

#[cw_serde]
pub struct CollectionInfo<TCollectionInfoExtension> {
    pub name: String,
    pub symbol: String,
    pub extension: TCollectionInfoExtension,
    pub updated_at: Timestamp,
}

#[cw_serde]
pub struct CollectionInfoExtension<TRoyaltyInfo> {
    pub description: String,
    pub image: String,
    pub external_link: Option<String>,
    pub explicit_content: Option<bool>,
    pub start_trading_time: Option<Timestamp>,
    pub royalty_info: Option<TRoyaltyInfo>,
}

pub trait Validate {
    fn validate(&self) -> Result<(), Cw721ContractError>;
}

impl Validate for Empty {
    fn validate(&self) -> Result<(), Cw721ContractError> {
        Ok(())
    }
}

impl Validate for Option<Empty> {
    fn validate(&self) -> Result<(), Cw721ContractError> {
        match self {
            Some(_) => Ok(()),
            None => Ok(()),
        }
    }
}

impl<TRoyaltyInfo> Validate for CollectionInfoExtension<TRoyaltyInfo> {
    /// Validates only extension, not royalty info!
    fn validate(&self) -> Result<(), Cw721ContractError> {
        // check description length, must not be empty and max 512 chars
        if self.description.is_empty() {
            return Err(Cw721ContractError::CollectionDescriptionEmpty {});
        }
        if self.description.len() > MAX_DESCRIPTION_LENGTH as usize {
            return Err(Cw721ContractError::CollectionDescriptionTooLong {});
        }

        // check images are URLs
        Url::parse(&self.image)?;
        if self.external_link.as_ref().is_some() {
            Url::parse(self.external_link.as_ref().unwrap())?;
        }

        Ok(())
    }
}

impl Validate for Option<CollectionInfoExtension<RoyaltyInfo>> {
    fn validate(&self) -> Result<(), Cw721ContractError> {
        match self {
            Some(ext) => {
                ext.validate()?;
                ext.royalty_info
                    .as_ref()
                    .map_or(Ok(()), |r| r.validate(None))
            }
            None => Ok(()),
        }
    }
}

impl Update<EmptyMsg> for Empty {
    fn update(&self, _msg: &EmptyMsg) -> Result<Self, crate::error::Cw721ContractError> {
        Ok(Empty::default())
    }
}

impl Update<EmptyMsg> for Option<Empty> {
    fn update(&self, _msg: &EmptyMsg) -> Result<Self, crate::error::Cw721ContractError> {
        match self {
            Some(ext) => Ok(Some(ext.clone())),
            None => Ok(Some(Empty::default())),
        }
    }
}

impl Update<CollectionInfoExtensionMsg<RoyaltyInfo>> for CollectionInfoExtension<RoyaltyInfo> {
    fn update(
        &self,
        msg: &CollectionInfoExtensionMsg<RoyaltyInfo>,
    ) -> Result<Self, crate::error::Cw721ContractError> {
        let mut extension = self.clone();
        // validate royalty before updating
        if let Some(royalty_info) = &extension.royalty_info {
            royalty_info.validate(msg.royalty_info.clone())?;
        }
        extension.description = msg.description.clone().unwrap_or(self.description.clone());
        extension.image = msg.image.clone().unwrap_or(self.image.clone());
        extension.external_link = msg.external_link.clone().or(self.external_link.clone());
        extension.explicit_content = msg.explicit_content.or(self.explicit_content);
        extension.start_trading_time = msg.start_trading_time.or(self.start_trading_time);
        extension.royalty_info = msg.royalty_info.clone().or(self.royalty_info.clone());

        // check description length, must not be empty and max 512 chars
        if extension.description.is_empty() {
            return Err(crate::error::Cw721ContractError::CollectionDescriptionEmpty {});
        }
        if extension.description.len() > MAX_DESCRIPTION_LENGTH as usize {
            return Err(crate::error::Cw721ContractError::CollectionDescriptionTooLong {});
        }

        // check images are URLs
        Url::parse(&extension.image)?;
        if extension.external_link.as_ref().is_some() {
            Url::parse(extension.external_link.as_ref().unwrap())?;
        }

        Ok(extension)
    }
}

impl Update<CollectionInfoExtensionMsg<RoyaltyInfo>>
    for Option<CollectionInfoExtension<RoyaltyInfo>>
{
    fn update(
        &self,
        msg: &CollectionInfoExtensionMsg<RoyaltyInfo>,
    ) -> Result<Self, crate::error::Cw721ContractError> {
        match self {
            Some(ext) => {
                let updated = ext.update(msg)?;
                Ok(Some(updated))
            }
            None => Ok(Some(CollectionInfoExtension {
                description: msg.description.clone().unwrap_or_default(),
                image: msg.image.clone().unwrap_or_default(),
                external_link: msg.external_link.clone(),
                explicit_content: msg.explicit_content,
                start_trading_time: msg.start_trading_time,
                royalty_info: msg.royalty_info.clone(),
            })),
        }
    }
}

#[cw_serde]
pub struct RoyaltyInfo {
    pub payment_address: Addr,
    pub share: Decimal,
}

impl RoyaltyInfo {
    pub fn validate(
        &self,
        new_royalty_info: Option<RoyaltyInfo>,
    ) -> Result<(), Cw721ContractError> {
        match new_royalty_info {
            Some(new_royalty_info) => {
                if self.share < new_royalty_info.share {
                    let share_delta = new_royalty_info.share.abs_diff(self.share);

                    if share_delta > Decimal::percent(MAX_SHARE_DELTA_PCT) {
                        return Err(Cw721ContractError::InvalidRoyalties(format!(
                            "Share increase cannot be greater than {MAX_SHARE_DELTA_PCT}%"
                        )));
                    }
                }
                if new_royalty_info.share > Decimal::percent(MAX_ROYALTY_SHARE_PCT) {
                    return Err(Cw721ContractError::InvalidRoyalties(format!(
                        "Share cannot be greater than {MAX_ROYALTY_SHARE_PCT}%"
                    )));
                }
                Ok(())
            }
            None => {
                if self.share > Decimal::percent(MAX_ROYALTY_SHARE_PCT) {
                    return Err(Cw721ContractError::InvalidRoyalties(format!(
                        "Share cannot be greater than {MAX_ROYALTY_SHARE_PCT}%"
                    )));
                }
                Ok(())
            }
        }
    }
}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}
