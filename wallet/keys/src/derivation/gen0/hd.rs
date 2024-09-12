use crate::derivation::traits::*;
use crate::imports::*;
use hmac::Mac;
use waglayla_addresses::{Address, Prefix as AddressPrefix, Version as AddressVersion};
use waglayla_bip32::types::{ChainCode, HmacSha512, KeyFingerprint, PublicKeyBytes, KEY_SIZE};
use waglayla_bip32::{
    AddressType, ChildNumber, DerivationPath, ExtendedKey, ExtendedKeyAttrs, ExtendedPrivateKey, ExtendedPublicKey, Prefix,
    PrivateKey, PublicKey, SecretKey, SecretKeyExt,
};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use std::fmt::Debug;

fn get_fingerprint<K>(private_key: &K) -> KeyFingerprint
where
    K: PrivateKey,
{
    let public_key_bytes = private_key.public_key().to_bytes();

    let digest = Ripemd160::digest(Sha256::digest(public_key_bytes));
    digest[..4].try_into().expect("digest truncated")
}

struct Inner {
    /// Derived public key
    public_key: secp256k1::PublicKey,
    /// Extended key attributes.
    attrs: ExtendedKeyAttrs,
    #[allow(dead_code)]
    fingerprint: KeyFingerprint,
    hmac: HmacSha512,
}

impl Inner {
    fn new(public_key: secp256k1::PublicKey, attrs: ExtendedKeyAttrs, hmac: HmacSha512) -> Self {
        Self { public_key, fingerprint: public_key.fingerprint(), hmac, attrs }
    }
}

#[derive(Clone)]
pub struct PubkeyDerivationManagerV0 {
    inner: Arc<Mutex<Option<Inner>>>,
    index: Arc<Mutex<u32>>,
    cache: Arc<Mutex<HashMap<u32, secp256k1::PublicKey>>>,
    use_cache: Arc<AtomicBool>,
}

impl PubkeyDerivationManagerV0 {
    pub fn new(
        public_key: secp256k1::PublicKey,
        attrs: ExtendedKeyAttrs,
        fingerprint: KeyFingerprint,
        hmac: HmacSha512,
        index: u32,
        use_cache: bool,
    ) -> Result<Self> {
        let wallet = Self {
            index: Arc::new(Mutex::new(index)),
            inner: Arc::new(Mutex::new(Some(Inner { public_key, attrs, fingerprint, hmac }))),
            cache: Arc::new(Mutex::new(HashMap::new())),
            use_cache: Arc::new(AtomicBool::new(use_cache)),
        };

        Ok(wallet)
    }

    fn set_key(&self, public_key: secp256k1::PublicKey, attrs: ExtendedKeyAttrs, hmac: HmacSha512, index: Option<u32>) {
        *self.cache.lock().unwrap() = HashMap::new();
        let new_inner = Inner::new(public_key, attrs, hmac);
        {
            *self.index.lock().unwrap() = index.unwrap_or(0);
        }
        let mut locked = self.opt_inner();
        if let Some(inner) = locked.as_mut() {
            inner.public_key = new_inner.public_key;
            inner.fingerprint = new_inner.fingerprint;
            inner.hmac = new_inner.hmac;
            inner.attrs = new_inner.attrs;
        } else {
            *locked = Some(new_inner)
        }
    }

    fn remove_key(&self) {
        *self.opt_inner() = None;
    }

    fn opt_inner(&self) -> MutexGuard<Option<Inner>> {
        self.inner.lock().unwrap()
    }

    fn public_key_(&self) -> Result<secp256k1::PublicKey> {
        let locked = self.opt_inner();
        let inner = locked
            .as_ref()
            .ok_or(crate::error::Error::Custom("PubkeyDerivationManagerV0 initialization is pending (Error: 101).".into()))?;
        Ok(inner.public_key)
    }
    fn index_(&self) -> Result<u32> {
        // let locked = self.opt_inner();
        // let inner =
        //     locked.as_ref().ok_or(crate::error::Error::Custom("PubkeyDerivationManagerV0 initialization is pending.".into()))?;
        // Ok(inner.index)
        Ok(*self.index.lock().unwrap())
    }

    fn use_cache(&self) -> bool {
        self.use_cache.load(Ordering::SeqCst)
    }

    pub fn cache(&self) -> Result<HashMap<u32, secp256k1::PublicKey>> {
        Ok(self.cache.lock()?.clone())
    }

    // pub fn derive_pubkey_range(&self, indexes: std::ops::Range<u32>) -> Result<Vec<secp256k1::PublicKey>> {
    //     let list = indexes.map(|index| self.derive_pubkey(index)).collect::<Vec<_>>();
    //     let keys = list.into_iter().collect::<Result<Vec<_>>>()?;
    //     // let keys = join_all(list).await.into_iter().collect::<Result<Vec<_>>>()?;
    //     Ok(keys)
    // }

    pub fn derive_pubkey_range(&self, indexes: std::ops::Range<u32>) -> Result<Vec<secp256k1::PublicKey>> {
        let use_cache = self.use_cache();
        let mut cache = self.cache.lock()?;
        let locked = self.opt_inner();
        let list: Vec<Result<secp256k1::PublicKey, crate::error::Error>> = if let Some(inner) = locked.as_ref() {
            indexes
                .map(|index| {
                    let (key, _chain_code) = WalletDerivationManagerV0::derive_public_key_child(
                        &inner.public_key,
                        ChildNumber::new(index, true)?,
                        inner.hmac.clone(),
                    )?;
                    //workflow_log::log_info!("use_cache: {use_cache}");
                    if use_cache {
                        //workflow_log::log_info!("cache insert: {:?}", key);
                        cache.insert(index, key);
                    }
                    Ok(key)
                })
                .collect::<Vec<_>>()
        } else {
            indexes
                .map(|index| {
                    if let Some(key) = cache.get(&index) {
                        Ok(*key)
                    } else {
                        Err(crate::error::Error::Custom("PubkeyDerivationManagerV0 initialization is pending  (Error: 102).".into()))
                    }
                })
                .collect::<Vec<_>>()
        };

        //let list = indexes.map(|index| self.derive_pubkey(index)).collect::<Vec<_>>();
        let keys = list.into_iter().collect::<Result<Vec<_>>>()?;
        // let keys = join_all(list).await.into_iter().collect::<Result<Vec<_>>>()?;
        Ok(keys)
    }

    pub fn derive_pubkey(&self, index: u32) -> Result<secp256k1::PublicKey> {
        //let use_cache = self.use_cache();
        let locked = self.opt_inner();
        if let Some(inner) = locked.as_ref() {
            let (key, _chain_code) = WalletDerivationManagerV0::derive_public_key_child(
                &inner.public_key,
                ChildNumber::new(index, true)?,
                inner.hmac.clone(),
            )?;
            //workflow_log::log_info!("use_cache: {use_cache}");
            if self.use_cache() {
                //workflow_log::log_info!("cache insert: {:?}", key);
                self.cache.lock()?.insert(index, key);
            }
            return Ok(key);
        } else if let Some(key) = self.cache.lock()?.get(&index) {
            return Ok(*key);
        }

        Err(crate::error::Error::Custom("PubkeyDerivationManagerV0 initialization is pending  (Error: 102).".into()))
    }

    pub fn create_address(key: &secp256k1::PublicKey, prefix: AddressPrefix, _ecdsa: bool) -> Result<Address> {
        let payload = &key.to_bytes()[1..];
        let address = Address::new(prefix, AddressVersion::PubKey, payload);

        Ok(address)
    }

    pub fn public_key(&self) -> ExtendedPublicKey<secp256k1::PublicKey> {
        self.into()
    }

    pub fn attrs(&self) -> ExtendedKeyAttrs {
        let locked = self.opt_inner();
        let inner = locked.as_ref().expect("PubkeyDerivationManagerV0 initialization is pending (Error: 103).");
        inner.attrs.clone()
    }

    /// Serialize the raw public key as a byte array.
    pub fn to_bytes(&self) -> PublicKeyBytes {
        self.public_key().to_bytes()
    }

    /// Serialize this key as an [`ExtendedKey`].
    pub fn to_extended_key(&self, prefix: Prefix) -> ExtendedKey {
        let mut key_bytes = [0u8; KEY_SIZE + 1];
        key_bytes[..].copy_from_slice(&self.to_bytes());
        ExtendedKey { prefix, attrs: self.attrs().clone(), key_bytes }
    }

    pub fn to_string(&self) -> Zeroizing<String> {
        Zeroizing::new(self.to_extended_key(Prefix::XPUB).to_string())
    }
}

