use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use data_processing_service::{
    config::DataProcessingConfig,
    stream::{StreamProcessor, StreamProcessorConfig},
    types::{DataRecord, ProcessingMode, StreamWindow},
};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

fn create_test_record(id: usize) -> DataRecord {
    DataRecord {
        id: Uuid::new_v4(),
        source: format!("benchmark_source_{}", id % 10),
        timestamp: chrono::Utc::now(),
        data: json!({
            "user_id": format!("user_{}", id % 1000),
            "event_type": "click",
            "value": id as f64 * 1.5,
            "metadata": {
                "session_id": format!("session_{}", id % 100),
                "page": format!("page_{}", id % 20)
            }
        }),
        content_type: "application/json".to_string(),
        size_bytes: 256,
        metadata: std::collections::HashMap::new(),
    }
}

fn bench_stream_processing_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = StreamProcessorConfig {
        buffer_size: 10000,
        batch_size: 100,
        processing_timeout: Duration::from_millis(100),
        max_concurrency: 10,
        processing_mode: ProcessingMode::RealTime,
        window_config: Some(StreamWindow {
            size: Duration::from_secs(60),
            slide: Duration::from_secs(10),
        }),
    };

    let mut group = c.benchmark_group("stream_processing_throughput");

    for record_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("process_records", record_count),
            record_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let processor = StreamProcessor::new(config.clone()).await.unwrap();
                    let records: Vec<DataRecord> = (0..count).map(create_test_record).collect();

                    let start = std::time::Instant::now();
                    for record in records {
                        processor.process_record(black_box(record)).await.unwrap();
                    }
                    let duration = start.elapsed();
                    black_box(duration)
                });
            },
        );
    }
    group.finish();
}

fn bench_windowing_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("windowing_operations");

    for window_size in [
        Duration::from_secs(10),
        Duration::from_secs(60),
        Duration::from_secs(300),
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::new("tumbling_window", format!("{}s", window_size.as_secs())),
            window_size,
            |b, &window_size| {
                b.to_async(&rt).iter(|| async {
                    let config = StreamProcessorConfig {
                        buffer_size: 1000,
                        batch_size: 50,
                        processing_timeout: Duration::from_millis(50),
                        max_concurrency: 4,
                        processing_mode: ProcessingMode::RealTime,
                        window_config: Some(StreamWindow {
                            size: window_size,
                            slide: window_size, // Tumbling window
                        }),
                    };

                    let processor = StreamProcessor::new(config).await.unwrap();
                    let records: Vec<DataRecord> = (0..1000).map(create_test_record).collect();

                    for record in records {
                        processor.process_record(black_box(record)).await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_aggregation_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("aggregation_performance");
    group.sample_size(20); // Reduce sample size for expensive operations

    for aggregation_type in ["sum", "avg", "count", "distinct_count"].iter() {
        group.bench_with_input(
            BenchmarkId::new("aggregation", aggregation_type),
            aggregation_type,
            |b, &agg_type| {
                b.to_async(&rt).iter(|| async {
                    let config = StreamProcessorConfig {
                        buffer_size: 5000,
                        batch_size: 100,
                        processing_timeout: Duration::from_millis(200),
                        max_concurrency: 8,
                        processing_mode: ProcessingMode::RealTime,
                        window_config: Some(StreamWindow {
                            size: Duration::from_secs(60),
                            slide: Duration::from_secs(10),
                        }),
                    };

                    let processor = StreamProcessor::new(config).await.unwrap();
                    let records: Vec<DataRecord> = (0..5000).map(create_test_record).collect();

                    // Simulate aggregation workload
                    for record in records {
                        let mut modified_record = record;
                        modified_record.data = json!({
                            "aggregation_type": agg_type,
                            "value": rand::random::<f64>() * 1000.0,
                            "group_key": format!("group_{}", rand::random::<u8>() % 10)
                        });
                        processor
                            .process_record(black_box(modified_record))
                            .await
                            .unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_processing");

    for concurrency in [1, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_streams", concurrency),
            concurrency,
            |b, &concurrency_level| {
                b.to_async(&rt).iter(|| async {
                    let config = StreamProcessorConfig {
                        buffer_size: 2000,
                        batch_size: 50,
                        processing_timeout: Duration::from_millis(100),
                        max_concurrency: concurrency_level,
                        processing_mode: ProcessingMode::RealTime,
                        window_config: Some(StreamWindow {
                            size: Duration::from_secs(30),
                            slide: Duration::from_secs(5),
                        }),
                    };

                    let processor = StreamProcessor::new(config).await.unwrap();
                    let records_per_stream = 500;

                    // Create concurrent processing tasks
                    let mut handles = Vec::new();
                    for stream_id in 0..concurrency_level {
                        let processor_clone = processor.clone();
                        let handle = tokio::spawn(async move {
                            let records: Vec<DataRecord> = (0..records_per_stream)
                                .map(|i| {
                                    let mut record =
                                        create_test_record(i + stream_id * records_per_stream);
                                    record.source = format!("stream_{}", stream_id);
                                    record
                                })
                                .collect();

                            for record in records {
                                processor_clone.process_record(record).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    // Wait for all streams to complete
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10); // Lower sample size for memory benchmarks

    for record_size in [1024, 10240, 102400].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_records", format!("{}bytes", record_size)),
            record_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let config = StreamProcessorConfig {
                        buffer_size: 100,
                        batch_size: 10,
                        processing_timeout: Duration::from_millis(500),
                        max_concurrency: 2,
                        processing_mode: ProcessingMode::RealTime,
                        window_config: Some(StreamWindow {
                            size: Duration::from_secs(10),
                            slide: Duration::from_secs(5),
                        }),
                    };

                    let processor = StreamProcessor::new(config).await.unwrap();

                    // Create large records
                    for i in 0..100 {
                        let large_data = "x".repeat(size);
                        let mut record = create_test_record(i);
                        record.data = json!({
                            "large_field": large_data,
                            "id": i
                        });
                        record.size_bytes = size + 100; // Approximate size

                        processor.process_record(black_box(record)).await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_stream_processing_throughput,
    bench_windowing_operations,
    bench_aggregation_performance,
    bench_concurrent_processing,
    bench_memory_usage
);
criterion_main!(benches);
