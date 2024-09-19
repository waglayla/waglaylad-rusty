use std::{convert::TryInto, mem::size_of};

use waglayla_consensus_core::{
    BlockHashMap,
    BlockHashSet,
    coinbase::*,
    errors::coinbase::{CoinbaseError, CoinbaseResult},
    subnets, tx::{ScriptPublicKey, ScriptVec, Transaction, TransactionOutput},
};

use crate::{constants, model::stores::ghostdag::GhostdagData};

const LENGTH_OF_BLUE_SCORE: usize = size_of::<u64>();
const LENGTH_OF_SUBSIDY: usize = size_of::<u64>();
const LENGTH_OF_SCRIPT_PUB_KEY_VERSION: usize = size_of::<u16>();
const LENGTH_OF_SCRIPT_PUB_KEY_LENGTH: usize = size_of::<u8>();

const MIN_PAYLOAD_LENGTH: usize =
    LENGTH_OF_BLUE_SCORE + LENGTH_OF_SUBSIDY + LENGTH_OF_SCRIPT_PUB_KEY_VERSION + LENGTH_OF_SCRIPT_PUB_KEY_LENGTH;

// We define a year as 365.25 days and a month as 365.25 / 12 = 30.4375
// SECONDS_PER_MONTH = 30.4375 * 24 * 60 * 60
const SECONDS_PER_MONTH: u64 = 2629800;

pub const SUBSIDY_BY_MONTH_TABLE_SIZE: usize = 366;
pub type SubsidyByMonthTable = [u64; SUBSIDY_BY_MONTH_TABLE_SIZE];

#[derive(Clone)]
pub struct CoinbaseManager {
    coinbase_payload_script_public_key_max_len: u8,
    max_coinbase_payload_len: usize,
    deflationary_phase_daa_score: u64,
    pre_deflationary_phase_base_subsidy: u64,
    target_time_per_block: u64,

    /// Precomputed number of blocks per month
    blocks_per_month: u64,

    /// Precomputed subsidy by month table
    subsidy_by_month_table: SubsidyByMonthTable,

    hf_relaunch_daa_score: u64,
}

/// Struct used to streamline payload parsing
struct PayloadParser<'a> {
    remaining: &'a [u8], // The unparsed remainder
}

impl<'a> PayloadParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { remaining: data }
    }

    /// Returns a slice with the first `n` bytes of `remaining`, while setting `remaining` to the remaining part
    fn take(&mut self, n: usize) -> &[u8] {
        let (segment, remaining) = self.remaining.split_at(n);
        self.remaining = remaining;
        segment
    }
}

impl CoinbaseManager {
    pub fn new(
        coinbase_payload_script_public_key_max_len: u8,
        max_coinbase_payload_len: usize,
        deflationary_phase_daa_score: u64,
        pre_deflationary_phase_base_subsidy: u64,
        target_time_per_block: u64,
        hf_relaunch_daa_score: u64,
    ) -> Self {
        assert!(1000 % target_time_per_block == 0);
        let bps = 1000 / target_time_per_block;
        let blocks_per_month = SECONDS_PER_MONTH * bps;

        // Precomputed subsidy by month table for the actual block per second rate
        // Here values are rounded up so that we keep the same number of rewarding months as in the original 1 BPS table.
        // In a 10 BPS network, the induced increase in total rewards is 51 WALA (see tests::calc_high_bps_total_rewards_delta())
        let subsidy_by_month_table: SubsidyByMonthTable = core::array::from_fn(|i| (SUBSIDY_BY_MONTH_TABLE[i] + bps - 1) / bps);
        Self {
            coinbase_payload_script_public_key_max_len,
            max_coinbase_payload_len,
            deflationary_phase_daa_score,
            pre_deflationary_phase_base_subsidy,
            target_time_per_block,
            blocks_per_month,
            subsidy_by_month_table,
            hf_relaunch_daa_score,
        }
    }