// #[wasm_bindgen]
impl PubkeyDerivationManagerV0 {
    // #[wasm_bindgen(getter, js_name = publicKey)]
    pub fn get_public_key(&self) -> String {
        self.public_key().to_string(None)
    }
}

impl From<&PubkeyDerivationManagerV0> for ExtendedPublicKey<secp256k1::PublicKey> {
    fn from(inner: &PubkeyDerivationManagerV0) -> ExtendedPublicKey<secp256k1::PublicKey> {
        ExtendedPublicKey { public_key: inner.public_key_().unwrap(), attrs: inner.attrs().clone() }
    }
}

#[async_trait]
impl PubkeyDerivationManagerTrait for PubkeyDerivationManagerV0 {
    fn new_pubkey(&self) -> Result<secp256k1::PublicKey> {
        self.set_index(self.index()? + 1)?;
        self.current_pubkey()
    }

    fn index(&self) -> Result<u32> {
        self.index_()
    }

    fn set_index(&self, index: u32) -> Result<()> {
        *self.index.lock().unwrap() = index;
        Ok(())
    }

    fn current_pubkey(&self) -> Result<secp256k1::PublicKey> {
        let index = self.index()?;
        //workflow_log::log_info!("current_pubkey");
        let key = self.derive_pubkey(index)?;

        Ok(key)
    }

    fn get_range(&self, range: std::ops::Range<u32>) -> Result<Vec<secp256k1::PublicKey>> {
        //workflow_log::log_info!("gen0: get_range {:?}", range);
        self.derive_pubkey_range(range)
    }

    fn get_cache(&self) -> Result<HashMap<u32, secp256k1::PublicKey>> {
        self.cache()
    }

    fn uninitialize(&self) -> Result<()> {
        self.remove_key();
        Ok(())
    }
}

#[derive(Clone)]
pub struct WalletDerivationManagerV0 {
    /// extended public key derived upto `m/<Purpose>'/972/<Account Index>'`
    extended_public_key: Option<ExtendedPublicKey<secp256k1::PublicKey>>,

    account_index: u64,
    /// receive address wallet
    receive_pubkey_manager: Arc<PubkeyDerivationManagerV0>,

    /// change address wallet
    change_pubkey_manager: Arc<PubkeyDerivationManagerV0>,
}

impl WalletDerivationManagerV0 {
    pub fn create_extended_key_from_xprv(xprv: &str, is_multisig: bool, account_index: u64) -> Result<(SecretKey, ExtendedKeyAttrs)> {
        let xprv_key = ExtendedPrivateKey::<SecretKey>::from_str(xprv)?;
        Self::derive_extended_key_from_master_key(xprv_key, is_multisig, account_index)
    }

    pub fn derive_extended_key_from_master_key(
        xprv_key: ExtendedPrivateKey<SecretKey>,
        _is_multisig: bool,
        account_index: u64,
    ) -> Result<(SecretKey, ExtendedKeyAttrs)> {
        let attrs = xprv_key.attrs();

        let (extended_private_key, attrs) = Self::create_extended_key(*xprv_key.private_key(), attrs.clone(), account_index)?;

        Ok((extended_private_key, attrs))
    }

    fn create_extended_key(
        mut private_key: SecretKey,
        mut attrs: ExtendedKeyAttrs,
        account_index: u64,
    ) -> Result<(SecretKey, ExtendedKeyAttrs)> {
        // if is_multisig && cosigner_index.is_none() {
        //     return Err("cosigner_index is required for multisig path derivation".to_string().into());
        // }
        let purpose = 44; //if is_multisig { 45 } else { 44 };
        let path = format!("{purpose}'/972/{account_index}'");
        // if let Some(cosigner_index) = cosigner_index {
        //     path = format!("{path}/{}", cosigner_index)
        // }
        // if let Some(address_type) = address_type {
        //     path = format!("{path}/{}", address_type.index());
        // }
        //println!("path: {path}");
        let children = path.split('/');
        for child in children {
            (private_key, attrs) = Self::derive_private_key(&private_key, &attrs, child.parse::<ChildNumber>()?)?;
            //println!("ccc: {child}, public_key : {:?}, attrs: {:?}", private_key.get_public_key(), attrs);
        }

        Ok((private_key, attrs))
    }

    pub fn build_derivate_path(account_index: u64, address_type: Option<AddressType>) -> Result<DerivationPath> {
        let purpose = 44;
        let mut path = format!("m/{purpose}'/972/{account_index}'");
        if let Some(address_type) = address_type {
            path = format!("{path}/{}'", address_type.index());
        }
        let path = path.parse::<DerivationPath>()?;
        Ok(path)
    }

    pub fn receive_pubkey_manager(&self) -> &PubkeyDerivationManagerV0 {
        &self.receive_pubkey_manager
    }
    pub fn change_pubkey_manager(&self) -> &PubkeyDerivationManagerV0 {
        &self.change_pubkey_manager
    }

    pub fn create_pubkey_manager(
        private_key: &secp256k1::SecretKey,
        address_type: AddressType,
        attrs: &ExtendedKeyAttrs,
    ) -> Result<PubkeyDerivationManagerV0> {
        let (private_key, attrs, hmac) = Self::create_pubkey_manager_data(private_key, address_type, attrs)?;
        PubkeyDerivationManagerV0::new(
            private_key.get_public_key(),
            attrs.clone(),
            private_key.get_public_key().fingerprint(),
            hmac,
            0,
            true,
        )
    }

    pub fn create_pubkey_manager_data(
        private_key: &secp256k1::SecretKey,
        address_type: AddressType,
        attrs: &ExtendedKeyAttrs,
    ) -> Result<(secp256k1::SecretKey, ExtendedKeyAttrs, HmacSha512)> {
        let (private_key, attrs) = Self::derive_private_key(private_key, attrs, ChildNumber::new(address_type.index(), true)?)?;
        let hmac = Self::create_hmac(&private_key, &attrs, true)?;

        Ok((private_key, attrs, hmac))
    }

    pub fn derive_public_key(
        public_key: &secp256k1::PublicKey,
        attrs: &ExtendedKeyAttrs,
        child_number: ChildNumber,
    ) -> Result<(secp256k1::PublicKey, ExtendedKeyAttrs)> {
        //let fingerprint = public_key.fingerprint();
        let digest = Ripemd160::digest(Sha256::digest(&public_key.to_bytes()[1..]));
        let fingerprint = digest[..4].try_into().expect("digest truncated");

        let mut hmac = HmacSha512::new_from_slice(&attrs.chain_code).map_err(waglayla_bip32::Error::Hmac)?;
        hmac.update(&public_key.to_bytes());

        let (key, chain_code) = Self::derive_public_key_child(public_key, child_number, hmac)?;

        let depth = attrs.depth.checked_add(1).ok_or(waglayla_bip32::Error::Depth)?;

        let attrs = ExtendedKeyAttrs { parent_fingerprint: fingerprint, child_number, chain_code, depth };

        Ok((key, attrs))
    }

    fn derive_public_key_child(
        key: &secp256k1::PublicKey,
        child_number: ChildNumber,
        mut hmac: HmacSha512,
    ) -> Result<(secp256k1::PublicKey, ChainCode)> {
        hmac.update(&child_number.to_bytes());

        let result = hmac.finalize().into_bytes();
        let (child_key, chain_code) = result.split_at(KEY_SIZE);

        // We should technically loop here if a `secret_key` is zero or overflows
        // the order of the underlying elliptic curve group, incrementing the
        // index, however per "Child key derivation (CKD) functions":
        // https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#child-key-derivation-ckd-functions
        //
        // > "Note: this has probability lower than 1 in 2^127."
        //
        // ...so instead, we simply return an error if this were ever to happen,
        // as the chances of it happening are vanishingly small.
        let key = key.derive_child(child_key.try_into()?)?;

        Ok((key, chain_code.try_into()?))
    }

    pub fn derive_key_by_path(
        xkey: &ExtendedPrivateKey<secp256k1::SecretKey>,
        path: DerivationPath,
    ) -> Result<(SecretKey, ExtendedKeyAttrs)> {
        let mut private_key = *xkey.private_key();
        let mut attrs = xkey.attrs().clone();
        for child in path {
            (private_key, attrs) = Self::derive_private_key(&private_key, &attrs, child)?;
        }

        Ok((private_key, attrs))
    }

    pub fn derive_private_key(
        private_key: &SecretKey,
        attrs: &ExtendedKeyAttrs,
        child_number: ChildNumber,
    ) -> Result<(SecretKey, ExtendedKeyAttrs)> {
        let fingerprint = get_fingerprint(private_key);

        let hmac = Self::create_hmac(private_key, attrs, child_number.is_hardened())?;

        let (private_key, chain_code) = Self::derive_key(private_key, child_number, hmac)?;

        let depth = attrs.depth.checked_add(1).ok_or(waglayla_bip32::Error::Depth)?;

        let attrs = ExtendedKeyAttrs { parent_fingerprint: fingerprint, child_number, chain_code, depth };

        Ok((private_key, attrs))
    }

