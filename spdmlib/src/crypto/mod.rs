// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

mod crypto_callbacks;

#[cfg(feature = "spdm-ring")]
mod spdm_ring;

pub use crypto_callbacks::{
    SpdmAead, SpdmAsymSign, SpdmAsymVerify, SpdmCertOperation, SpdmDhe, SpdmDheKeyExchange,
    SpdmHash, SpdmHkdf, SpdmHmac,
};

use conquer_once::spin::OnceCell;

static CRYPTO_HASH: OnceCell<SpdmHash> = OnceCell::uninit();
static CRYPTO_HMAC: OnceCell<SpdmHmac> = OnceCell::uninit();
static CRYPTO_AEAD: OnceCell<SpdmAead> = OnceCell::uninit();
static CRYPTO_ASYM_SIGN: OnceCell<SpdmAsymSign> = OnceCell::uninit();
static CRYPTO_ASYM_VERIFY: OnceCell<SpdmAsymVerify> = OnceCell::uninit();
static CRYPTO_DHE: OnceCell<SpdmDhe> = OnceCell::uninit();
static CRYPTO_CERT_OPERATION: OnceCell<SpdmCertOperation> = OnceCell::uninit();
static CRYPTO_HKDF: OnceCell<SpdmHkdf> = OnceCell::uninit();