    #[cfg(test)]
    #[inline]
    pub fn bps(&self) -> u64 {
        1000 / self.target_time_per_block
    }

    pub fn expected_coinbase_transaction<T: AsRef<[u8]>>(
        &self,
        daa_score: u64,
        miner_data: MinerData<T>,
        ghostdag_data: &GhostdagData,
        mergeset_rewards: &BlockHashMap<BlockRewardData>,
        mergeset_non_daa: &BlockHashSet,
    ) -> CoinbaseResult<CoinbaseTransactionTemplate> {
        let mut outputs = Vec::with_capacity(ghostdag_data.mergeset_blues.len() + 1); // + 1 for possible red reward

        // Add an output for each mergeset blue block (∩ DAA window), paying to the script reported by the block.
        // Note that combinatorically it is nearly impossible for a blue block to be non-DAA
        for blue in ghostdag_data.mergeset_blues.iter().filter(|h| !mergeset_non_daa.contains(h)) {
            let reward_data = mergeset_rewards.get(blue).unwrap();
            if reward_data.subsidy + reward_data.total_fees > 0 {
                outputs
                    .push(TransactionOutput::new(reward_data.subsidy + reward_data.total_fees, reward_data.script_public_key.clone()));
            }
        }

        // Collect all rewards from mergeset reds ∩ DAA window and create a
        // single output rewarding all to the current block (the "merging" block)
        let mut red_reward = 0u64;
        for red in ghostdag_data.mergeset_reds.iter().filter(|h| !mergeset_non_daa.contains(h)) {
            let reward_data = mergeset_rewards.get(red).unwrap();
            red_reward += reward_data.subsidy + reward_data.total_fees;
        }
        if red_reward > 0 {
            outputs.push(TransactionOutput::new(red_reward, miner_data.script_public_key.clone()));
        }

        // Build the current block's payload
        let subsidy = self.calc_block_subsidy(daa_score);
        let payload = self.serialize_coinbase_payload(&CoinbaseData { blue_score: ghostdag_data.blue_score, subsidy, miner_data })?;

        Ok(CoinbaseTransactionTemplate {
            tx: Transaction::new(constants::TX_VERSION, vec![], outputs, 0, subnets::SUBNETWORK_ID_COINBASE, 0, payload),
            has_red_reward: red_reward > 0,
        })
    }

    pub fn serialize_coinbase_payload<T: AsRef<[u8]>>(&self, data: &CoinbaseData<T>) -> CoinbaseResult<Vec<u8>> {
        let script_pub_key_len = data.miner_data.script_public_key.script().len();
        if script_pub_key_len > self.coinbase_payload_script_public_key_max_len as usize {
            return Err(CoinbaseError::PayloadScriptPublicKeyLenAboveMax(
                script_pub_key_len,
                self.coinbase_payload_script_public_key_max_len,
            ));
        }
        let payload: Vec<u8> = data.blue_score.to_le_bytes().iter().copied()                    // Blue score                   (u64)
            .chain(data.subsidy.to_le_bytes().iter().copied())                                  // Subsidy                      (u64)
            .chain(data.miner_data.script_public_key.version().to_le_bytes().iter().copied())   // Script public key version    (u16)
            .chain((script_pub_key_len as u8).to_le_bytes().iter().copied())                    // Script public key length     (u8)
            .chain(data.miner_data.script_public_key.script().iter().copied())                  // Script public key
            .chain(data.miner_data.extra_data.as_ref().iter().copied())                         // Extra data
            .collect();

        Ok(payload)
    }

