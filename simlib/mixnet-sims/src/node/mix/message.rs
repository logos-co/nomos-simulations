use uuid::Uuid;

pub type PayloadId = [u8; 16];

pub struct Payload(PayloadId);

impl Payload {
    pub fn new() -> Self {
        Self(Uuid::new_v4().into_bytes())
    }

    pub fn id(&self) -> PayloadId {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn load(data: Vec<u8>) -> Self {
        assert_eq!(data.len(), 16);
        Self(data.try_into().unwrap())
    }
}
