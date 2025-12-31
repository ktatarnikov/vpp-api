use crate::message::SockMsgHeader;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::warn;
use vpp_api_message::VppApiMessage;

/// Writes a serialized message object to the provided async writer.
///
/// # Arguments
/// * `writer` - The async writer to write to
/// * `msg` - The message to serialize and write
/// * `message_ids` - Map of message names to their IDs
/// * `config` - The bincode configuration to use for encoding
/// * `has_socket_header` - flag indicates that message socket header is expected
///
/// # Errors
/// Returns an error if the message ID cannot be found or if writing fails
pub async fn write_object<W, T, C, F>(
    writer: &mut W,
    msg: &T,
    resolve_id: &F,
    config: C,
    has_socket_header: bool,
) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    T: Serialize + VppApiMessage,
    C: bincode_next::config::Config,
    F: Fn(String) -> Result<u16>,
{
    let name = &T::get_message_name_and_crc();
    let msg_id: u16 = resolve_id(name.clone())?;

    let mut encoded = bincode_next::serde::encode_to_vec(msg_id, config)?;
    let encoded_msg = bincode_next::serde::encode_to_vec(msg, config)?;

    encoded.extend_from_slice(&encoded_msg);

    write_frame(writer, &encoded, config, has_socket_header).await
}

/// Writes a framed message to the provided async writer.
///
/// # Arguments
/// * `writer` - The async writer to write to
/// * `bytes` - The message bytes to write
/// * `config` - The bincode configuration to use for encoding
/// * `has_socket_header` - flag indicates that message socket header is expected
///
/// # Errors
/// Returns an error if encoding the header or writing fails
async fn write_frame<W, C>(
    writer: &mut W,
    bytes: &[u8],
    config: C,
    has_socket_header: bool,
) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    C: bincode_next::config::Config,
{
    if has_socket_header {
        let hdr = SockMsgHeader {
            _q: 0,
            msglen: bytes.len() as u32,
            gc_mark: 0,
        };
        let hdre = bincode_next::serde::encode_to_vec(&hdr, config)?;

        writer.write_all(&hdre).await?;
    }
    writer.write_all(bytes).await?;
    writer.flush().await?;

    Ok(())
}

/// Reads a deserialized message object from the provided async reader.
///
/// # Arguments
/// * `reader` - The async reader to read from
/// * `message_ids` - Map of message names to their IDs
/// * `config` - The bincode configuration to use for decoding
/// * `has_socket_header` - flag indicates that message socket header is expected
///
/// # Errors
/// Returns an error if the message ID cannot be found, if the message ID doesn't match
/// the expected ID, or if reading or decoding fails
pub async fn read_object<R, T, C, F>(
    reader: &mut R,
    resolve_id: &F,
    config: C,
    has_socket_header: bool,
) -> Result<T>
where
    R: AsyncReadExt + Unpin,
    T: for<'a> Deserialize<'a> + VppApiMessage,
    C: bincode_next::config::Config,
    F: Fn(String) -> Result<u16>,
{
    let name = &T::get_message_name_and_crc();
    let expected_msg_id: u16 = resolve_id(name.clone())?;
    let encoded = read_frame(reader, config, has_socket_header).await?;
    let (msg_id, data) = split_into_id_and_msg(&encoded)?;
    if msg_id == expected_msg_id {
        let (decode_result, _) = bincode_next::serde::decode_from_slice(&data, config)?;
        Ok(decode_result)
    } else {
        Err(anyhow!(
            "Unexpected message id '{msg_id}', Expected '{expected_msg_id}'"
        ))
    }
}

/// Reads a deserialized message object from a byte slice.
///
/// # Arguments
/// * `slice` - The byte slice containing the encoded message
/// * `message_ids` - Map of message names to their IDs
/// * `config` - The bincode configuration to use for decoding
///
/// # Errors
/// Returns an error if the message ID cannot be found, if the message ID doesn't match
/// the expected ID, or if decoding fails
pub fn read_object_from_slice<T, C>(
    slice: &[u8],
    message_ids: &HashMap<String, u16>,
    config: C,
) -> Result<T>
where
    T: for<'a> Deserialize<'a> + VppApiMessage,
    C: bincode_next::config::Config,
{
    let name = &T::get_message_name_and_crc();
    let expected_msg_id = message_ids
        .get(name)
        .ok_or(anyhow!("Cannot find message id for {name}"))?;

    let (msg_id, data) = split_into_id_and_msg(slice)?;
    if msg_id == *expected_msg_id {
        read_msg_from_slice(&data, config)
    } else {
        Err(anyhow!(
            "Unexpected message id '{msg_id}', Expected '{expected_msg_id}'"
        ))
    }
}