    pub fn modify_coinbase_payload<T: AsRef<[u8]>>(&self, mut payload: Vec<u8>, miner_data: &MinerData<T>) -> CoinbaseResult<Vec<u8>> {
        let script_pub_key_len = miner_data.script_public_key.script().len();
        if script_pub_key_len > self.coinbase_payload_script_public_key_max_len as usize {
            return Err(CoinbaseError::PayloadScriptPublicKeyLenAboveMax(
                script_pub_key_len,
                self.coinbase_payload_script_public_key_max_len,
            ));
        }

        // Keep only blue score and subsidy. Note that truncate does not modify capacity, so
        // the usual case where the payloads are the same size will not trigger a reallocation
        payload.truncate(LENGTH_OF_BLUE_SCORE + LENGTH_OF_SUBSIDY);
        payload.extend(
            miner_data.script_public_key.version().to_le_bytes().iter().copied() // Script public key version (u16)
                .chain((script_pub_key_len as u8).to_le_bytes().iter().copied()) // Script public key length  (u8)
                .chain(miner_data.script_public_key.script().iter().copied())    // Script public key
                .chain(miner_data.extra_data.as_ref().iter().copied()), // Extra data
        );

        Ok(payload)
    }

    pub fn deserialize_coinbase_payload<'a>(&self, payload: &'a [u8]) -> CoinbaseResult<CoinbaseData<&'a [u8]>> {
        if payload.len() < MIN_PAYLOAD_LENGTH {
            return Err(CoinbaseError::PayloadLenBelowMin(payload.len(), MIN_PAYLOAD_LENGTH));
        }

        if payload.len() > self.max_coinbase_payload_len {
            return Err(CoinbaseError::PayloadLenAboveMax(payload.len(), self.max_coinbase_payload_len));
        }

        let mut parser = PayloadParser::new(payload);

        let blue_score = u64::from_le_bytes(parser.take(LENGTH_OF_BLUE_SCORE).try_into().unwrap());
        let subsidy = u64::from_le_bytes(parser.take(LENGTH_OF_SUBSIDY).try_into().unwrap());
        let script_pub_key_version = u16::from_le_bytes(parser.take(LENGTH_OF_SCRIPT_PUB_KEY_VERSION).try_into().unwrap());
        let script_pub_key_len = u8::from_le_bytes(parser.take(LENGTH_OF_SCRIPT_PUB_KEY_LENGTH).try_into().unwrap());

        if script_pub_key_len > self.coinbase_payload_script_public_key_max_len {
            return Err(CoinbaseError::PayloadScriptPublicKeyLenAboveMax(
                script_pub_key_len as usize,
                self.coinbase_payload_script_public_key_max_len,
            ));
        }

        if parser.remaining.len() < script_pub_key_len as usize {
            return Err(CoinbaseError::PayloadCantContainScriptPublicKey(
                payload.len(),
                MIN_PAYLOAD_LENGTH + script_pub_key_len as usize,
            ));
        }

        let script_public_key =
            ScriptPublicKey::new(script_pub_key_version, ScriptVec::from_slice(parser.take(script_pub_key_len as usize)));
        let extra_data = parser.remaining;

        Ok(CoinbaseData { blue_score, subsidy, miner_data: MinerData { script_public_key, extra_data } })
    }

    pub fn calc_block_subsidy(&self, daa_score: u64) -> u64 {
        if daa_score < self.deflationary_phase_daa_score {
            return self.pre_deflationary_phase_base_subsidy;
        }

        let months_since_deflationary_phase_started =
            ((daa_score - self.deflationary_phase_daa_score) / self.blocks_per_month) as usize;
        if months_since_deflationary_phase_started >= self.subsidy_by_month_table.len() {
            *(self.subsidy_by_month_table).last().unwrap()
        } else {
            self.subsidy_by_month_table[months_since_deflationary_phase_started]
        }
    }

    #[cfg(test)]
    pub fn legacy_calc_block_subsidy(&self, daa_score: u64) -> u64 {
        if daa_score < self.deflationary_phase_daa_score {
            return self.pre_deflationary_phase_base_subsidy;
        }

        // Note that this calculation implicitly assumes that block per second = 1 (by assuming daa score diff is in second units).
        let months_since_deflationary_phase_started = (daa_score - self.deflationary_phase_daa_score) / SECONDS_PER_MONTH;
        assert!(months_since_deflationary_phase_started <= usize::MAX as u64);
        let months_since_deflationary_phase_started: usize = months_since_deflationary_phase_started as usize;
        if months_since_deflationary_phase_started >= SUBSIDY_BY_MONTH_TABLE.len() {
            *SUBSIDY_BY_MONTH_TABLE.last().unwrap()
        } else {
            SUBSIDY_BY_MONTH_TABLE[months_since_deflationary_phase_started]
        }
    }
}

