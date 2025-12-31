use crate::shmem::shmem_bindgen::*;
use anyhow::{Result, anyhow};
use std::ffi::CString;
use std::os::raw::{c_int, c_ushort, c_void};

pub(crate) fn vac_mem_init_wrapper() {
    unsafe { vac_mem_init(0) };
}

pub(crate) fn vac_set_error_handler_wrapper(callback: vac_error_callback_t) {
    unsafe { vac_set_error_handler(callback) };
}

pub(crate) fn vac_disconnect_wrapper() {
    unsafe { vac_disconnect() };
}

pub(crate) fn vac_connect_wrapper(
    name: String,
    chroot_prefix: Option<String>,
    callback: vac_callback_t,
    rx_qlen: i32,
) -> Result<()> {
    let name_c = CString::new(name)?;
    let chroot_prefix_c = chroot_prefix.map(|x| CString::new(x).unwrap());

    let name_arg = name_c.as_ptr();
    let chroot_prefix_arg = if let Some(p) = chroot_prefix_c {
        p.as_ptr()
    } else {
        std::ptr::null_mut()
    };

    let err = unsafe { vac_connect(name_arg, chroot_prefix_arg, callback, rx_qlen) };

    if err < 0 {
        return Err(std::io::Error::other(format!("vac_connect returned {}", err)).into());
    }

    Ok(())
}

pub(crate) fn vac_write_wrapper(msg: Vec<u8>) -> Result<()> {
    let wr_len = msg.len();
    let err = unsafe { vac_write(msg.as_ptr(), wr_len as i32) };
    if err < 0 {
        return Err(anyhow!("vac_write returned {err}"));
    }
    Ok(())
}

pub(crate) fn vac_read_wrapper(timeout: u16) -> Result<Vec<u8>> {
    let mut ptr: *mut u8 = std::ptr::null_mut();
    let mut len: c_int = 0;
    let rc = unsafe { vac_read(&mut ptr, &mut len, timeout as c_ushort) };
    if rc < 0 {
        return Err(anyhow!("vac_read returned {rc}"));
    }

    if ptr.is_null() || len <= 0 {
        return Ok(Vec::new());
    }

    // Turn the (ptr,len) pair into a Rust slice
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let data = slice.to_vec(); // copy into owned Vec<u8>

    // IMPORTANT: free ptr according to the C library's rules
    unsafe {
        vac_free(ptr as *mut c_void);
    }

    Ok(data)
}

pub(crate) fn vac_get_msg_index_wrapper(name: String) -> Option<u16> {
    let name_c = CString::new(name).unwrap();
    let id = unsafe { vac_get_msg_index(name_c.as_ptr() as *const u8) };
    if id > 0 && id < 65536 {
        Some(id as u16)
    } else {
        None
    }
}

pub enum VacErrorNo {
    VacSvmQueueSub1 = -1,
    VacSvmQueueSub2 = -2,
    VacNotConnected = -3,
    VacShmNotReady = -4,
    VacTimeout = -5,
}
