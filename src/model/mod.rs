use serde::de::MapAccess;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use serde_json::Value;
use std::collections::HashMap;

pub mod error;

#[derive(Debug, Clone)]
pub struct Product {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReqDownstream {
    pub endpoint: String,
    #[serde(with = "http_serde::method")]
    pub method: http::Method,
    #[serde(with = "http_serde::header_map")]
    pub headers: http::HeaderMap,
    pub queries: HashMap<String, String>,
    // #[serde( serialize_with = "payload_serialize", deserialize_with = "payload_deserialize")]
    pub payload: String,
}

// customized serialize/deserialize functions for the JSON payload, in case the JSON serialization
// for ReqDownstream needs payload to be serialized as actual JSON object and not just a string in
// JSON syntax
#[allow(dead_code)]
fn payload_serialize<S: Serializer>(payload: &str, s: S) -> Result<S::Ok, S::Error> {
    let v: &RawValue = serde_json::from_str(payload).map_err(serde::ser::Error::custom)?; //expect("invalid json");
    v.serialize(s)
}

#[allow(dead_code)]
fn payload_deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<String, D::Error> {
    struct RawValueVisitor;
    impl<'de> serde::de::Visitor<'de> for RawValueVisitor {
        type Value = String;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a json object")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut json_obj: M) -> Result<String, M::Error> {
            let mut map = HashMap::<String, Value>::new();
            while let Some((key, val)) = json_obj.next_entry::<String, Value>()? {
                map.insert(key, val);
            }
            Ok(serde_json::to_string(&map).unwrap_or("".to_string()))
        }
    }
    let v: String = de.deserialize_map(RawValueVisitor {})?;
    Ok(v)
}

#[test]
fn test_serialize() {
    use http::HeaderMap;
    let a = ReqDownstream {
        payload: r#"{
           "foo": 2307
        }"#
        .to_string(),
        endpoint: "".to_string(),
        method: http::Method::GET,
        headers: HeaderMap::new(),
        queries: HashMap::new(),
    };
    println!("a: {a:#?}");

    let r = serde_json::to_string(&a).unwrap();
    println!("r: {r:#?}");
    let aa = serde_json::from_str(&r).unwrap();
    println!("aa: {aa:#?}");
    assert_eq!(a, aa);
    let bitc_r = bitcode::serialize(&a).unwrap();
    println!("bitc_r: {:#?}", std::str::from_utf8(&bitc_r));
    let bitc_a: ReqDownstream = bitcode::deserialize(&bitc_r).unwrap();
    println!("bitc_a: {bitc_a:#?}");
    assert_eq!(a, bitc_a);
}