    fn derive_key(private_key: &SecretKey, child_number: ChildNumber, mut hmac: HmacSha512) -> Result<(SecretKey, ChainCode)> {
        hmac.update(&child_number.to_bytes());

        let result = hmac.finalize().into_bytes();
        let (child_key, chain_code) = result.split_at(KEY_SIZE);

        // We should technically loop here if a `secret_key` is zero or overflows
        // the order of the underlying elliptic curve group, incrementing the
        // index, however per "Child key derivation (CKD) functions":
        // https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#child-key-derivation-ckd-functions
        //
        // > "Note: this has probability lower than 1 in 2^127."
        //
        // ...so instead, we simply return an error if this were ever to happen,
        // as the chances of it happening are vanishingly small.
        let private_key = private_key.derive_child(child_key.try_into()?)?;

        Ok((private_key, chain_code.try_into()?))
    }

    pub fn create_hmac<K>(private_key: &K, attrs: &ExtendedKeyAttrs, hardened: bool) -> Result<HmacSha512>
    where
        K: PrivateKey<PublicKey = secp256k1::PublicKey>,
    {
        let mut hmac = HmacSha512::new_from_slice(&attrs.chain_code).map_err(waglayla_bip32::Error::Hmac)?;
        if hardened {
            hmac.update(&[0]);
            hmac.update(&private_key.to_bytes());
        } else {
            hmac.update(&private_key.public_key().to_bytes()[1..]);
        }

        Ok(hmac)
    }

    fn extended_public_key(&self) -> ExtendedPublicKey<secp256k1::PublicKey> {
        self.extended_public_key.clone().expect("WalletDerivationManagerV0 initialization is pending (Error: 104)")
    }

    /// Serialize the raw public key as a byte array.
    pub fn to_bytes(&self) -> PublicKeyBytes {
        self.extended_public_key().to_bytes()
    }

    pub fn attrs(&self) -> ExtendedKeyAttrs {
        self.extended_public_key().attrs().clone()
    }

    /// Serialize this key as a self-[`Zeroizing`] `String`.
    pub fn to_string(&self) -> Zeroizing<String> {
        let key = self.extended_public_key().to_string(Some(Prefix::KPUB));
        Zeroizing::new(key)
    }

    fn from_extended_private_key(private_key: secp256k1::SecretKey, account_index: u64, attrs: ExtendedKeyAttrs) -> Result<Self> {
        let receive_wallet = Self::create_pubkey_manager(&private_key, AddressType::Receive, &attrs)?;
        let change_wallet = Self::create_pubkey_manager(&private_key, AddressType::Change, &attrs)?;

        let extended_public_key = ExtendedPublicKey { public_key: private_key.get_public_key(), attrs };
        let wallet: WalletDerivationManagerV0 = Self {
            extended_public_key: Some(extended_public_key),
            account_index,
            receive_pubkey_manager: Arc::new(receive_wallet),
            change_pubkey_manager: Arc::new(change_wallet),
        };

        Ok(wallet)
    }

    pub fn create_uninitialized(
        account_index: u64,
        receive_keys: Option<HashMap<u32, secp256k1::PublicKey>>,
        change_keys: Option<HashMap<u32, secp256k1::PublicKey>>,
    ) -> Result<Self> {
        let receive_wallet = PubkeyDerivationManagerV0 {
            index: Arc::new(Mutex::new(0)),
            use_cache: Arc::new(AtomicBool::new(true)),
            cache: Arc::new(Mutex::new(receive_keys.unwrap_or_default())),
            inner: Arc::new(Mutex::new(None)),
        };
        let change_wallet = PubkeyDerivationManagerV0 {
            index: Arc::new(Mutex::new(0)),
            use_cache: Arc::new(AtomicBool::new(true)),
            cache: Arc::new(Mutex::new(change_keys.unwrap_or_default())),
            inner: Arc::new(Mutex::new(None)),
        };
        let wallet = Self {
            extended_public_key: None,
            account_index,
            receive_pubkey_manager: Arc::new(receive_wallet),
            change_pubkey_manager: Arc::new(change_wallet),
        };

        Ok(wallet)
    }

    // set master key "xprvxxxxxx"
    pub fn set_key(&self, key: String, index: Option<u32>) -> Result<()> {
        let (private_key, attrs) = Self::create_extended_key_from_xprv(&key, false, self.account_index)?;

        let (private_key_, attrs_, hmac_) = Self::create_pubkey_manager_data(&private_key, AddressType::Receive, &attrs)?;
        self.receive_pubkey_manager.set_key(private_key_.get_public_key(), attrs_, hmac_, index);

        let (private_key_, attrs_, hmac_) = Self::create_pubkey_manager_data(&private_key, AddressType::Change, &attrs)?;
        self.change_pubkey_manager.set_key(private_key_.get_public_key(), attrs_, hmac_, index);

        Ok(())
    }

    pub fn remove_key(&self) -> Result<()> {
        self.receive_pubkey_manager.remove_key();
        self.change_pubkey_manager.remove_key();
        Ok(())
    }
}

impl Debug for WalletDerivationManagerV0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletAccount")
            .field("depth", &self.attrs().depth)
            .field("child_number", &self.attrs().child_number)
            .field("chain_code", &faster_hex::hex_string(&self.attrs().chain_code))
            .field("public_key", &faster_hex::hex_string(&self.to_bytes()))
            .field("parent_fingerprint", &self.attrs().parent_fingerprint)
            .finish()
    }
}

#[async_trait]
impl WalletDerivationManagerTrait for WalletDerivationManagerV0 {
    /// build wallet from root/master private key
    fn from_master_xprv(xprv: &str, _is_multisig: bool, account_index: u64, _cosigner_index: Option<u32>) -> Result<Self> {
        let xprv_key = ExtendedPrivateKey::<SecretKey>::from_str(xprv)?;
        let attrs = xprv_key.attrs();

        let (extended_private_key, attrs) = Self::create_extended_key(*xprv_key.private_key(), attrs.clone(), account_index)?;

        let wallet = Self::from_extended_private_key(extended_private_key, account_index, attrs)?;

        Ok(wallet)
    }

    fn from_extended_public_key_str(_xpub: &str, _cosigner_index: Option<u32>) -> Result<Self> {
        unreachable!();
    }

    fn from_extended_public_key(
        _extended_public_key: ExtendedPublicKey<secp256k1::PublicKey>,
        _cosigner_index: Option<u32>,
    ) -> Result<Self> {
        unreachable!();
    }

    fn receive_pubkey_manager(&self) -> Arc<dyn PubkeyDerivationManagerTrait> {
        self.receive_pubkey_manager.clone()
    }

    fn change_pubkey_manager(&self) -> Arc<dyn PubkeyDerivationManagerTrait> {
        self.change_pubkey_manager.clone()
    }

    #[inline(always)]
    fn new_receive_pubkey(&self) -> Result<secp256k1::PublicKey> {
        self.receive_pubkey_manager.new_pubkey()
    }

    #[inline(always)]
    fn new_change_pubkey(&self) -> Result<secp256k1::PublicKey> {
        self.change_pubkey_manager.new_pubkey()
    }

    #[inline(always)]
    fn receive_pubkey(&self) -> Result<secp256k1::PublicKey> {
        self.receive_pubkey_manager.current_pubkey()
    }

    #[inline(always)]
    fn change_pubkey(&self) -> Result<secp256k1::PublicKey> {
        self.change_pubkey_manager.current_pubkey()
    }

    #[inline(always)]
    fn derive_receive_pubkey(&self, index: u32) -> Result<secp256k1::PublicKey> {
        self.receive_pubkey_manager.derive_pubkey(index)
    }

    #[inline(always)]
    fn derive_change_pubkey(&self, index: u32) -> Result<secp256k1::PublicKey> {
        self.change_pubkey_manager.derive_pubkey(index)
    }

    fn initialize(&self, key: String, index: Option<u32>) -> Result<()> {
        self.set_key(key, index)?;
        Ok(())
    }
    fn uninitialize(&self) -> Result<()> {
        self.remove_key()?;
        Ok(())
    }
}

