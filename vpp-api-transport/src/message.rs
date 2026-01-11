use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use vpp_api_encoding::typ::*;
use vpp_api_message::VppApiMessage;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SockMsgHeader {
    pub _q: u64,
    pub msglen: u32,
    pub gc_mark: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct RawControlPing {
    pub client_index: u32,
    pub context: u32,
}

impl VppApiMessage for RawControlPing {
    fn get_message_name_and_crc() -> String {
        "control_ping_51077d14".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context;
    }

    fn set_client_index(&mut self, client_index: u32) {
        self.client_index = client_index;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct RawControlPingReply {
    pub context: u32,
    pub retval: i32,
    pub client_index: u32,
    pub vpe_pid: u32,
}

impl VppApiMessage for RawControlPingReply {
    fn get_message_name_and_crc() -> String {
        "control_ping_reply_f6b0b8ca".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context;
    }

    fn set_client_index(&mut self, client_index: u32) {
        self.client_index = client_index;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RawCliInband {
    pub client_index: u32,
    pub context: u32,
    pub cmd: VariableSizeString,
}

impl RawCliInband {
    pub fn new(cmd: &str) -> Result<Self> {
        Ok(RawCliInband {
            client_index: 0,
            context: 0,
            cmd: cmd.try_into().map_err(|e| anyhow!("{e}"))?,
        })
    }
}

impl VppApiMessage for RawCliInband {
    fn get_message_name_and_crc() -> String {
        "cli_inband_f8377302".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context;
    }

    fn set_client_index(&mut self, client_index: u32) {
        self.client_index = client_index;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RawCliInbandReply {
    pub context: u32,
    pub retval: i32,
    pub reply: VariableSizeString,
}

impl VppApiMessage for RawCliInbandReply {
    fn get_message_name_and_crc() -> String {
        "cli_inband_reply_05879051".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context;
    }

    fn set_client_index(&mut self, _client_index: u32) {}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct MsgSockClntCreate {
    pub context: u32,
    pub name: FixedSizeString<typenum::U64>,
}

impl TryFrom<&str> for MsgSockClntCreate {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let name_fixed_str = value.try_into().map_err(|e| anyhow!("{e}"))?;
        Ok(MsgSockClntCreate {
            context: 0,
            name: name_fixed_str,
        })
    }
}

impl MsgSockClntCreate {
    pub fn get_message_id() -> u16 {
        15
    }
}

impl VppApiMessage for MsgSockClntCreate {
    fn get_message_name_and_crc() -> String {
        "sockclnt_create_455fb9c4".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context
    }

    fn set_client_index(&mut self, _client_index: u32) {}
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MsgSockClntCreateReplyHdr {
    pub client_index: u32,
    pub context: u32,
    pub response: i32,
    pub index: u32,
    pub count: u16,
    pub message_table: VariableSizeArray<MessageTableEntry>,
}

impl MsgSockClntCreateReplyHdr {
    pub fn get_message_id() -> u16 {
        16
    }
}

impl VppApiMessage for MsgSockClntCreateReplyHdr {
    fn get_message_name_and_crc() -> String {
        "sockclnt_create_reply_35166268".into()
    }

    fn set_context(&mut self, context: u32) {
        self.context = context
    }

    fn set_client_index(&mut self, client_index: u32) {
        self.client_index = client_index;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MessageTableEntry {
    pub index: u16,
    pub name: FixedSizeString<typenum::U64>,
}
