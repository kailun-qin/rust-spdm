// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![forbid(unsafe_code)]

use crate::common;
use crate::msgs::SpdmCodec;
use codec::enum_builder;
use codec::{Codec, Reader, Writer};

enum_builder! {
    @U8
    EnumName: SpdmErrorCode;
    EnumVal{
        SpdmErrorInvalidRequest => 0x1,
        SpdmErrorInvalidSession => 0x2,
        SpdmErrorBusy => 0x3,
        SpdmErrorUnexpectedRequest => 0x4,
        SpdmErrorUnspecified => 0x5,
        SpdmErrorDecryptError => 0x6,
        SpdmErrorUnsupportedRequest => 0x7,
        SpdmErrorRequestInFlight => 0x8,
        SpdmErrorInvalidResponseCode => 0x9,
        SpdmErrorSessionLimitExceeded => 0xA,
        SpdmErrorMajorVersionMismatch => 0x41,
        SpdmErrorResponseNotReady => 0x42,
        SpdmErrorRequestResynch => 0x43,
        SpdmErrorVendorDefined => 0xFF
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmErrorResponseNoneExtData {}

impl SpdmCodec for SpdmErrorResponseNoneExtData {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, _bytes: &mut Writer) {}

    fn spdm_read(
        _context: &mut common::SpdmContext,
        _r: &mut Reader,
    ) -> Option<SpdmErrorResponseNoneExtData> {
        Some(SpdmErrorResponseNoneExtData {})
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmErrorResponseNotReadyExtData {
    pub rdt_exponent: u8,
    pub request_code: u8,
    pub token: u8,
    pub tdtm: u8,
}

impl SpdmCodec for SpdmErrorResponseNotReadyExtData {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        self.rdt_exponent.encode(bytes);
        self.request_code.encode(bytes);
        self.token.encode(bytes);
        self.tdtm.encode(bytes);
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmErrorResponseNotReadyExtData> {
        let rdt_exponent = u8::read(r)?;
        let request_code = u8::read(r)?;
        let token = u8::read(r)?;
        let tdtm = u8::read(r)?;

        Some(SpdmErrorResponseNotReadyExtData {
            rdt_exponent,
            request_code,
            token,
            tdtm,
        })
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmErrorResponseVendorExtData {
    pub data_size: u8,
    pub data: [u8; 32],
}

impl SpdmCodec for SpdmErrorResponseVendorExtData {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        for d in self.data.iter().take(self.data_size as usize) {
            d.encode(bytes);
        }
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmErrorResponseVendorExtData> {
        let mut data_size = 0;
        let mut data = [0u8; 32];

        for d in &mut data {
            let result = u8::read(r);
            match result {
                Some(v) => {
                    *d = v;
                    data_size += 1;
                }
                None => {
                    break;
                }
            }
        }

        Some(SpdmErrorResponseVendorExtData { data_size, data })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SpdmErrorResponseExtData {
    SpdmErrorExtDataNone(SpdmErrorResponseNoneExtData),
    SpdmErrorExtDataNotReady(SpdmErrorResponseNotReadyExtData),
    SpdmErrorExtDataVendorDefined(SpdmErrorResponseVendorExtData),
}
impl Default for SpdmErrorResponseExtData {
    fn default() -> SpdmErrorResponseExtData {
        SpdmErrorResponseExtData::SpdmErrorExtDataNone(SpdmErrorResponseNoneExtData {})
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SpdmErrorResponsePayload {
    pub error_code: SpdmErrorCode,
    pub error_data: u8,
    pub extended_data: SpdmErrorResponseExtData,
}

impl SpdmCodec for SpdmErrorResponsePayload {
    fn spdm_encode(&self, context: &mut common::SpdmContext, bytes: &mut Writer) {
        self.error_code.encode(bytes); // param1
        self.error_data.encode(bytes); // param2

        match &self.extended_data {
            SpdmErrorResponseExtData::SpdmErrorExtDataNotReady(extended_data) => {
                extended_data.spdm_encode(context, bytes);
            }
            SpdmErrorResponseExtData::SpdmErrorExtDataVendorDefined(extended_data) => {
                extended_data.spdm_encode(context, bytes);
            }
            SpdmErrorResponseExtData::SpdmErrorExtDataNone(extended_data) => {
                extended_data.spdm_encode(context, bytes);
            }
        }
    }

    fn spdm_read(
        context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmErrorResponsePayload> {
        let error_code = SpdmErrorCode::read(r)?; // param1
        let error_data = u8::read(r)?; // param2

        let extended_data = match error_code {
            SpdmErrorCode::SpdmErrorResponseNotReady => {
                Some(SpdmErrorResponseExtData::SpdmErrorExtDataNotReady(
                    SpdmErrorResponseNotReadyExtData::spdm_read(context, r)?,
                ))
            }
            SpdmErrorCode::SpdmErrorVendorDefined => {
                Some(SpdmErrorResponseExtData::SpdmErrorExtDataVendorDefined(
                    SpdmErrorResponseVendorExtData::spdm_read(context, r)?,
                ))
            }
            _ => Some(SpdmErrorResponseExtData::SpdmErrorExtDataNone(
                SpdmErrorResponseNoneExtData::spdm_read(context, r)?,
            )),
        };

        let extended_data = extended_data?;

        Some(SpdmErrorResponsePayload {
            error_code,
            error_data,
            extended_data,
        })
    }
}