pub mod hash {
    use super::CRYPTO_HASH;
    use crate::crypto::SpdmHash;
    use crate::msgs::{SpdmBaseHashAlgo, SpdmDigestStruct};

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmHash = SpdmHash {
        hash_all_cb: |_base_hash_algo: SpdmBaseHashAlgo,
                      _data: &[u8]|
         -> Option<SpdmDigestStruct> { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::hash_impl::DEFAULT;

    pub fn register(context: SpdmHash) -> bool {
        CRYPTO_HASH.try_init_once(|| context).is_ok()
    }

    pub fn hash_all(base_hash_algo: SpdmBaseHashAlgo, data: &[u8]) -> Option<SpdmDigestStruct> {
        (CRYPTO_HASH.try_get_or_init(|| DEFAULT).ok()?.hash_all_cb)(base_hash_algo, data)
    }
}

pub mod hmac {
    use super::CRYPTO_HMAC;
    use crate::crypto::SpdmHmac;
    use crate::error::SpdmResult;
    use crate::msgs::{SpdmBaseHashAlgo, SpdmDigestStruct};

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmHmac = SpdmHmac {
        hmac_cb: |_base_hash_algo: SpdmBaseHashAlgo,
                  _key: &[u8],
                  _data: &[u8]|
         -> Option<SpdmDigestStruct> { unimplemented!() },
        hmac_verify_cb: |_base_hash_algo: SpdmBaseHashAlgo,
                         _key: &[u8],
                         _data: &[u8],
                         _hmac: &SpdmDigestStruct|
         -> SpdmResult { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::hmac_impl::DEFAULT;

    pub fn register(context: SpdmHmac) -> bool {
        CRYPTO_HMAC.try_init_once(|| context).is_ok()
    }

    pub fn hmac(
        base_hash_algo: SpdmBaseHashAlgo,
        key: &[u8],
        data: &[u8],
    ) -> Option<SpdmDigestStruct> {
        (CRYPTO_HMAC.try_get_or_init(|| DEFAULT).ok()?.hmac_cb)(base_hash_algo, key, data)
    }

    pub fn hmac_verify(
        base_hash_algo: SpdmBaseHashAlgo,
        key: &[u8],
        data: &[u8],
        hmac: &SpdmDigestStruct,
    ) -> SpdmResult {
        (CRYPTO_HMAC
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .hmac_verify_cb)(base_hash_algo, key, data, hmac)
    }
}

pub mod asym_sign {
    use super::CRYPTO_ASYM_SIGN;
    use crate::crypto::SpdmAsymSign;
    use crate::msgs::{SpdmBaseAsymAlgo, SpdmBaseHashAlgo, SpdmSignatureStruct};

    pub fn register(context: SpdmAsymSign) -> bool {
        CRYPTO_ASYM_SIGN.try_init_once(|| context).is_ok()
    }

    static DEFAULT: SpdmAsymSign = SpdmAsymSign {
        sign_cb: |_base_hash_algo: SpdmBaseHashAlgo,
                  _base_asym_algo: SpdmBaseAsymAlgo,
                  _data: &[u8]|
         -> Option<SpdmSignatureStruct> { unimplemented!() },
    };

    pub fn sign(
        base_hash_algo: SpdmBaseHashAlgo,
        base_asym_algo: SpdmBaseAsymAlgo,
        data: &[u8],
    ) -> Option<SpdmSignatureStruct> {
        (CRYPTO_ASYM_SIGN.try_get_or_init(|| DEFAULT).ok()?.sign_cb)(
            base_hash_algo,
            base_asym_algo,
            data,
        )
    }
}

pub mod asym_verify {
    use super::CRYPTO_ASYM_VERIFY;
    use crate::crypto::SpdmAsymVerify;
    use crate::error::SpdmResult;
    use crate::msgs::{SpdmBaseAsymAlgo, SpdmBaseHashAlgo, SpdmSignatureStruct};

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmAsymVerify = SpdmAsymVerify {
        verify_cb: |_base_hash_algo: SpdmBaseHashAlgo,
                    _base_asym_algo: SpdmBaseAsymAlgo,
                    _public_cert_der: &[u8],
                    _data: &[u8],
                    _signature: &SpdmSignatureStruct|
         -> SpdmResult { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::asym_verify_impl::DEFAULT;

    pub fn register(context: SpdmAsymVerify) -> bool {
        CRYPTO_ASYM_VERIFY.try_get_or_init(|| context).is_ok()
    }

    pub fn verify(
        base_hash_algo: SpdmBaseHashAlgo,
        base_asym_algo: SpdmBaseAsymAlgo,
        public_cert_der: &[u8],
        data: &[u8],
        signature: &SpdmSignatureStruct,
    ) -> SpdmResult {
        (CRYPTO_ASYM_VERIFY
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .verify_cb)(
            base_hash_algo,
            base_asym_algo,
            public_cert_der,
            data,
            signature,
        )
    }
}

pub mod dhe {
    extern crate alloc;
    use alloc::boxed::Box;

    use super::CRYPTO_DHE;
    use crate::crypto::{SpdmDhe, SpdmDheKeyExchange};
    use crate::msgs::{SpdmDheAlgo, SpdmDheExchangeStruct};

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmDhe =
        SpdmDhe {
            generate_key_pair_cb: |_dhe_algo: SpdmDheAlgo| -> Option<(
                SpdmDheExchangeStruct,
                Box<dyn SpdmDheKeyExchange>,
            )> { unimplemented!() },
        };
    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::dhe_impl::DEFAULT;

    pub fn register(context: SpdmDhe) -> bool {
        CRYPTO_DHE.try_init_once(|| context).is_ok()
    }

    pub fn generate_key_pair(
        dhe_algo: SpdmDheAlgo,
    ) -> Option<(SpdmDheExchangeStruct, Box<dyn SpdmDheKeyExchange>)> {
        (CRYPTO_DHE
            .try_get_or_init(|| DEFAULT)
            .ok()?
            .generate_key_pair_cb)(dhe_algo)
    }
}

pub mod cert_operation {
    use super::CRYPTO_CERT_OPERATION;
    use crate::crypto::SpdmCertOperation;
    use crate::error::SpdmResult;

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmCertOperation = SpdmCertOperation {
        get_cert_from_cert_chain_cb: |_cert_chain: &[u8],
                                      _index: isize|
         -> SpdmResult<(usize, usize)> { unimplemented!() },
        verify_cert_chain_cb: |_cert_chain: &[u8]| -> SpdmResult { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::cert_operation_impl::DEFAULT;

    pub fn register(context: SpdmCertOperation) -> bool {
        CRYPTO_CERT_OPERATION.try_init_once(|| context).is_ok()
    }

    pub fn get_cert_from_cert_chain(cert_chain: &[u8], index: isize) -> SpdmResult<(usize, usize)> {
        (CRYPTO_CERT_OPERATION
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .get_cert_from_cert_chain_cb)(cert_chain, index)
    }

    pub fn verify_cert_chain(cert_chain: &[u8]) -> SpdmResult {
        (CRYPTO_CERT_OPERATION
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .verify_cert_chain_cb)(cert_chain)
    }
}

pub mod hkdf {
    use super::CRYPTO_HKDF;
    use crate::crypto::SpdmHkdf;
    use crate::msgs::{SpdmBaseHashAlgo, SpdmDigestStruct};

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmHkdf = SpdmHkdf {
        hkdf_expand_cb: |_hash_algo: SpdmBaseHashAlgo,
                         _pk: &[u8],
                         _info: &[u8],
                         _out_size: u16|
         -> Option<SpdmDigestStruct> { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::hkdf_impl::DEFAULT;

    pub fn register(context: SpdmHkdf) -> bool {
        CRYPTO_HKDF.try_init_once(|| context).is_ok()
    }

    pub fn hkdf_expand(
        hash_algo: SpdmBaseHashAlgo,
        pk: &[u8],
        info: &[u8],
        out_size: u16,
    ) -> Option<SpdmDigestStruct> {
        (CRYPTO_HKDF.try_get_or_init(|| DEFAULT).ok()?.hkdf_expand_cb)(
            hash_algo, pk, info, out_size,
        )
    }
}

pub mod aead {
    use super::CRYPTO_AEAD;
    use crate::crypto::SpdmAead;
    use crate::error::SpdmResult;
    use crate::msgs::SpdmAeadAlgo;

    #[cfg(not(any(feature = "spdm-ring")))]
    static DEFAULT: SpdmAead = SpdmAead {
        encrypt_cb: |_aead_algo: SpdmAeadAlgo,
                     _key: &[u8],
                     _iv: &[u8],
                     _aad: &[u8],
                     _plain_text: &[u8],
                     _tag: &mut [u8],
                     _cipher_text: &mut [u8]|
         -> SpdmResult<(usize, usize)> { unimplemented!() },
        decrypt_cb: |_aead_algo: SpdmAeadAlgo,
                     _key: &[u8],
                     _iv: &[u8],
                     _aad: &[u8],
                     _cipher_text: &[u8],
                     _tag: &[u8],
                     _plain_text: &mut [u8]|
         -> SpdmResult<usize> { unimplemented!() },
    };

    #[cfg(feature = "spdm-ring")]
    use super::spdm_ring::aead_impl::DEFAULT;

    pub fn register(context: SpdmAead) -> bool {
        CRYPTO_AEAD.try_init_once(|| context).is_ok()
    }

    pub fn encrypt(
        aead_algo: SpdmAeadAlgo,
        key: &[u8],
        iv: &[u8],
        aad: &[u8],
        plain_text: &[u8],
        tag: &mut [u8],
        cipher_text: &mut [u8],
    ) -> SpdmResult<(usize, usize)> {
        (CRYPTO_AEAD
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .encrypt_cb)(aead_algo, key, iv, aad, plain_text, tag, cipher_text)
    }

    pub fn decrypt(
        aead_algo: SpdmAeadAlgo,
        key: &[u8],
        iv: &[u8],
        aad: &[u8],
        cipher_text: &[u8],
        tag: &[u8],
        plain_text: &mut [u8],
    ) -> SpdmResult<usize> {
        (CRYPTO_AEAD
            .try_get_or_init(|| DEFAULT)
            .map_err(|_| spdm_err!(EFAULT))?
            .decrypt_cb)(aead_algo, key, iv, aad, cipher_text, tag, plain_text)
    }
}
