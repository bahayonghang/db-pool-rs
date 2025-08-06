use crate::core::error::{ConversionError, Result};
use crate::core::types::DatabaseValue;
use crate::databases::traits::{DatabaseRow, TypeConverter};
use polars::prelude::*;
use std::collections::HashMap;
use tiberius::{Row, ColumnData};
use uuid::Uuid;

/// MSSQL行数据
pub struct MSSQLRow {
    row: Row,
    column_names: Vec<String>,
}

impl MSSQLRow {
    pub fn new(row: Row) -> Self {
        let column_names = row
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        Self { row, column_names }
    }
}

impl DatabaseRow for MSSQLRow {
    fn column_count(&self) -> usize {
        self.row.len()
    }

    fn column_names(&self) -> Vec<String> {
        self.column_names.clone()
    }

    fn get_value(&self, index: usize) -> Option<DatabaseValue> {
        if index >= self.row.len() {
            return None;
        }

        // 使用try_get来处理可能的错误
        // 由于我们不知道确切的类型，我们需要尝试不同的类型
        
        // 尝试字符串
        if let Ok(Some(val)) = self.row.try_get::<&str, _>(index) {
            return Some(DatabaseValue::String(val.to_string()));
        }
        
        // 尝试整数
        if let Ok(Some(val)) = self.row.try_get::<i32, _>(index) {
            return Some(DatabaseValue::I32(val));
        }
        
        if let Ok(Some(val)) = self.row.try_get::<i64, _>(index) {
            return Some(DatabaseValue::I64(val));
        }
        
        // 尝试浮点数
        if let Ok(Some(val)) = self.row.try_get::<f32, _>(index) {
            return Some(DatabaseValue::F32(val));
        }
        
        if let Ok(Some(val)) = self.row.try_get::<f64, _>(index) {
            return Some(DatabaseValue::F64(val));
        }
        
        // 尝试布尔值
        if let Ok(Some(val)) = self.row.try_get::<bool, _>(index) {
            return Some(DatabaseValue::Bool(val));
        }
        
        // 尝试UUID
        if let Ok(Some(val)) = self.row.try_get::<uuid::Uuid, _>(index) {
            return Some(DatabaseValue::Uuid(val));
        }
        
        // 如果都失败了，返回None
        None
    }

    fn get_value_by_name(&self, name: &str) -> Option<DatabaseValue> {
        // 找到列的索引
        let index = self.column_names.iter().position(|n| n == name)?;
        self.get_value(index)
    }

    fn to_map(&self) -> HashMap<String, DatabaseValue> {
        let mut map = HashMap::new();
        
        for (i, name) in self.column_names.iter().enumerate() {
            if let Some(value) = self.get_value(i) {
                map.insert(name.clone(), value);
            }
        }

        map
    }
}

impl MSSQLRow {
}

/// MSSQL类型转换器
pub struct MSSQLTypeConverter;

impl TypeConverter for MSSQLTypeConverter {
    fn rows_to_dataframe<R: DatabaseRow>(rows: Vec<R>) -> Result<DataFrame> {
        if rows.is_empty() {
            return Ok(DataFrame::empty());
        }

        let column_names = rows[0].column_names();
        let column_count = column_names.len();
        
        // 为每列创建向量
        let mut columns: Vec<Vec<AnyValue>> = vec![Vec::new(); column_count];

        // 填充数据
        for row in &rows {
            for (col_idx, _) in column_names.iter().enumerate() {
                if let Some(value) = row.get_value(col_idx) {
                    // 直接移动value，避免借用
                    let any_value = Self::database_value_to_any_value(value);
                    columns[col_idx].push(any_value);
                } else {
                    columns[col_idx].push(AnyValue::Null);
                }
            }
        }

        // 创建DataFrame
        let mut df_columns = Vec::new();
        for (i, col_name) in column_names.iter().enumerate() {
            let series = Self::create_series_from_values(col_name, &columns[i])?;
            df_columns.push(series);
        }

        DataFrame::new(df_columns)
            .map_err(|e| ConversionError::DataFrameConversion(e.to_string()).into())
    }

