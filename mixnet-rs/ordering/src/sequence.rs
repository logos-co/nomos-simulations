use std::fs::File;

use ordering::message::DataMessage;

#[derive(Debug)]
pub struct SequenceWriter {
    noise_buf: u32,
    writer: csv::Writer<File>,
}

impl SequenceWriter {
    pub fn new(path: &str) -> Self {
        Self {
            noise_buf: 0,
            writer: csv::Writer::from_path(path).unwrap(),
        }
    }

    pub fn flush(&mut self) {
        self.clear_buf();
        self.writer.flush().unwrap();
    }

    fn clear_buf(&mut self) {
        if self.noise_buf > 0 {
            self.writer
                .write_record(&[format!("-{}", self.noise_buf)])
                .unwrap();
            self.noise_buf = 0;
        }
    }

    pub fn add_message(&mut self, msg: &DataMessage) {
        self.clear_buf();
        self.writer.write_record(&[msg.to_string()]).unwrap();
    }

    pub fn add_noise(&mut self) {
        self.noise_buf += 1;
    }
}
