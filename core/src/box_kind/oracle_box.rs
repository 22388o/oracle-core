use std::convert::TryFrom;

use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

use crate::contracts::oracle::OracleContract;
use crate::contracts::oracle::OracleContractError;

pub trait OracleBox {
    fn contract(&self) -> &OracleContract;
    fn oracle_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn public_key(&self) -> ProveDlog;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Debug, Error)]
pub enum OracleBoxError {
    #[error("oracle box: no tokens found")]
    NoTokens,
    #[error("oracle box: no reward token found")]
    NoRewardToken,
    #[error("oracle box: no public key in R4")]
    NoPublicKey,
    #[error("oracle box: no epoch counter in R5")]
    NoEpochCounter,
    #[error("oracle box: no data point in R6")]
    NoDataPoint,
    #[error("oracle contract: {0:?}")]
    OracleContractError(#[from] OracleContractError),
}

// TODO: convert this one and others to named structs
#[derive(Clone)]
pub struct OracleBoxWrapper(ErgoBox, OracleContract);

impl OracleBoxWrapper {
    pub fn new(b: ErgoBox) -> Result<Self, OracleBoxError> {
        let _oracle_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .get(0)
            .ok_or(OracleBoxError::NoTokens)?
            .token_id
            .clone();
        let _reward_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .get(1)
            .ok_or(OracleBoxError::NoRewardToken)?
            .token_id
            .clone();

        if b.get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(OracleBoxError::NoPublicKey)?
            .try_extract_into::<EcPoint>()
            .is_err()
        {
            return Err(OracleBoxError::NoPublicKey);
        }

        if b.get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(OracleBoxError::NoEpochCounter)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(OracleBoxError::NoEpochCounter);
        }

        if b.get_register(NonMandatoryRegisterId::R6.into())
            .ok_or(OracleBoxError::NoDataPoint)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(OracleBoxError::NoDataPoint);
        }

        let contract = OracleContract::from_ergo_tree(b.ergo_tree.clone())?;

        Ok(Self(b, contract))
    }
}

impl OracleBox for OracleBoxWrapper {
    fn oracle_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }

    fn reward_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(1).unwrap().clone()
    }

    fn public_key(&self) -> ProveDlog {
        self.0
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
            .into()
    }

    fn epoch_counter(&self) -> u32 {
        self.0
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    fn rate(&self) -> u64 {
        self.0
            .get_register(NonMandatoryRegisterId::R6.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    fn get_box(&self) -> &ErgoBox {
        &self.0
    }

    fn contract(&self) -> &OracleContract {
        &self.1
    }
}

impl TryFrom<ErgoBox> for OracleBoxWrapper {
    type Error = OracleBoxError;

    fn try_from(value: ErgoBox) -> Result<Self, Self::Error> {
        OracleBoxWrapper::new(value)
    }
}

impl From<OracleBoxWrapper> for ErgoBox {
    fn from(w: OracleBoxWrapper) -> Self {
        w.0.clone()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_oracle_box_candidate(
    contract: &OracleContract,
    public_key: ProveDlog,
    datapoint: u64,
    epoch_counter: u32,
    oracle_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, (*public_key.h).clone().into());
    builder.set_register_value(NonMandatoryRegisterId::R5, (epoch_counter as i32).into());
    builder.set_register_value(NonMandatoryRegisterId::R6, (datapoint as i64).into());
    builder.add_token(oracle_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}

/// Make an ergo box candidate to be an output box on data point colection (refresh action)
/// Without data point and epoch counter to prevent it to be used as input on next collection
pub fn make_collected_oracle_box_candidate(
    contract: &OracleContract,
    public_key: ProveDlog,
    oracle_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, (*public_key.h).clone().into());
    builder.add_token(oracle_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}
