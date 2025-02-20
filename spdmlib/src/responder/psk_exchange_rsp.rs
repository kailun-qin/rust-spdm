// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![forbid(unsafe_code)]

use config::MAX_SPDM_PSK_CONTEXT_SIZE;

use crate::responder::*;

use crate::common::ManagedBuffer;

impl<'a> ResponderContext<'a> {
    pub fn handle_spdm_psk_exchange(&mut self, bytes: &[u8]) {
        let mut reader = Reader::init(bytes);
        SpdmMessageHeader::read(&mut reader);

        let psk_exchange_req =
            SpdmPskExchangeRequestPayload::spdm_read(&mut self.common, &mut reader);
        if let Some(psk_exchange_req) = psk_exchange_req {
            debug!("!!! psk_exchange req : {:02x?}\n", psk_exchange_req);

            if (psk_exchange_req.measurement_summary_hash_type
                == SpdmMeasurementSummaryHashType::SpdmMeasurementSummaryHashTypeTcb)
                || (psk_exchange_req.measurement_summary_hash_type
                    == SpdmMeasurementSummaryHashType::SpdmMeasurementSummaryHashTypeAll)
            {
                self.common.runtime_info.need_measurement_summary_hash = true;
            } else {
                self.common.runtime_info.need_measurement_summary_hash = false;
            }
        } else {
            error!("!!! psk_exchange req : fail !!!\n");
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        info!("send spdm psk_exchange rsp\n");

        let psk_context = [0xbb; MAX_SPDM_PSK_CONTEXT_SIZE];

        let rsp_session_id = 0xFFFD;

        let mut send_buffer = [0u8; config::MAX_SPDM_TRANSPORT_SIZE];
        let mut writer = Writer::init(&mut send_buffer);
        let mut opaque = SpdmOpaqueStruct {
            data_size: crate::common::OPAQUE_DATA_VERSION_SELECTION.len() as u16,
            ..Default::default()
        };
        opaque.data[..(opaque.data_size as usize)]
            .copy_from_slice(crate::common::OPAQUE_DATA_VERSION_SELECTION.as_ref());
        let response = SpdmMessage {
            header: SpdmMessageHeader {
                version: SpdmVersion::SpdmVersion11,
                request_response_code: SpdmResponseResponseCode::SpdmResponsePskExchangeRsp,
            },
            payload: SpdmMessagePayload::SpdmPskExchangeResponse(SpdmPskExchangeResponsePayload {
                heartbeat_period: 0x0,
                rsp_session_id,
                measurement_summary_hash: SpdmDigestStruct {
                    data_size: self.common.negotiate_info.base_hash_sel.get_size(),
                    data: [0xaa; SPDM_MAX_HASH_SIZE],
                },
                psk_context: SpdmPskContextStruct {
                    data_size: self.common.negotiate_info.base_hash_sel.get_size(),
                    data: psk_context,
                },
                opaque,
                verify_data: SpdmDigestStruct {
                    data_size: self.common.negotiate_info.base_hash_sel.get_size(),
                    data: [0xcc; SPDM_MAX_HASH_SIZE],
                },
            }),
        };

        response.spdm_encode(&mut self.common, &mut writer);
        let used = writer.used();

        let base_hash_size = self.common.negotiate_info.base_hash_sel.get_size() as usize;

        let mut message_k = ManagedBuffer::default();
        if message_k.append_message(&bytes[..reader.used()]).is_none() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        let temp_used = used - base_hash_size;
        if message_k
            .append_message(&send_buffer[..temp_used])
            .is_none()
        {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        // create session - generate the handshake secret (including finished_key)
        let th1 = self.common.calc_rsp_transcript_hash(true, &message_k, None);
        if th1.is_err() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }
        let th1 = th1.unwrap();
        debug!("!!! th1 : {:02x?}\n", th1.as_ref());
        let hash_algo = self.common.negotiate_info.base_hash_sel;
        let dhe_algo = self.common.negotiate_info.dhe_sel;
        let aead_algo = self.common.negotiate_info.aead_sel;
        let key_schedule_algo = self.common.negotiate_info.key_schedule_sel;
        let sequence_number_count = self.common.transport_encap.get_sequence_number_count();
        let max_random_count = self.common.transport_encap.get_max_random_count();

        let session = self.common.get_next_avaiable_session();
        if session.is_none() {
            error!("!!! too many sessions : fail !!!\n");
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        let session = session.unwrap();
        let session_id =
            ((psk_exchange_req.unwrap().req_session_id as u32) << 16) + rsp_session_id as u32;
        session.setup(session_id).unwrap();
        session.set_use_psk(true);
        let mut psk_key = SpdmDheFinalKeyStruct {
            data_size: b"TestPskData\0".len() as u16,
            ..Default::default()
        };
        psk_key.data[0..(psk_key.data_size as usize)].copy_from_slice(b"TestPskData\0");
        session.set_crypto_param(hash_algo, dhe_algo, aead_algo, key_schedule_algo);
        session.set_transport_param(sequence_number_count, max_random_count);
        session.set_dhe_secret(&psk_key); // TBD
        session.generate_handshake_secret(&th1).unwrap();

        // generate HMAC with finished_key
        let transcript_data = self.common.calc_rsp_transcript_data(true, &message_k, None);
        if transcript_data.is_err() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }
        let transcript_data = transcript_data.unwrap();

        let session = self.common.get_session_via_id(session_id).unwrap();
        let hmac = session.generate_hmac_with_response_finished_key(transcript_data.as_ref());
        if hmac.is_err() {
            let _ = session.teardown(session_id);
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }
        let hmac = hmac.unwrap();
        if message_k.append_message(hmac.as_ref()).is_none() {
            let _ = session.teardown(session_id);
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }
        session.runtime_info.message_k = message_k;

        // patch the message before send
        send_buffer[(used - base_hash_size)..used].copy_from_slice(hmac.as_ref());

        let _ = self.send_message(&send_buffer[0..used]);
        let session = self.common.get_session_via_id(session_id).unwrap();
        // change state after message is sent.
        session.set_session_state(crate::session::SpdmSessionState::SpdmSessionHandshaking);
    }
}
