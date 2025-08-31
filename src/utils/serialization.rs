// Bincode compatibility layer for seamless migration from 1.x to 2.x API
use crate::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};

/// Serialize data using bincode 2.0 with standard configuration
pub fn serialize<T: Serialize + bincode::Encode>(data: &T) -> Result<Vec<u8>> {
    let config = bincode::config::standard();
    bincode::encode_to_vec(data, config)
        .map_err(|e| BlockchainError::Serialization(format!("Serialization failed: {e}")))
}

/// Deserialize data using bincode 2.0 with standard configuration
pub fn deserialize<T>(bytes: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de> + bincode::Decode<()>,
{
    let config = bincode::config::standard();
    let (data, _) = bincode::decode_from_slice(bytes, config)
        .map_err(|e| BlockchainError::Serialization(format!("Deserialization failed: {e}")))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
    struct TestData {
        id: u64,
        name: String,
        values: Vec<i32>,
    }

    #[test]
    fn test_serialize_deserialize() {
        let original = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1, 2, 3, 4, 5],
        };

        let serialized = serialize(&original).expect("Serialization should work");
        let deserialized: TestData = deserialize(&serialized).expect("Deserialization should work");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_serialize_empty_data() {
        let empty_vec: Vec<u8> = vec![];
        let serialized = serialize(&empty_vec).expect("Should serialize empty vector");
        let deserialized: Vec<u8> =
            deserialize(&serialized).expect("Should deserialize empty vector");
        assert_eq!(empty_vec, deserialized);
    }

    #[test]
    fn test_deserialize_invalid_data() {
        let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result: Result<TestData> = deserialize(&invalid_bytes);
        assert!(result.is_err());
    }
}