/*
    This table was pre-calculated by calling `calcDeflationaryPeriodBlockSubsidyFloatCalc` (in waglaylad-go) for all months until reaching 0 subsidy.
    To regenerate this table, run `TestBuildSubsidyTable` in coinbasemanager_test.go (note the `deflationaryPhaseBaseSubsidy` therein).
    These values apply to 1 block per second.
*/
#[rustfmt::skip]
const SUBSIDY_BY_MONTH_TABLE: [u64; 430] = [
    	50000000000, 50000000000, 50000000000, 47193715634, 44544935907, 42044820762, 39685026299, 37457676921, 35355339059, 33370996354, 31498026247, 29730177875, 28061551207, 26486577358, 25000000000, 23596857817, 22272467953, 21022410381, 19842513149, 18728838460, 17677669529, 16685498177, 15749013123, 14865088937, 14030775603, 13243288679, 12500000000, 
	11798428908, 11136233976, 10511205190, 9921256574, 9364419230, 8838834764, 8342749088, 7874506561, 7432544468, 7015387801, 6621644339, 6250000000, 5899214454, 5568116988, 5255602595, 4960628287, 4682209615, 4419417382, 4171374544, 3937253280, 3716272234, 3507693900, 3310822169, 3125000000, 2949607227, 
	2784058494, 2627801297, 2480314143, 2341104807, 2209708691, 2085687272, 1968626640, 1858136117, 1753846950, 1655411084, 1562500000, 1474803613, 1392029247, 1313900648, 1240157071, 1170552403, 1104854345, 1042843636, 984313320, 929068058, 876923475, 827705542, 781250000, 737401806, 696014623, 
	656950324, 620078535, 585276201, 552427172, 521421818, 492156660, 464534029, 438461737, 413852771, 390625000, 368700903, 348007311, 328475162, 310039267, 292638100, 276213586, 260710909, 246078330, 232267014, 219230868, 206926385, 195312500, 184350451, 174003655, 164237581, 
	155019633, 146319050, 138106793, 130355454, 123039165, 116133507, 109615434, 103463192, 97656250, 92175225, 87001827, 82118790, 77509816, 73159525, 69053396, 65177727, 61519582, 58066753, 54807717, 51731596, 48828125, 46087612, 43500913, 41059395, 38754908, 
	36579762, 34526698, 32588863, 30759791, 29033376, 27403858, 25865798, 24414062, 23043806, 21750456, 20529697, 19377454, 18289881, 17263349, 16294431, 15379895, 14516688, 13701929, 12932899, 12207031, 11521903, 10875228, 10264848, 9688727, 9144940, 
	8631674, 8147215, 7689947, 7258344, 6850964, 6466449, 6103515, 5760951, 5437614, 5132424, 4844363, 4572470, 4315837, 4073607, 3844973, 3629172, 3425482, 3233224, 3051757, 2880475, 2718807, 2566212, 2422181, 2286235, 2157918, 
	2036803, 1922486, 1814586, 1712741, 1616612, 1525878, 1440237, 1359403, 1283106, 1211090, 1143117, 1078959, 1018401, 961243, 907293, 856370, 808306, 762939, 720118, 679701, 641553, 605545, 571558, 539479, 509200, 
	480621, 453646, 428185, 404153, 381469, 360059, 339850, 320776, 302772, 285779, 269739, 254600, 240310, 226823, 214092, 202076, 190734, 180029, 169925, 160388, 151386, 142889, 134869, 127300, 120155, 
	113411, 107046, 101038, 95367, 90014, 84962, 80194, 75693, 71444, 67434, 63650, 60077, 56705, 53523, 50519, 47683, 45007, 42481, 40097, 37846, 35722, 33717, 31825, 30038, 28352, 
	26761, 25259, 23841, 22503, 21240, 20048, 18923, 17861, 16858, 15912, 15019, 14176, 13380, 12629, 11920, 11251, 10620, 10024, 9461, 8930, 8429, 7956, 7509, 7088, 6690, 
	6314, 5960, 5625, 5310, 5012, 4730, 4465, 4214, 3978, 3754, 3544, 3345, 3157, 2980, 2812, 2655, 2506, 2365, 2232, 2107, 1989, 1877, 1772, 1672, 1578, 
	1490, 1406, 1327, 1253, 1182, 1116, 1053, 994, 938, 886, 836, 789, 745, 703, 663, 626, 591, 558, 526, 497, 469, 443, 418, 394, 372, 
	351, 331, 313, 295, 279, 263, 248, 234, 221, 209, 197, 186, 175, 165, 156, 147, 139, 131, 124, 117, 110, 104, 98, 93, 87, 
	82, 78, 73, 69, 65, 62, 58, 55, 52, 49, 46, 43, 41, 39, 36, 34, 32, 31, 29, 27, 26, 24, 23, 21, 20, 
	19, 18, 17, 16, 15, 14, 13, 13, 12, 11, 10, 10, 9, 9, 8, 8, 7, 7, 6, 6, 6, 5, 5, 5, 4, 
	4, 4, 4, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 
	1, 1, 0,
];