/// Decodes a message of type `T` from a byte slice.
///
/// # Arguments
/// * `slice` - The byte slice containing the encoded message
/// * `config` - The bincode configuration to use for decoding
///
/// # Errors
/// Returns an error if decoding fails
pub fn read_msg_from_slice<T, C>(slice: &[u8], config: C) -> Result<T>
where
    T: for<'a> Deserialize<'a>,
    C: bincode_next::config::Config,
{
    let (decode_result, _) = bincode_next::serde::decode_from_slice(slice, config)?;
    Ok(decode_result)
}

/// Reads a framed message from the provided async reader.
///
/// # Arguments
/// * `reader` - The async reader to read from
/// * `config` - The bincode configuration to use for decoding
/// * `has_socket_header` - flag indicates that message socket header should prepend the message
///
/// # Returns
/// Returns a vector of bytes containing the message payload
///
/// # Errors
/// Returns an error if the header is invalid, if the message length is invalid,
/// or if reading fails
async fn read_frame<R, C>(reader: &mut R, config: C, has_socket_header: bool) -> Result<Vec<u8>>
where
    R: AsyncReadExt + Unpin,
    C: bincode_next::config::Config,
{
    if has_socket_header {
        let header = read_header(reader, config).await?;

        match header.msglen.try_into() {
            Ok(msglen) => {
                if msglen == 0 {
                    return Err(anyhow!("Invalid header, header.msglen == 0"));
                }
                let mut data = vec![0; msglen];
                if let Err(e) = reader.read_exact(&mut data).await {
                    warn!("expected {} byte message, got error: {:?}", msglen, e);
                    return Err(anyhow!("Invalid Message {e}"));
                }
                Ok(data)
            }
            Err(e) => Err(anyhow!(
                "msg length {} couldn't be converted to usize: {}",
                header.msglen,
                e
            )),
        }
    } else {
        let mut data = vec![0; 655336];
        match reader.read(&mut data).await {
            Err(e) => {
                warn!("expected message, got error: {:?}", e);
                Err(anyhow!("Invalid Message {e}"))
            }
            Ok(0) => Err(anyhow!("Invalid Message: 0 bytes len")),
            Ok(n) => Ok(data[0..n].to_vec()),
        }
    }
}

/// Reads and decodes a message header from the provided async reader.
///
/// # Arguments
/// * `reader` - The async reader to read from
/// * `config` - The bincode configuration to use for decoding
///
/// # Returns
/// Returns the decoded message header
///
/// # Errors
/// Returns an error if reading fails or if the header cannot be decoded
async fn read_header<R, C>(reader: &mut R, config: C) -> Result<SockMsgHeader>
where
    R: AsyncReadExt + Unpin,
    C: bincode_next::config::Config,
{
    let mut header_buf = [0; 16];

    if let Err(e) = reader.read_exact(&mut header_buf).await {
        warn!("read invalid header: {:?} err: {:?}", header_buf, e);
        return Err(anyhow!("Invalid header"));
    }
    let (header, _): (SockMsgHeader, usize) =
        bincode_next::serde::decode_from_slice(&header_buf[..], config)?;
    Ok(header)
}

/// Splits encoded message data into message ID and payload.
///
/// # Arguments
/// * `data` - The byte slice containing the encoded message ID and payload
///
/// # Returns
/// Returns a tuple of (message_id, payload_bytes)
///
/// # Errors
/// Returns an error if the data length is less than 3 bytes
fn split_into_id_and_msg(data: &[u8]) -> Result<(u16, Vec<u8>)> {
    if data.len() < 3 {
        return Err(anyhow!(
            "short read message len: {}  {:x?}",
            data.len(),
            data
        ));
    }
    let msg_id: u16 = ((data[0] as u16) << 8) + (data[1] as u16);
    Ok((msg_id, data[2..].to_vec()))
}
