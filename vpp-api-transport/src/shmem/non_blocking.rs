use crate::helpers::*;
use crate::message::*;
use crate::shmem::shmem_bindgen::*;
use crate::shmem::vac::*;
use anyhow::{Result, anyhow};
use bincode_next::config::BigEndian;
use bincode_next::config::Configuration;
use bincode_next::config::Fixint;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Cursor;
use std::os::raw::c_void;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU32;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use vpp_api_message::VppApiMessage;

/// A non-blocking asynchronous client for communicating with VPP via shared memory.
///
/// This client manages the lifecycle of a connection to the VPP API and provides
/// methods for sending and receiving messages asynchronously.
pub struct Client {
    /// The client index assigned by VPP upon connection.
    client_index: u32,
    /// An atomic counter for generating unique context IDs for each message.
    context_id: AtomicU32,
    /// The encoder configuration for serializing messages.
    config: Configuration<BigEndian, Fixint>,
    /// A resolver function that maps message names to their message IDs.
    resolver: Box<dyn Fn(String) -> Result<u16> + 'static>,
}

impl Client {
    /// Connects to the VPP API via shared memory.
    ///
    /// Initializes a connection to the VPP daemon by setting up the global message queue
    /// and performing all necessary initialization steps.
    ///
    /// # Arguments
    ///
    /// * `name` - The name identifier for this client connection.
    /// * `chroot_prefix` - Optional chroot prefix path for VPP API shared memory path.
    /// * `rx_qlen` - The receive queue length for buffering incoming messages.
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created `Client` instance or an error if connection fails.
    pub async fn connect(
        name: &str,
        chroot_prefix: Option<String>,
        rx_qlen: i32,
    ) -> Result<Client> {
        let name = name.into();

        init_global_queue();

        tokio::task::spawn_blocking(move || {
            vac_mem_init_wrapper();
            vac_set_error_handler_wrapper(Some(vac_error_handler));
            vac_connect_wrapper(name, chroot_prefix, Some(vac_write_callback), rx_qlen)?;
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
            resolver: Box::new(resolve_id),
        })
    }

    /// Creates a new encoder configuration for message serialization.
    ///
    /// Configures the bincode encoder with big-endian byte order and fixed-size integer encoding.
    ///
    /// # Returns
    ///
    /// A `Configuration` object with the appropriate settings for VPP message encoding.
    fn new_encoder() -> Configuration<BigEndian, Fixint> {
        bincode_next::config::legacy()
            .with_big_endian()
            .with_fixed_int_encoding()
    }

    /// Sends a message and receives the corresponding response asynchronously.
    ///
    /// This method is a convenience wrapper that sends a message to VPP and waits for
    /// the corresponding response in a single operation.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send, must implement `Serialize` and `VppApiMessage`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the response message or an error if the operation fails.
    pub async fn send_rcv<T, R>(&mut self, msg: T) -> Result<R>
    where
        T: Serialize + VppApiMessage,
        R: DeserializeOwned + VppApiMessage,
    {
        let receiver = get_global_receiver()?;
        let mut receiver_guard = receiver.lock().await;
        self.send(msg).await?;
        self.receive(&mut receiver_guard).await
    }

    /// Sends a message to VPP asynchronously.
    ///
    /// Prepares the message with client index and context, serializes it using the configured
    /// encoder, and sends it to VPP via the VAC interface on a blocking task.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send, must implement `Serialize` and `VppApiMessage`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error if serialization or sending fails.
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

    /// Receives a message from VPP asynchronously.
    ///
    /// Waits for a message from the global receiver queue and deserializes it using the configured
    /// decoder.
    ///
    /// # Arguments
    ///
    /// * `receiver` - A mutable reference to the message receiver queue.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized message or an error if reception or deserialization fails.
    async fn receive<R>(&mut self, receiver: &mut tokio::sync::mpsc::Receiver<Vec<u8>>) -> Result<R>
    where
        R: DeserializeOwned + VppApiMessage,
    {
        let msg = receiver
            .recv()
            .await
            .ok_or(anyhow!("empty response received from global queue"))?;
        let mut reader = Cursor::new(msg);
        read_object(&mut reader, &self.resolver, self.config, false).await
    }

    /// Sends a control ping message to VPP and retrieves the response.
    ///
    /// This is a diagnostic method to verify connectivity with the VPP daemon.
    ///
    /// # Returns

