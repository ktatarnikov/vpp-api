use crate::helpers::*;
use crate::message::*;
use crate::shmem::vac::*;
use anyhow::{Result, anyhow};
use bincode_next::config::BigEndian;
use bincode_next::config::Configuration;
use bincode_next::config::Fixint;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Cursor;
use std::os::raw::c_void;
use std::sync::atomic::AtomicU32;
use vpp_api_message::VppApiMessage;

pub struct Client {
    client_index: u32,
    context_id: AtomicU32,
    timeout_seconds: u16,
    config: Configuration<BigEndian, Fixint>,
    resolver: Box<dyn Fn(String) -> Result<u16> + 'static>,
}

impl Client {
    pub async fn connect(
        name: &str,
        chroot_prefix: Option<String>,
        rx_qlen: i32,
        timeout_seconds: u16,
    ) -> Result<Client> {
        let name = name.into();
        tokio::task::spawn_blocking(move || {
            vac_mem_init_wrapper();
            vac_set_error_handler_wrapper(Some(vac_error_handler));
            vac_connect_wrapper(name, chroot_prefix, None, rx_qlen)?;
            Ok::<(), anyhow::Error>(())
        })
        .await??;

        let resolve_id = |name: String| {
            vac_get_msg_index_wrapper(name.to_owned())
                .ok_or(anyhow!("Cannot find message id for {}", name))
        };

        let config = Client::new_encoder();
        Ok(Client {
            context_id: AtomicU32::new(1),
            client_index: 0,
            config,
            timeout_seconds,
            resolver: Box::new(resolve_id),
        })
    }

    fn new_encoder() -> Configuration<BigEndian, Fixint> {
        bincode_next::config::legacy()
            .with_big_endian()
            .with_fixed_int_encoding()
    }

    pub async fn send_rcv<T, R>(&mut self, msg: T) -> Result<R>
    where
        T: Serialize + VppApiMessage,
        R: DeserializeOwned + VppApiMessage,
    {
        self.send(msg).await?;
        self.receive().await
    }

    async fn send<T>(&mut self, mut msg: T) -> Result<()>
    where
        T: Serialize + VppApiMessage,
    {
        msg.set_client_index(self.client_index);
        msg.set_context(self.get_next_context());
        let mut writer: Vec<u8> = Vec::new();
        write_object(&mut writer, &msg, &self.resolver, self.config, false).await?;
        tokio::task::spawn_blocking(|| vac_write_wrapper(writer)).await?
    }

    async fn receive<R>(&mut self) -> Result<R>
    where
        R: DeserializeOwned + VppApiMessage,
    {
        let timeout_seconds = self.timeout_seconds;
        let msg = tokio::task::spawn_blocking(move || vac_read_wrapper(timeout_seconds)).await??;

        let mut reader = Cursor::new(msg);
        read_object(&mut reader, &self.resolver, self.config, false).await
    }

    pub async fn send_control_ping(&mut self) -> Result<()> {
        self.send(RawControlPing::default()).await
    }

    pub async fn rcv_control_ping_reply(&mut self) -> Result<i32> {
        let reply = self.receive::<RawControlPingReply>().await?;
        Ok(reply.retval)
    }

    pub async fn run_cli_inband(&mut self, cmd: &str) -> Result<String> {
        let in_msg = RawCliInband::new(cmd)?;
        let out_msg: RawCliInbandReply = self.send_rcv(in_msg).await?;
        Ok(out_msg.reply.to_string())
    }

    fn get_next_context(&mut self) -> u32 {
        self.context_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn get_message_index(&self, name: &String) -> Result<u16> {
        self.resolver.as_ref()(name.to_owned())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        vac_disconnect_wrapper()
    }
}
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn vac_error_handler(_arg: *const c_void, _msg: *const u8, _msg_len: i32) {
    let msg = unsafe { std::slice::from_raw_parts(_msg, _msg_len as usize) };
    let msg_string = String::from_utf8_lossy(msg);
    println!("Error: {} ", msg_string);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_connect() {
        let mut client = Client::connect("test_blocking_client", None, 32, 2)
            .await
            .unwrap();

        client.send_control_ping().await.unwrap();
        let res = client.rcv_control_ping_reply().await.unwrap();
        assert_eq!(res, 0);

        let s = client.run_cli_inband("show version").await.unwrap();
        assert!(s.starts_with("vpp "));
        println!("\n {s}");
    }
}
