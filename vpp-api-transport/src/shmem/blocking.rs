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

/// A blocking client for communicating with VPP through shared memory.
///
/// The `Client` manages connections to the VPP API, handles message encoding/decoding,
/// and provides methods for sending and receiving messages.
pub struct Client {
    /// The client index assigned by VPP.
    client_index: u32,
    /// Atomic counter for generating unique context IDs for each request.
    context_id: AtomicU32,
    /// Timeout in seconds for blocking operations.
    timeout_seconds: u16,
    /// Bincode serialization configuration for message encoding.
    config: Configuration<BigEndian, Fixint>,
    /// Function to resolve message names to their VPP API indices.
    resolver: Box<dyn Fn(String) -> Result<u16> + 'static>,
}

impl Client {
    /// Establishes a connection to the VPP API through shared memory.
    ///
    /// # Parameters
    /// * `name` - The name of the client connecting to VPP
    /// * `chroot_prefix` - Optional chroot prefix for the shared memory path
    /// * `rx_qlen` - Receive queue length for the connection
    /// * `timeout_seconds` - Timeout duration in seconds for blocking operations
    ///
    /// # Returns
    /// A Result containing a new Client instance or an error if connection fails
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

    /// Creates a new bincode encoder configuration.
    ///
    /// # Returns
    /// A `Configuration` instance set up with big-endian byte order and fixed integer encoding.
    fn new_encoder() -> Configuration<BigEndian, Fixint> {
        bincode_next::config::legacy()
            .with_big_endian()
            .with_fixed_int_encoding()
    }

    /// Sends a message and receives a reply in a single operation.
    ///
    /// # Type Parameters
    /// * `T` - The message type to send (must implement Serialize and VppApiMessage)
    /// * `R` - The expected reply type (must implement DeserializeOwned and VppApiMessage)
    ///
    /// # Parameters
    /// * `msg` - The message to send to VPP
    ///
    /// # Returns
    /// A Result containing the received reply message or an error if the operation fails
    pub async fn send_rcv<T, R>(&mut self, msg: T) -> Result<R>
    where
        T: Serialize + VppApiMessage,
        R: DeserializeOwned + VppApiMessage,
    {
        self.send(msg).await?;
        self.receive().await
    }

    /// Sends a message to VPP through shared memory.
    ///
    /// # Type Parameters
    /// * `T` - The message type to send (must implement Serialize and VppApiMessage)
    ///
    /// # Parameters
    /// * `msg` - The message to send; will be modified to include client index and context ID
    ///
    /// # Returns
    /// A Result indicating success or an error if the send operation fails
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

    /// Receives a message from VPP through shared memory.
    ///
    /// # Type Parameters
    /// * `R` - The expected message type to receive (must implement DeserializeOwned and VppApiMessage)
    ///
    /// # Returns
    /// A Result containing the received message or an error if the receive operation fails or times out
    async fn receive<R>(&mut self) -> Result<R>
    where
        R: DeserializeOwned + VppApiMessage,
    {
        let timeout_seconds = self.timeout_seconds;
        let msg = tokio::task::spawn_blocking(move || vac_read_wrapper(timeout_seconds)).await??;

        let mut reader = Cursor::new(msg);
        read_object(&mut reader, &self.resolver, self.config, false).await
    }

    /// Sends a control ping message to VPP.
    ///
    /// # Returns
    /// A Result indicating success or an error if the send operation fails
    pub async fn send_control_ping(&mut self) -> Result<()> {
        self.send(RawControlPing::default()).await
    }

    /// Receives a control ping reply from VPP.
    ///
    /// # Returns
    /// A Result containing the return value from the control ping reply or an error if the receive fails
    pub async fn rcv_control_ping_reply(&mut self) -> Result<i32> {
        let reply = self.receive::<RawControlPingReply>().await?;
        Ok(reply.retval)
    }

    /// Executes a CLI command in-band and returns the output.
    ///
    /// # Parameters
    /// * `cmd` - The CLI command string to execute
    ///
    /// # Returns
    /// A Result containing the command output as a String or an error if the operation fails
    pub async fn run_cli_inband(&mut self, cmd: &str) -> Result<String> {
        let in_msg = RawCliInband::new(cmd)?;
        let out_msg: RawCliInbandReply = self.send_rcv(in_msg).await?;
        Ok(out_msg.reply.to_string())
    }

    /// Generates and returns the next context ID for a request.
    ///
    /// # Returns
    /// A unique u32 context ID for the next message request
    fn get_next_context(&mut self) -> u32 {
        self.context_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    /// Resolves a message name to its VPP API index.
    ///
    /// # Parameters
    /// * `name` - The name of the message to resolve
    ///
    /// # Returns
    /// A Result containing the message index (u16) or an error if the message name cannot be resolved
    pub fn get_message_index(&self, name: &String) -> Result<u16> {
        self.resolver.as_ref()(name.to_owned())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        vac_disconnect_wrapper()
    }
}
/// Error handler callback for VAC (VPP API Client) errors.
///
/// # Safety
/// This function is called from C code and must handle raw pointers safely.
/// The `_msg` pointer is valid for `_msg_len` bytes during the call.
///
/// # Parameters
/// * `_arg` - Unused opaque argument passed by VAC
/// * `msg` - Raw pointer to the error message bytes
/// * `msg_len` - Length of the error message in bytes
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn vac_error_handler(_arg: *const c_void, msg: *const u8, msg_len: i32) {
    let msg = unsafe { std::slice::from_raw_parts(msg, msg_len as usize) };
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
