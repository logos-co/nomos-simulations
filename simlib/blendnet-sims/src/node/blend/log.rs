#[macro_export]
macro_rules! log {
    ($topic:expr, $msg:expr) => {
        tracing::info!(
            "Topic:{}: {}",
            $topic,
            serde_json::to_string(&$msg).unwrap()
        );
    };
}
