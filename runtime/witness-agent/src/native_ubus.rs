//! FFI transport for the read-only C libubus shim.
use super::{read_only_method, ObservationError, UbusTransport};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize,
};
use serde_json::Value;
use std::{collections::HashSet, fmt, time::Duration};

/// Present on every platform so callers can explicitly report unavailable
/// native observation rather than falling back to a command or HTTP gateway.
pub struct NativeUbusTransport;

#[cfg(all(target_os = "linux", feature = "native-ubus"))]
mod ffi {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        os::raw::{c_char, c_int},
        ptr,
    };
    unsafe extern "C" {
        fn intent_ubus_invoke_json(
            object: *const c_char,
            method: *const c_char,
            request: *const c_char,
            timeout_ms: c_int,
            max_response_bytes: usize,
            response: *mut *mut c_char,
        ) -> c_int;
        fn intent_ubus_free(value: *mut c_char);
    }
    impl UbusTransport for NativeUbusTransport {
        fn invoke(
            &self,
            object: &str,
            method: &str,
            request: &Value,
            timeout: Duration,
            max_response_bytes: usize,
        ) -> Result<Value, ObservationError> {
            if !read_only_method(object, method) {
                return Err(ObservationError::UnsupportedMethod {
                    object: object.into(),
                    method: method.into(),
                });
            }
            let object = CString::new(object)
                .map_err(|_| ObservationError::Malformed("NUL ubus object".into()))?;
            let method = CString::new(method)
                .map_err(|_| ObservationError::Malformed("NUL ubus method".into()))?;
            let request = CString::new(request.to_string())
                .map_err(|_| ObservationError::Malformed("NUL ubus request".into()))?;
            let mut response = ptr::null_mut();
            let result = unsafe {
                intent_ubus_invoke_json(
                    object.as_ptr(),
                    method.as_ptr(),
                    request.as_ptr(),
                    timeout.as_millis().min(i32::MAX as u128) as c_int,
                    max_response_bytes,
                    &mut response,
                )
            };
            if result != 0 {
                return Err(match result {
                    7_003 => ObservationError::Denied("ubus permission denied".into()),
                    7_001 => ObservationError::ResponseTooLarge,
                    7_002 => ObservationError::Timeout,
                    _ => ObservationError::Unavailable(format!("libubus status {result}")),
                });
            }
            if response.is_null() {
                return Err(ObservationError::Malformed(
                    "libubus returned no response".into(),
                ));
            }
            let bytes = unsafe { CStr::from_ptr(response).to_bytes().to_vec() };
            unsafe {
                intent_ubus_free(response);
            }
            if bytes.len() > max_response_bytes {
                return Err(ObservationError::ResponseTooLarge);
            }
            parse_response_json(&bytes)
        }
    }
}

/// `serde_json::Value` accepts duplicate map keys by overwriting the earlier
/// value. Responses are evidence, so ambiguity is rejected at this boundary.
#[allow(dead_code)] // Called by the Linux/native-ubus FFI implementation.
fn parse_response_json(bytes: &[u8]) -> Result<Value, ObservationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictValue::deserialize(&mut deserializer)
        .map_err(|_| ObservationError::Malformed("invalid or duplicate-key libubus JSON".into()))?
        .0;
    deserializer
        .end()
        .map_err(|_| ObservationError::Malformed("invalid libubus JSON trailing data".into()))?;
    Ok(value)
}

struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}
struct StrictValueVisitor;
impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(v)))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(v.into())))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(v.into())))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(v)
            .map(|n| StrictValue(Value::Number(n)))
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(v.into())))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(v)))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<StrictValue>()? {
            values.push(value.0);
        }
        Ok(StrictValue(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut keys = HashSet::new();
        let mut values = serde_json::Map::new();
        while let Some((key, value)) = map.next_entry::<String, StrictValue>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            values.insert(key, value.0);
        }
        Ok(StrictValue(Value::Object(values)))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_response_json;
    #[test]
    fn response_json_rejects_duplicate_keys_at_any_depth() {
        assert!(parse_response_json(br#"{"access":true,"access":false}"#).is_err());
        assert!(parse_response_json(br#"{"nested":{"x":1,"x":2}}"#).is_err());
        assert!(parse_response_json(br#"{"access":true}"#).is_ok());
    }
}

#[cfg(not(all(target_os = "linux", feature = "native-ubus")))]
impl UbusTransport for NativeUbusTransport {
    fn invoke(
        &self,
        object: &str,
        method: &str,
        _: &Value,
        _: Duration,
        _: usize,
    ) -> Result<Value, ObservationError> {
        if !read_only_method(object, method) {
            return Err(ObservationError::UnsupportedMethod {
                object: object.into(),
                method: method.into(),
            });
        }
        Err(ObservationError::Unavailable("native-ubus requires Linux with feature native-ubus and OpenWrt libubus/libubox headers".into()))
    }
}
