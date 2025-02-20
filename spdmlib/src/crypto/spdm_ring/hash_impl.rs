// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use crate::crypto::SpdmHash;
use crate::msgs::{SpdmBaseHashAlgo, SpdmDigestStruct};

pub static DEFAULT: SpdmHash = SpdmHash {
    hash_all_cb: hash_all,
};

fn hash_all(base_hash_algo: SpdmBaseHashAlgo, data: &[u8]) -> Option<SpdmDigestStruct> {
    let algorithm = match base_hash_algo {
        SpdmBaseHashAlgo::TPM_ALG_SHA_256 => &ring::digest::SHA256,
        SpdmBaseHashAlgo::TPM_ALG_SHA_384 => &ring::digest::SHA384,
        SpdmBaseHashAlgo::TPM_ALG_SHA_512 => &ring::digest::SHA512,
        _ => return None,
    };
    let digest_value = ring::digest::digest(algorithm, data);
    Some(SpdmDigestStruct::from(digest_value.as_ref()))
}
