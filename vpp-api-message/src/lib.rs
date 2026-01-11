/// Trait for VPP API messages.
///
/// Defines the interface for VPP API message types, providing methods to retrieve
/// message metadata and set message context information.
pub trait VppApiMessage {
    /// Returns the message name and CRC32 checksum as a formatted string.
    ///
    /// # Returns
    ///
    /// A `String` containing the message name and CRC in the format "name_crc".
    fn get_message_name_and_crc() -> String;

    /// Sets the context field for this message.
    ///
    /// # Arguments
    ///
    /// * `context` - The context identifier to associate with this message.
    fn set_context(&mut self, context: u32);

    /// Sets the client index field for this message.
    ///
    /// # Arguments
    ///
    /// * `client_index` - The client index identifier to associate with this message.
    fn set_client_index(&mut self, client_index: u32);
}