#[cfg(test)]
mod tests {
    use waglayla_consensus_core::{
        config::params::{Params, TESTNET11_PARAMS},
        constants::SOMPI_PER_WAGLAYLA,
        network::NetworkId,
        tx::scriptvec,
    };

    use crate::params::MAINNET_PARAMS;

    use super::*;

    #[test]
    fn calc_high_bps_total_rewards_delta() {
        const SECONDS_PER_MONTH: u64 = 2629800;

        let legacy_cbm = create_legacy_manager();
        let pre_deflationary_rewards = legacy_cbm.pre_deflationary_phase_base_subsidy * legacy_cbm.deflationary_phase_daa_score;
        let total_rewards: u64 = pre_deflationary_rewards + SUBSIDY_BY_MONTH_TABLE.iter().map(|x| x * SECONDS_PER_MONTH).sum::<u64>();
        let testnet_11_bps = TESTNET11_PARAMS.bps();
        let total_high_bps_rewards_rounded_up: u64 = pre_deflationary_rewards
            + SUBSIDY_BY_MONTH_TABLE
            .iter()
            .map(|x| ((x + testnet_11_bps - 1) / testnet_11_bps * testnet_11_bps) * SECONDS_PER_MONTH)
            .sum::<u64>();

        let cbm = create_manager(&TESTNET11_PARAMS);
        let total_high_bps_rewards: u64 =
            pre_deflationary_rewards + cbm.subsidy_by_month_table.iter().map(|x| x * cbm.blocks_per_month).sum::<u64>();
        assert_eq!(total_high_bps_rewards_rounded_up, total_high_bps_rewards, "subsidy adjusted to bps must be rounded up");

        let delta = total_high_bps_rewards as i64 - total_rewards as i64;

        println!("Total rewards: {} sompi => {} WALA", total_rewards, total_rewards / SOMPI_PER_WAGLAYLA);
        println!("Total high bps rewards: {} sompi => {} WALA", total_high_bps_rewards, total_high_bps_rewards / SOMPI_PER_WAGLAYLA);
        println!("Delta: {} sompi => {} WALA", delta, delta / SOMPI_PER_WAGLAYLA as i64);
    }

