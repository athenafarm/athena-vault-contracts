use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read};
use athena::controller::UserRole;

const KEY_CONFIG: &[u8] = b"config";
const PREFIX_KEY_USER_ROLE: &[u8] = b"user_role";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub governance: CanonicalAddr,
    pub treasury: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    Ok(singleton_read(storage, KEY_CONFIG).load()?)
}

pub fn store_user_role(
    storage: &mut dyn Storage,
    user: &CanonicalAddr,
    user_role: &UserRole,
) -> StdResult<()> {
    bucket(storage, PREFIX_KEY_USER_ROLE).save(&user.as_slice(), user_role)
}

pub fn read_user_role(storage: &dyn Storage, user: &CanonicalAddr) -> StdResult<UserRole> {
    bucket_read(storage, PREFIX_KEY_USER_ROLE).load(&user.as_slice())
}
