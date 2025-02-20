// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![forbid(unsafe_code)]

use crate::responder::*;

use crate::common::ManagedBuffer;

use crate::crypto;

impl<'a> ResponderContext<'a> {
    pub fn handle_spdm_key_exchange(&mut self, bytes: &[u8]) {
        let mut reader = Reader::init(bytes);
        SpdmMessageHeader::read(&mut reader);

        let key_exchange_req =
            SpdmKeyExchangeRequestPayload::spdm_read(&mut self.common, &mut reader);
        if let Some(key_exchange_req) = key_exchange_req {
            debug!("!!! key_exchange req : {:02x?}\n", key_exchange_req);

            if (key_exchange_req.measurement_summary_hash_type
                == SpdmMeasurementSummaryHashType::SpdmMeasurementSummaryHashTypeTcb)
                || (key_exchange_req.measurement_summary_hash_type
                    == SpdmMeasurementSummaryHashType::SpdmMeasurementSummaryHashTypeAll)
            {
                self.common.runtime_info.need_measurement_summary_hash = true;
            } else {
                self.common.runtime_info.need_measurement_summary_hash = false;
            }
        } else {
            error!("!!! key_exchange req : fail !!!\n");
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        info!("send spdm key_exchange rsp\n");

        let (exchange, key_exchange_context) =
            crypto::dhe::generate_key_pair(self.common.negotiate_info.dhe_sel).unwrap();

        debug!("!!! exchange data : {:02x?}\n", exchange);

        debug!(
            "!!! exchange data (peer) : {:02x?}\n",
            &key_exchange_req.unwrap().exchange
        );

        let final_key = key_exchange_context
            .compute_final_key(&key_exchange_req.unwrap().exchange)
            .unwrap();

        debug!("!!! final_key : {:02x?}\n", final_key.as_ref());

        let random = [0xafu8; SPDM_RANDOM_SIZE];

        let rsp_session_id = 0xFFFE;

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
                request_response_code: SpdmResponseResponseCode::SpdmResponseKeyExchangeRsp,
            },
            payload: SpdmMessagePayload::SpdmKeyExchangeResponse(SpdmKeyExchangeResponsePayload {
                heartbeat_period: 0x0,
                rsp_session_id,
                mut_auth_req: SpdmKeyExchangeMutAuthAttributes::empty(),
                req_slot_id: 0x0,
                random: SpdmRandomStruct { data: random },
                exchange,
                measurement_summary_hash: SpdmDigestStruct {
                    data_size: self.common.negotiate_info.base_hash_sel.get_size(),
                    data: [0xaa; SPDM_MAX_HASH_SIZE],
                },
                opaque,
                signature: SpdmSignatureStruct {
                    data_size: self.common.negotiate_info.base_asym_sel.get_size(),
                    data: [0xbb; SPDM_MAX_ASYM_KEY_SIZE],
                },
                verify_data: SpdmDigestStruct {
                    data_size: self.common.negotiate_info.base_hash_sel.get_size(),
                    data: [0xcc; SPDM_MAX_HASH_SIZE],
                },
            }),
        };

        response.spdm_encode(&mut self.common, &mut writer);
        let used = writer.used();

        // generat signature
        let base_asym_size = self.common.negotiate_info.base_asym_sel.get_size() as usize;
        let base_hash_size = self.common.negotiate_info.base_hash_sel.get_size() as usize;

        let mut message_k = ManagedBuffer::default();
        if message_k.append_message(&bytes[..reader.used()]).is_none() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        let temp_used = used - base_asym_size - base_hash_size;
        if message_k
            .append_message(&send_buffer[..temp_used])
            .is_none()
        {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        let signature = self.common.generate_key_exchange_rsp_signature(&message_k);
        if signature.is_err() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }
        let signature = signature.unwrap();
        if message_k.append_message(signature.as_ref()).is_none() {
            self.send_spdm_error(SpdmErrorCode::SpdmErrorInvalidRequest, 0);
            return;
        }

        // create session - generate the handshake secret (including finished_key)
        let th1 = self
            .common
            .calc_rsp_transcript_hash(false, &message_k, None);
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
            ((key_exchange_req.unwrap().req_session_id as u32) << 16) + rsp_session_id as u32;
        session.setup(session_id).unwrap();
        session.set_use_psk(false);
        session.set_crypto_param(hash_algo, dhe_algo, aead_algo, key_schedule_algo);
        session.set_transport_param(sequence_number_count, max_random_count);
        session.set_dhe_secret(&final_key);
        session.generate_handshake_secret(&th1).unwrap();

        // generate HMAC with finished_key
        let transcript_data = self
            .common
            .calc_rsp_transcript_data(false, &message_k, None);
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
        send_buffer[(used - base_hash_size - base_asym_size)..(used - base_hash_size)]
            .copy_from_slice(signature.as_ref());
        send_buffer[(used - base_hash_size)..used].copy_from_slice(hmac.as_ref());

        let _ = self.send_message(&send_buffer[0..used]);
        let session = self.common.get_session_via_id(session_id).unwrap();
        // change state after message is sent.
        session.set_session_state(crate::session::SpdmSessionState::SpdmSessionHandshaking);
    }
}