    #[test]
    fn subsidy_by_month_table_test() {
        let cbm = create_legacy_manager();
        cbm.subsidy_by_month_table.iter().enumerate().for_each(|(i, x)| {
            assert_eq!(SUBSIDY_BY_MONTH_TABLE[i], *x, "for 1 BPS, const table and precomputed values must match");
        });

        for network_id in NetworkId::iter() {
            let cbm = create_manager(&network_id.into());
            cbm.subsidy_by_month_table.iter().enumerate().for_each(|(i, x)| {
                assert_eq!(
                    (SUBSIDY_BY_MONTH_TABLE[i] + cbm.bps() - 1) / cbm.bps(),
                    *x,
                    "{}: locally computed and precomputed values must match",
                    network_id
                );
            });
        }
    }

    #[ignore] // TODO:
    #[test]
    fn subsidy_test() {
        const PRE_DEFLATIONARY_PHASE_BASE_SUBSIDY: u64 = 50000000000;
        const DEFLATIONARY_PHASE_INITIAL_SUBSIDY: u64 = 50000000000;
        const SECONDS_PER_MONTH: u64 = 2629800;
        const SECONDS_PER_HALVING: u64 = SECONDS_PER_MONTH * 12;

        for network_id in NetworkId::iter() {
            let params = &network_id.into();
            let cbm = create_manager(params);

            let pre_deflationary_phase_base_subsidy = PRE_DEFLATIONARY_PHASE_BASE_SUBSIDY / params.bps();
            let deflationary_phase_initial_subsidy = DEFLATIONARY_PHASE_INITIAL_SUBSIDY / params.bps();
            let blocks_per_halving = SECONDS_PER_HALVING * params.bps();

            struct Test {
                name: &'static str,
                daa_score: u64,
                expected: u64,
            }

            let tests = vec![
                Test { name: "first mined block", daa_score: 1, expected: pre_deflationary_phase_base_subsidy },
                Test {
                    name: "before deflationary phase",
                    daa_score: params.deflationary_phase_daa_score - 1,
                    expected: pre_deflationary_phase_base_subsidy,
                },
                Test {
                    name: "start of deflationary phase",
                    daa_score: params.deflationary_phase_daa_score,
                    expected: deflationary_phase_initial_subsidy,
                },
                Test {
                    name: "after one halving",
                    daa_score: params.deflationary_phase_daa_score + blocks_per_halving,
                    expected: deflationary_phase_initial_subsidy / 2,
                },
                Test {
                    name: "after 2 halvings",
                    daa_score: params.deflationary_phase_daa_score + 2 * blocks_per_halving,
                    expected: deflationary_phase_initial_subsidy / 4,
                },
                Test {
                    name: "after 5 halvings",
                    daa_score: params.deflationary_phase_daa_score + 5 * blocks_per_halving,
                    expected: deflationary_phase_initial_subsidy / 32,
                },
                Test {
                    name: "after 32 halvings",
                    daa_score: params.deflationary_phase_daa_score + 32 * blocks_per_halving,
                    expected: ((DEFLATIONARY_PHASE_INITIAL_SUBSIDY / 2_u64.pow(32)) + cbm.bps() - 1) / cbm.bps(),
                },
                Test {
                    name: "just before subsidy depleted",
                    daa_score: params.deflationary_phase_daa_score + 35 * blocks_per_halving,
                    expected: 1,
                },
                Test {
                    name: "after subsidy depleted",
                    daa_score: params.deflationary_phase_daa_score + 36 * blocks_per_halving,
                    expected: 0,
                },
            ];

            for t in tests {
                assert_eq!(cbm.calc_block_subsidy(t.daa_score), t.expected, "{} test '{}' failed", network_id, t.name);
                if params.bps() == 1 {
                    assert_eq!(cbm.legacy_calc_block_subsidy(t.daa_score), t.expected, "{} test '{}' failed", network_id, t.name);
                }
            }
        }
    }

