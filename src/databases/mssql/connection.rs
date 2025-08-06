use crate::core::error::{QueryError, Result};
use crate::core::types::{QueryParams, DatabaseValue};
use crate::databases::traits::{DatabaseConnection, DatabaseRow};
use crate::databases::mssql::types::MSSQLRow;
use tiberius::{Client, Row, QueryItem};
use tokio::net::TcpStream;
use tokio_util::compat::Compat;
use futures::TryStreamExt;

/// MSSQL数据库连接
pub struct MSSQLConnection {
    client: Option<Client<Compat<TcpStream>>>,
    in_transaction: bool,
}

impl MSSQLConnection {
    /// 创建新的MSSQL连接
    pub fn new(client: Client<Compat<TcpStream>>) -> Self {
        Self {
            client: Some(client),
            in_transaction: false,
        }
    }

    /// 转换查询参数
    fn convert_params(params: Option<QueryParams>) -> Vec<(String, DatabaseValue)> {
        match params {
            Some(param_map) => {
                param_map
                    .into_iter()
                    .collect()
            }
            None => Vec::new(),
        }
    }

    /// 将DatabaseValue转换为tiberius支持的类型
    /// 返回一个装箱的trait对象，避免临时值问题
    fn database_value_to_tiberius_param(value: DatabaseValue) -> Box<dyn tiberius::ToSql + Send + Sync> {
        match value {
            DatabaseValue::Null => Box::new(Option::<i32>::None),
            DatabaseValue::Bool(b) => Box::new(b),
            DatabaseValue::I32(i) => Box::new(i),
            DatabaseValue::I64(i) => Box::new(i),
            DatabaseValue::F32(f) => Box::new(f),
            DatabaseValue::F64(f) => Box::new(f),
            DatabaseValue::String(s) => Box::new(s),
            DatabaseValue::Bytes(b) => Box::new(b),
            DatabaseValue::DateTime(dt) => Box::new(dt.timestamp()),
            DatabaseValue::Uuid(u) => Box::new(u),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseConnection for MSSQLConnection {
    type Row = MSSQLRow;

    async fn query(&mut self, sql: &str, params: Option<QueryParams>) -> Result<Vec<Self::Row>> {
        let client = self.client.as_mut().ok_or_else(|| QueryError::ExecutionFailed("Connection closed".to_string()))?;
        
        let mut query = client
            .query(sql, &[])
            .await
            .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;

        let mut rows = Vec::new();

        while let Some(item) = query.try_next().await
            .map_err(|e| QueryError::ResultProcessing(e.to_string()))? 
        {
            match item {
                QueryItem::Row(row) => {
                    rows.push(MSSQLRow::new(row));
                }
                QueryItem::Metadata(_) => {
                    // 忽略元数据
                }
            }
        }

        Ok(rows)
    }

    async fn execute(&mut self, sql: &str, params: Option<QueryParams>) -> Result<u64> {
        let client = self.client.as_mut().ok_or_else(|| QueryError::ExecutionFailed("Connection closed".to_string()))?;
        
        let result = client
            .execute(sql, &[])
            .await
            .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;

        Ok(result.rows_affected().len() as u64)
    }

    async fn begin_transaction(&mut self) -> Result<()> {
        if self.in_transaction {
            return Err(QueryError::ExecutionFailed("已在事务中".to_string()).into());
        }

        let client = self.client.as_mut().ok_or_else(|| QueryError::ExecutionFailed("Connection closed".to_string()))?;
        
        client
            .simple_query("BEGIN TRANSACTION")
            .await
            .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;

        self.in_transaction = true;
        Ok(())
    }

    async fn commit_transaction(&mut self) -> Result<()> {
        if !self.in_transaction {
            return Err(QueryError::ExecutionFailed("不在事务中".to_string()).into());
        }

        let client = self.client.as_mut().ok_or_else(|| QueryError::ExecutionFailed("Connection closed".to_string()))?;
        
        client
            .simple_query("COMMIT TRANSACTION")
            .await
            .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;

        self.in_transaction = false;
        Ok(())
    }

    async fn rollback_transaction(&mut self) -> Result<()> {
        if !self.in_transaction {
            return Err(QueryError::ExecutionFailed("不在事务中".to_string()).into());
        }

        let client = self.client.as_mut().ok_or_else(|| QueryError::ExecutionFailed("Connection closed".to_string()))?;
        
        client
            .simple_query("ROLLBACK TRANSACTION")
            .await
            .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;

        self.in_transaction = false;
        Ok(())
    }

    async fn is_valid(&mut self) -> bool {
        if let Some(client) = self.client.as_mut() {
            match client.simple_query("SELECT 1").await {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    async fn close(&mut self) -> Result<()> {
        if self.in_transaction {
            let _ = self.rollback_transaction().await;
        }

        if let Some(client) = self.client.take() {
            client
                .close()
                .await
                .map_err(|e| QueryError::ExecutionFailed(e.to_string()))?;
        }

        Ok(())
    }
}