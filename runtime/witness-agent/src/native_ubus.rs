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

#[cfg(unix)]
#[path = "native_worker.rs"]
mod native_worker;

#[cfg(all(target_os = "linux", feature = "native-ubus"))]
pub use native_worker::helper_main;

#[cfg(all(target_os = "linux", feature = "native-ubus"))]
impl UbusTransport for NativeUbusTransport {
    fn invoke(
        &self,
        object: &str,
        method: &str,
        request: &Value,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Value, ObservationError> {
        native_worker::invoke(object, method, request, timeout, max_response_bytes)
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