    #[test]
    fn payload_serialization_test() {
        let cbm = create_manager(&MAINNET_PARAMS);

        let script_data = [33u8, 255];
        let extra_data = [2u8, 3];
        let data = CoinbaseData {
            blue_score: 56,
            subsidy: 50000000000,
            miner_data: MinerData {
                script_public_key: ScriptPublicKey::new(0, ScriptVec::from_slice(&script_data)),
                extra_data: &extra_data as &[u8],
            },
        };

        let payload = cbm.serialize_coinbase_payload(&data).unwrap();
        let deserialized_data = cbm.deserialize_coinbase_payload(&payload).unwrap();

        assert_eq!(data, deserialized_data);

        // Test an actual mainnet payload
        let payload_hex =
            "b612c90100000000041a763e07000000000022202b32443ff740012157716d81216d09aebc39e5493c93a7181d92cb756c02c560ac302e31322e382f";
        let mut payload = vec![0u8; payload_hex.len() / 2];
        faster_hex::hex_decode(payload_hex.as_bytes(), &mut payload).unwrap();
        let deserialized_data = cbm.deserialize_coinbase_payload(&payload).unwrap();

        let expected_data = CoinbaseData {
            blue_score: 29954742,
            subsidy: 31112698372,
            miner_data: MinerData {
                script_public_key: ScriptPublicKey::new(
                    0,
                    scriptvec![
                        32, 43, 50, 68, 63, 247, 64, 1, 33, 87, 113, 109, 129, 33, 109, 9, 174, 188, 57, 229, 73, 60, 147, 167, 24,
                        29, 146, 203, 117, 108, 2, 197, 96, 172,
                    ],
                ),
                extra_data: &[48u8, 46, 49, 50, 46, 56, 47] as &[u8],
            },
        };
        assert_eq!(expected_data, deserialized_data);
    }

    #[test]
    fn modify_payload_test() {
        let cbm = create_manager(&MAINNET_PARAMS);

        let script_data = [33u8, 255];
        let extra_data = [2u8, 3, 23, 98];
        let data = CoinbaseData {
            blue_score: 56345,
            subsidy: 50000000000,
            miner_data: MinerData {
                script_public_key: ScriptPublicKey::new(0, ScriptVec::from_slice(&script_data)),
                extra_data: &extra_data,
            },
        };

        let data2 = CoinbaseData {
            blue_score: data.blue_score,
            subsidy: data.subsidy,
            miner_data: MinerData {
                // Modify only miner data
                script_public_key: ScriptPublicKey::new(0, ScriptVec::from_slice(&[33u8, 255, 33])),
                extra_data: &[2u8, 3, 23, 98, 34, 34] as &[u8],
            },
        };

        let mut payload = cbm.serialize_coinbase_payload(&data).unwrap();
        payload = cbm.modify_coinbase_payload(payload, &data2.miner_data).unwrap(); // Update the payload with the modified miner data
        let deserialized_data = cbm.deserialize_coinbase_payload(&payload).unwrap();

        assert_eq!(data2, deserialized_data);
    }

    fn create_manager(params: &Params) -> CoinbaseManager {
        CoinbaseManager::new(
            params.coinbase_payload_script_public_key_max_len,
            params.max_coinbase_payload_len,
            params.deflationary_phase_daa_score,
            params.pre_deflationary_phase_base_subsidy,
            params.target_time_per_block,
            params.hf_relaunch_daa_score,
        )
    }

    /// Return a CoinbaseManager with legacy golang 1 BPS properties
    fn create_legacy_manager() -> CoinbaseManager {
        CoinbaseManager::new(150, 204, 7889400, 50000000000, 1000, 0)
    }
}