    pub async fn control_ping(&mut self) -> Result<i32> {
        let reply: RawControlPingReply = self.send_rcv(RawControlPing::default()).await?;
        Ok(reply.retval)
    }
    /// Executes a VPP CLI command and retrieves the output.
    ///
    /// Sends an in-band CLI command to VPP and waits for the response containing the command output.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The CLI command to execute as a string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the command output as a string or an error if the operation fails.
    ///
    /// A `Result` containing the return value from the control ping response or an error if the operation fails.
    /// Generates and returns the next unique context ID for a message.
    ///

    pub async fn run_cli_inband(&mut self, cmd: &str) -> Result<String> {
        let in_msg = RawCliInband::new(cmd)?;
        let out_msg: RawCliInbandReply = self.send_rcv(in_msg).await?;
        Ok(out_msg.reply.to_string())
    }

    /// Atomically increments and returns the context ID counter to ensure each message
    /// has a unique identifier for correlation with responses.
    ///
    /// # Returns
    ///
    /// The next available context ID.

    fn get_next_context(&mut self) -> u32 {
        self.context_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }
    ///
    /// Resolves a message name to its message ID.
    ///
    /// Uses the resolver function to look up the message ID for a given message name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the message to resolve.
    ///
    /// # Returns
    ///
    /// A `Result` containing the message ID or an error if the message name cannot be resolved.
    pub fn get_message_index(&self, name: &String) -> Result<u16> {
        self.resolver.as_ref()(name.to_owned())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        vac_disconnect_wrapper()
    }
}

#[derive(Debug)]
struct GlobalQueue {
    sender: mpsc::Sender<Vec<u8>>,
    receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
}

static QUEUE: OnceLock<GlobalQueue> = OnceLock::new();

///
/// Initializes the global message queue for inter-task communication.
///
/// Creates a new MPSC channel with a buffer capacity of 16 and stores it in a static OnceLock.
/// This queue is used to route messages received from VPP to waiting tasks.
pub fn init_global_queue() {
    let (sender, receiver) = mpsc::channel::<Vec<u8>>(16);
    let queue = GlobalQueue {
        sender,
        receiver: Arc::new(Mutex::new(receiver)),
    };
    QUEUE.get_or_init(|| queue);
}

///
/// Retrieves the global message receiver from the queue.
///
/// Returns a cloned reference to the mutex-protected receiver for receiving messages
/// from VPP.
pub fn get_global_receiver() -> Result<Arc<Mutex<mpsc::Receiver<Vec<u8>>>>> {
    let queue = QUEUE.get().ok_or(anyhow!("global queue not initialized"))?;
    Ok(queue.receiver.clone())
}

///
/// Retrieves the global message sender from the queue.
///
/// Returns a static reference to the sender for posting messages received from VPP
/// into the global queue.
///
/// # Panics
///
/// Panics if the global queue has not been initialized.
/// # Returns
///
/// A `Result` containing the receiver or an error if the global queue has not been initialized.
pub fn get_global_sender() -> &'static mpsc::Sender<Vec<u8>> {
    let queue = QUEUE.get().expect("global queue not initialized");
    &queue.sender
}

/// Callback function invoked by the VAC layer when data is ready to send to VPP.
///
/// Converts the raw data pointer to a vector, sends it through the global message sender,
/// and frees the allocated memory.
///
/// # Arguments
///
/// * `raw_data` - A pointer to the message data.
/// * `len` - The length of the message data in bytes.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure
/// that `raw_data` is valid and points to at least `len` bytes of readable memory.
///
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn vac_write_callback(raw_data: *const u8, len: i32) {
    let sender = get_global_sender();
    let msg = unsafe { std::slice::from_raw_parts(raw_data, len as usize) }.to_vec();
    sender.blocking_send(msg).ok();
    unsafe {
        vac_free(raw_data as *mut c_void);
    }
}

/// Callback function invoked by the VAC layer when an error occurs.
/// Converts the raw error message to a UTF-8 string and prints it to stdout.
///
/// # Arguments
///
/// * `_arg` - An optional context pointer (currently unused).
/// * `msg` - A pointer to the error message data.
/// * `msg_len` - The length of the error message in bytes.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure
/// that `msg` is valid and points to at least `msg_len` bytes of readable memory.
///
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
        let mut client = Client::connect("test_non_blocking_client", None, 32)
            .await
            .unwrap();

        let res = client.control_ping().await.unwrap();
        assert_eq!(res, 0);

        let s = client.run_cli_inband("show version").await.unwrap();
        assert!(s.starts_with("vpp "));
        println!("\n {s}");
    }
}
