// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![forbid(unsafe_code)]

use crate::common::{self, SpdmDeviceIo, SpdmTransportEncap};
use crate::config;
use crate::error::SpdmResult;
use crate::msgs::*;

pub struct RequesterContext<'a> {
    pub common: common::SpdmContext<'a>,
}

impl<'a> RequesterContext<'a> {
    pub fn new(
        device_io: &'a mut dyn SpdmDeviceIo,
        transport_encap: &'a mut dyn SpdmTransportEncap,
        config_info: common::SpdmConfigInfo,
        provision_info: common::SpdmProvisionInfo,
    ) -> Self {
        RequesterContext {
            common: common::SpdmContext::new(
                device_io,
                transport_encap,
                config_info,
                provision_info,
            ),
        }
    }

    pub fn init_connection(&mut self) -> SpdmResult {
        let result = self.send_receive_spdm_version();
        if result.is_err() {
            return result;
        }
        let result = self.send_receive_spdm_capability();
        if result.is_err() {
            return result;
        }
        self.send_receive_spdm_algorithm()
    }

    pub fn start_session(
        &mut self,
        use_psk: bool,
        slot_id: u8,
        measurement_summary_hash_type: SpdmMeasurementSummaryHashType,
    ) -> SpdmResult<u32> {
        if !use_psk {
            let result =
                self.send_receive_spdm_key_exchange(slot_id, measurement_summary_hash_type);
            if let Ok(session_id) = result {
                let result = self.send_receive_spdm_finish(session_id);
                if result.is_ok() {
                    Ok(session_id)
                } else {
                    spdm_result_err!(EIO)
                }
            } else {
                spdm_result_err!(EIO)
            }
        } else {
            let result = self.send_receive_spdm_psk_exchange(measurement_summary_hash_type);
            if let Ok(session_id) = result {
                let result = self.send_receive_spdm_psk_finish(session_id);
                if result.is_ok() {
                    Ok(session_id)
                } else {
                    spdm_result_err!(EIO)
                }
            } else {
                spdm_result_err!(EIO)
            }
        }
    }

    pub fn end_session(&mut self, session_id: u32) -> SpdmResult {
        let _result = self.send_receive_spdm_end_session(session_id);
        Ok(())
    }

    pub fn send_message(&mut self, send_buffer: &[u8]) -> SpdmResult {
        let mut transport_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];
        let used =
            self.common
                .transport_encap
                .encap(&send_buffer[..], &mut transport_buffer, false)?;
        self.common.device_io.send(&transport_buffer[..used])
    }

    pub fn send_secured_message(&mut self, session_id: u32, send_buffer: &[u8]) -> SpdmResult {
        let mut app_buffer = [0u8; config::MAX_SPDM_MESSAGE_BUFFER_SIZE];
        let used = self
            .common
            .transport_encap
            .encap_app(send_buffer, &mut app_buffer)?;

        let spdm_session = self
            .common
            .get_session_via_id(session_id)
            .ok_or(spdm_err!(EINVAL))?;

        let mut encoded_send_buffer = [0u8; config::MAX_SPDM_MESSAGE_BUFFER_SIZE];
        let encode_size = spdm_session.encode_spdm_secured_message(
            &app_buffer[0..used],
            &mut encoded_send_buffer,
            true,
        )?;

        let mut transport_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];
        let used = self.common.transport_encap.encap(
            &encoded_send_buffer[..encode_size],
            &mut transport_buffer,
            true,
        )?;
        self.common.device_io.send(&transport_buffer[..used])
    }

    pub fn receive_message(&mut self, receive_buffer: &mut [u8]) -> SpdmResult<usize> {
        info!("receive_message!\n");

        let mut transport_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];
        let used = self
            .common
            .device_io
            .receive(&mut transport_buffer)
            .map_err(|_| spdm_err!(EIO))?;
        let (used, secured_message) = self
            .common
            .transport_encap
            .decap(&transport_buffer[..used], receive_buffer)?;

        if secured_message {
            return spdm_result_err!(EFAULT);
        }

        Ok(used)
    }

    pub fn receive_secured_message(
        &mut self,
        session_id: u32,
        receive_buffer: &mut [u8],
    ) -> SpdmResult<usize> {
        info!("receive_secured_message!\n");

        let mut transport_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];
        let mut encoded_receive_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];

        let used = self
            .common
            .device_io
            .receive(&mut transport_buffer)
            .map_err(|_| spdm_err!(EIO))?;
        let (used, secured_message) = self
            .common
            .transport_encap
            .decap(&transport_buffer[..used], &mut encoded_receive_buffer)?;

        if !secured_message {
            return spdm_result_err!(EFAULT);
        }

        let spdm_session = self
            .common
            .get_session_via_id(session_id)
            .ok_or(spdm_err!(EINVAL))?;

        let mut app_buffer = [0u8; config::MAX_SPDM_MESSAGE_BUFFER_SIZE];
        let decode_size = spdm_session.decode_spdm_secured_message(
            &encoded_receive_buffer[..used],
            &mut app_buffer,
            false,
        )?;

        let used = self
            .common
            .transport_encap
            .decap_app(&app_buffer[0..decode_size], receive_buffer)?;

        Ok(used)
    }
}
