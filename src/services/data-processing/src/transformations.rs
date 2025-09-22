//! Data transformations module for the Data Processing Service
//!
//! This module provides comprehensive data transformation capabilities including:
//! - Field mapping and data type conversions
//! - Data validation and cleansing
//! - Schema evolution and migration
//! - Custom transformation pipelines
//! - Format conversion (JSON, CSV, Avro, etc.)

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{DataProcessingError, Result, TransformationError},
    types::{DataRecord, ErrorSeverity, ProcessingError, ProcessingWarning},
};

/// Transformation engine for processing data records
pub struct TransformationEngine {
    transformations: Vec<Box<dyn DataTransformation + Send + Sync>>,
    validation_rules: Vec<Box<dyn ValidationRule + Send + Sync>>,
}

/// Data transformation trait
pub trait DataTransformation {
    /// Transform a data record
    fn transform(&self, record: &mut DataRecord) -> Result<TransformationResult>;

    /// Get transformation name
    fn name(&self) -> &str;

    /// Check if transformation is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Validation rule trait
pub trait ValidationRule {
    /// Validate a data record
    fn validate(&self, record: &DataRecord) -> ValidationResult;

    /// Get rule name
    fn name(&self) -> &str;

    /// Get rule severity
    fn severity(&self) -> ErrorSeverity {
        ErrorSeverity::Medium
    }
}

/// Transformation result
#[derive(Debug, Clone)]
pub struct TransformationResult {
    pub success: bool,
    pub modified_fields: Vec<String>,
    pub warnings: Vec<ProcessingWarning>,
    pub errors: Vec<ProcessingError>,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ProcessingError>,
    pub warnings: Vec<ProcessingWarning>,
}

/// Field mapping transformation
pub struct FieldMappingTransformation {
    name: String,
    mappings: HashMap<String, String>,
    remove_unmapped: bool,
}

/// Type conversion transformation
pub struct TypeConversionTransformation {
    name: String,
    conversions: HashMap<String, TargetType>,
}

/// Data cleansing transformation
pub struct DataCleansingTransformation {
    name: String,
    rules: Vec<CleansingRule>,
}

/// Schema evolution transformation
pub struct SchemaEvolutionTransformation {
    name: String,
    from_version: String,
    to_version: String,
    evolution_rules: Vec<EvolutionRule>,
}

/// Format conversion transformation
pub struct FormatConversionTransformation {
    name: String,
    source_format: DataFormat,
    target_format: DataFormat,
}

/// Required field validation
pub struct RequiredFieldValidation {
    name: String,
    required_fields: Vec<String>,
}

/// Data range validation
pub struct DataRangeValidation {
    name: String,
    field_ranges: HashMap<String, (f64, f64)>,
}

/// Pattern validation
pub struct PatternValidation {
    name: String,
    field_patterns: HashMap<String, Regex>,
}

/// Target data types for conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Json,
    Array,
}

/// Data cleansing rules
#[derive(Debug, Clone)]
pub enum CleansingRule {
    TrimWhitespace(String),
    RemoveNulls(String),
    ReplaceValue {
        field: String,
        from: String,
        to: String,
    },
    NormalizeCase {
        field: String,
        case_type: CaseType,
    },
    RemoveSpecialChars(String),
}

/// Case normalization types
#[derive(Debug, Clone)]
pub enum CaseType {
    Upper,
    Lower,
    Title,
    Camel,
    Snake,
}

/// Schema evolution rules
#[derive(Debug, Clone)]
pub enum EvolutionRule {
    AddField {
        name: String,
        default_value: Value,
    },
    RemoveField(String),
    RenameField {
        from: String,
        to: String,
    },
    ChangeType {
        field: String,
        new_type: TargetType,
    },
    MergeFields {
        sources: Vec<String>,
        target: String,
    },
    SplitField {
        source: String,
        targets: Vec<String>,
    },
}

/// Data format types
#[derive(Debug, Clone, PartialEq)]
pub enum DataFormat {
    Json,
    Csv,
    Xml,
    Avro,
    Parquet,
    MessagePack,
}

impl TransformationEngine {
    /// Create a new transformation engine
    pub fn new() -> Self {
        Self {
            transformations: Vec::new(),
            validation_rules: Vec::new(),
        }
    }

    /// Add a transformation to the engine
    pub fn add_transformation(
        &mut self,
        transformation: Box<dyn DataTransformation + Send + Sync>,
    ) {
        self.transformations.push(transformation);
    }

    /// Add a validation rule to the engine
    pub fn add_validation_rule(&mut self, rule: Box<dyn ValidationRule + Send + Sync>) {
        self.validation_rules.push(rule);
    }

