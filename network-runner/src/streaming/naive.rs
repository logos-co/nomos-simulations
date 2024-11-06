use super::{Receivers, StreamSettings, Subscriber, SubscriberFormat};
use crate::output_processors::{Record, RecordType, Runtime};
use crossbeam::channel::{Receiver, Sender};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaiveSettings {
    pub path: PathBuf,
    #[serde(default = "SubscriberFormat::json")]
    pub format: SubscriberFormat,
}

impl TryFrom<StreamSettings> for NaiveSettings {
    type Error = String;

    fn try_from(settings: StreamSettings) -> Result<Self, Self::Error> {
        match settings {
            StreamSettings::Naive(settings) => Ok(settings),
            _ => Err("naive settings can't be created".into()),
        }
    }
}

impl Default for NaiveSettings {
    fn default() -> Self {
        let mut tmp = std::env::temp_dir();
        tmp.push("simulation");
        tmp.set_extension("data");
        Self {
            path: tmp,
            format: SubscriberFormat::Csv,
        }
    }
}

#[derive(Debug)]
pub struct NaiveSubscriber<R> {
    file: Mutex<File>,
    recvs: Receivers<R>,
    initialized: AtomicBool,
    format: SubscriberFormat,
}

impl<R> Subscriber for NaiveSubscriber<R>
where
    R: crate::output_processors::Record + Serialize,
{
    type Record = R;

    type Settings = NaiveSettings;

    fn new(
        record_recv: Receiver<Arc<Self::Record>>,
        stop_recv: Receiver<Sender<()>>,
        settings: Self::Settings,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut opts = OpenOptions::new();
        let recvs = Receivers {
            stop_rx: stop_recv,
            recv: record_recv,
        };
        let this = NaiveSubscriber {
            file: Mutex::new(
                opts.truncate(true)
                    .create(true)
                    .read(true)
                    .write(true)
                    .open(&settings.path)?,
            ),
            recvs,
            initialized: AtomicBool::new(false),
            format: settings.format,
        };
        tracing::info!(
            target = "simulation",
            "subscribed to {}",
            settings.path.display()
        );
        Ok(this)
    }

    fn next(&self) -> Option<anyhow::Result<Arc<Self::Record>>> {
        Some(self.recvs.recv.recv().map_err(From::from))
    }

    fn run(self) -> anyhow::Result<()> {
        loop {
            crossbeam::select! {
                recv(self.recvs.stop_rx) -> finish_tx => {
                    // Flush remaining messages after stop signal.
                    while let Ok(msg) = self.recvs.recv.try_recv() {
                        self.sink(msg)?;
                    }

                    // collect the run time meta
                    self.sink(Arc::new(R::from(Runtime::load()?)))?;

                    finish_tx?.send(())?
                }
                recv(self.recvs.recv) -> msg => {
                    self.sink(msg?)?;
                }
            }
        }
    }

    fn sink(&self, state: Arc<Self::Record>) -> anyhow::Result<()> {
        let mut file = self.file.lock();
        match self.format {
            SubscriberFormat::Json => {
                write_json_record(&mut *file, &self.initialized, &*state)?;
            }
            SubscriberFormat::Csv => {
                write_csv_record(&mut *file, &self.initialized, &*state)?;
            }
            SubscriberFormat::Parquet => {
                panic!("native subscriber does not support parquet format")
            }
        }

        Ok(())
    }

    fn subscribe_data_type() -> RecordType {
        RecordType::Data
    }
}

impl<R> Drop for NaiveSubscriber<R> {
    fn drop(&mut self) {
        if SubscriberFormat::Json == self.format {
            let mut file = self.file.lock();
            // To construct a valid json format, we need to overwrite the last comma
            if let Err(e) = file
                .seek(std::io::SeekFrom::End(-1))
                .and_then(|_| file.write_all(b"]}"))
            {
                tracing::error!(target="simulations", err=%e, "fail to close json format");
            }
        }
    }
}

fn write_json_record<W: std::io::Write, R: Record>(
    mut w: W,
    initialized: &AtomicBool,
    record: &R,
) -> std::io::Result<()> {
    if !initialized.load(Ordering::Acquire) {
        w.write_all(b"{\"records\": [")?;
        initialized.store(true, Ordering::Release);
    }
    for data in record.data() {
        serde_json::to_writer(&mut w, data)?;
        w.write_all(b",")?;
    }
    Ok(())
}

fn write_csv_record<W: std::io::Write, R: Record>(
    w: &mut W,
    initialized: &AtomicBool,
    record: &R,
) -> csv::Result<()> {
    // If have not write csv header, then write it
    let mut w = if !initialized.load(Ordering::Acquire) {
        initialized.store(true, Ordering::Release);
        csv::WriterBuilder::new().has_headers(true).from_writer(w)
    } else {
        csv::WriterBuilder::new().has_headers(false).from_writer(w)
    };
    for data in record.data() {
        w.serialize(data).map_err(|e| {
            tracing::error!(target = "simulations", err = %e, "fail to write CSV record");
            e
        })?;
        w.flush()?;
    }
    Ok(())
}
