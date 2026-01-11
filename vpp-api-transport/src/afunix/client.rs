use crate::helpers::*;
use crate::message::*;
use anyhow::{Result, anyhow};
use bincode_next::config::BigEndian;
use bincode_next::config::Configuration;
use bincode_next::config::Fixint;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::AtomicU32;
use tokio::net::UnixStream;
use vpp_api_message::VppApiMessage;

/// A client for communicating with the VPP API over a Unix socket.
///
/// This client establishes a connection to the VPP API server using a Unix domain socket,
/// handles message serialization/deserialization, and provides methods for sending and
/// receiving API messages.
pub struct Client {
    /// The underlying Unix socket stream for communication with the VPP API server.
    stream: UnixStream,
    /// The unique client index assigned by the VPP API server.
    client_index: u32,
    /// An atomic counter for generating unique context IDs for API requests.
    context_id: AtomicU32,
    /// A mapping of message names to their corresponding message IDs.
    // message_name_to_id: HashMap<String, u16>,
    /// The bincode configuration used for serializing/deserializing messages.
    config: Configuration<BigEndian, Fixint>,

    resolver: Box<dyn Fn(String) -> Result<u16> + 'static>,
}

impl Client {
    /// Establishes a connection to the VPP API server at the specified socket path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Unix socket (e.g., "/run/vpp/api.sock")
    /// * `name` - The name to register with the VPP API server
    ///
    /// # Returns
    ///
    /// A new `Client` instance if the connection is successful, or an error otherwise.
    pub async fn connect(path: &str, name: &str) -> Result<Client> {
        let config = Client::new_encoder();
        let mut stream = UnixStream::connect(path).await?;
        let mut message_name_to_id = HashMap::new();

        message_name_to_id.insert(
            MsgSockClntCreate::get_message_name_and_crc(),
            MsgSockClntCreate::get_message_id(),
        );
        message_name_to_id.insert(
            MsgSockClntCreateReplyHdr::get_message_name_and_crc(),
            MsgSockClntCreateReplyHdr::get_message_id(),
        );

        let resolve_id = |name| {
            message_name_to_id
                .get(&name)
                .cloned()
                .ok_or(anyhow!("Cannot find message id for {}", name))
        };

        let create_msg: MsgSockClntCreate = name.try_into()?;
        write_object(&mut stream, &create_msg, &resolve_id, config, true).await?;

        let reply: MsgSockClntCreateReplyHdr =
            read_object(&mut stream, &resolve_id, config, true).await?;

        for msg_entry in reply.message_table.0.iter() {
            let msg_name: String = msg_entry.name.to_string();
            message_name_to_id.insert(msg_name, msg_entry.index);
        }
        let client_index = reply.index;

        let resolve_id = move |name| {
            message_name_to_id
                .get(&name)
                .cloned()
                .ok_or(anyhow!("Cannot find message id for {}", name))
        };

        Ok(Client {
            context_id: AtomicU32::new(1),
            client_index,
            stream,
            config,
            resolver: Box::new(resolve_id),
        })
    }

    /// Creates a new bincode configuration for message encoding/decoding.
    fn new_encoder() -> Configuration<BigEndian, Fixint> {
        bincode_next::config::legacy()
            .with_big_endian()
            .with_fixed_int_encoding()
    }

    /// Sends a message and waits for a response.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send
    ///
    /// # Returns
    ///
    /// The response message or an error.
    pub async fn send_rcv<T, R>(&mut self, msg: T) -> Result<R>
    where
        T: Serialize + VppApiMessage,
        R: DeserializeOwned + VppApiMessage,
    {
        self.send(msg).await?;
        self.receive().await
    }

    /// Sends a message to the VPP API server.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error otherwise.
    pub async fn send<T>(&mut self, mut msg: T) -> Result<()>
    where
        T: Serialize + VppApiMessage,
    {
        msg.set_client_index(self.client_index);
        msg.set_context(self.get_next_context());

        write_object(&mut self.stream, &msg, &self.resolver, self.config, true).await
    }

    /// Receives a message from the VPP API server.
    ///
    /// # Returns
    ///
    /// The received message or an error.
    pub async fn receive<R>(&mut self) -> Result<R>
    where
        R: DeserializeOwned + VppApiMessage,
    {
        read_object(&mut self.stream, &self.resolver, self.config, true).await
    }

    /// Sends a control ping message to the VPP API server.
    pub async fn send_control_ping(&mut self) -> Result<()> {
        self.send(RawControlPing::default()).await
    }

    /// Receives a control ping reply from the VPP API server.
    ///
    /// # Returns
    ///
    /// The return value from the control ping reply.
    pub async fn rcv_control_ping_reply(&mut self) -> Result<i32> {
        let reply = self.receive::<RawControlPingReply>().await?;
        Ok(reply.retval)
    }

    /// Executes a VPP CLI command and returns the output.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The CLI command to execute
    ///
    /// # Returns
    ///
    /// The CLI command output or an error.
    pub async fn run_cli_inband(&mut self, cmd: &str) -> Result<String> {
        let in_msg = RawCliInband::new(cmd)?;
        let out_msg: RawCliInbandReply = self.send_rcv(in_msg).await?;
        Ok(out_msg.reply.to_string())
    }

    /// Returns the client index assigned by the VPP API server.
    pub fn get_client_index(&self) -> u32 {
        self.client_index
    }

    /// Generates the next unique context ID for an API request.
    fn get_next_context(&mut self) -> u32 {
        self.context_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn get_message_index(&self, name: &String) -> Result<u16> {
        self.resolver.as_ref()(name.to_owned())
    }

    /// Disconnects from the VPP API server.
    pub fn disconnect(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_afunix_connect() {
        let mut client = Client::connect("/run/vpp/api.sock", "socket-client")
            .await
            .unwrap();

        client.send_control_ping().await.unwrap();
        let res = client.rcv_control_ping_reply().await.unwrap();
        assert_eq!(res, 0);

        let s = client.run_cli_inband("show version").await.unwrap();
        assert!(s.starts_with("vpp "));
        println!("\n {s}");

        client.disconnect();
    }
}
