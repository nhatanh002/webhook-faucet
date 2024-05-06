use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::ClientConfig;

pub fn create_kafka_producer() -> anyhow::Result<FutureProducer> {
    let url = &crate::config::get().kafka_url;
    let kafka_tx_id = crate::config::get().kafka_tx_id.as_str();

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", url)
        .set("message.timeout.ms", "5000")
        .set("allow.auto.create.topics", "true")
        .set("transactional.id", kafka_tx_id)
        .create()
        .expect("Producer creation error");
    producer
        .init_transactions(std::time::Duration::from_millis(5000))
        .inspect_err(|e| tracing::error!("shit happened: {e:#?}"))?;

    Ok(producer)
}

pub fn make_kafka_payload<'a>(
    msg: &'a Vec<u8>,
    topic: &'a String,
    key: &'a String,
) -> FutureRecord<'a, String, Vec<u8>> {
    let record: FutureRecord<String, Vec<u8>> = FutureRecord::to(topic).payload(msg).key(key);
    record
    // state.producer().send_result(record)?.await??;
    // tracing::info!("Message sent with data: {message:?}");
    // Ok("Message sent!")
}
