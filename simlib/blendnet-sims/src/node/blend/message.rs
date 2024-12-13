use uuid::Uuid;

pub type PayloadId = String;

pub struct Payload(Uuid);

impl Payload {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn id(&self) -> PayloadId {
        self.0.to_string()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn load(data: Vec<u8>) -> Self {
        assert_eq!(data.len(), 16);
        Self(data.try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::Payload;

    #[test]
    fn payload() {
        let payload = Payload::new();
        println!("{}", payload.id());
        let bytes = payload.as_bytes();
        assert_eq!(bytes.len(), 16);
        let loaded_payload = Payload::load(bytes.to_vec());
        assert_eq!(bytes, loaded_payload.as_bytes());
    }
}
