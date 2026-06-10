pub(crate) fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            for lower in c.to_lowercase() {
                result.push(lower);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub(crate) fn convert_keys_camel_to_snake(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let snake = camel_to_snake(&key);
                if snake != key {
                    if let Some(v) = map.remove(&key) {
                        map.insert(snake, v);
                    }
                }
            }
            for v in map.values_mut() {
                convert_keys_camel_to_snake(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                convert_keys_camel_to_snake(v);
            }
        }
        _ => {}
    }
}

pub(crate) fn deep_merge(a: &mut serde_json::Value, b: serde_json::Value) {
    match (a, b) {
        (serde_json::Value::Object(ref mut a_map), serde_json::Value::Object(b_map)) => {
            for (k, v) in b_map {
                if a_map.contains_key(&k) && v.is_object() {
                    deep_merge(&mut a_map[&k], v);
                } else {
                    a_map.insert(k, v);
                }
            }
        }
        (a, b) => *a = b,
    }
}
