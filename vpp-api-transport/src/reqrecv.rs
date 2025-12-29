#![allow(
    dead_code,
    unused_mut,
    unused_variables,
    unused_must_use,
    non_camel_case_types,
    unused_imports
)]
use super::error::Result;
use crate::VppApiTransport;
use bincode_next::config::BigEndian;
use bincode_next::config::Fixint;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Read, Write};
use std::ops::Add;
use std::time::{Duration, SystemTime};
use vpp_api_message::VppApiMessage;

fn get_encoder() -> bincode_next::config::Configuration<BigEndian, Fixint> {
    bincode_next::config::legacy()
        .with_big_endian()
        .with_fixed_int_encoding()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPing {
    pub client_index: u32,
    pub context: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPingReply {
    pub context: u32,
    pub retval: i32,
    pub client_index: u32,
    pub vpe_pid: u32,
}

pub fn send_recv_one<
    'a,
    T: Serialize + Deserialize<'a> + VppApiMessage,
    TR: Serialize + DeserializeOwned + VppApiMessage,
>(
    m: &T,
    t: &mut dyn VppApiTransport,
) -> Result<TR> {
    let name = &T::get_message_name_and_crc();
    let reply_name = &TR::get_message_name_and_crc();
    let vl_msg_id = t.get_msg_index(name).unwrap();
    let reply_vl_msg_id = t.get_msg_index(reply_name).unwrap();

    let mut v = bincode_next::serde::encode_to_vec(vl_msg_id, get_encoder()).unwrap();
    let msg: Vec<u8> = bincode_next::serde::encode_to_vec(m, get_encoder())?;

    trace!(
        "About to send msg: {} id: {} reply_id: {} msg:{:x?}",
        name, &vl_msg_id, &reply_vl_msg_id, &msg,
    );

    v.extend_from_slice(&msg);
    match t.write(&v) {
        Ok(i) => {
            if i < v.len() {
                return Err(format!("Short write.  wrote {}, of {} bytes", &i, v.len()).into());
            } else {
                trace!("Wrote {} bytes to socket", &i);
            }
        }
        Err(e) => {
            error!("error writing message for {}  {}", name, e);
            return Err(e.into());
        }
    }
    loop {
        trace!("msg: {} waiting for reply", name);
        match t.read_one_msg_id_and_msg() {
            Ok((msg_id, data)) => {
                trace!("msg: {} id: {} data: {:x?}", name, msg_id, &data);
                if msg_id == reply_vl_msg_id {
                    let (res, _) = bincode_next::serde::decode_from_slice(&data, get_encoder())?;
                    return Ok(res);
                }
            }
            Err(e) => {
                error!("error from vpp: {:?}", &e);
                return Err(e);
            }
        }
    }
}
pub fn send_recv_many<
    'a,
    T: Serialize + Deserialize<'a> + VppApiMessage,
    TR: Serialize + DeserializeOwned + VppApiMessage + std::fmt::Debug + Clone,
>(
    m: &T,
    t: &mut dyn VppApiTransport,
) -> Result<Vec<TR>> {
    let name = &T::get_message_name_and_crc();
    let reply_name = &TR::get_message_name_and_crc();
    let control_ping_id = t.get_msg_index("control_ping_51077d14").unwrap();
    let control_ping_id_reply = t.get_msg_index("control_ping_reply_f6b0b8ca").unwrap();
    let vl_msg_id = t.get_msg_index(name).unwrap();
    let reply_vl_msg_id = t.get_msg_index(reply_name).unwrap();

    let mut v = bincode_next::serde::encode_to_vec(vl_msg_id, get_encoder())?;
    let msg = bincode_next::serde::encode_to_vec(m, get_encoder())?;

    let control_ping = ControlPing {
        client_index: t.get_client_index(),
        context: 0,
    };

    let mut c = bincode_next::serde::encode_to_vec(control_ping_id, get_encoder())?;
    let control_ping_message = bincode_next::serde::encode_to_vec(control_ping, get_encoder())?;

    c.extend_from_slice(&control_ping_message);
    v.extend_from_slice(&msg);
    let mut out: Vec<u8> = vec![];
    t.write_all(&v)?; // Dump message
    t.write_all(&c)?; // Ping message
    // dbg!(control_ping_id_reply);
    let mut out: Vec<TR> = vec![];
    let mut count = 0;
    loop {
        trace!("Reached loop");
        match t.read_one_msg_id_and_msg() {
            Ok((msg_id, data)) => {
                trace!(
                    "msg: {} id: {} ctrl_id: {} reply_id: {} data: {:x?}",
                    name, msg_id, &control_ping_id_reply, &reply_vl_msg_id, &data
                );
                trace!("data.len: {}", data.len());
                if msg_id == control_ping_id_reply {
                    trace!("finished. returning {:?}", out);
                    return Ok(out);
                }
                if msg_id == reply_vl_msg_id {
                    trace!("Received the intended message; attempt to deserialize");
                    let (res, _) = bincode_next::serde::decode_from_slice(&data, get_encoder())?;
                    trace!("Next thing will be the reply");
                    out.extend_from_slice(&[res]);
                } else {
                    trace!("Checking the next message for the reply id");
                }
            }
            Err(e) => {
                error!("error from vpp: {:?}", &e);
                return Err(e);
            }
        }
    }
}

pub fn send_recv_msg<'a, T: Serialize + Deserialize<'a>, TR: Serialize + DeserializeOwned>(
    name: &str,
    m: &T,
    t: &mut dyn VppApiTransport,
    reply_name: &str,
) -> TR {
    let vl_msg_id = t.get_msg_index(name).unwrap();
    let reply_vl_msg_id = t.get_msg_index(reply_name).unwrap();

    let mut v = bincode_next::serde::encode_to_vec(vl_msg_id, get_encoder()).unwrap();
    let msg = bincode_next::serde::encode_to_vec(m, get_encoder()).unwrap();

    trace!(
        "About to send msg: {} id: {} reply_id: {} msg:{:x?}",
        name, &vl_msg_id, &reply_vl_msg_id, &msg,
    );

    v.extend_from_slice(&msg);
    match t.write(&v) {
        Ok(i) => {
            if i < v.len() {
                panic!("Short write.  wrote {}, of {} bytes", &i, v.len());
            } else {
                trace!("Wrote {} bytes to socket", &i);
            }
        }
        Err(e) => {
            panic!("error writing message for {}  {}", name, e);
        }
    }
    loop {
        trace!("msg: {} waiting for reply", name);
        let res = t.read_one_msg_id_and_msg();
        // dbg!(&res);
        if let Ok((msg_id, data)) = res {
            trace!("msg: {} id: {} data: {:x?}", name, msg_id, &data);
            if msg_id == reply_vl_msg_id {
                let (res, _) =
                    bincode_next::serde::decode_from_slice(&data, get_encoder()).unwrap();
                return res;
            }
        } else {
            panic!("Result is an error: {:?}", &res);
        }
    }
}
pub fn send_bulk_msg<
    'a,
    T: Serialize + Deserialize<'a>,
    TR: Serialize + DeserializeOwned + std::fmt::Debug + Clone,
>(
    name: &str,
    m: &T,
    t: &mut dyn VppApiTransport,
    reply_name: &str,
) -> Vec<TR> {
    let control_ping_id = t.get_msg_index("control_ping_51077d14").unwrap();
    let control_ping_id_reply = t.get_msg_index("control_ping_reply_f6b0b8ca").unwrap();
    let vl_msg_id = t.get_msg_index(name).unwrap();
    let reply_vl_msg_id = t.get_msg_index(reply_name).unwrap();

    let mut v = bincode_next::serde::encode_to_vec(vl_msg_id, get_encoder()).unwrap();
    let msg = bincode_next::serde::encode_to_vec(m, get_encoder()).unwrap();

    let control_ping = ControlPing {
        client_index: t.get_client_index(),
        context: 0,
    };

    let mut c = bincode_next::serde::encode_to_vec(control_ping_id, get_encoder()).unwrap();
    let mut control_ping_message =
        bincode_next::serde::encode_to_vec(control_ping, get_encoder()).unwrap();

    c.extend_from_slice(&control_ping_message);
    v.extend_from_slice(&msg);
    let mut out: Vec<u8> = vec![];
    t.write_all(&v).unwrap(); // Dump message
    t.write_all(&c).unwrap(); // Ping message
    // dbg!(control_ping_id_reply);
    let mut out: Vec<TR> = vec![];
    let mut count = 0;
    loop {
        trace!("Reached loop");
        let res = t.read_one_msg_id_and_msg();
        if let Ok((msg_id, data)) = res {
            trace!(
                "msg: {} id: {} ctrl_id: {} reply_id: {} data: {:x?}",
                name, msg_id, &control_ping_id_reply, &reply_vl_msg_id, &data
            );
            trace!("data.len: {}", data.len());
            if msg_id == control_ping_id_reply {
                trace!("finished. returning {:?}", out);
                return out;
            }
            if msg_id == reply_vl_msg_id {
                trace!("Received the intended message; attempt to deserialize");
                let (res, _) =
                    bincode_next::serde::decode_from_slice(&data, get_encoder()).unwrap();

                trace!("Next thing will be the reply");
                out.extend_from_slice(&[res]);
            } else {
                trace!("Checking the next message for the reply id");
            }
        } else {
            panic!("Result is an error: {:?}", &res);
        }
    }
}
