use crate::converter::{consensus::ConsensusConverter, index::IndexConverter};
use waglayla_notify::collector::CollectorFrom;

pub(crate) type CollectorFromConsensus = CollectorFrom<ConsensusConverter>;

pub(crate) type CollectorFromIndex = CollectorFrom<IndexConverter>;
