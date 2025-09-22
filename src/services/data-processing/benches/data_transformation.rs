use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use data_processing_service::{
    transformations::{TransformationEngine, TransformationRule, TransformationType},
    types::{DataRecord, ProcessingMode},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

fn create_transformation_record(id: usize, data_type: &str) -> DataRecord {
    let base_data = match data_type {
        "simple" => json!({
            "id": id,
            "name": format!("item_{}", id),
            "value": id as f64 * 1.5,
            "category": format!("cat_{}", id % 5)
        }),
        "nested" => json!({
            "id": id,
            "user": {
                "name": format!("user_{}", id),
                "profile": {
                    "age": id % 100,
                    "preferences": {
                        "theme": "dark",
                        "language": "en"
                    }
                }
            },
            "metadata": {
                "created_at": chrono::Utc::now().to_rfc3339(),
                "tags": ["tag1", "tag2", "tag3"]
            }
        }),
        "array" => json!({
            "id": id,
            "items": (0..10).map(|i| json!({
                "item_id": i,
                "name": format!("item_{}_{}", id, i),
                "value": rand::random::<f64>() * 100.0
            })).collect::<Vec<_>>(),
            "categories": (0..5).map(|i| format!("category_{}", i)).collect::<Vec<_>>()
        }),
        "large" => {
            let large_text = "Lorem ipsum ".repeat(100);
            json!({
                "id": id,
                "content": large_text,
                "binary_data": (0..1000).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
                "repeated_data": (0..50).map(|i| json!({
                    "index": i,
                    "data": format!("data_{}_{}", id, i)
                })).collect::<Vec<_>>()
            })
        }
        _ => json!({"id": id, "default": true}),
    };

    DataRecord {
        id: Uuid::new_v4(),
        source: format!("transformation_source_{}", id % 3),
        timestamp: chrono::Utc::now(),
        data: base_data,
        content_type: "application/json".to_string(),
        size_bytes: match data_type {
            "simple" => 128,
            "nested" => 512,
            "array" => 1024,
            "large" => 10240,
            _ => 64,
        },
        metadata: HashMap::new(),
    }
}

fn bench_field_extraction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("field_extraction");

    for data_complexity in ["simple", "nested", "array"].iter() {
        group.bench_with_input(
            BenchmarkId::new("extract_fields", data_complexity),
            data_complexity,
            |b, &complexity| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    let extraction_rules = match complexity {
                        "simple" => vec![TransformationRule {
                            id: Uuid::new_v4(),
                            name: "extract_id".to_string(),
                            transformation_type: TransformationType::FieldExtraction,
                            source_field: "id".to_string(),
                            target_field: "extracted_id".to_string(),
                            parameters: HashMap::new(),
                            condition: None,
                        }],
                        "nested" => vec![TransformationRule {
                            id: Uuid::new_v4(),
                            name: "extract_nested".to_string(),
                            transformation_type: TransformationType::FieldExtraction,
                            source_field: "user.profile.age".to_string(),
                            target_field: "user_age".to_string(),
                            parameters: HashMap::new(),
                            condition: None,
                        }],
                        "array" => vec![TransformationRule {
                            id: Uuid::new_v4(),
                            name: "extract_array_length".to_string(),
                            transformation_type: TransformationType::FieldExtraction,
                            source_field: "items.length".to_string(),
                            target_field: "item_count".to_string(),
                            parameters: HashMap::new(),
                            condition: None,
                        }],
                        _ => vec![],
                    };

                    let records: Vec<DataRecord> = (0..1000)
                        .map(|i| create_transformation_record(i, complexity))
                        .collect();

                    for rule in extraction_rules {
                        for record in &records {
                            black_box(
                                engine
                                    .apply_transformation(record.clone(), &rule)
                                    .await
                                    .unwrap(),
                            );
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_data_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("data_validation");

    for validation_type in ["type_check", "range_check", "pattern_match", "custom_rule"].iter() {
        group.bench_with_input(
            BenchmarkId::new("validation", validation_type),
            validation_type,
            |b, &val_type| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    let validation_rule = match val_type {
                        "type_check" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "type_validation".to_string(),
                            transformation_type: TransformationType::Validation,
                            source_field: "id".to_string(),
                            target_field: "".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("type".to_string(), "number".to_string());
                                params
                            },
                            condition: None,
                        },
                        "range_check" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "range_validation".to_string(),
                            transformation_type: TransformationType::Validation,
                            source_field: "value".to_string(),
                            target_field: "".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("min".to_string(), "0".to_string());
                                params.insert("max".to_string(), "10000".to_string());
                                params
                            },
                            condition: None,
                        },
                        "pattern_match" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "pattern_validation".to_string(),
                            transformation_type: TransformationType::Validation,
                            source_field: "name".to_string(),
                            target_field: "".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("pattern".to_string(), r"^item_\d+$".to_string());
                                params
                            },
                            condition: None,
                        },
                        "custom_rule" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "custom_validation".to_string(),
                            transformation_type: TransformationType::Validation,
                            source_field: "category".to_string(),
                            target_field: "".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert(
                                    "allowed_values".to_string(),
                                    "cat_0,cat_1,cat_2,cat_3,cat_4".to_string(),
                                );
                                params
                            },
                            condition: None,
                        },
                        _ => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "default_validation".to_string(),
                            transformation_type: TransformationType::Validation,
                            source_field: "id".to_string(),
                            target_field: "".to_string(),
                            parameters: HashMap::new(),
                            condition: None,
                        },
                    };

                    let records: Vec<DataRecord> = (0..5000)
                        .map(|i| create_transformation_record(i, "simple"))
                        .collect();

                    for record in records {
                        black_box(
                            engine
                                .apply_transformation(record, &validation_rule)
                                .await
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_data_enrichment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("data_enrichment");

    for enrichment_type in ["lookup", "computation", "aggregation", "external_api"].iter() {
        group.bench_with_input(
            BenchmarkId::new("enrichment", enrichment_type),
            enrichment_type,
            |b, &enrich_type| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    let enrichment_rule = match enrich_type {
                        "lookup" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "lookup_enrichment".to_string(),
                            transformation_type: TransformationType::Enrichment,
                            source_field: "category".to_string(),
                            target_field: "category_name".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("lookup_table".to_string(), "categories".to_string());
                                params
                                    .insert("lookup_key".to_string(), "category_code".to_string());
                                params.insert(
                                    "lookup_value".to_string(),
                                    "category_name".to_string(),
                                );
                                params
                            },
                            condition: None,
                        },
                        "computation" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "computed_enrichment".to_string(),
                            transformation_type: TransformationType::Enrichment,
                            source_field: "value".to_string(),
                            target_field: "value_squared".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("operation".to_string(), "square".to_string());
                                params
                            },
                            condition: None,
                        },
                        "aggregation" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "aggregation_enrichment".to_string(),
                            transformation_type: TransformationType::Enrichment,
                            source_field: "id".to_string(),
                            target_field: "group_stats".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert("group_by".to_string(), "category".to_string());
                                params.insert("operation".to_string(), "count".to_string());
                                params
                            },
                            condition: None,
                        },
                        "external_api" => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "api_enrichment".to_string(),
                            transformation_type: TransformationType::Enrichment,
                            source_field: "id".to_string(),
                            target_field: "external_data".to_string(),
                            parameters: {
                                let mut params = HashMap::new();
                                params.insert(
                                    "api_endpoint".to_string(),
                                    "http://localhost:8080/enrich".to_string(),
                                );
                                params.insert("timeout_ms".to_string(), "1000".to_string());
                                params
                            },
                            condition: None,
                        },
                        _ => TransformationRule {
                            id: Uuid::new_v4(),
                            name: "default_enrichment".to_string(),
                            transformation_type: TransformationType::Enrichment,
                            source_field: "id".to_string(),
                            target_field: "enriched".to_string(),
                            parameters: HashMap::new(),
                            condition: None,
                        },
                    };

                    let records: Vec<DataRecord> = (0..2000)
                        .map(|i| create_transformation_record(i, "simple"))
                        .collect();

                    for record in records {
                        black_box(
                            engine
                                .apply_transformation(record, &enrichment_rule)
                                .await
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_format_conversion(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("format_conversion");

    for format_pair in [
        ("json", "avro"),
        ("json", "parquet"),
        ("csv", "json"),
        ("xml", "json"),
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::new("conversion", format!("{}_{}", format_pair.0, format_pair.1)),
            format_pair,
            |b, &(from_format, to_format)| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    let conversion_rule = TransformationRule {
                        id: Uuid::new_v4(),
                        name: format!("convert_{}_to_{}", from_format, to_format),
                        transformation_type: TransformationType::FormatConversion,
                        source_field: "".to_string(),
                        target_field: "".to_string(),
                        parameters: {
                            let mut params = HashMap::new();
                            params.insert("from_format".to_string(), from_format.to_string());
                            params.insert("to_format".to_string(), to_format.to_string());
                            params
                        },
                        condition: None,
                    };

                    let records: Vec<DataRecord> = (0..1000)
                        .map(|i| create_transformation_record(i, "nested"))
                        .collect();

                    for record in records {
                        black_box(
                            engine
                                .apply_transformation(record, &conversion_rule)
                                .await
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_pipeline_transformation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("pipeline_transformation");

    for pipeline_length in [3, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("pipeline", format!("{}_steps", pipeline_length)),
            pipeline_length,
            |b, &steps| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    // Create a pipeline of transformations
                    let pipeline_rules: Vec<TransformationRule> = (0..steps)
                        .map(|i| TransformationRule {
                            id: Uuid::new_v4(),
                            name: format!("step_{}", i),
                            transformation_type: match i % 4 {
                                0 => TransformationType::FieldExtraction,
                                1 => TransformationType::Validation,
                                2 => TransformationType::Enrichment,
                                3 => TransformationType::FormatConversion,
                                _ => TransformationType::FieldExtraction,
                            },
                            source_field: match i % 4 {
                                0 => "id".to_string(),
                                1 => "value".to_string(),
                                2 => "category".to_string(),
                                3 => "".to_string(),
                                _ => "id".to_string(),
                            },
                            target_field: format!("result_{}", i),
                            parameters: {
                                let mut params = HashMap::new();
                                match i % 4 {
                                    1 => {
                                        params.insert("min".to_string(), "0".to_string());
                                        params.insert("max".to_string(), "1000000".to_string());
                                    }
                                    2 => {
                                        params.insert(
                                            "operation".to_string(),
                                            "multiply".to_string(),
                                        );
                                        params.insert("factor".to_string(), "2".to_string());
                                    }
                                    3 => {
                                        params
                                            .insert("from_format".to_string(), "json".to_string());
                                        params.insert("to_format".to_string(), "json".to_string());
                                    }
                                    _ => {}
                                }
                                params
                            },
                            condition: None,
                        })
                        .collect();

                    let records: Vec<DataRecord> = (0..1000)
                        .map(|i| create_transformation_record(i, "simple"))
                        .collect();

                    // Apply pipeline transformations
                    for record in records {
                        let mut current_record = record;
                        for rule in &pipeline_rules {
                            current_record = engine
                                .apply_transformation(current_record, rule)
                                .await
                                .unwrap();
                        }
                        black_box(current_record);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_large_data_transformation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("large_data_transformation");
    group.sample_size(10); // Lower sample size for large data
    group.measurement_time(Duration::from_secs(30));

    for data_size in ["1KB", "10KB", "100KB", "1MB"].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_data", data_size),
            data_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let engine = TransformationEngine::new().await.unwrap();

                    let transformation_rule = TransformationRule {
                        id: Uuid::new_v4(),
                        name: "large_data_transform".to_string(),
                        transformation_type: TransformationType::FieldExtraction,
                        source_field: "content".to_string(),
                        target_field: "content_length".to_string(),
                        parameters: {
                            let mut params = HashMap::new();
                            params.insert("operation".to_string(), "length".to_string());
                            params
                        },
                        condition: None,
                    };

                    let record_count = match size {
                        "1KB" => 1000,
                        "10KB" => 100,
                        "100KB" => 10,
                        "1MB" => 1,
                        _ => 100,
                    };

                    let records: Vec<DataRecord> = (0..record_count)
                        .map(|i| create_transformation_record(i, "large"))
                        .collect();

                    for record in records {
                        black_box(
                            engine
                                .apply_transformation(record, &transformation_rule)
                                .await
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_field_extraction,
    bench_data_validation,
    bench_data_enrichment,
    bench_format_conversion,
    bench_pipeline_transformation,
    bench_large_data_transformation
);
criterion_main!(benches);