// #[cfg(test)]
// use super::hd_;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    //use super::hd_;
    use super::{PubkeyDerivationManagerV0, WalletDerivationManagerTrait, WalletDerivationManagerV0};
    use waglayla_addresses::Prefix;

    fn gen0_receive_addresses() -> Vec<&'static str> {
        vec![
            "waglayla:qqnklfz9safc78p30y5c9q6p2rvxhj35uhnh96uunklak0tjn2x5w5jqzqtwp",
            "waglayla:qrd9efkvg3pg34sgp6ztwyv3r569qlc43wa5w8nfs302532dzj47knu04aftm",
            "waglayla:qq9k5qju48zv4wuw6kjxdktyhm602enshpjzhp0lssdm73n7tl7l2fgc4utt4",
            "waglayla:qprpml6ytf4g85tgfhz63vks3hxq5mmc3ezxg5kc2aq3f7pmzedxx6a4h8j0f",
            "waglayla:qq7dzqep3elaf0hrqjg4t265px8k2eh2u4lmt78w4ph022gze2ahu64cg5tqa",
            "waglayla:qrx0uzsnagrzw259amacvae8lrlx2kl2h4dy8lg9p4dze2e5zkn0w8facwwnh",
            "waglayla:qr86w2yky258lrqxfc3w55hua6vsf6rshs3jq20ka00pvze34umek35m9ealc",
            "waglayla:qq6gaad4ul2akwg3dz4jlqvmy3vjtkvdmfsfx6gxs76xafh2drwyv5dm54xaz",
            "waglayla:qq9x43w57fg3l6jpyl9ytqf5k2czxqmtttecwfw6nu657hcsuf8zjfveld7yj",
            "waglayla:qr9pzwfce8va3c23m2lwc3up7xl2ngpqjwscs5wwu02nc0wlwgamjuma2j7qs",
            "waglayla:qr3spcpku68mk9mjcq5qfk4at47aawxl2gz4kzndvu5jn4vzz79djffeqhjnl",
            "waglayla:qp4v6d6lyn8k025fkal869sh6w7csw85gj930u9r5ml7anncqz6s7u7fy7fpf",
            "waglayla:qzuas3nekcyl3uv6p8y5jrstchfweue0tpryttn6v0k4vc305rreje5tvf0t6",
            "waglayla:qpy00e8t4zd5ju8069zwsml2m7z3t607s87k0c66ud338ge682qwqlv7xj64c",
            "waglayla:qrs04ra3yl33ejhx6dneqhm29ztdgmwrxw7ugatmecqqm9x5xvmrx499qn2mh",
            "waglayla:qq5qertse2y6p7vpjcef59ezuvhtdu028ucvvsn90htxvxycavreg506hx2r4",
            "waglayla:qrv30p7gatspj5x4u6drdux2ns5k08qxa3jmvh64ffxcqnxz925gsl203p008",
            "waglayla:qqfupvd2mm6rwswkxs0zp9lzttn690grhjx922wtpt7gfnsjdhk0zhhp07mhr",
            "waglayla:qq2un0yhn4npc0rt2yjkp4aepz4j2rkryp59xlp6cvh0l5rqsndewr9zth6h8",
            "waglayla:qzams4ymck03wfqj4xzvj39ufxl080h4jp32wa8hna2hua9kj6t6cldumhm2p",
            "waglayla:qrzngzau800s9esxr5kq5kytp5l2enttf8xaag2pfz8s0e4k535767arhku9w",
            "waglayla:qpkpfagtqaxgzp8ngd3mwqf5n3pnqprp0dljukq0srlv4h0z09ckx9wfl9qsg",
            "waglayla:qqgxfpgzthxq4t2grv7jcshc0r9szsqttffh5cq7500lnxmpagvr6duvmfhqe",
            "waglayla:qq7m66z6dgdvqzmtg4zllh46l978cpud33zx7kcgcnf359glz0ucjw0tdzg6t",
            "waglayla:qrf6gzyhlfmmmd7yt7h45rrt37cpuzuyztyudwg3gfl3lpqenvk9jh4eptada",
            "waglayla:qznrj6r0yw3e3fjmy2ffa3wmkzcjaftljc9j360dwum8hpay3jdgjkxwe5g0w",
            "waglayla:qrh7p9x2kh0ps9crvgrths55rannuawc2lppzdn28na0yu9dmw5nkzhfnz4ps",
            "waglayla:qqd7g3skxjp7desmz99wy762uk59q8hqxxgm6tcgm0kw49x9d0l82la2evv05",
            "waglayla:qzxamdddkg429xexzd39dzlvpnwpvt0202a3hhvstdct49gv9yzx57xxazlfn",
            "waglayla:qzc8w4t4jxpwntqnm6fyl80c2e74mrunzk4l0yuuq6cmm355mq2e2350k56kw",
            "waglayla:qpeumknudvt6vpvkv9rahptrxu3wdjte62cz4nh33qc65gjvc6xuznhvuhhwz",
            "waglayla:qp7cdnnlcfa8r0fy7yduuhsexyagqpp9cqd8efj9v07r43fpnmg6qmuxd4d9g",
            "waglayla:qp7wxlf0hec690n6at259qww600sqakft8dnn2ujr6a7sk35snh5u2jjc0lnu",
            "waglayla:qzpczl9smaz7axyqmnkvd0694z7jpfrcgl9lka0h0t8fqy8efzqhv7nv86dad",
            "waglayla:qpfxwpv26rr7zydqdpmxtevch0qpgaldypjrctnhcrt5lccy6d8dupdqye970",
            "waglayla:qzj4vc7yw663v3akdldfcp6r2g69pej5kdc0jusp349yq57yk7xrk69ygc04h",
            "waglayla:qq2dha0feemswy9twtk9fys7tmd9gus8nnl38kqt86qdvq34rvymj27t6za8u",
            "waglayla:qpsx92u08vse22yhm4w0s56jf8drxa9al208a7dycl88ppc22eyjuxn7qzmkh",
            "waglayla:qptr20fsz9lpklpzyynnttjwf848cw2s8mqyzddkmyr4q4yclhm2z2az8l2tn",
            "waglayla:qzecr9vqwxas7d3rlt9s6dt5ku9xacwqvlsxjdkn3n3sa7q7kttqwuk380c9j",
            "waglayla:qq004kxhnwh39z3du9hu5yednllspuu33x3gz5zlvj6t8kac9urnv7alw6ava",
            "waglayla:qq3e77faqa2auktf3jq7lj4vnaf70p856vlxg28dat2wcz22tjttkxqvkrhky",
            "waglayla:qr83hneey4c9846xxn2uvszx42jyx20fpnyrpamy8cy8dhdpljq4xf85l3679",
            "waglayla:qz7wphuhuvx9ac2mp5td50dq25mzpkca9r2d35n2ek929e6qa72rq6hvtr95l",
            "waglayla:qrgsrdp3ag630cpjfzrvfa9gd4dafnrpmf2qwk4cy5mum7tk0ph4cd84kpxk8",
            "waglayla:qr4dhfm6cpp50q0lsg2drzv0nj5n4r57adfpxkwss3hf53wau2stumm2g89y0",
            "waglayla:qzrc652du8tapgrv7rfkmykqzeep8jrgsjeynypldq9mfn5phcyxk3xl8cfpn",
            "waglayla:qzauugr73lu4rjryhqmczk699775yshltpdxsxxd0str7jkttyxgwjr654e2c",
            "waglayla:qq2a7m6pjgm85erx3nhhex9uqgsjtkup09t2tukappztyz4f9ykas32uqc305",
            "waglayla:qrnjfugy6c9eg5g60fhfnh36069pzpz7z0t9nuzrg5whd6e6ut2ns98l3y5ra",
            "waglayla:qrhnvydk5dt2q9vk2f37vf848zztq4ex06rvwq5x3tymle73q08wzkqfwtatc",
            "waglayla:qrchv5j6sqmwpk9fumd2jz6na26ulxgcy7uwjlg95nur6mukhdcmvvxug5nkr",
            "waglayla:qq26pgvl5f4x3rdrf5jw9zn2e02n8xae4yvp7m4mfqf0n0mldjthjshc5g9my",
            "waglayla:qrmdeltxu3gzjgfpehucyufsm08fm924akwm3x05uzp8m45tr0raskkdlgelm",
            "waglayla:qrvzeg6qqqx6lvv0d3kt22ghj8lr2jvfpaypp8hgyyn75a9qmjqvypxrnwkq8",
            "waglayla:qqx5krm2a3ulccu8g0wn42lvernz6h42s7rk9yxd3t7xt062jvslwaj44gz2g",
            "waglayla:qql4warf635653r050ppwk9lm8vln2wdwucjnhljxtqnxk2x4axfgm4g4hzl5",
            "waglayla:qqgrtx4nuhjavpwwrfsa7akg6fcna7dmjtpgc69f6ysg8vzjrmwwsjr8j9xpr",
            "waglayla:qrny80e7zurf9sq9pzcesafyat030zkqnt4w02aa9xl8xvh9w0r867ga8cp7n",
            "waglayla:qp0yve4h89udt5rvpzwf3qrecdcscdgfq420eh2d9v43d96t0lwkw07y6jdwu",
            "waglayla:qrlx73us8hrfe2g78uw84aqerz9ea889rwc3e7pezvwv7rakcr3mk2wm9xm44",
            "waglayla:qrpjp0m0x2708vazdajlfct6e2pnxc2xk5kndz7glg2akug2fl48j8qa2ztp8",
            "waglayla:qr82t672mwqrqym3p8s00aevqkp67x2hdrhj7079shsdxz4kecf3j45vf3q22",
            "waglayla:qzqkv08jvktzyxl9829d0lgg2h2quurq0gr263atmpj9zevaj2ze5z3pmdcg0",
            "waglayla:qz0cg9990rddlscth27syxcgr6x6xxkyjjyn6jn9lgd7r3pd6064cmkdl06kt",
            "waglayla:qza4cgmzy4x3ztlhmaf3v3fnr8ghazd7lengrpdrtfsspyaz0p7yssp2cjtkk",
            "waglayla:qp44w4lq42ck4zm9r0gga8uz6ghzug3jcd4ju9cmnn3ld9sz3r3sk4hjtkf4u",
            "waglayla:qqa6k29l06ht6vvtspfgq7qvflyum3ya4k98rnglhpuvnsus3jynjwxmv8d5d",
            "waglayla:qz6rmppc4h9zkzv8x79e4dflt5pk0vfr0fnv86qfq4j7lu2mluex7zpvv42cm",
            "waglayla:qqlzdmud9mwfgsmy7zk8ut0p0wvxtrllzt26hv6jffjdy224jartw0p42847n",
            "waglayla:qpvf0wx35uwda732xpgu7fakh37ucdudm335msw4f4aw7fuv3unzx7hnypsv0",
            "waglayla:qzhafa8n9st86gxk07rehpy8fdghy669l8sy57l3fae97r6yc6hlxz4rcss7g",
            "waglayla:qr36fmpfggppn6ch9u5rwflhy5tpgyfhfvtmkglln089f3g38j4tcum4tv3z4",
            "waglayla:qz8r3qrdzfkfp9raq3etxhg5sl6zwuvmpggnprhlhch5ksapj37ty6slewydq",
            "waglayla:qrct5xhxtp87qn3hjnx9e5eatlkez3sw70zywm5n3a8vvzuhqtdez5uhd5ne2",
            "waglayla:qr57llq4lhrd0cf58553najxj4fd4kh4m69tp2s8dlmqh2353sm0v9fn5hg6e",
            "waglayla:qpqqn25lhhyhz9aflteadkzxrvhy390rpjlmcauf5ry5feyvawff2qllms89r",
            "waglayla:qz00pye8ezdsm6h9j6840dzv2cgv8qkrd8a77efl2ealv82vu4l65cj4hvtlc",
            "waglayla:qq2z5vfeqpcvh2f0x8atn67jf6en4vzqhu0ahd9w0fr8ngzgc2fl2mrmyjkwu",
            "waglayla:qz62rs7guer4lyahu5j9xsrn38mcnmnshjl984v5sq8ldtz6m48tqatljzfh5",
            "waglayla:qzmsd5k3h8ztc4ulp0rgnz7httxy7tre6quswrp60xh9emxmw8lvkvf8fys3s",
            "waglayla:qz4patdle0j4q8cg93fs9qkk2uu8tm42jse0x5nn2ssrqsphlptfxp67st0vv",
            "waglayla:qpkzst9yfzcvdfdymkdt69gt7rm3r2ztcjrarl0ss09jcgxzpjvkxz28m25kk",
            "waglayla:qrksn3kunxwkpfudhdwwjhpvsuklz2eq684ghf087zsnvheywpxfvgysnfq0g",
            "waglayla:qzzxrs6wkqnfpyk4gnsn9tajl8rrw2tznecu7uxp62emgmc62u4qsk5udslls",
            "waglayla:qrd26p83evu7remt400h60r370q05y9y3t2eygw0a8ya4n6sp4wacpsefkuyg",
            "waglayla:qzvw3r65mhxa5ekgwdnazlhdqhmxazacht80s2yh9fuw2nxwy23a5rryr8nnr",
            "waglayla:qptu8eegz7y050qxq32ece5sydpdgage07ussm8vuaged9anl62qsq2fwtc6t",
            "waglayla:qza9y7xmw3s8ms63pdc94al4xnllw95kzlegnsuk0zyw2hvzx5e557mrlr0jp",
            "waglayla:qq75ps5c4de6jrg3vq4nz8gtvsflh79pulwf7avcrs2s0z9z6psw6s6muwadp",
            "waglayla:qp3085yvwxj2v52u7dv5v5w63k9vlf677zlya2krj5jpp69w2e3gk6ktuv8ql",
            "waglayla:qqjaqpnzxfqwkuuyjd7qvulgx804uta9wtkdntphc36kc3nj9xgg29sft82sl",
            "waglayla:qprptscwd4tyhjh2eyc9ve5paxcap7k88mz84q0sk36ajhlje3a5kdf7qt56w",
            "waglayla:qq7mf20qh9g4rtf4h76wepcpjem0x7jq39qy875ra2jk4m8gzc7452m2tnz96",
            "waglayla:qpydw5azt092uhwscnn96pflcnyn5e264f2lxmxhufj27cptzz8evw5hghynp",
            "waglayla:qzm375sk4xgacy0smneq9kuwza8g2l664cer3vlmv7mvwg0m5nw8u0scdv84q",
            "waglayla:qrw8r594tdzy026rqpe4pa830qxcsjqhzlv7p438x939292kvqaxvsgfe56m8",
            "waglayla:qppe5llh7m75z084xrjt0y5thfss5u6srl945ln2r4039ce937pwwz5lanqxy",
            "waglayla:qqw55sj3x3tvvpy0ufk0rarz0zxnmj2avhukvswgy4h0d6cxxmy0kqfr8lsnd",
            "waglayla:qzrmdyudtf7uv7g5f5pnv0x93r3c85084rgd8mhxgync66rkpjml26a28tdjl",
        ]
    }

    fn gen0_change_addresses() -> Vec<&'static str> {
        vec![
            "waglayla:qrp03wulr8z7cnr3lmwhpeuv5arthvnaydafgay8y3fg35fazclpc6zngq6zh",
            "waglayla:qpyum9jfp5ryf0wt9a36cpvp0tnj54kfnuqxjyad6eyn59qtg0cn606fkklpu",
            "waglayla:qp8p7vy9gtt6r5e77zaelgag68dvdf8kw4hts0mtmrcxm28sgjqdqvrtmua56",
            "waglayla:qzsyzlp0xega2u82s5l235lschekxkpexju9jsrqscak2393wjkdcnltaa0et",
            "waglayla:qpxvpdfpr5jxlz3szrhdc8ggh33asyvg4w9lgvc207ju8zflmxsmgnt3fqdq6",
            "waglayla:qz28qjteugexrat7c437hzv2wky5dwve862r2ahjuz8ry0m3jhd9z72v4h8w9",
            "waglayla:qz8cus3d2l4l4g3um93cy9nccmquvq62st2aan3xnet88cakhtljuk69seejg",
            "waglayla:qzczlu9crsn9f5n74sx3hnjv2aag83asrndc4crzg2eazngzlt0wq90zqsfm7",
            "waglayla:qqemqezzrgg99jp0tr8egwgnalqwma4z7jdnxjqqlyp6da0yktg5x9qfe9mwx",
            "waglayla:qr0nfhyhqx6lt95lr0nf59lgskjqlsnq4tk4uwlxejxzj63f2g2acs7c3nvtv",
            "waglayla:qqp0s3dacp46fvcaq5v2zl43smk2apzslawjqml6fhudfczp5d9n2p0s34t0s",
            "waglayla:qzac4rjzem4rvzr6kt2yjlq7whawzj9ra9calpw0euf507fdwuskq567ej2yt",
            "waglayla:qrupjagxeqqzahlxtpraj5u4fd7x3p6l97npplge87pgeywkju47zqcdua4yg",
            "waglayla:qz208ms8heafvt90d28cpm3x7qvav87e3a2hgcz0e5t3d84xmlvcqqx9wg9pg",
            "waglayla:qq5357axc5ag8hzytf66p3fzw8d578h7xyfm4x4cpr3lp0wallglk6sfmklua",
            "waglayla:qzsjhgefa98e4fsk58znu03mwzw7ymj7t4392l69kp0pgml2ymqm63njk5vxf",
            "waglayla:qplnwp0lxzwykmxrqphu62drmem2d09kfzplfek8z7cwt4s3vkkakvek89fhv",
            "waglayla:qr4cm8smzgt8gzg33csv9mrsnvj9809ffun89cqsw65q3a37vmqx5ng67x8h4",
            "waglayla:qpj0d7nznxp3nn2kyqsvm0ns38hzdk7dhj8g90cnrv9jda8xw5q2y8v8q7yh3",
            "waglayla:qp4qt5cjrq73nuatnlwnk90lz5kqpd4mpqm53x7h3lpu74phz6zm5g9qmy00m",
            "waglayla:qzjrlcwkl2mssucyyemnxs95ezruv04m8yyek65fyxzavntm9dxtkva30sv3u",
            "waglayla:qz24dfwl08naydszahrppkfmkp2ztsh5frylgwr0wqvjqwnuscvmwg2u0raml",
            "waglayla:qqy8pv5sv9quqce26fhn0lygjmuzrprlt90qz6d4k2afg0uaefptgva6l52ee",
            "waglayla:qpmqpmnwhqv7ng24dh6mj6zqm0zptkgv0fvetcgqgv8vdukk3y59ycm40t66l",
            "waglayla:qrumw263pj7gw8jqye7kd58gqq6lgnv2fjvevuf55wptvqp0r5ryj4f29upt3",
            "waglayla:qzv60vtkmaaxgp4kfj86yjxt9w03qgxma5rmfsgwupeguxhgtnq0ytrnfj2gp",
            "waglayla:qzyn8xpvuh8vfsp0zd8rc3990dgwlhrukt26xdqt0zcu5mm8jsjcyf5x95cwl",
            "waglayla:qzrvh8zyclunxu3dfuqyp5yv853ejeqqkfp2gcyyyq3mju5ame5xsv3g857ky",
            "waglayla:qpfkj0emekeqvsc925cnna9mt8zhtazfwcjfjd3kss4f8fvensppz24wckvcx",
            "waglayla:qq2hv6nhxegvex8vqaun6cjpmgu6lelf6l6mfz4565zn3qjwjlu0kmlfutzgr",
            "waglayla:qrnclejggdsg4ds8fxmgcmn22sy2w5704c6d9smug7ydyd65grzk23ty4jzyv",
            "waglayla:qz74fxk35jc0g8s4u76uxcdahahhumu4ttzfmcu94vqkymla33lmkykxwfqdn",
            "waglayla:qpmpe7s45qmx3gzehuhh8nra9x5sk3s5hdr6e7jlyhtrjq6zhtt6cmp8r6hs3",
            "waglayla:qzz4v2h3s2y7dvpsy6pt0urrjx3rw658t25g6uj9jfvx4f8vwhetc604fe5l9",
            "waglayla:qqz06pskea8ktjwfn90y46l366cxlt8hw844ry5xz0cwv5gflyn9vasks28s3",
            "waglayla:qzja0zah9ctrlg2fs6e87lac2zal8kngn77njncm6kv6kxmcl5cwkjq4c62mq",
            "waglayla:qzue8jx7h2edm3rjtk4fcjl9qmq59wrhg6ql2r4ru5dmc4723lq0zf4jjsxvk",
            "waglayla:qp0yeh6savdtyglh9ete3qpshtdgmv2j2yaw70suhthh9aklhg227erlpvdrc",
            "waglayla:qrzp9ttdmpc94gjxarq97y3stguw9u72ze02hd7nl30whydz44uhudmk0xsxh",
            "waglayla:qzyfayq2tu5q6t5azlr89ptq0mcplv8m4zdmmtrve26cevydfkn26qgna8acl",
            "waglayla:qr6w0un2pde7sm29793srwqwq5p2vqhq8q39l4g6dhx2x9p0wg8ly673pm4na",
            "waglayla:qpp2qwmk7v3tlfxcg0gvd960f90t6fx42wtfszjnh5m8y8j5cygtwkk62v3wv",
            "waglayla:qqp6p2rmml9anfs09wqsu2e6j4mjmndczju7psnkm9hh84j42k9cwm3lcgxwn",
            "waglayla:qz3ze0g3n9xe3dzs98h5xf3wfk3wlzz3h2zg3cppvaeq4xcamhpe7xfa08ek5",
            "waglayla:qqgjuzgapalk9k8zm0vp0nndtvfk043drm8n0hk05px40tv8eaxejzne0ylgy",
            "waglayla:qraklk33dys3js328admu2vpdj37zhc05fnr983c3new2etks3fz5078yw8ng",
            "waglayla:qzm6tvhluq0cvy9cuhwuaye05wch62gcnf0nsmjpw2gur3a7pyrhgcdga35fp",
            "waglayla:qqexh4wwjuncvmm7cycyrpjwnuknvk50f9zmahjt0m2tmex5zx02uarmtdkv8",
            "waglayla:qredxcud88qfq39zltc3sc6425g6d36v66sx6rv236f42g65tc4yx7nzsmnak",
            "waglayla:qpnuv59xjnj49quayyyn7n0zyunu9u4q7650s8w85k9ef6379ermkrgyl92mw",
            "waglayla:qpfvr7qvsy0hxhy2hg2jtm5kr8vnvr49d7yn9wpcymf8pjeekpnq2ql34s7pc",
            "waglayla:qph0vh0wtu59yklxpgthfr7upnya28duzkkgr5l43urj6qvy65stk77nutmd8",
            "waglayla:qq9dmujd78f4et7nq3qdquuq4gth29r2ln2qt054qyy5rpyjfymyuj4y6nq4r",
            "waglayla:qpdt4tz7yc2atpdu5g9kkx0v4lsjsd6jdw3euc25rv6pmn5nvakxkag8c5tde",
            "waglayla:qz9yfegr2aw2skxf4lsdw4v4jruemapw6ehkv5x0yp0985kz5p6wc4sy2p7m2",
            "waglayla:qr9guyx4j9an7rnml36vfwpurn8nut3g4534j5wvkv4aqvdsn05mv7ty3crnm",
            "waglayla:qz7a4mu29gf8ly68s34mpe53s8jd5gxzrmu8vqjre44rdfvhnlpl678napd6d",
            "waglayla:qry4n3pu0f293n7r03k5fty0eksdhnn869vyqcr8td8stcn6l4ql7ew0ldsnq",
            "waglayla:qp5tw4rpvkezcvpcdz8pwln04fxhawekuyfvhrcyjcljpcdkctmucfru40xtl",
            "waglayla:qpkwrwgmh6zh5jfw2stleumkdxjnj4uyxxems3ucy2rk4z7mrnpjyrx68qz6j",
            "waglayla:qzzfgs3lh80avxx7nc0kp7k8esntaaegy63er6vf8vwuxmw3z42wc8ldgjh7h",
            "waglayla:qrakpce50ps6zfjhrux5y99kf75z936rmg20h3tryjht4g5kldwmut4jwg49v",
            "waglayla:qzgay26sfqzmnjtzhtase4sy9ucddfyey3x335z7kmpqlkh4h3laxtl5mlsq9",
            "waglayla:qzsjnxw8ezs7yjzxgy3900548ufz27yjh2g53n8se3qth3spn78jzcgh8luh8",
            "waglayla:qrcngzqx23q82rtuu47ytr68n5974mlczhz485hk93vmhe3a4lq4x85fp2sx7",
            "waglayla:qpncnvnxen0hxn75d6aheel7r6apeylggmyt90a5aapy5aynj8ypgny0nva6z",
            "waglayla:qrg582jtu2vdqym7mysc9ngrhm3jhysswlrf8q8cjadcm34ckeyngqe8vvym5",
            "waglayla:qzrjslkurxallygypt5avdfxzl4ee889fd363t7xdnqyp7lzl4ehxzscy32pz",
            "waglayla:qrr9p4m6wj9nzw6calhk3gus7j2e7q2w88v9swr4hghmxkutqyvfxcksamrkp",
            "waglayla:qzj7clnh0zz7la55yxawsr4lt00av5fkxtel74gfpm96cak49lgdz6vev8hwz",
            "waglayla:qzpnspfucuefnfmxlhuswh5e92lzv9wp7kn25g95xx5nnsvzmyygchet9sz95",
            "waglayla:qrtw6fz7wt73zvyp36vcajfk9sajgl8jxpzxamv56un5lpp9wwunsd6s2vzy0",
            "waglayla:qpq3n27p3nhn3x22jjawchvpl80x6a63faujp0xt6uyx04plcetwu2jga8407",
            "waglayla:qq7de2y9ed6cq5cysd2w897l682s4mtve92s2l075nr8fq3xq2k42xtpe0xaz",
            "waglayla:qp8ccwx0sfscktvz4pus2gh9zckyswgdhw9npxq85wx4ekcwhxv3yezkfcwx2",
            "waglayla:qpfmre8d6nru9v6lfn3u643aa2jq9gjs89pe499cna8fpsr0h39868e4vnn73",
            "waglayla:qrvnmyphgyqpenm9xe0qqsfdul2xf9ylmkxvxjxkrvq3rfn6sy895s8jgaefa",
            "waglayla:qzf43vv4ytjzy46srr6amxr0yhyh392hygq089m932e3w9su602fqxga35a7t",
            "waglayla:qz7kme8jqvvx7kamr7r2kdhwt4jwaq0glqdh7rh6jneymv3nhz0hu2tpgd9ld",
            "waglayla:qzrgx4h0w3jzy39ade57lynljn2kcqay4ralmnjvr7anz3vzg3yaqj0h5k4pd",
            "waglayla:qrevlha74yuz6sltmh9w0qjmj3gt3xrt2s7z4e8f4narf355tuq55pj5uaefs",
            "waglayla:qq2c6p62l2z43sakuee5tspr7ctpfv370kuq2fmmqk76ya4wcdmmyk4z06pks",
            "waglayla:qpcf9yfxzss3cjh70n3wau3uhq844txz6pw2sd507lnkygv06xtm5yajwqdyk",
            "waglayla:qzjm2uk405lzzmyn4ad9x6736qy4gxw84vkdpylrjmegzv0e3nqrkna5cnwly",
            "waglayla:qz4rfm4drdvj9yqz4pzx68zjq5zmgueclwmzd9femj4rm0x9n5m8qyk3cfxsf",
            "waglayla:qr8h52caava83pk77nraxaea7g2yvumjjp29f82lyh2qcdx47ngcy40estyl7",
            "waglayla:qp2uxlg9mtehpj3cx83stwq9tjv2tu3cm8dcf62xzvy5t75jputccclm89r4d",
            "waglayla:qr9kp5p0k3mx8n8qwkfckppm3q3c4pup347n2qygfq80hxsljtu2srm34g4h8",
            "waglayla:qrlpxflqrspyn8rjk93lst8tja0xt6jzmv7msmzugpjn5t7j7w3c2kdxljtxc",
            "waglayla:qzc7rk8gm7k0z27j9gjag54e2k46tghscryhwe549g24340sha4kuv2kv4dul",
            "waglayla:qrr7v7zu9qpleenec5s5rl7frxwemrewtehzlm47pa8lkqqgy3nw6eq8v0sv5",
            "waglayla:qzu5ent4t0f4fzz0muf5qqmrspqn4re35w77mlujzfsnjtpglhg8segj8m6n3",
            "waglayla:qznp2z9dn4dfapk478mv8cpr5zh8qj69wv2mpydfzw7weh9aacjvs7ryvtfgz",
            "waglayla:qqd7xdpywvlmrc86weqay2z3ve85f25tdfffn6phd47shmtsrrzzw6ulp2vhr",
            "waglayla:qrl4rhex484u46n8y2u9jhf24qefp4ua5hyfechz78p4hl64t648z5ln07h7j",
            "waglayla:qzhmxv8p8gsn3vnf8xqp2ashcvc39a54fpnlwgztcw4wg0g7wuv8c5w6d6pmg",
            "waglayla:qpuz7tpwy49dnjc8udfsm9m65pkv80ey8x722wyaq9ehjjmfywx3gm5sd380m",
            "waglayla:qpgtjsa4f3nnkt62ukyq2eu83w0u7fap906txwajqf5t5uxt9tqmjrk0n9hzy",
            "waglayla:qzlp093qcsspd0nzs8x9v6kxuy2x938hhpn3jw9l8s6lafykwe8nxpqe4e59w",
            "waglayla:qzlv8cya2gej9y2szg2zj9krrgdwfxr8250apcz7r72rhmk0lv9nk7rn8akju",
        ]
    }

    #[tokio::test]
    async fn hd_wallet_gen0_set_key() {
        let master_xprv =
            "xprv9s21ZrQH143K3knsajkUfEx2ZVqX9iGm188iNqYL32yMVuMEFmNHudgmYmdU4NaNNKisDaGwV1kSGAagNyyGTTCpe1ysw6so31sx3PUCDCt";
        //println!("################################################################# 1111");
        let hd_wallet = WalletDerivationManagerV0::from_master_xprv(master_xprv, false, 0, None);
        assert!(hd_wallet.is_ok(), "Could not parse key");
        let hd_wallet = hd_wallet.unwrap();

        let hd_wallet_test = WalletDerivationManagerV0::create_uninitialized(0, None, None);
        assert!(hd_wallet_test.is_ok(), "Could not create empty wallet");
        let hd_wallet_test = hd_wallet_test.unwrap();

        let pubkey = hd_wallet_test.derive_receive_pubkey(0);
        assert!(pubkey.is_err(), "Should be error here");

        let res = hd_wallet_test.set_key(master_xprv.into(), None);
        assert!(res.is_ok(), "wallet_test.set_key() failed");

        for index in 0..20 {
            let pubkey = hd_wallet.derive_receive_pubkey(index).unwrap();
            let address1: String = PubkeyDerivationManagerV0::create_address(&pubkey, Prefix::Mainnet, false).unwrap().into();

            let pubkey = hd_wallet_test.derive_receive_pubkey(index).unwrap();
            let address2: String = PubkeyDerivationManagerV0::create_address(&pubkey, Prefix::Mainnet, false).unwrap().into();
            assert_eq!(address1, address2, "receive address at {index} failed");
        }

        let res = hd_wallet_test.remove_key();
        assert!(res.is_ok(), "wallet_test.remove_key() failed");

        let pubkey = hd_wallet_test.derive_receive_pubkey(0);
        assert!(pubkey.is_ok(), "Should be ok, as cache should return upto 0..20 keys");

        let pubkey = hd_wallet_test.derive_receive_pubkey(21);
        assert!(pubkey.is_err(), "Should be error here");
    }

    #[tokio::test]
    async fn hd_wallet_gen0() {
        let master_xprv =
            "xprv9s21ZrQH143K3knsajkUfEx2ZVqX9iGm188iNqYL32yMVuMEFmNHudgmYmdU4NaNNKisDaGwV1kSGAagNyyGTTCpe1ysw6so31sx3PUCDCt";
        //println!("################################################################# 1111");
        let hd_wallet = WalletDerivationManagerV0::from_master_xprv(master_xprv, false, 0, None);
        assert!(hd_wallet.is_ok(), "Could not parse key");

        //println!("################################################################# 2222");
        //let hd_wallet2 = hd_::WalletDerivationManagerV0::from_master_xprv(master_xprv, false, 0, None).await;
        //assert!(hd_wallet2.is_ok(), "Could not parse key1");

        let hd_wallet = hd_wallet.unwrap();
        //let hd_wallet2 = hd_wallet2.unwrap();

        let receive_addresses = gen0_receive_addresses();
        let change_addresses = gen0_change_addresses();

        //println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        //println!("hd_wallet1: {:?}", hd_wallet.receive_pubkey_manager().public_key());
        //println!("hd_wallet2: {:?}", hd_wallet2.receive_pubkey_manager.public_key());

        // let pubkey = hd_wallet2.derive_receive_pubkey(0).await.unwrap();
        // let address: String = hd_::PubkeyDerivationManagerV0::create_address(&pubkey, Prefix::Mainnet, false).unwrap().into();
        // assert_eq!(receive_addresses[0], address, "receive address at 0 failed $$$$ ");

        for index in 0..20 {
            let pubkey = hd_wallet.derive_receive_pubkey(index).unwrap();
            let address: String = PubkeyDerivationManagerV0::create_address(&pubkey, Prefix::Mainnet, false).unwrap().into();
            assert_eq!(receive_addresses[index as usize], address, "receive address at {index} failed");
            let pubkey = hd_wallet.derive_change_pubkey(index).unwrap();
            let address: String = PubkeyDerivationManagerV0::create_address(&pubkey, Prefix::Mainnet, false).unwrap().into();
            assert_eq!(change_addresses[index as usize], address, "change address at {index} failed");
        }
    }

    #[tokio::test]
    async fn generate_addresses_by_range() {
        let master_xprv =
            "xprv9s21ZrQH143K3knsajkUfEx2ZVqX9iGm188iNqYL32yMVuMEFmNHudgmYmdU4NaNNKisDaGwV1kSGAagNyyGTTCpe1ysw6so31sx3PUCDCt";

        let hd_wallet = WalletDerivationManagerV0::from_master_xprv(master_xprv, false, 0, None);
        assert!(hd_wallet.is_ok(), "Could not parse key");
        let hd_wallet = hd_wallet.unwrap();
        let pubkeys = hd_wallet.receive_pubkey_manager().derive_pubkey_range(0..20).unwrap();
        let addresses_receive = pubkeys
            .into_iter()
            .map(|k| PubkeyDerivationManagerV0::create_address(&k, Prefix::Mainnet, false).unwrap().to_string())
            .collect::<Vec<String>>();

        let pubkeys = hd_wallet.change_pubkey_manager().derive_pubkey_range(0..20).unwrap();
        let addresses_change = pubkeys
            .into_iter()
            .map(|k| PubkeyDerivationManagerV0::create_address(&k, Prefix::Mainnet, false).unwrap().to_string())
            .collect::<Vec<String>>();
        println!("receive addresses: {addresses_receive:#?}");
        println!("change addresses: {addresses_change:#?}");
        let receive_addresses = gen0_receive_addresses();
        let change_addresses = gen0_change_addresses();
        for index in 0..20 {
            assert_eq!(receive_addresses[index], addresses_receive[index], "receive address at {index} failed");
            assert_eq!(change_addresses[index], addresses_change[index], "change address at {index} failed");
        }
    }

    #[tokio::test]
    async fn generate_waglaylatest_addresses() {
        let receive_addresses = [
            "waglaylatest:qqz22l98sf8jun72rwh5rqe2tm8lhwtdxdmynrz4ypwak427qed5juktjt7ju",
            "waglaylatest:qz880h6s4fwyumlslklt4jjwm7y5lcqyy8v5jc88gsncpuza0y76xuktmrx75",
            "waglaylatest:qrxa994gjclvhnluxfet3056wwhrs02ptaj7gx04jlknjmlkmp9dx0tdchl42",
            "waglaylatest:qpqecy54rahaj4xadjm6my2a20fqmjysgrva3ya0nk2azhr90yrzyce6mpfps",
            "waglaylatest:qzq3sc6jkr946fh3ycs0zg0vfz2jts54aa27rwy4ncqz9tm9ytnxsyns7ad6e",
            "waglaylatest:qq4vl7f82y2snr9warpy85f46sde0m0s8874p2rsq6p77fzccyflyuvcez6mr",
            "waglaylatest:qq5kqzu76363zptuwt7kysqq9rmslcfypnyckqr4zjxfljx7p8mlwthe5q26v",
            "waglaylatest:qqad0qrj6y032jqxuygcyayvu2z8cza9hlvn8m89z3u6s6s8hg3dyav40hczu",
            "waglaylatest:qpwkdpyf766ny56zuj47ax63l689wgg27rv90xr2pruk5px8sstcg9g7hkdsf",
            "waglaylatest:qpn0vug0j36xfdycq7nl6wczvqnhc22d6ayvhs646h76rv3pdpa87vj9rycuc",
            "waglaylatest:qz4c7eg9uernmsqmt429lvj5f85qsqzt6dgy8r53aefz39m77w2mgm3najzyr",
            "waglaylatest:qqzfgqmmxrznec9hl35xwa8h6hs5mcr7lt7ep6j6373lxfq9jpj464sazd05p",
            "waglaylatest:qr9033gap4pscrhkwyp0cpmpy62a9pmcpqm2y4k29qqlktceulm7y4dru7f0h",
            "waglaylatest:qq3ktnql8uxwyj0kq6gq4vp8gm5ftnlvq0aphr55hl6u0u8dp49mqul4l4zc4",
            "waglaylatest:qqrewmx4gpuekvk8grenkvj2hp7xt0c35rxgq383f6gy223c4ud5s58ptm6er",
            "waglaylatest:qrhck7qaem2g2wtpqvjxtkpf87vd0ul8d8x70tu2zes3amcz70reghy0tlheh",
            "waglaylatest:qq4lnkxy9cdylkwnkhmz9z0cctfcqx8rzd4agdhzdvkmllrvc34nw0feewtj2",
            "waglaylatest:qzdt4wh0k63ndsv3m7t4n7flxu28qh3zdgh6ag684teervsfzzkcu32fmyva4",
            "waglaylatest:qqqng97tn6lfex3je7n0tr64e36zmzfyhpck2jeqts2ruatz3r5aswcy4yuxu",
            "waglaylatest:qq2je8w0ltztef0ygljpcqx055kcxgxtsffwu7ujxzjfhk5p5rqlwxearwav4",
        ];

        let change_addresses = vec![
            "waglaylatest:qq3p8lvqyhzh37qgh2vf9u79l7h85pnmypg8z0tmp0tfl70zjm2cv2hgkfmnl",
            "waglaylatest:qpl00d5thmm3c5w3lj9cwx94dejjjx667rh3ey4sp0tkrmhsyd7rg9p27me7m",
            "waglaylatest:qq407023vckl5u85u6w698fqu3ungs598z3xucc2mhr9vy0hug5vvxuyqvcnq",
            "waglaylatest:qzl0qcvjfuwrrgzz83fuu272j7n9g03xfzp0g0f9jq5kll4rjfct536mv55uq",
            "waglaylatest:qp6l8n5meyut2yvpyw2dqrrcgc3t6jxflheh9j8s2f75quepdl4qvg4hnfexd",
            "waglaylatest:qqw0uhr54kpyna0zrya6q7w2kya84ydgcvsdwaehayk8pn40d4y6sxpp3g3hh",
            "waglaylatest:qr5kjerrvnru7w49umrc0jtws6hpf7s22ur9nav0fsazs8kyy8ydwgrafxpw9",
            "waglaylatest:qqd8lyeya58hjym2xlw7th2wuenlptydmvzrzu53gxft0e2d844sv8cu3ymac",
            "waglaylatest:qr0cs9lrdwjesuw5vf0x5rj78ecayphu60vt29smjerusqmec9w96acyggwv0",
            "waglaylatest:qq089gr7p4rggwjqwh34mmdlsa357vprzl4q0dzn9c92egfs5aj5xc8h3j7rg",
            "waglaylatest:qzs6m6nmkqczmxtjzptzzyl46nwwgq6hymk8jz3csg2h0lh0rpqjk28hxgr9t",
            "waglaylatest:qr4k0fs6z47chukqv82walvyjmztd6czaqlk0kfdwr90rv3zwu5hjnlcldh2e",
            "waglaylatest:qpgcua8savrpy7ggdxm0cq2uqgcd4a9skc39fld5avy3dvdcdsjssh3g72sun",
            "waglaylatest:qq2hllm2ff2rwgq3cyaczvusw5tr5ugfz2dtaedqxhuktz6sywves25p4nkn7",
            "waglaylatest:qrr2a2lttpx8uaj0qtd80cl90h5qx7c9xgsdqzcfm2rntme9vuxpzjc5zwl3z",
            "waglaylatest:qqa8tjjr9ngudgh2gxyjevjazmgpx3v6zc3zn3aka38gm3erl6xx59pmcm6l8",
            "waglaylatest:qqllkscqj7jd8tugj3rsl9r67evgandgnznekwl48cwp80jx6cut2e3aht768",
            "waglaylatest:qq83n9wrk2ujn2hayyt74qfrctjp803csz5lsdzp0dslu7wue2ps5d5dd6aj2",
            "waglaylatest:qz5qk6nvffsgdcujma3gq5rr2lr2q6yjw87n3w6asc0uj3rr8z8pk7sl927wk",
            "waglaylatest:qr55n5vkaq6lxcwzl6522nz86dj7ntl76nergy0u2j99v8w8lhyv6gshlq3wz",
        ];

        let master_xprv =
            "xprv9s21ZrQH143K2rS8XAhiRk3NmkNRriFDrywGNQsWQqq8byBgBUt6A5uwTqYdZ3o5oDtKx7FuvNC1H1zPo7D5PXhszZTVgAvs79ehfTGESBh";

        let hd_wallet = WalletDerivationManagerV0::from_master_xprv(master_xprv, false, 0, None);
        assert!(hd_wallet.is_ok(), "Could not parse key");
        let hd_wallet = hd_wallet.unwrap();

        for index in 0..20 {
            let key = hd_wallet.derive_receive_pubkey(index).unwrap();
            //let address = Address::new(Prefix::Testnet, waglayla_addresses::Version::PubKey, key.to_bytes());
            let address = PubkeyDerivationManagerV0::create_address(&key, Prefix::Testnet, false).unwrap();
            //receive_addresses.push(String::from(address));
            assert_eq!(receive_addresses[index as usize], address.to_string(), "receive address at {index} failed");
            let key = hd_wallet.derive_change_pubkey(index).unwrap();
            let address = PubkeyDerivationManagerV0::create_address(&key, Prefix::Testnet, false).unwrap();
            assert_eq!(change_addresses[index as usize], address.to_string(), "change address at {index} failed");
        }

        println!("receive_addresses: {receive_addresses:#?}");
    }
}