    fn database_value_to_any_value(value: DatabaseValue) -> AnyValue<'static> {
        match value {
            DatabaseValue::Null => AnyValue::Null,
            DatabaseValue::Bool(b) => AnyValue::Boolean(b),
            DatabaseValue::I32(i) => AnyValue::Int32(i),
            DatabaseValue::I64(i) => AnyValue::Int64(i),
            DatabaseValue::F32(f) => AnyValue::Float32(f),
            DatabaseValue::F64(f) => AnyValue::Float64(f),
            DatabaseValue::String(s) => AnyValue::StringOwned(s.into()),
            DatabaseValue::Bytes(b) => AnyValue::BinaryOwned(b),
            DatabaseValue::DateTime(dt) => {
                AnyValue::Datetime(
                    dt.timestamp_millis(),
                    TimeUnit::Milliseconds,
                    &None,
                )
            }
            DatabaseValue::Uuid(u) => AnyValue::StringOwned(u.to_string().into()),
        }
    }

    fn convert_params(params: &crate::core::types::QueryParams) -> Result<Vec<(String, DatabaseValue)>> {
        Ok(params.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
}

impl MSSQLTypeConverter {
    /// 从AnyValue向量创建Series
    fn create_series_from_values(name: &str, values: &[AnyValue]) -> Result<Series> {
        if values.is_empty() {
            return Ok(Series::new_empty(name, &DataType::Null));
        }

        // 推断数据类型
        let data_type = Self::infer_data_type(values);

        match data_type {
            DataType::Boolean => {
                let bool_values: Vec<Option<bool>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Boolean(b) => Some(*b),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, bool_values))
            }
            DataType::Int32 => {
                let int_values: Vec<Option<i32>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Int32(i) => Some(*i),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, int_values))
            }
            DataType::Int64 => {
                let int_values: Vec<Option<i64>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Int64(i) => Some(*i),
                        AnyValue::Int32(i) => Some(*i as i64),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, int_values))
            }
            DataType::Float32 => {
                let float_values: Vec<Option<f32>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Float32(f) => Some(*f),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, float_values))
            }
            DataType::Float64 => {
                let float_values: Vec<Option<f64>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Float64(f) => Some(*f),
                        AnyValue::Float32(f) => Some(*f as f64),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, float_values))
            }
            DataType::String => {
                let string_values: Vec<Option<String>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::String(s) => Some(s.to_string()),
                        AnyValue::StringOwned(s) => Some(s.to_string()),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, string_values))
            }
            DataType::Datetime(TimeUnit::Milliseconds, _) => {
                let datetime_values: Vec<Option<i64>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Datetime(dt, _, _) => Some(*dt),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, datetime_values).cast(&DataType::Datetime(TimeUnit::Milliseconds, None))
                    .map_err(|e| ConversionError::TypeConversion(e.to_string()))?)
            }
            _ => {
                // 默认转换为字符串
                let string_values: Vec<Option<String>> = values
                    .iter()
                    .map(|v| match v {
                        AnyValue::Null => None,
                        _ => Some(format!("{:?}", v)),
                    })
                    .collect();
                Ok(Series::new(name, string_values))
            }
        }
    }

    /// 推断数据类型
    fn infer_data_type(values: &[AnyValue]) -> DataType {
        for value in values {
            match value {
                AnyValue::Boolean(_) => return DataType::Boolean,
                AnyValue::Int32(_) => return DataType::Int32,
                AnyValue::Int64(_) => return DataType::Int64,
                AnyValue::Float32(_) => return DataType::Float32,
                AnyValue::Float64(_) => return DataType::Float64,
                AnyValue::String(_) | AnyValue::StringOwned(_) => return DataType::String,
                AnyValue::Datetime(_, time_unit, _) => {
                    return DataType::Datetime(*time_unit, None);
                }
                AnyValue::Binary(_) | AnyValue::BinaryOwned(_) => return DataType::Binary,
                AnyValue::Null => continue,
                _ => return DataType::String,
            }
        }
        DataType::Null
    }
}