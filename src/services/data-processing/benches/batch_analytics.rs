use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use data_processing_service::{
    batch::{BatchJob, BatchJobStatus, BatchProcessor, BatchProcessorConfig},
    types::{DataRecord, ProcessingMode},
};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

fn create_analytics_record(id: usize, category: &str) -> DataRecord {
    DataRecord {
        id: Uuid::new_v4(),
        source: format!("analytics_source_{}", id % 5),
        timestamp: chrono::Utc::now(),
        data: json!({
            "user_id": format!("user_{}", id % 10000),
            "category": category,
            "value": (id as f64) * 0.75 + (rand::random::<f64>() * 100.0),
            "dimensions": {
                "country": format!("country_{}", id % 50),
                "device": format!("device_{}", id % 10),
                "channel": format!("channel_{}", id % 5)
            },
            "metrics": {
                "revenue": rand::random::<f64>() * 1000.0,
                "conversion_rate": rand::random::<f64>(),
                "session_duration": rand::random::<u32>() % 3600
            }
        }),
        content_type: "application/json".to_string(),
        size_bytes: 512,
        metadata: std::collections::HashMap::new(),
    }
}

fn bench_batch_job_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_job_creation");

    for job_size in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_batch_job", job_size),
            job_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let config = BatchProcessorConfig {
                        max_concurrent_jobs: 10,
                        job_timeout: Duration::from_secs(300),
                        checkpoint_interval: Duration::from_secs(30),
                        retry_attempts: 3,
                        processing_mode: ProcessingMode::Batch,
                        output_format: "json".to_string(),
                    };

                    let processor = BatchProcessor::new(config).await.unwrap();

                    // Create records for batch job
                    let records: Vec<DataRecord> = (0..size)
                        .map(|i| create_analytics_record(i, "sales"))
                        .collect();

                    let job = BatchJob {
                        id: Uuid::new_v4(),
                        name: format!("benchmark_job_{}", size),
                        status: BatchJobStatus::Pending,
                        created_at: chrono::Utc::now(),
                        started_at: None,
                        completed_at: None,
                        total_records: records.len() as u64,
                        processed_records: 0,
                        failed_records: 0,
                        progress_percentage: 0.0,
                        error_message: None,
                        output_location: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    black_box(processor.submit_job(job, records).await.unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_aggregation_queries(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("aggregation_queries");
    group.sample_size(10); // Reduce sample size for expensive operations

    for aggregation_type in ["sum", "avg", "count", "group_by", "window_function"].iter() {
        group.bench_with_input(
            BenchmarkId::new("aggregation", aggregation_type),
            aggregation_type,
            |b, &agg_type| {
                b.to_async(&rt).iter(|| async {
                    let config = BatchProcessorConfig {
                        max_concurrent_jobs: 4,
                        job_timeout: Duration::from_secs(60),
                        checkpoint_interval: Duration::from_secs(10),
                        retry_attempts: 2,
                        processing_mode: ProcessingMode::Batch,
                        output_format: "parquet".to_string(),
                    };

                    let processor = BatchProcessor::new(config).await.unwrap();

                    // Create diverse dataset for aggregation
                    let records: Vec<DataRecord> = (0..50000)
                        .map(|i| create_analytics_record(i, &format!("category_{}", i % 10)))
                        .collect();

                    let job = BatchJob {
                        id: Uuid::new_v4(),
                        name: format!("agg_benchmark_{}", agg_type),
                        status: BatchJobStatus::Pending,
                        created_at: chrono::Utc::now(),
                        started_at: None,
                        completed_at: None,
                        total_records: records.len() as u64,
                        processed_records: 0,
                        failed_records: 0,
                        progress_percentage: 0.0,
                        error_message: None,
                        output_location: None,
                        metadata: {
                            let mut metadata = std::collections::HashMap::new();
                            metadata.insert("aggregation_type".to_string(), agg_type.to_string());
                            metadata
                        },
                    };

                    black_box(processor.execute_job(job, records).await.unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_data_transformation_pipeline(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("data_transformation");

    for pipeline_complexity in ["simple", "medium", "complex"].iter() {
        group.bench_with_input(
            BenchmarkId::new("transformation_pipeline", pipeline_complexity),
            pipeline_complexity,
            |b, &complexity| {
                b.to_async(&rt).iter(|| async {
                    let config = BatchProcessorConfig {
                        max_concurrent_jobs: 6,
                        job_timeout: Duration::from_secs(120),
                        checkpoint_interval: Duration::from_secs(15),
                        retry_attempts: 3,
                        processing_mode: ProcessingMode::Batch,
                        output_format: "json".to_string(),
                    };

                    let processor = BatchProcessor::new(config).await.unwrap();

                    let record_count = match complexity {
                        "simple" => 10000,
                        "medium" => 25000,
                        "complex" => 50000,
                        _ => 10000,
                    };

                    let records: Vec<DataRecord> = (0..record_count)
                        .map(|i| {
                            let mut record = create_analytics_record(i, "transformation");
                            // Add complexity based on type
                            match complexity {
                                "medium" => {
                                    record.data["nested"] = json!({
                                        "level1": {
                                            "level2": {
                                                "data": format!("nested_data_{}", i)
                                            }
                                        }
                                    });
                                }
                                "complex" => {
                                    record.data["array_data"] = json!((0..10)
                                        .map(|j| {
                                            json!({
                                                "id": j,
                                                "value": rand::random::<f64>(),
                                                "metadata": format!("meta_{}_{}", i, j)
                                            })
                                        })
                                        .collect::<Vec<_>>());
                                }
                                _ => {}
                            }
                            record
                        })
                        .collect();

                    let job = BatchJob {
                        id: Uuid::new_v4(),
                        name: format!("transform_benchmark_{}", complexity),
                        status: BatchJobStatus::Pending,
                        created_at: chrono::Utc::now(),
                        started_at: None,
                        completed_at: None,
                        total_records: records.len() as u64,
                        processed_records: 0,
                        failed_records: 0,
                        progress_percentage: 0.0,
                        error_message: None,
                        output_location: None,
                        metadata: {
                            let mut metadata = std::collections::HashMap::new();
                            metadata.insert("complexity".to_string(), complexity.to_string());
                            metadata
                        },
                    };

                    black_box(processor.execute_job(job, records).await.unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_concurrent_batch_jobs(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_batch_jobs");
    group.sample_size(5); // Lower sample size for concurrent benchmarks

    for job_count in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_jobs", job_count),
            job_count,
            |b, &num_jobs| {
                b.to_async(&rt).iter(|| async {
                    let config = BatchProcessorConfig {
                        max_concurrent_jobs: num_jobs,
                        job_timeout: Duration::from_secs(180),
                        checkpoint_interval: Duration::from_secs(20),
                        retry_attempts: 2,
                        processing_mode: ProcessingMode::Batch,
                        output_format: "parquet".to_string(),
                    };

                    let processor = BatchProcessor::new(config).await.unwrap();

                    let mut job_handles = Vec::new();

                    for job_id in 0..num_jobs {
                        let processor_clone = processor.clone();
                        let handle = tokio::spawn(async move {
                            let records: Vec<DataRecord> = (0..10000)
                                .map(|i| create_analytics_record(i + job_id * 10000, "concurrent"))
                                .collect();

                            let job = BatchJob {
                                id: Uuid::new_v4(),
                                name: format!("concurrent_job_{}", job_id),
                                status: BatchJobStatus::Pending,
                                created_at: chrono::Utc::now(),
                                started_at: None,
                                completed_at: None,
                                total_records: records.len() as u64,
                                processed_records: 0,
                                failed_records: 0,
                                progress_percentage: 0.0,
                                error_message: None,
                                output_location: None,
                                metadata: {
                                    let mut metadata = std::collections::HashMap::new();
                                    metadata.insert("job_id".to_string(), job_id.to_string());
                                    metadata
                                },
                            };

                            processor_clone.execute_job(job, records).await.unwrap()
                        });
                        job_handles.push(handle);
                    }

                    // Wait for all jobs to complete
                    for handle in job_handles {
                        black_box(handle.await.unwrap());
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_large_dataset_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("large_dataset_processing");
    group.sample_size(3); // Very low sample size for large datasets
    group.measurement_time(Duration::from_secs(30)); // Longer measurement time

    for dataset_size in [100000, 500000, 1000000].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_dataset", dataset_size),
            dataset_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let config = BatchProcessorConfig {
                        max_concurrent_jobs: 8,
                        job_timeout: Duration::from_secs(600),
                        checkpoint_interval: Duration::from_secs(60),
                        retry_attempts: 1,
                        processing_mode: ProcessingMode::Batch,
                        output_format: "parquet".to_string(),
                    };

                    let processor = BatchProcessor::new(config).await.unwrap();

                    // Generate large dataset in chunks to manage memory
                    let chunk_size = 10000;
                    let mut all_records = Vec::new();

                    for chunk in 0..(size / chunk_size) {
                        let chunk_records: Vec<DataRecord> = (0..chunk_size)
                            .map(|i| {
                                create_analytics_record(i + chunk * chunk_size, "large_dataset")
                            })
                            .collect();
                        all_records.extend(chunk_records);
                    }

                    let job = BatchJob {
                        id: Uuid::new_v4(),
                        name: format!("large_dataset_job_{}", size),
                        status: BatchJobStatus::Pending,
                        created_at: chrono::Utc::now(),
                        started_at: None,
                        completed_at: None,
                        total_records: all_records.len() as u64,
                        processed_records: 0,
                        failed_records: 0,
                        progress_percentage: 0.0,
                        error_message: None,
                        output_location: None,
                        metadata: {
                            let mut metadata = std::collections::HashMap::new();
                            metadata.insert("dataset_size".to_string(), size.to_string());
                            metadata
                        },
                    };

                    black_box(processor.execute_job(job, all_records).await.unwrap());
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_batch_job_creation,
    bench_aggregation_queries,
    bench_data_transformation_pipeline,
    bench_concurrent_batch_jobs,
    bench_large_dataset_processing
);
criterion_main!(benches);
