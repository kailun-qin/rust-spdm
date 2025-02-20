// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![forbid(unsafe_code)]

use crate::common;
use crate::config;
use crate::msgs::SpdmCodec;
use crate::msgs::SpdmVersion;
use codec::{Codec, Reader, Writer};

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmGetVersionRequestPayload {}

impl SpdmCodec for SpdmGetVersionRequestPayload {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        0u8.encode(bytes); // param1
        0u8.encode(bytes); // param2
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmGetVersionRequestPayload> {
        u8::read(r)?; // param1
        u8::read(r)?; // param2

        Some(SpdmGetVersionRequestPayload {})
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmVersionStruct {
    pub update: u8,
    pub version: SpdmVersion,
}

impl Codec for SpdmVersionStruct {
    fn encode(&self, bytes: &mut Writer) {
        self.update.encode(bytes);
        self.version.encode(bytes);
    }
    fn read(r: &mut Reader) -> Option<SpdmVersionStruct> {
        let update = u8::read(r)?;
        let version = SpdmVersion::read(r)?;
        Some(SpdmVersionStruct { update, version })
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmVersionResponsePayload {
    pub version_number_entry_count: u8,
    pub versions: [SpdmVersionStruct; config::MAX_SPDM_VERSION_COUNT],
}

impl SpdmCodec for SpdmVersionResponsePayload {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        0u8.encode(bytes); // param1
        0u8.encode(bytes); // param2

        0u8.encode(bytes); // reserved
        self.version_number_entry_count.encode(bytes);

        for version in self
            .versions
            .iter()
            .take(self.version_number_entry_count as usize)
        {
            version.encode(bytes);
        }
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmVersionResponsePayload> {
        u8::read(r)?; // param1
        u8::read(r)?; // param2

        u8::read(r)?; // reserved
        let version_number_entry_count = u8::read(r)?;

        let mut versions = [SpdmVersionStruct {
            update: 0,
            version: SpdmVersion::SpdmVersion10,
        }; config::MAX_SPDM_VERSION_COUNT];
        for version in versions
            .iter_mut()
            .take(version_number_entry_count as usize)
        {
            *version = SpdmVersionStruct::read(r)?;
        }
        Some(SpdmVersionResponsePayload {
            version_number_entry_count,
            versions,
        })
    }
}