    /// Transform and validate a data record
    pub fn process_record(
        &self,
        mut record: DataRecord,
    ) -> Result<(DataRecord, Vec<ProcessingWarning>, Vec<ProcessingError>)> {
        let mut all_warnings = Vec::new();
        let mut all_errors = Vec::new();

        // Apply transformations
        for transformation in &self.transformations {
            if !transformation.is_enabled() {
                continue;
            }

            match transformation.transform(&mut record) {
                Ok(result) => {
                    all_warnings.extend(result.warnings);
                    all_errors.extend(result.errors);
                }
                Err(e) => {
                    all_errors.push(ProcessingError {
                        code: "TRANSFORMATION_ERROR".to_string(),
                        message: e.to_string(),
                        field: Some(transformation.name().to_string()),
                        severity: ErrorSeverity::High,
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // Apply validations
        for rule in &self.validation_rules {
            let validation_result = rule.validate(&record);
            all_warnings.extend(validation_result.warnings);
            all_errors.extend(validation_result.errors);
        }

        Ok((record, all_warnings, all_errors))
    }
}

impl FieldMappingTransformation {
    pub fn new(name: String, mappings: HashMap<String, String>, remove_unmapped: bool) -> Self {
        Self {
            name,
            mappings,
            remove_unmapped,
        }
    }
}

impl DataTransformation for FieldMappingTransformation {
    fn transform(&self, record: &mut DataRecord) -> Result<TransformationResult> {
        let mut modified_fields = Vec::new();
        let mut warnings = Vec::new();

        if let Value::Object(ref mut obj) = record.data {
            let mut new_obj = serde_json::Map::new();

            for (old_key, new_key) in &self.mappings {
                if let Some(value) = obj.remove(old_key) {
                    new_obj.insert(new_key.clone(), value);
                    modified_fields.push(new_key.clone());
                }
            }

            // Keep unmapped fields if configured
            if !self.remove_unmapped {
                for (key, value) in obj.iter() {
                    new_obj.insert(key.clone(), value.clone());
                }
            } else {
                for key in obj.keys() {
                    warnings.push(ProcessingWarning {
                        code: "UNMAPPED_FIELD".to_string(),
                        message: format!("Field '{}' was removed (unmapped)", key),
                        field: Some(key.clone()),
                        timestamp: Utc::now(),
                    });
                }
            }

            record.data = Value::Object(new_obj);
        }

        Ok(TransformationResult {
            success: true,
            modified_fields,
            warnings,
            errors: Vec::new(),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TypeConversionTransformation {
    pub fn new(name: String, conversions: HashMap<String, TargetType>) -> Self {
        Self { name, conversions }
    }

    fn convert_value(&self, value: &Value, target_type: &TargetType) -> Result<Value> {
        match target_type {
            TargetType::String => Ok(Value::String(value.to_string())),
            TargetType::Integer => match value {
                Value::Number(n) if n.is_i64() => Ok(value.clone()),
                Value::Number(n) => Ok(Value::Number(serde_json::Number::from(
                    n.as_f64().unwrap() as i64,
                ))),
                Value::String(s) => match s.parse::<i64>() {
                    Ok(i) => Ok(Value::Number(serde_json::Number::from(i))),
                    Err(_) => Err(TransformationError::TypeMismatch {
                        expected: "Integer".to_string(),
                        actual: "String".to_string(),
                    }
                    .into()),
                },
                _ => Err(TransformationError::TypeMismatch {
                    expected: "Integer".to_string(),
                    actual: format!("{:?}", value),
                }
                .into()),
            },
            TargetType::Float => match value {
                Value::Number(n) => Ok(Value::Number(
                    serde_json::Number::from_f64(n.as_f64().unwrap()).unwrap(),
                )),
                Value::String(s) => match s.parse::<f64>() {
                    Ok(f) => Ok(Value::Number(serde_json::Number::from_f64(f).unwrap())),
                    Err(_) => Err(TransformationError::TypeMismatch {
                        expected: "Float".to_string(),
                        actual: "String".to_string(),
                    }
                    .into()),
                },
                _ => Err(TransformationError::TypeMismatch {
                    expected: "Float".to_string(),
                    actual: format!("{:?}", value),
                }
                .into()),
            },
            TargetType::Boolean => match value {
                Value::Bool(_) => Ok(value.clone()),
                Value::String(s) => match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => Ok(Value::Bool(true)),
                    "false" | "0" | "no" | "off" => Ok(Value::Bool(false)),
                    _ => Err(TransformationError::TypeMismatch {
                        expected: "Boolean".to_string(),
                        actual: "String".to_string(),
                    }
                    .into()),
                },
                Value::Number(n) => Ok(Value::Bool(n.as_f64().unwrap() != 0.0)),
                _ => Err(TransformationError::TypeMismatch {
                    expected: "Boolean".to_string(),
                    actual: format!("{:?}", value),
                }
                .into()),
            },
            TargetType::Json => Ok(value.clone()),
            TargetType::Array => match value {
                Value::Array(_) => Ok(value.clone()),
                _ => Ok(Value::Array(vec![value.clone()])),
            },
            TargetType::DateTime => {
                match value {
                    Value::String(s) => {
                        // Try to parse various datetime formats
                        if let Ok(_) = chrono::DateTime::parse_from_rfc3339(s) {
                            Ok(value.clone())
                        } else {
                            Err(TransformationError::TypeMismatch {
                                expected: "DateTime".to_string(),
                                actual: "String".to_string(),
                            }
                            .into())
                        }
                    }
                    _ => Err(TransformationError::TypeMismatch {
                        expected: "DateTime".to_string(),
                        actual: format!("{:?}", value),
                    }
                    .into()),
                }
            }
        }
    }
}

impl DataTransformation for TypeConversionTransformation {
    fn transform(&self, record: &mut DataRecord) -> Result<TransformationResult> {
        let mut modified_fields = Vec::new();
        let mut errors = Vec::new();

        if let Value::Object(ref mut obj) = record.data {
            for (field, target_type) in &self.conversions {
                if let Some(value) = obj.get(field) {
                    match self.convert_value(value, target_type) {
                        Ok(converted_value) => {
                            obj.insert(field.clone(), converted_value);
                            modified_fields.push(field.clone());
                        }
                        Err(e) => {
                            errors.push(ProcessingError {
                                code: "TYPE_CONVERSION_ERROR".to_string(),
                                message: e.to_string(),
                                field: Some(field.clone()),
                                severity: ErrorSeverity::Medium,
                                timestamp: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        Ok(TransformationResult {
            success: errors.is_empty(),
            modified_fields,
            warnings: Vec::new(),
            errors,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl RequiredFieldValidation {
    pub fn new(name: String, required_fields: Vec<String>) -> Self {
        Self {
            name,
            required_fields,
        }
    }
}

impl ValidationRule for RequiredFieldValidation {
    fn validate(&self, record: &DataRecord) -> ValidationResult {
        let mut errors = Vec::new();

        if let Value::Object(obj) = &record.data {
            for field in &self.required_fields {
                if !obj.contains_key(field) {
                    errors.push(ProcessingError {
                        code: "MISSING_REQUIRED_FIELD".to_string(),
                        message: format!("Required field '{}' is missing", field),
                        field: Some(field.clone()),
                        severity: ErrorSeverity::High,
                        timestamp: Utc::now(),
                    });
                } else if obj.get(field).unwrap().is_null() {
                    errors.push(ProcessingError {
                        code: "NULL_REQUIRED_FIELD".to_string(),
                        message: format!("Required field '{}' is null", field),
                        field: Some(field.clone()),
                        severity: ErrorSeverity::High,
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn severity(&self) -> ErrorSeverity {
        ErrorSeverity::High
    }
}

impl Default for TransformationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_field_mapping_transformation() {
        let mut mappings = HashMap::new();
        mappings.insert("old_name".to_string(), "new_name".to_string());

        let transformation =
            FieldMappingTransformation::new("field_mapping".to_string(), mappings, false);

        let mut record = DataRecord::default();
        record.data = json!({
            "old_name": "test_value",
            "other_field": "other_value"
        });

        let result = transformation.transform(&mut record).unwrap();

        assert!(result.success);
        assert_eq!(result.modified_fields, vec!["new_name"]);
        assert!(record.data.get("new_name").is_some());
        assert!(record.data.get("old_name").is_none());
    }

    #[test]
    fn test_type_conversion_transformation() {
        let mut conversions = HashMap::new();
        conversions.insert("numeric_string".to_string(), TargetType::Integer);

        let transformation =
            TypeConversionTransformation::new("type_conversion".to_string(), conversions);

        let mut record = DataRecord::default();
        record.data = json!({
            "numeric_string": "123"
        });

        let result = transformation.transform(&mut record).unwrap();

        assert!(result.success);
        assert!(record.data.get("numeric_string").unwrap().is_number());
    }

    #[test]
    fn test_required_field_validation() {
        let validation = RequiredFieldValidation::new(
            "required_fields".to_string(),
            vec!["id".to_string(), "name".to_string()],
        );

        let mut record = DataRecord::default();
        record.data = json!({
            "id": "123"
            // missing "name" field
        });

        let result = validation.validate(&record);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn test_transformation_engine() {
        let mut engine = TransformationEngine::new();

        let mut mappings = HashMap::new();
        mappings.insert("old_field".to_string(), "new_field".to_string());

        engine.add_transformation(Box::new(FieldMappingTransformation::new(
            "mapping".to_string(),
            mappings,
            false,
        )));

        engine.add_validation_rule(Box::new(RequiredFieldValidation::new(
            "validation".to_string(),
            vec!["new_field".to_string()],
        )));

        let mut record = DataRecord::default();
        record.data = json!({
            "old_field": "test_value"
        });

        let (processed_record, warnings, errors) = engine.process_record(record).unwrap();

        assert!(errors.is_empty());
        assert!(processed_record.data.get("new_field").is_some());
    }
}
