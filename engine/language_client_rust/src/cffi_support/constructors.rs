use anyhow::{anyhow, Context, Result};
use prost::Message;
use std::mem::transmute_copy;
use std::os::raw::c_char;

use crate::baml::cffi::{
    cffi_object_response::Response as CffiObjectResponseVariant,
    cffi_object_response_success::Result as CffiObjectResponseSuccess, CffiMapEntry,
    CffiObjectConstructorArgs, CffiObjectResponse, CffiObjectType, CffiRawObject,
};
use crate::ffi;

pub(crate) fn construct_object(
    object_type: CffiObjectType,
    kwargs: Vec<CffiMapEntry>,
) -> Result<CffiRawObject> {
    let args = CffiObjectConstructorArgs {
        r#type: object_type as i32,
        kwargs,
    };

    let mut encoded_args = Vec::new();
    args.encode(&mut encoded_args)
        .context("failed to encode object constructor arguments")?;

    let buffer =
        ffi::call_object_constructor(encoded_args.as_ptr() as *const c_char, encoded_args.len());

    let (ptr, len): (*const i8, usize) = unsafe { transmute_copy(&buffer) };
    if ptr.is_null() || len == 0 {
        ffi::free_buffer(buffer);
        return Err(anyhow!(
            "object constructor returned empty response for {:?}",
            object_type
        ));
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec();
    ffi::free_buffer(buffer);

    decode_constructor_response(&bytes).with_context(|| {
        format!(
            "failed to decode constructor response for {:?}",
            object_type
        )
    })
}

fn decode_constructor_response(bytes: &[u8]) -> Result<CffiRawObject> {
    match CffiObjectResponse::decode(bytes) {
        Ok(response) => match response.response {
            Some(CffiObjectResponseVariant::Success(success)) => match success.result {
                Some(CffiObjectResponseSuccess::Object(object)) => Ok(object),
                Some(CffiObjectResponseSuccess::Objects(_)) => Err(anyhow!(
                    "constructor returned multiple objects; expected a single object"
                )),
                Some(CffiObjectResponseSuccess::Value(_)) => Err(anyhow!(
                    "constructor returned a value; expected an object reference"
                )),
                None => Err(anyhow!("constructor response missing result payload")),
            },
            Some(CffiObjectResponseVariant::Error(err)) => {
                Err(anyhow!("constructor error: {}", err.error))
            }
            None => Err(anyhow!("constructor response missing payload")),
        },
        Err(decode_err) => {
            if let Ok(message) = std::str::from_utf8(bytes) {
                Err(anyhow!("constructor error: {message}"))
            } else {
                Err(anyhow!(
                    "failed to decode constructor response: {decode_err}"
                ))
            }
        }
    }
}
